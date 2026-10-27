//! Shared pieces of the B17 public-trades tools (see docs/B17_PUBLIC_TRADES_PROTOCOL.md):
//! the nine-column table, the strict CSV reader, the canonical digest, the durable
//! sync procedure and the process counters. Included by the examples with `#[path]`.
#![allow(dead_code)]

use anyhow::{Context, Result, bail, ensure};
use arrow_array::{
    Array, ArrayRef, BooleanArray, Float64Array, Int64Array, LargeStringArray, RecordBatch,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use arrow_select::filter::filter_record_batch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
    sync::Arc,
};

pub const BLOCK_ROWS: usize = 32_768;
/// Sixty seconds in microseconds (the unit of Binance spot timestamps from 2025).
pub const WINDOW_US: i64 = 60_000_000;
pub const SYMBOL: &str = "PAXGUSDT";
pub const SCALE: i64 = 100_000_000;
pub const SOURCE_FILE: &str = "trades.parquet";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    V1,
    V2,
}

impl Variant {
    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "v1" => Ok(Self::V1),
            "v2" => Ok(Self::V2),
            other => bail!("unknown variant {other}; use v1 or v2"),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::V1 => "v1",
            Self::V2 => "v2",
        }
    }
}

/// The nine-column table of section 2 of the protocol. Nothing is nullable.
pub fn schema(variant: Variant) -> SchemaRef {
    let (price, qty, kind) = match variant {
        Variant::V1 => ("price", "qty", DataType::Float64),
        Variant::V2 => ("price_e8", "qty_e8", DataType::Int64),
    };
    Arc::new(Schema::new_with_metadata(
        vec![
            Field::new("agg_id", DataType::Int64, false),
            Field::new(price, kind.clone(), false),
            Field::new(qty, kind, false),
            Field::new("first_id", DataType::Int64, false),
            Field::new("last_id", DataType::Int64, false),
            Field::new("transact_time", DataType::Int64, false),
            Field::new("is_buyer_maker", DataType::Boolean, false),
            Field::new("is_best_match", DataType::Boolean, false),
            Field::new("symbol", DataType::LargeUtf8, false),
        ],
        HashMap::from([("transact_time_unit".to_string(), "us".to_string())]),
    ))
}

// ----------------------------------------------------------------- CSV reader

/// Strict decimal: digits, a dot, exactly eight digits. Nothing else is accepted.
pub fn strict_decimal(text: &str) -> Result<()> {
    let (whole, fraction) = text
        .split_once('.')
        .with_context(|| format!("decimal without fraction: {text}"))?;
    ensure!(
        !whole.is_empty()
            && whole.bytes().all(|c| c.is_ascii_digit())
            && fraction.len() == 8
            && fraction.bytes().all(|c| c.is_ascii_digit()),
        "decimal is not digits.eight-digits: {text}"
    );
    Ok(())
}

fn strict_unsigned(text: &str) -> Result<i64> {
    ensure!(
        !text.is_empty() && text.bytes().all(|c| c.is_ascii_digit()),
        "not an unsigned integer: {text}"
    );
    Ok(text.parse::<i64>()?)
}

fn strict_bool(text: &str) -> Result<bool> {
    match text {
        "True" => Ok(true),
        "False" => Ok(false),
        other => bail!("not a True/False literal: {other}"),
    }
}

/// Accumulates parsed CSV rows and checks the facts the protocol relies on.
pub struct TableBuilder {
    variant: Variant,
    symbol: String,
    agg_id: Vec<i64>,
    price_f: Vec<f64>,
    qty_f: Vec<f64>,
    price_i: Vec<i64>,
    qty_i: Vec<i64>,
    first_id: Vec<i64>,
    last_id: Vec<i64>,
    time: Vec<i64>,
    maker: Vec<bool>,
    best: Vec<bool>,
}

impl TableBuilder {
    pub fn new(variant: Variant, symbol: &str) -> Self {
        Self {
            variant,
            symbol: symbol.to_string(),
            agg_id: vec![],
            price_f: vec![],
            qty_f: vec![],
            price_i: vec![],
            qty_i: vec![],
            first_id: vec![],
            last_id: vec![],
            time: vec![],
            maker: vec![],
            best: vec![],
        }
    }

    pub fn rows(&self) -> usize {
        self.agg_id.len()
    }

