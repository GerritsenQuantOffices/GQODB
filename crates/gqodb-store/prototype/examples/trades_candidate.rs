//! B17 Rust adapter: GQODB and the Parquet configurations of
//! docs/B17_PUBLIC_TRADES_PROTOCOL.md behind the common adapter contract.
//!
//!   trades_candidate PHASE --candidate ID --variant v1|v2 --source SOURCE.parquet
//!                    --store DIR [--symbol S --start T --end T]
//!
//! PHASE is noop, write, read-full, read-range, source-read or normalise. The last
//! stdout line is one JSON object. The clock stops before any digest is computed.
#![forbid(unsafe_code)]

#[path = "support/public_trades.rs"]
mod public_trades;

use anyhow::{Context, Result, bail, ensure};
use arrow_array::{Array, Float64Array, Int64Array, RecordBatch};
use arrow_schema::DataType;
use gqodb_blocks::real::Mode;
use gqodb_store::{Reader, Writer};
use parquet::{
    arrow::{ArrowWriter, arrow_reader::ParquetRecordBatchReaderBuilder},
    basic::{BrotliLevel, Compression, Encoding},
    file::{
        metadata::KeyValue,
        properties::{EnabledStatistics, WriterProperties, WriterVersion},
        statistics::Statistics,
    },
};
use public_trades::{BLOCK_ROWS, SCALE, Variant};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Candidate {
    Gqodb,
    DeltaBssLz4,
    DictionaryLz4,
    DeltaBssSnappy,
    DeltaBssBrotli5,
}

impl Candidate {
    fn parse(name: &str) -> Result<Self> {
        Ok(match name {
            "gqodb" => Self::Gqodb,
            "parquet-delta-bss-lz4" => Self::DeltaBssLz4,
            "parquet-dictionary-lz4" => Self::DictionaryLz4,
            "parquet-delta-bss-snappy" => Self::DeltaBssSnappy,
            "parquet-delta-bss-brotli5" => Self::DeltaBssBrotli5,
            other => bail!("unknown Rust candidate {other}"),
        })
    }
    fn file_name(self) -> &'static str {
        if self == Self::Gqodb {
            "trades.gqodb.ob"
        } else {
            "trades.parquet"
        }
    }
}

fn option<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn required<'a>(args: &'a [String], name: &str) -> Result<&'a str> {
    option(args, name).with_context(|| format!("missing {name}"))
}

/// Parquet writer properties as used by the B06/B12 controls (real.rs, storage_benchmark.rs):
/// Parquet V2, page statistics, 32,768-row groups, per-type encodings.
fn properties(candidate: Candidate, batch: &RecordBatch) -> WriterProperties {
    let compression = match candidate {
        Candidate::DeltaBssSnappy => Compression::SNAPPY,
        Candidate::DeltaBssBrotli5 => Compression::BROTLI(BrotliLevel::try_new(5).unwrap()),
        _ => Compression::LZ4_RAW,
    };
    let metadata = batch
        .schema()
        .metadata()
        .iter()
        .map(|(k, v)| KeyValue::new(k.clone(), v.clone()))
        .collect::<Vec<_>>();
    let mut props = WriterProperties::builder()
        .set_writer_version(WriterVersion::PARQUET_2_0)
        .set_max_row_group_row_count(Some(BLOCK_ROWS))
        .set_data_page_row_count_limit(65_536)
        .set_data_page_size_limit(1_048_576)
        .set_statistics_enabled(EnabledStatistics::Page)
        .set_dictionary_enabled(candidate == Candidate::DictionaryLz4)
        .set_compression(compression)
        .set_key_value_metadata(Some(metadata));
    for field in batch.schema().fields() {
        let encoding = match field.data_type() {
            DataType::Int64 => Encoding::DELTA_BINARY_PACKED,
            DataType::Float64 => Encoding::BYTE_STREAM_SPLIT,
            DataType::LargeUtf8 => Encoding::DELTA_BYTE_ARRAY,
            _ => Encoding::PLAIN,
        };
        props = props.set_column_encoding(field.name().as_str().into(), encoding);
    }
    props.build()
}

fn write_candidate(candidate: Candidate, path: &Path, batches: &[RecordBatch]) -> Result<()> {
    if candidate == Candidate::Gqodb {
        let mut writer = Writer::create_with_codec(
            path,
            batches[0].schema(),
            "transact_time",
            "symbol",
            Mode::Gqodb,
        )?;
        for batch in batches {
            writer.append(batch)?;
        }
        writer.finish()
    } else {
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        let mut writer = ArrowWriter::try_new(
            file,
            batches[0].schema(),
            Some(properties(candidate, &batches[0])),
        )?;
        for batch in batches {
            writer.write(batch)?;
            writer.flush()?;
        }
        writer.into_inner()?.sync_all()?;
        Ok(())
    }
}

