//! Current Rust Parquet controls. No native compression dependencies enabled.
use crate::data::{Columns, Kind, MAX_ROWS};
use anyhow::{Result, ensure};
use arrow_array::{Array, ArrayRef, Int64Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use bytes::Bytes;
use parquet::arrow::{ArrowWriter, arrow_reader::ParquetRecordBatchReaderBuilder};
use parquet::basic::{BrotliLevel, Compression, Encoding};
use parquet::file::properties::{EnabledStatistics, WriterProperties, WriterVersion};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ParquetMode {
    PlainLz4,
    DeltaLz4,
    DictionaryLz4,
    DeltaSnappy,
    DeltaBrotli,
}

pub fn encode(data: &Columns, mode: ParquetMode) -> Result<Vec<u8>> {
    data.validate()?;
    let fields: Vec<_> = data
        .kind
        .names()
        .into_iter()
        .map(|name| Field::new(name, DataType::Int64, false))
        .collect();
    let metadata = HashMap::from([("gqodb.schema_id".into(), (data.kind as u8).to_string())]);
    let schema = Arc::new(Schema::new_with_metadata(fields, metadata));
    let arrays: Vec<ArrayRef> = data
        .values
        .iter()
        .map(|v| Arc::new(Int64Array::from(v.clone())) as ArrayRef)
        .collect();
    let batch = RecordBatch::try_new(schema.clone(), arrays)?;
    let compression = match mode {
        ParquetMode::DeltaSnappy => Compression::SNAPPY,
        ParquetMode::DeltaBrotli => Compression::BROTLI(BrotliLevel::try_new(5)?),
        _ => Compression::LZ4_RAW,
    };
    let properties = WriterProperties::builder()
        .set_writer_version(WriterVersion::PARQUET_2_0)
        .set_created_by("gqodb B01 / parquet 59.3.0".into())
        .set_max_row_group_row_count(Some(data.rows().max(1)))
        .set_data_page_row_count_limit(65_536)
        .set_data_page_size_limit(1_048_576)
        .set_statistics_enabled(EnabledStatistics::Page)
        .set_dictionary_enabled(matches!(mode, ParquetMode::DictionaryLz4))
        .set_encoding(if matches!(mode, ParquetMode::PlainLz4) {
            Encoding::PLAIN
        } else {
            Encoding::DELTA_BINARY_PACKED
        })
        .set_compression(compression)
        .build();
    let mut out = Vec::new();
    let mut writer = ArrowWriter::try_new(&mut out, schema, Some(properties))?;
    writer.write(&batch)?;
    writer.close()?;
    // Equal whole-block integrity check; strip these four bytes for standard Parquet.
    out.extend_from_slice(&crc32fast::hash(&out).to_le_bytes());
    Ok(out)
}

/// Run a call into the `parquet` / `arrow-ipc` dependencies and turn an upstream panic into an
/// ordinary decode error.
///
/// Crafted input reaches panics inside those crates: `arrow-ipc` 59.3.0 `convert.rs` calls
/// `unimplemented!`, `unwrap` and `expect` on unknown or missing type tags of the embedded
/// `ARROW:schema`, and `parquet` 59.3.0 `encodings/decoding.rs` underflows a counter on a forged
/// DELTA_BINARY_PACKED header (a panic with overflow checks, a wrapped counter without). The
/// panic is contained here; inputs that decode, and what they decode to, are unchanged. This
/// relies on unwinding panics (the default); with `panic = "abort"` the process would still abort.
pub(crate) fn contain<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .map_err(|_| anyhow::anyhow!("malformed Parquet input rejected by the Parquet reader"))?
}

/// Open a Parquet footer; refuse inputs that would hang or panic the Parquet reader.
pub(crate) fn open(input: Bytes) -> Result<ParquetRecordBatchReaderBuilder<Bytes>> {
    let bytes = input.clone();
    refuse_unbounded_footer(&bytes)?;
    let builder = contain(|| Ok(ParquetRecordBatchReaderBuilder::try_new(input)?))?;
    refuse_unbounded_page_headers(&bytes, builder.metadata())?;
    Ok(builder)
}

/// Upper bound on ZERO-BYTE (boolean) list/map elements skipped inside one footer or page header.
/// Neither a real footer nor a real page header contains a list of booleans, so any valid file is
/// far below it.
const MAX_SKIPPED_ELEMENTS: u64 = 4096;

enum Walk {
    /// A page header skips more elements than any real header contains.
    Absurd,
    /// Anything else the walker does not understand; the Parquet reader reports it.
    Unparsed,
}

/// Minimal Thrift compact-protocol walker over one page header.
struct Header<'a> {
    bytes: &'a [u8],
    pos: usize,
    skipped: u64,
    /// Page headers are read through `io::Read`, whose `skip_bytes` (`io::copy` of a `take`) does not
    /// fail at end of input: a list or map of doubles, binaries or UUIDs then costs one loop
    /// iteration per element for free, like a list of booleans. Real page headers contain no list
    /// or map at all, so every declared size is charged. The footer is read from a slice, where
    /// only boolean elements are free, and real footers do contain lists (schema, row groups), so
    /// there only boolean-element lists and maps are charged.
    every_list_is_suspect: bool,
}