    /// One CSV line without its newline. Eight fields, no header, no quoting.
    pub fn push_line(&mut self, line: &str) -> Result<()> {
        ensure!(!line.contains('\r'), "carriage return in CSV line");
        let fields: Vec<&str> = line.split(',').collect();
        ensure!(
            fields.len() == 8,
            "expected 8 fields, found {}",
            fields.len()
        );
        let agg = strict_unsigned(fields[0])?;
        if let Some(&previous) = self.agg_id.last() {
            ensure!(agg > previous, "agg_id not strictly increasing at {agg}");
        }
        let time = strict_unsigned(fields[5])?;
        if let Some(&previous) = self.time.last() {
            ensure!(time >= previous, "transact_time decreases at agg_id {agg}");
        }
        strict_decimal(fields[1])?;
        strict_decimal(fields[2])?;
        match self.variant {
            Variant::V1 => {
                self.price_f.push(fields[1].parse::<f64>()?);
                self.qty_f.push(fields[2].parse::<f64>()?);
            }
            Variant::V2 => {
                self.price_i.push(gqodb_blocks::market::decimal(fields[1])?);
                self.qty_i.push(gqodb_blocks::market::decimal(fields[2])?);
            }
        }
        self.agg_id.push(agg);
        self.first_id.push(strict_unsigned(fields[3])?);
        self.last_id.push(strict_unsigned(fields[4])?);
        self.time.push(time);
        self.maker.push(strict_bool(fields[6])?);
        self.best.push(strict_bool(fields[7])?);
        Ok(())
    }

    /// Splits the accumulated rows into 32,768-row batches in input order.
    pub fn into_batches(self) -> Result<Vec<RecordBatch>> {
        let target = schema(self.variant);
        let n = self.agg_id.len();
        ensure!(n > 0, "no rows");
        let mut batches = Vec::new();
        let mut start = 0;
        while start < n {
            let end = (start + BLOCK_ROWS).min(n);
            let range = start..end;
            let (price, qty): (ArrayRef, ArrayRef) = match self.variant {
                Variant::V1 => (
                    Arc::new(Float64Array::from(self.price_f[range.clone()].to_vec())),
                    Arc::new(Float64Array::from(self.qty_f[range.clone()].to_vec())),
                ),
                Variant::V2 => (
                    Arc::new(Int64Array::from(self.price_i[range.clone()].to_vec())),
                    Arc::new(Int64Array::from(self.qty_i[range.clone()].to_vec())),
                ),
            };
            batches.push(RecordBatch::try_new(
                target.clone(),
                vec![
                    Arc::new(Int64Array::from(self.agg_id[range.clone()].to_vec())),
                    price,
                    qty,
                    Arc::new(Int64Array::from(self.first_id[range.clone()].to_vec())),
                    Arc::new(Int64Array::from(self.last_id[range.clone()].to_vec())),
                    Arc::new(Int64Array::from(self.time[range.clone()].to_vec())),
                    Arc::new(BooleanArray::from(self.maker[range.clone()].to_vec())),
                    Arc::new(BooleanArray::from(self.best[range.clone()].to_vec())),
                    Arc::new(LargeStringArray::from(vec![
                        self.symbol.as_str();
                        end - start
                    ])),
                ],
            )?);
            start = end;
        }
        Ok(batches)
    }
}

// ------------------------------------------------------------------ digest

fn type_token(data_type: &DataType) -> Result<&'static str> {
    Ok(match data_type {
        DataType::Int64 => "i64",
        DataType::Float64 => "f64",
        DataType::Boolean => "bool",
        DataType::Utf8 | DataType::LargeUtf8 => "utf8",
        other => bail!("unsupported column type {other:?}"),
    })
}

fn for_each_str(column: &ArrayRef, mut visit: impl FnMut(&[u8])) -> Result<()> {
    if let Some(a) = column.as_any().downcast_ref::<LargeStringArray>() {
        (0..a.len()).for_each(|i| visit(a.value(i).as_bytes()));
        Ok(())
    } else if let Some(a) = column.as_any().downcast_ref::<StringArray>() {
        (0..a.len()).for_each(|i| visit(a.value(i).as_bytes()));
        Ok(())
    } else {
        bail!("not a string column")
    }
}