type GroupFilter<'a> = &'a dyn Fn(usize, &parquet::file::metadata::RowGroupMetaData) -> bool;

fn read_parquet_file(
    path: &Path,
    groups: Option<GroupFilter>,
) -> Result<(Vec<RecordBatch>, usize)> {
    let builder = ParquetRecordBatchReaderBuilder::try_new(File::open(path)?)?;
    let selected: Vec<usize> = builder
        .metadata()
        .row_groups()
        .iter()
        .enumerate()
        .filter(|(i, g)| groups.is_none_or(|keep| keep(*i, g)))
        .map(|(i, _)| i)
        .collect();
    let count = selected.len();
    let batches = builder
        .with_row_groups(selected)
        .with_batch_size(BLOCK_ROWS)
        .build()?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok((batches, count))
}

/// Row-group pruning on time and symbol statistics, then an exact filter (as in B12).
fn parquet_range(
    path: &Path,
    symbol: &str,
    start: i64,
    end: i64,
) -> Result<(Vec<RecordBatch>, usize)> {
    let keep = |_: usize, group: &parquet::file::metadata::RowGroupMetaData| {
        let columns = group.columns();
        let (mut time_ok, mut symbol_ok) = (true, true);
        for column in columns {
            let name = column.column_descr().name();
            match (name, column.statistics()) {
                ("transact_time", Some(Statistics::Int64(s)))
                    if s.min_opt().is_some_and(|v| *v > end)
                        || s.max_opt().is_some_and(|v| *v < start) =>
                {
                    time_ok = false;
                }
                ("symbol", Some(Statistics::ByteArray(s)))
                    if s.min_opt().is_some_and(|v| v.data() > symbol.as_bytes())
                        || s.max_opt().is_some_and(|v| v.data() < symbol.as_bytes()) =>
                {
                    symbol_ok = false;
                }
                _ => {}
            }
        }
        time_ok && symbol_ok
    };
    let (batches, count) = read_parquet_file(path, Some(&keep))?;
    Ok((batches, count))
}

fn restamp(batches: Vec<RecordBatch>, variant: Variant) -> Result<Vec<RecordBatch>> {
    let target = public_trades::schema(variant);
    batches
        .into_iter()
        .map(|b| Ok(RecordBatch::try_new(target.clone(), b.columns().to_vec())?))
        .collect()
}

struct Stop {
    op_ns: u128,
    cpu_ns: u64,
    rss_hwm_kb: u64,
}

fn stop(begin: Instant) -> Stop {
    // The counters are read at the moment the clock stops, before any hashing.
    let op_ns = begin.elapsed().as_nanos();
    Stop {
        op_ns,
        cpu_ns: public_trades::cpu_ns(),
        rss_hwm_kb: public_trades::vm_hwm_kb(),
    }
}

fn stop_json(s: &Stop) -> Value {
    json!({"op_ns": s.op_ns.to_string(), "cpu_ns": s.cpu_ns, "rss_hwm_kb": s.rss_hwm_kb})
}

fn merge(mut a: Value, b: Value) -> Value {
    if let (Some(x), Some(y)) = (a.as_object_mut(), b.as_object()) {
        for (k, v) in y {
            x.insert(k.clone(), v.clone());
        }
    }
    a
}

/// V1 to V2 with the proof of section 2: every double must convert back to identical bits.
fn normalise(batches: &[RecordBatch]) -> Result<Vec<RecordBatch>> {
    let target = public_trades::schema(Variant::V2);
    batches
        .iter()
        .map(|batch| {
            let mut columns = batch.columns().to_vec();
            for index in [1, 2] {
                let source = batch
                    .column(index)
                    .as_any()
                    .downcast_ref::<Float64Array>()
                    .context("V1 float column")?;
                let mut out = Vec::with_capacity(source.len());
                for &v in source.values() {
                    let scaled = (v * SCALE as f64).round();
                    ensure!(
                        scaled.is_finite() && scaled.abs() < 9.007_199_254_740_992e15,
                        "value {v} is outside the exact range"
                    );
                    let n = scaled as i64;
                    ensure!(
                        (n as f64 / SCALE as f64).to_bits() == v.to_bits(),
                        "value {v} does not convert back to identical bits"
                    );
                    out.push(n);
                }
                columns[index] = Arc::new(Int64Array::from(out));
            }
            Ok(RecordBatch::try_new(target.clone(), columns)?)
        })
        .collect()
}