impl Header<'_> {
    fn byte(&mut self) -> Result<u8, Walk> {
        let b = *self.bytes.get(self.pos).ok_or(Walk::Unparsed)?;
        self.pos += 1;
        Ok(b)
    }
    fn varint(&mut self) -> Result<u64, Walk> {
        let mut v = 0u64;
        for shift in (0..70).step_by(7) {
            let b = self.byte()?;
            v |= u64::from(b & 127).checked_shl(shift).unwrap_or(0);
            if b & 128 == 0 {
                return Ok(v);
            }
        }
        Err(Walk::Unparsed)
    }
    fn charge(&mut self, n: u64) -> Result<(), Walk> {
        self.skipped = self.skipped.saturating_add(n);
        if self.skipped > MAX_SKIPPED_ELEMENTS {
            Err(Walk::Absurd)
        } else {
            Ok(())
        }
    }
    fn advance(&mut self, n: u64) -> Result<(), Walk> {
        let end = usize::try_from(n)
            .ok()
            .and_then(|n| self.pos.checked_add(n));
        match end {
            Some(end) if end <= self.bytes.len() => {
                self.pos = end;
                Ok(())
            }
            _ => Err(Walk::Unparsed),
        }
    }
    /// Skip one value of compact-protocol type `ty` (1/2 bool, 3 byte, 4-6 ints, 7 double,
    /// 8 binary, 9 list, 10 set, 11 map, 12 struct, 13 uuid).
    fn skip(&mut self, ty: u8, depth: u8) -> Result<(), Walk> {
        if depth == 0 {
            return Err(Walk::Unparsed);
        }
        match ty {
            1 | 2 => Ok(()),
            3 => self.advance(1),
            4..=6 => self.varint().map(|_| ()),
            7 => self.advance(8),
            8 => {
                let n = self.varint()?;
                self.advance(n)
            }
            9 | 10 => {
                let head = self.byte()?;
                let size = if head >> 4 == 15 {
                    self.varint()?
                } else {
                    u64::from(head >> 4)
                };
                if self.every_list_is_suspect || matches!(head & 15, 1 | 2) {
                    self.charge(size)?;
                }
                for _ in 0..size {
                    self.skip(head & 15, depth - 1)?;
                }
                Ok(())
            }
            11 => {
                let size = self.varint()?;
                if size > 0 {
                    let kv = self.byte()?;
                    let free = |t: u8| matches!(t, 1 | 2);
                    if self.every_list_is_suspect || (free(kv >> 4) && free(kv & 15)) {
                        self.charge(size.saturating_mul(2))?;
                    }
                    for _ in 0..size {
                        self.skip(kv >> 4, depth - 1)?;
                        self.skip(kv & 15, depth - 1)?;
                    }
                }
                Ok(())
            }
            12 => self.fields(depth - 1).map(|_| ()),
            13 => self.advance(16),
            _ => Err(Walk::Unparsed),
        }
    }
    /// Walk a struct; returns field 3 (`compressed_page_size`, i32) if present at this level.
    fn fields(&mut self, depth: u8) -> Result<Option<i64>, Walk> {
        let (mut last, mut compressed) = (0i64, None);
        loop {
            let head = self.byte()?;
            if head == 0 {
                return Ok(compressed);
            }
            let id = if head >> 4 == 0 {
                let z = self.varint()?;
                ((z >> 1) as i64) ^ -((z & 1) as i64)
            } else {
                last.saturating_add(i64::from(head >> 4))
            };
            last = id;
            if id == 3 && head & 15 == 5 {
                let z = self.varint()?;
                compressed = Some(((z >> 1) as i64) ^ -((z & 1) as i64));
            } else {
                self.skip(head & 15, depth)?;
            }
        }
    }
}

/// The same unbounded skip loop runs while the file footer (`FileMetaData`) is parsed, before any
/// page is read, so it has to be walked before `ParquetRecordBatchReaderBuilder::try_new`
/// (parquet 59.3.0 `file/metadata/thrift/mod.rs` skips unknown fields with `skip_till_depth`).
/// Footer layout: `... footer | footer length (u32 LE) | "PAR1"`. A footer this walk cannot place
/// or parse is left to the Parquet reader, which reports it.
fn refuse_unbounded_footer(file: &Bytes) -> Result<()> {
    let Some(tail) = file.len().checked_sub(8) else {
        return Ok(());
    };
    if &file[tail + 4..] != b"PAR1" {
        return Ok(());
    }
    let length = u32::from_le_bytes(file[tail..tail + 4].try_into().unwrap()) as usize;
    let Some(start) = tail.checked_sub(length) else {
        return Ok(());
    };
    let mut footer = Header {
        bytes: &file[start..tail],
        pos: 0,
        skipped: 0,
        every_list_is_suspect: false,
    };
    match footer.fields(64) {
        Err(Walk::Absurd) => anyhow::bail!("footer skips an unbounded number of elements"),
        _ => Ok(()),
    }
}