/// Canonical digest of protocol section 3. Column-major, rows in the given order.
pub fn digest_batches(batches: &[RecordBatch]) -> Result<String> {
    ensure!(!batches.is_empty(), "digest of no batches");
    let schema = batches[0].schema();
    let rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    let mut hasher = Sha256::new();
    for (index, field) in schema.fields().iter().enumerate() {
        let token = type_token(field.data_type())?;
        hasher.update(format!("col|{}|{token}|{rows}\n", field.name()).as_bytes());
        for batch in batches {
            ensure!(
                batch.schema().field(index).data_type() == field.data_type(),
                "column type changes between batches"
            );
            ensure!(
                batch.column(index).null_count() == 0,
                "null in column {}",
                field.name()
            );
        }
        match token {
            "i64" => {
                for batch in batches {
                    let a = batch
                        .column(index)
                        .as_any()
                        .downcast_ref::<Int64Array>()
                        .unwrap();
                    let mut bytes = Vec::with_capacity(a.len() * 8);
                    a.values()
                        .iter()
                        .for_each(|v| bytes.extend_from_slice(&v.to_le_bytes()));
                    hasher.update(&bytes);
                }
            }
            "f64" => {
                for batch in batches {
                    let a = batch
                        .column(index)
                        .as_any()
                        .downcast_ref::<Float64Array>()
                        .unwrap();
                    let mut bytes = Vec::with_capacity(a.len() * 8);
                    a.values()
                        .iter()
                        .for_each(|v| bytes.extend_from_slice(&v.to_bits().to_le_bytes()));
                    hasher.update(&bytes);
                }
            }
            "bool" => {
                for batch in batches {
                    let a = batch
                        .column(index)
                        .as_any()
                        .downcast_ref::<BooleanArray>()
                        .unwrap();
                    let bytes: Vec<u8> = (0..a.len()).map(|i| u8::from(a.value(i))).collect();
                    hasher.update(&bytes);
                }
            }
            _ => {
                let mut total = 0u64;
                for batch in batches {
                    for_each_str(batch.column(index), |s| total += s.len() as u64)?;
                }
                hasher.update(total.to_le_bytes());
                for batch in batches {
                    let mut bytes = Vec::new();
                    for_each_str(batch.column(index), |s| {
                        bytes.extend_from_slice(&(s.len() as u32).to_le_bytes())
                    })?;
                    hasher.update(&bytes);
                }
                for batch in batches {
                    let mut bytes = Vec::new();
                    for_each_str(batch.column(index), |s| bytes.extend_from_slice(s))?;
                    hasher.update(&bytes);
                }
            }
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn sha256_file(path: &Path) -> Result<String> {
    gqodb_blocks::market::hash(path.to_str().context("non-UTF-8 path")?)
}

// ------------------------------------------------------------------ windows

pub fn time_column(batch: &RecordBatch) -> &Int64Array {
    batch
        .column_by_name("transact_time")
        .expect("transact_time column")
        .as_any()
        .downcast_ref::<Int64Array>()
        .expect("transact_time is Int64")
}

/// Protocol section 5: T0 is the time of the row at index floor(n/2), T1 = T0 + 60 s.
pub fn window_bounds(batches: &[RecordBatch]) -> (i64, i64) {
    let n: usize = batches.iter().map(RecordBatch::num_rows).sum();
    let mut index = n / 2;
    for batch in batches {
        if index < batch.num_rows() {
            let t0 = time_column(batch).value(index);
            return (t0, t0.saturating_add(WINDOW_US));
        }
        index -= batch.num_rows();
    }
    unreachable!("window index is inside the table")
}

/// Rows of `symbol` whose time is within the inclusive bounds, in input order.
pub fn select_window(
    batches: &[RecordBatch],
    symbol: &str,
    start: i64,
    end: i64,
) -> Result<Vec<RecordBatch>> {
    let mut out = Vec::new();
    for batch in batches {
        let t = time_column(batch);
        let mut symbols = vec![];
        for_each_str(
            batch.column_by_name("symbol").context("symbol column")?,
            |s| symbols.push(s == symbol.as_bytes()),
        )?;
        let mask = BooleanArray::from(
            (0..batch.num_rows())
                .map(|i| symbols[i] && t.value(i) >= start && t.value(i) <= end)
                .collect::<Vec<_>>(),
        );
        let selected = filter_record_batch(batch, &mask)?;
        if selected.num_rows() > 0 {
            out.push(selected);
        }
    }
    Ok(out)
}

// -------------------------------------------------------------- Parquet source

/// Reads a source/candidate Parquet file into 32,768-row batches carrying exactly the
/// protocol schema; any other column, type or nullability is an error.
pub fn read_parquet(path: &Path, variant: Variant) -> Result<Vec<RecordBatch>> {
    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(path)?)?
        .with_batch_size(BLOCK_ROWS)
        .build()?;
    let target = schema(variant);
    let mut batches = Vec::new();
    for batch in reader {
        let batch = batch?;
        batches.push(RecordBatch::try_new(
            target.clone(),
            batch.columns().to_vec(),
        )?);
    }
    ensure!(!batches.is_empty(), "empty Parquet file");
    Ok(batches)
}

// -------------------------------------------------------------- durability

fn walk(dir: &Path, files: &mut Vec<PathBuf>, dirs: &mut Vec<PathBuf>) -> std::io::Result<()> {
    dirs.push(dir.to_path_buf());
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            walk(&entry.path(), files, dirs)?;
        } else if kind.is_file() {
            files.push(entry.path());
        }
    }
    Ok(())
}