fn denormalise(batches: &[RecordBatch], v1: &[RecordBatch]) -> Result<()> {
    for (b2, b1) in batches.iter().zip(v1) {
        for index in [1, 2] {
            let ints = b2
                .column(index)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap();
            let floats = b1
                .column(index)
                .as_any()
                .downcast_ref::<Float64Array>()
                .unwrap();
            for (n, f) in ints.values().iter().zip(floats.values()) {
                ensure!(
                    (*n as f64 / SCALE as f64).to_bits() == f.to_bits(),
                    "denormalisation mismatch"
                );
            }
        }
    }
    Ok(())
}

fn run(args: &[String]) -> Result<Value> {
    let phase = args.first().map(String::as_str).context("missing phase")?;
    if phase == "noop" {
        let s = stop(Instant::now());
        return Ok(merge(json!({"phase": "noop"}), stop_json(&s)));
    }
    let candidate = Candidate::parse(required(args, "--candidate")?)?;
    let variant = Variant::parse(required(args, "--variant")?)?;
    let source = PathBuf::from(required(args, "--source")?);
    let store = PathBuf::from(required(args, "--store")?);
    let file = store.join(candidate.file_name());
    let base = json!({"phase": phase, "candidate": required(args, "--candidate")?, "variant": variant.name()});
    match phase {
        "write" => {
            ensure!(!store.exists(), "store directory exists");
            fs::create_dir_all(store.parent().context("store has no parent")?)?;
            let begin = Instant::now();
            let batches = public_trades::read_parquet(&source, variant)?;
            fs::create_dir(&store)?;
            write_candidate(candidate, &file, &batches)?;
            public_trades::fsync_tree(&store)?;
            let s = stop(begin);
            let rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
            Ok(merge(
                merge(base, stop_json(&s)),
                json!({"rows": rows, "blocks": batches.len()}),
            ))
        }
        "read-full" => {
            let begin = Instant::now();
            let batches = if candidate == Candidate::Gqodb {
                Reader::open(&file)?.read_all()?
            } else {
                restamp(read_parquet_file(&file, None)?.0, variant)?
            };
            let s = stop(begin);
            let digest = public_trades::digest_batches(&batches)?;
            let rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
            Ok(merge(
                merge(base, stop_json(&s)),
                json!({"rows": rows, "digest": digest}),
            ))
        }
        "read-range" => {
            let symbol = option(args, "--symbol").unwrap_or(public_trades::SYMBOL);
            let start: i64 = required(args, "--start")?.parse()?;
            let end: i64 = required(args, "--end")?.parse()?;
            let begin = Instant::now();
            let (batches, blocks) = if candidate == Candidate::Gqodb {
                let result = Reader::open(&file)?.query(symbol, start, end)?;
                (result.batches, result.stats.blocks_read)
            } else {
                let (read, groups) = parquet_range(&file, symbol, start, end)?;
                let read = restamp(read, variant)?;
                (
                    public_trades::select_window(&read, symbol, start, end)?,
                    groups,
                )
            };
            let s = stop(begin);
            let rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
            let digest = if batches.is_empty() {
                Value::Null
            } else {
                json!(public_trades::digest_batches(&batches)?)
            };
            Ok(merge(
                merge(base, stop_json(&s)),
                json!({"rows": rows, "blocks_read": blocks, "digest": digest}),
            ))
        }
        "source-read" => {
            let begin = Instant::now();
            let batches = public_trades::read_parquet(&source, variant)?;
            let s = stop(begin);
            let digest = public_trades::digest_batches(&batches)?;
            Ok(merge(merge(base, stop_json(&s)), json!({"digest": digest})))
        }
        "normalise" => {
            ensure!(variant == Variant::V1, "normalise starts from the V1 table");
            let v1 = public_trades::read_parquet(&source, Variant::V1)?;
            let begin = Instant::now();
            let v2 = normalise(&v1)?;
            let normalise_ns = begin.elapsed().as_nanos();
            let begin = Instant::now();
            denormalise(&v2, &v1)?;
            let denormalise_ns = begin.elapsed().as_nanos();
            let digest = public_trades::digest_batches(&v2)?;
            Ok(merge(
                base,
                json!({
                    "op_ns": normalise_ns.to_string(),
                    "denormalise_ns": denormalise_ns.to_string(),
                    "digest": digest,
                    "cpu_ns": public_trades::cpu_ns(),
                    "rss_hwm_kb": public_trades::vm_hwm_kb(),
                }),
            ))
        }
        other => bail!("unknown phase {other}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(value) => println!("{}", serde_json::to_string(&value).unwrap()),
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::exit(1);
        }
    }
}