/// The `parquet` crate skips unknown page-header fields recursively and without a size bound: a
/// list or map of N elements that cost no input bytes (booleans, or doubles/binaries read past the
/// end of input) costs N iterations, and a 2 KB file keeps one core busy for minutes
/// (parquet 59.3.0 `parquet_thrift.rs`, `skip_till_depth`).
/// Walk every page header of every column chunk first, with a budget on skipped elements, and
/// refuse the input when the budget is exceeded. A walk that cannot parse something is not a
/// refusal: the Parquet reader decides. Valid files (no lists in page headers) are never refused.
fn refuse_unbounded_page_headers(
    file: &Bytes,
    meta: &parquet::file::metadata::ParquetMetaData,
) -> Result<()> {
    for group in meta.row_groups() {
        for column in group.columns() {
            let first = column
                .dictionary_page_offset()
                .unwrap_or(column.data_page_offset());
            let (Ok(start), Ok(len)) = (
                u64::try_from(first),
                u64::try_from(column.compressed_size()),
            ) else {
                continue;
            };
            // Clip to the bytes present: the reader may start a header inside a chunk whose
            // declared length runs past the end of the file.
            let end = start.saturating_add(len).min(file.len() as u64);
            if start >= end {
                continue;
            }
            let (mut pos, end) = (start as usize, end as usize);
            while pos < end {
                let mut header = Header {
                    // The Parquet reader hands the header parser the whole rest of the file, not just the
                    // column chunk, so the walk must be allowed to run past `end` as well.
                    bytes: &file[pos..],
                    pos: 0,
                    skipped: 0,
                    every_list_is_suspect: true,
                };
                let r = header.fields(64);
                match r {
                    Err(Walk::Absurd) => {
                        anyhow::bail!("page header skips an unbounded number of elements")
                    }
                    Err(Walk::Unparsed) => break,
                    Ok(Some(size)) if size >= 0 => match pos
                        .checked_add(header.pos)
                        .and_then(|p| p.checked_add(size as usize))
                    {
                        Some(next) if next > pos => pos = next,
                        _ => break,
                    },
                    Ok(_) => break,
                }
            }
        }
    }
    Ok(())
}

fn checked_file(input: Bytes) -> Result<Bytes> {
    ensure!(
        input.len() >= 12 && input.len() <= 128 * 1024 * 1024,
        "invalid parquet block length"
    );
    let n = input.len() - 4;
    let crc = u32::from_le_bytes(input[n..].try_into().unwrap());
    ensure!(
        crc32fast::hash(&input[..n]) == crc,
        "parquet envelope checksum mismatch"
    );
    Ok(input.slice(..n))
}

pub fn decode(input: Bytes) -> Result<Columns> {
    let builder = open(checked_file(input)?)?;
    let schema = builder.schema();
    let id: u8 = schema
        .metadata()
        .get("gqodb.schema_id")
        .ok_or_else(|| anyhow::anyhow!("missing schema ID"))?
        .parse()?;
    let kind = Kind::from_id(id)?;
    ensure!(schema.fields().len() == 8, "invalid column count");
    for (field, name) in schema.fields().iter().zip(kind.names()) {
        ensure!(
            field.name() == name && field.data_type() == &DataType::Int64 && !field.is_nullable(),
            "invalid schema"
        );
    }
    let rows = builder.metadata().file_metadata().num_rows();
    ensure!(rows >= 0 && rows <= MAX_ROWS as i64, "invalid row count");
    let mut values: Vec<Vec<i64>> = (0..8).map(|_| Vec::with_capacity(rows as usize)).collect();
    let mut reader = contain(|| Ok(builder.with_batch_size(rows.max(1) as usize).build()?))?;
    while let Some(batch) = contain(|| Ok(reader.next().transpose()?))? {
        for (dst, src) in values.iter_mut().zip(batch.columns()) {
            let src = src
                .as_any()
                .downcast_ref::<Int64Array>()
                .ok_or_else(|| anyhow::anyhow!("invalid integer array"))?;
            ensure!(src.null_count() == 0, "unexpected nulls");
            dst.extend_from_slice(src.values());
        }
    }
    let data = Columns { kind, values };
    data.validate()?;
    ensure!(data.rows() == rows as usize, "row count mismatch");
    Ok(data)
}

/// Non-column-chunk bytes, including footer/indexes and external CRC.
pub fn framing_bytes(input: Bytes) -> Result<usize> {
    let total = input.len();
    let reader = open(checked_file(input)?)?;
    let chunks: i64 = reader
        .metadata()
        .row_groups()
        .iter()
        .flat_map(|r| r.columns())
        .map(|c| c.compressed_size())
        .sum();
    ensure!(
        chunks >= 0 && chunks as usize <= total,
        "invalid column sizes"
    );
    Ok(total - chunks as usize)
}