/// Flush one regular file to stable storage.
///
/// Unix: a read-only handle is enough for `fsync`. Windows: `sync_all` is
/// `FlushFileBuffers`, which needs a handle with write access, so the file is opened for
/// writing (no create, no truncate: the contents are untouched).
fn fsync_file(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    let file = fs::OpenOptions::new().write(true).open(path)?;
    #[cfg(not(windows))]
    let file = File::open(path)?;
    file.sync_all()
}

/// Flush one directory entry table to stable storage.
///
/// Unix: `fsync` on a read-only directory handle, which is what makes a newly created or
/// renamed entry durable. Windows: there is no equivalent through the standard library;
/// a directory cannot be opened as a plain file (`Access is denied`) and Windows does not
/// offer a documented way to flush a directory handle. The Windows build therefore flushes
/// the file contents only; directory entries rely on the file system's own metadata
/// journalling (NTFS), for which this function gives no guarantee. It still fails if the
/// path is not an existing directory, so a wrong path is not silently accepted.
fn fsync_dir(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let meta = fs::metadata(path)?;
        if !meta.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                "fsync_dir: not a directory",
            ));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        File::open(path)?.sync_all()
    }
}

/// Protocol section 5: fsync every regular file, then every directory bottom-up, then
/// the parent of the output root. Works for a single file as well as for a directory.
///
/// Durability differs by platform. On Linux and macOS this is the full procedure above.
/// On Windows only the file contents are flushed (see [`fsync_dir`]): the directory steps
/// check that the path exists but do not flush, so a Windows run is not comparable with a
/// Unix run on the "directories have been fsynced" part of the protocol.
pub fn fsync_tree(root: &Path) -> std::io::Result<()> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    if root.is_dir() {
        walk(root, &mut files, &mut dirs)?;
    } else {
        files.push(root.to_path_buf());
    }
    for file in &files {
        fsync_file(file)?;
    }
    dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    for dir in &dirs {
        fsync_dir(dir)?;
    }
    if let Some(parent) = root.parent() {
        let parent = if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        };
        fsync_dir(parent)?;
    }
    Ok(())
}

// ------------------------------------------------------------ process counters

/// On-CPU nanoseconds of all threads of this process (schedstat, first field).
pub fn cpu_ns() -> u64 {
    let mut total = 0;
    if let Ok(tasks) = fs::read_dir("/proc/self/task") {
        for task in tasks.flatten() {
            if let Ok(text) = fs::read_to_string(task.path().join("schedstat")) {
                total += text
                    .split_whitespace()
                    .next()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0);
            }
        }
    }
    total
}

/// Peak resident set size so far, in kB (VmHWM).
pub fn vm_hwm_kb() -> u64 {
    fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("VmHWM:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

// ------------------------------------------------------- synthetic edge table

/// Deterministic edge table: bit patterns the CSV grammar cannot express (negative
/// zero, subnormals, infinities, NaN payloads, full-range ids), other symbols, and
/// the empty and non-ASCII symbol. Used to prove that every adapter is bit-exact.
pub fn synthetic_edge(variant: Variant) -> Result<Vec<RecordBatch>> {
    let float_bits: [u64; 16] = [
        0x0000_0000_0000_0000, // +0.0
        0x8000_0000_0000_0000, // -0.0
        0x0000_0000_0000_0001, // smallest subnormal
        0x0010_0000_0000_0000, // MIN_POSITIVE
        0x3E45_798E_E230_8C3A, // 1e-8
        0x3FD3_3333_3333_3333, // 0.3
        0x3FD3_3333_3333_3334, // 0.30000000000000004
        0x40AF_4023_D70A_3D71, // an ordinary price-like value
        0x4197_D784_0000_0000, // 1e8
        0x7FEF_FFFF_FFFF_FFFF, // MAX
        0x7FF0_0000_0000_0000, // +inf
        0xFFF0_0000_0000_0000, // -inf
        0x7FF8_0000_0000_0000, // canonical quiet NaN
        0x7FF8_0000_0000_0001, // NaN with payload
        0xFFF8_0000_0000_0123, // negative NaN with payload
        0x3FF0_0000_0000_0000, // 1.0
    ];
    let int_values: [i64; 16] = [
        i64::MIN,
        -1,
        0,
        1,
        2,
        99_999_999_999_999_999,
        4_000_170_000_000,
        1_234_567_812_345_678,
        i64::MAX,
        i64::MAX - 1,
        9_007_199_254_740_993,
        100_000_000,
        10_000_000_000_000_000,
        4_611_686_018_427_387_904,
        -9_007_199_254_740_993,
        7,
    ];
    let n = 64usize;
    let agg: Vec<i64> = (0..n as i64)
        .map(|i| match i {
            60 => 9_007_199_254_740_993,
            61 => 4_611_686_018_427_387_904,
            62 => i64::MAX - 1,
            63 => i64::MAX,
            _ => i,
        })
        .collect();
    let time: Vec<i64> = (0..n as i64)
        .map(|i| match i {
            60 => 4_611_686_018_427_387_000,
            61 => 4_611_686_018_427_387_904,
            62 => i64::MAX - 1,
            63 => i64::MAX,
            _ => 1_759_276_803_099_156 + i * 7_000_000 + i % 3,
        })
        .collect();
    let symbols: Vec<&str> = (0..n)
        .map(|i| match i % 8 {
            3 => "ZZZUSDT",
            5 => "",
            6 => "\u{c5}\u{df}\u{20ac}",
            _ => SYMBOL,
        })
        .collect();
    let (price, qty): (ArrayRef, ArrayRef) = match variant {
        Variant::V1 => (
            Arc::new(Float64Array::from(
                (0..n)
                    .map(|i| f64::from_bits(float_bits[i % 16]))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                (0..n)
                    .map(|i| f64::from_bits(float_bits[(i * 5 + 1) % 16]))
                    .collect::<Vec<_>>(),
            )),
        ),
        Variant::V2 => (
            Arc::new(Int64Array::from(
                (0..n).map(|i| int_values[i % 16]).collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(
                (0..n)
                    .map(|i| int_values[(i * 5 + 1) % 16])
                    .collect::<Vec<_>>(),
            )),
        ),
    };
    let first: Vec<i64> = (0..n).map(|i| int_values[(i * 3) % 16]).collect();
    let last: Vec<i64> = (0..n)
        .map(|i| {
            if i % 4 == 0 {
                first[i]
            } else {
                int_values[(i * 7) % 16]
            }
        })
        .collect();
    let batch = RecordBatch::try_new(
        schema(variant),
        vec![
            Arc::new(Int64Array::from(agg)),
            price,
            qty,
            Arc::new(Int64Array::from(first)),
            Arc::new(Int64Array::from(last)),
            Arc::new(Int64Array::from(time)),
            Arc::new(BooleanArray::from(
                (0..n).map(|i| i % 3 == 0).collect::<Vec<_>>(),
            )),
            Arc::new(BooleanArray::from(
                (0..n).map(|i| i % 5 != 0).collect::<Vec<_>>(),
            )),
            Arc::new(LargeStringArray::from(symbols)),
        ],
    )?;
    // Several small blocks so that block pruning and multi-block paths are exercised.
    Ok((0..n)
        .step_by(20)
        .map(|s| batch.slice(s, 20.min(n - s)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builder(variant: Variant, lines: &[&str]) -> Result<Vec<RecordBatch>> {
        let mut b = TableBuilder::new(variant, SYMBOL);
        for line in lines {
            b.push_line(line)?;
        }
        b.into_batches()
    }

    const OK: [&str; 3] = [
        "10,4000.17000000,0.00200000,5,5,1759276803099156,True,True",
        "11,0.00000001,0.00000000,6,8,1759276803099156,False,True",
        "12,99999999.99999999,12345.67890123,9,9,1759276803100000,True,True",
    ];

    #[test]
    fn strict_grammar() {
        assert!(strict_decimal("1.00000000").is_ok());
        for bad in [
            "1.0",
            "1",
            ".00000000",
            "-1.00000000",
            "1.000000000",
            "1e3.00000000",
            "NaN",
        ] {
            assert!(strict_decimal(bad).is_err(), "{bad}");
        }
        assert!(builder(Variant::V1, &["1,1.00000000,1.00000000,1,1,5,true,True"]).is_err());
        assert!(builder(Variant::V1, &["1,1.00000000,1.00000000,1,1,5,True"]).is_err());
        assert!(builder(Variant::V1, &["1,1.00000000,1.00000000,1,1,5,True,True\r"]).is_err());
        assert!(
            builder(
                Variant::V1,
                &[
                    "1,1.00000000,1.00000000,1,1,5,True,True",
                    "1,1.00000000,1.00000000,1,1,5,True,True"
                ]
            )
            .is_err()
        );
        assert!(
            builder(
                Variant::V1,
                &[
                    "1,1.00000000,1.00000000,1,1,5,True,True",
                    "2,1.00000000,1.00000000,1,1,4,True,True"
                ]
            )
            .is_err()
        );
    }

    #[test]
    fn v1_and_v2_agree_through_text() -> Result<()> {
        let v1 = builder(Variant::V1, &OK)?;
        let v2 = builder(Variant::V2, &OK)?;
        let prices: Vec<f64> = v1[0]
            .column(1)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap()
            .values()
            .to_vec();
        assert_eq!(prices[0].to_bits(), 4000.17f64.to_bits());
        let ints = v2[0]
            .column(1)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(ints.values(), &[400_017_000_000, 1, 9_999_999_999_999_999]);
        Ok(())
    }

    #[test]
    fn digest_distinguishes_signed_zero_nan_payload_and_types() -> Result<()> {
        let base = synthetic_edge(Variant::V1)?;
        let d = digest_batches(&base)?;
        assert_eq!(
            d,
            digest_batches(&synthetic_edge(Variant::V1)?)?,
            "deterministic"
        );
        // Same values re-chunked into one batch hash identically.
        let schema = base[0].schema();
        let one = arrow_select::concat::concat_batches(&schema, &base)?;
        assert_eq!(d, digest_batches(&[one.clone()])?);
        // Flip the sign of a zero: digest must change.
        let mut prices = one
            .column(1)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap()
            .values()
            .to_vec();
        assert_eq!(prices[0].to_bits(), 0);
        prices[0] = -0.0;
        let mut columns = one.columns().to_vec();
        columns[1] = Arc::new(Float64Array::from(prices));
        let flipped = RecordBatch::try_new(schema.clone(), columns)?;
        assert_ne!(d, digest_batches(&[flipped])?);
        // Windowing: bounds come from the middle row and the selection is inclusive.
        let (t0, t1) = window_bounds(&base);
        assert_eq!(t1 - t0, WINDOW_US);
        let selected = select_window(&base, SYMBOL, t0, t1)?;
        assert!(selected.iter().map(RecordBatch::num_rows).sum::<usize>() >= 1);
        Ok(())
    }

    #[test]
    fn fsync_tree_handles_files_and_directories() -> Result<()> {
        let root = std::env::temp_dir().join(format!("b17-fsync-{}", std::process::id()));
        fs::create_dir_all(root.join("a/b"))?;
        fs::write(root.join("a/b/f"), b"x")?;
        fsync_tree(&root)?;
        fsync_tree(&root.join("a/b/f"))?;
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    /// Syncing must not touch the contents, and wrong paths must still be errors on every
    /// platform (the Windows directory step is a check, not a flush).
    #[test]
    fn fsync_tree_keeps_contents_and_rejects_missing_paths() -> Result<()> {
        let root = std::env::temp_dir().join(format!("b17-fsync-keep-{}", std::process::id()));
        fs::create_dir_all(root.join("d"))?;
        fs::write(root.join("d/f1"), b"payload-1")?;
        fs::write(root.join("f2"), b"")?;
        fsync_tree(&root)?;
        assert_eq!(fs::read(root.join("d/f1"))?, b"payload-1");
        assert_eq!(fs::read(root.join("f2"))?, b"");
        assert!(fsync_tree(&root.join("missing")).is_err());
        assert!(fsync_dir(&root.join("missing")).is_err());
        fs::remove_dir_all(&root)?;
        Ok(())
    }
}
