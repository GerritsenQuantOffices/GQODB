#![forbid(unsafe_code)]
//! Checked dispatch over existing native tick/book codecs; no new wire container.
use anyhow::{Context, Result, bail, ensure};
use arrow_array::{Array, ArrayRef, Int8Array, Int64Array, LargeStringArray, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use bytes::Bytes;
use gqodb_blocks::real::{self, Mode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::Arc,
};
pub const MAX_BLOCK_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_METADATA_BYTES: usize = 1024 * 1024;
pub const MAX_DECODED_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_SEGMENT_BYTES: u64 = 1024 * 1024 * 1024;
pub const MAX_SEGMENT_BLOCKS: usize = 256;
pub const MAX_SEGMENT_ROWS: usize = 16 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Tick,
    Orderbook,
}
impl Family {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "tick" => Ok(Self::Tick),
            "orderbook" => Ok(Self::Orderbook),
            _ => bail!("family must be tick or orderbook"),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Tick => "tick",
            Self::Orderbook => "orderbook",
        }
    }
    fn mode(self) -> Mode {
        match self {
            Self::Tick => Mode::Gqodb,
            Self::Orderbook => Mode::GqodbBook,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    AdaptiveBlockV3,
    BookBlockV4,
    NativeSegmentV1,
}
impl Format {
    fn family(self) -> Family {
        match self {
            Self::AdaptiveBlockV3 => Family::Tick,
            _ => Family::Orderbook,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub format: Format,
    pub codec_profile: &'static str,
    pub file_sha256: String,
    pub file_bytes: u64,
    pub blocks: usize,
    pub rows: usize,
    pub arrow_schema: Schema,
    pub normalized_contract_valid: bool,
    pub normalized_contract_error: Option<String>,
    pub fully_decoded: bool,
}
pub fn detect(bytes: &[u8]) -> Result<Format> {
    ensure!(bytes.len() >= 8, "truncated format magic");
    match &bytes[..8] {
        b"GQOREAL3" => Ok(Format::AdaptiveBlockV3),
        b"GQOREAL4" => Ok(Format::BookBlockV4),
        b"GQOBHDR1" => Ok(Format::NativeSegmentV1),
        _ => bail!("unknown native codec/version; extension does not select decoder"),
    }
}
fn field(schema: &Schema, name: &str, ty: DataType, nullable: bool) -> Result<()> {
    let field = schema.field_with_name(name)?;
    ensure!(
        *field.data_type() == ty && (nullable || !field.is_nullable()),
        "wrong type/nullability for {name}"
    );
    Ok(())
}
pub fn validate_schema(schema: &Schema, family: Family) -> Result<()> {
    ensure!(
        !schema.fields().is_empty() && schema.fields().len() <= 64,
        "schema column count exceeded"
    );
    let mut names = std::collections::BTreeSet::new();
    for field in schema.fields() {
        ensure!(
            !field.name().is_empty() && field.name().len() <= 128 && names.insert(field.name()),
            "invalid or duplicate field name"
        );
    }
    let m = schema.metadata();
    ensure!(
        m.get("gqodb.family").map(String::as_str) == Some(family.name()),
        "missing/mismatched gqodb.family"
    );
    ensure!(
        m.get("gqodb.schema_version").map(String::as_str) == Some("1"),
        "unknown normalized schema version"
    );
    let sha = m
        .get("gqodb.source_sha256")
        .context("missing source SHA256")?;
    ensure!(
        sha.len() == 64
            && sha
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && sha != &"0".repeat(64),
        "invalid source SHA256"
    );
    for name in ["gqodb.price_decimals", "gqodb.quantity_decimals"] {
        let decimals: u8 = m.get(name).context("missing integer scale")?.parse()?;
        ensure!(decimals <= 12, "decimal scale exceeds 12");
    }
    for name in ["source_utc_ns", "received_utc_ns", "clock_error_ns"] {
        field(schema, name, DataType::Int64, true)?;
    }
    for name in ["available_utc_ns", "sequence"] {
        field(schema, name, DataType::Int64, false)?;
    }
    field(schema, "symbol", DataType::LargeUtf8, false)?;
    match family {
        Family::Tick => match m.get("gqodb.subtype").map(String::as_str) {
            Some("trade") => {
                for name in ["price", "quantity"] {
                    field(schema, name, DataType::Int64, false)?;
                }
                field(schema, "side", DataType::Int8, false)?;
            }
            Some("quote") => {
                for name in ["bid", "ask", "bid_quantity", "ask_quantity"] {
                    field(schema, name, DataType::Int64, false)?;
                }
            }
            _ => bail!("tick subtype must explicitly be trade or quote"),
        },
        Family::Orderbook => {
            let depth: usize = m
                .get("gqodb.depth")
                .context("missing book depth")?
                .parse()?;
            ensure!((1..=100_000).contains(&depth), "invalid source book depth");
            for name in ["price", "quantity", "level"] {
                field(schema, name, DataType::Int64, false)?;
            }
            for name in ["side", "action"] {
                field(schema, name, DataType::Int8, false)?;
            }
        }
    }
    Ok(())
}
/// Preflight normalized dimensions and worst-case Arrow string expansion before decoding.
fn preflight(bytes: &[u8], format: Format) -> Result<Schema> {
    ensure!(
        (32..=MAX_BLOCK_BYTES).contains(&bytes.len()),
        "native block byte bound"
    );
    let end = bytes.len() - 4;
    ensure!(
        crc32fast::hash(&bytes[..end]) == u32::from_le_bytes(bytes[end..].try_into()?),
        "native block checksum mismatch"
    );
    let meta = u32::from_le_bytes(bytes[8..12].try_into()?) as usize;
    ensure!(
        meta <= MAX_METADATA_BYTES && meta <= end - 12,
        "schema metadata bound"
    );
    let layout: serde_json::Value = serde_json::from_slice(&bytes[12..12 + meta])?;
    let schema: Schema =
        serde_json::from_value(layout.get("schema").context("missing schema")?.clone())?;
    ensure!(
        !schema.fields().is_empty() && schema.fields().len() <= 64,
        "schema field bound"
    );
    let columns = layout["columns"]
        .as_array()
        .context("missing column mapping")?;
    ensure!(
        columns.len() == schema.fields().len(),
        "schema/column mapping mismatch"
    );
    let payload = &bytes[12 + meta..end];
    ensure!(payload.len() >= 20, "truncated native payload");
    let (rows, cols) = match format {
        Format::AdaptiveBlockV3 => {
            ensure!(
                &payload[..8] == b"GQOADP02" && payload[9] == 0,
                "unknown adaptive payload version/schema"
            );
            (
                u32::from_le_bytes(payload[12..16].try_into()?) as usize,
                u16::from_le_bytes(payload[10..12].try_into()?) as usize,
            )
        }
        Format::BookBlockV4 => {
            ensure!(
                &payload[..8] == b"GQOBOOK1",
                "unknown orderbook payload version"
            );
            (
                u32::from_le_bytes(payload[8..12].try_into()?) as usize,
                u32::from_le_bytes(payload[12..16].try_into()?) as usize,
            )
        }
        _ => bail!("segment is not a block"),
    };
    ensure!(
        rows <= gqodb_blocks::data::MAX_ROWS && (1..=64).contains(&cols),
        "native row/column bound"
    );
    let mut estimate = rows
        .checked_mul(cols + schema.fields().len())
        .and_then(|v| v.checked_mul(8))
        .context("decode size overflow")?;
    for (field, column) in schema.fields().iter().zip(columns) {
        if field.data_type() == &DataType::LargeUtf8 {
            let dictionary = column["dictionary"]
                .as_array()
                .context("missing string dictionary")?;
            let width = dictionary
                .iter()
                .map(|s| s.as_str().map(str::len).context("invalid dictionary entry"))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .max()
                .unwrap_or(0);
            estimate = estimate
                .checked_add(rows.checked_mul(width).context("string size overflow")?)
                .context("decode size overflow")?;
        }
    }
    ensure!(
        estimate <= MAX_DECODED_BYTES,
        "worst-case decoded expansion exceeds 128 MiB"
    );
    Ok(schema)
}
pub fn encode(batch: &RecordBatch, family: Family) -> Result<Vec<u8>> {
    validate_schema(batch.schema().as_ref(), family)?;
    ensure!(
        serde_json::to_vec(batch.schema().as_ref())?.len() <= MAX_METADATA_BYTES,
        "schema metadata bound"
    );
    ensure!(
        batch.num_rows() <= gqodb_blocks::data::MAX_ROWS
            && batch.get_array_memory_size() <= MAX_DECODED_BYTES,
        "batch memory/row bound"
    );
    let bytes = real::encode(batch, family.mode())?;
    preflight(&bytes, detect(&bytes)?)?;
    Ok(bytes)
}
pub fn decode(bytes: Bytes, expected: Option<Family>) -> Result<RecordBatch> {
    let format = detect(&bytes)?;
    ensure!(
        format != Format::NativeSegmentV1,
        "segment requires file reader"
    );
    let schema = preflight(&bytes, format)?;
    if let Some(expected) = expected {
        ensure!(
            format.family() == expected,
            "physical codec family mismatch"
        );
        validate_schema(&schema, expected)?;
    }
    let batch = real::decode(bytes, format.family().mode())?;
    ensure!(
        batch.schema().as_ref() == &schema,
        "decoded schema mismatch"
    );
    Ok(batch)
}
fn open_regular(path: &Path) -> Result<File> {
    ensure!(
        std::fs::symlink_metadata(path)?.is_file(),
        "regular file required"
    );
    let file = File::open(path)?;
    ensure!(file.metadata()?.is_file(), "regular file required");
    Ok(file)
}
fn checksum(file: &mut File) -> Result<String> {
    file.seek(SeekFrom::Start(0))?;
    let mut h = Sha256::new();
    let mut bytes = 0_u64;
    let mut buf = [0_u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        ensure!(bytes <= MAX_SEGMENT_BYTES, "file grew beyond limit");
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn preflight_segment(file: &mut File, reader: &gqodb_store::Reader) -> Result<()> {
    ensure!(
        reader.blocks().len() <= MAX_SEGMENT_BLOCKS,
        "segment block count bound"
    );
    let rows: usize = reader.blocks().iter().map(|b| b.rows).sum();
    ensure!(rows <= MAX_SEGMENT_ROWS, "segment total row bound");
    for block in reader.blocks() {
        file.seek(SeekFrom::Start(block.offset))?;
        let mut head = [0_u8; 24];
        file.read_exact(&mut head)?;
        ensure!(
            &head[..8] == b"GQOBBLK1"
                && crc32fast::hash(&head[..20]) == u32::from_le_bytes(head[20..24].try_into()?),
            "invalid segment block header"
        );
        let meta = u32::from_le_bytes(head[8..12].try_into()?) as usize;
        let payload = u32::from_le_bytes(head[12..16].try_into()?) as usize;
        ensure!(
            meta <= MAX_METADATA_BYTES
                && payload <= MAX_BLOCK_BYTES
                && (24 + meta + payload) as u64 == block.length,
            "segment frame length bound"
        );
        let mut bytes = vec![0_u8; meta + payload];
        file.read_exact(&mut bytes)?;
        ensure!(
            crc32fast::hash(&bytes) == u32::from_le_bytes(head[16..20].try_into()?),
            "segment frame checksum"
        );
        let format = detect(&bytes[meta..])?;
        let expected = match reader.codec() {
            Mode::Gqodb => Format::AdaptiveBlockV3,
            _ => Format::BookBlockV4,
        };
        ensure!(format == expected, "segment/header codec mismatch");
        preflight(&bytes[meta..], format)?;
    }
    Ok(())
}
pub fn inspect(path: &Path, expected: Option<Family>) -> Result<Report> {
    let mut file = open_regular(path)?;
    let before = file.metadata()?;
    let len = before.len();
    ensure!((8..=MAX_SEGMENT_BYTES).contains(&len), "file size bound");
    let mut magic = [0_u8; 8];
    file.read_exact(&mut magic)?;
    let format = detect(&magic)?;
    let (schema, blocks, rows, family) = if format == Format::NativeSegmentV1 {
        let mut reader = gqodb_store::Reader::open(path)?;
        let family = match reader.codec() {
            Mode::Gqodb => Family::Tick,
            _ => Family::Orderbook,
        };
        ensure!(
            expected.is_none_or(|e| e == family),
            "segment codec family mismatch"
        );
        preflight_segment(&mut file, &reader)?;
        if expected.is_some() {
            validate_schema(reader.schema().as_ref(), family)?;
        }
        let mut rows = 0;
        reader.visit_blocks(|batch| {
            rows += batch.num_rows();
            if let Some(family) = expected {
                validate_schema(batch.schema().as_ref(), family)?;
            }
            Ok(())
        })?;
        (
            reader.schema().as_ref().clone(),
            reader.blocks().len(),
            rows,
            family,
        )
    } else {
        ensure!(len as usize <= MAX_BLOCK_BYTES, "block size bound");
        file.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(MAX_BLOCK_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        let batch = decode(Bytes::from(bytes), expected)?;
        (
            batch.schema().as_ref().clone(),
            1,
            batch.num_rows(),
            format.family(),
        )
    };
    let contract = validate_schema(&schema, family);
    let report = Report {
        schema_version: 1,
        format,
        codec_profile: family.mode().name(),
        file_sha256: checksum(&mut file)?,
        file_bytes: len,
        blocks,
        rows,
        arrow_schema: schema,
        normalized_contract_valid: contract.is_ok(),
        normalized_contract_error: contract.err().map(|e| e.to_string()),
        fully_decoded: true,
    };
    let after = file.metadata()?;
    ensure!(
        after.len() == len && before.modified()? == after.modified()?,
        "file changed during inspection"
    );
    Ok(report)
}
pub fn write_block(path: &Path, batch: &RecordBatch, family: Family) -> Result<()> {
    let bytes = encode(batch, family)?;
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}
pub fn write_segment(path: &Path, batches: &[RecordBatch]) -> Result<()> {
    write_segment_with_family(path, batches, Family::Orderbook)
}
pub fn write_segment_with_family(
    path: &Path,
    batches: &[RecordBatch],
    family: Family,
) -> Result<()> {
    ensure!(
        !batches.is_empty() && batches.len() <= MAX_SEGMENT_BLOCKS,
        "segment block bound"
    );
    let schema = batches[0].schema();
    let mut rows = 0_usize;
    for batch in batches {
        ensure!(batch.schema() == schema, "segment schema changed");
        validate_schema(batch.schema().as_ref(), family)?;
        rows = rows.checked_add(batch.num_rows()).context("row overflow")?;
        ensure!(rows <= MAX_SEGMENT_ROWS, "segment row bound");
        encode(batch, family)?;
    }
    let mut writer = gqodb_store::Writer::create_with_codec(
        path,
        schema,
        "available_utc_ns",
        "symbol",
        family.mode(),
    )?;
    for batch in batches {
        writer.append(batch)?;
    }
    writer.finish()
}
pub fn fixture(family: Family) -> Result<RecordBatch> {
    let mut metadata = HashMap::from([
        ("gqodb.family".into(), family.name().into()),
        ("gqodb.schema_version".into(), "1".into()),
        ("gqodb.source_sha256".into(), "1".repeat(64)),
        ("gqodb.price_decimals".into(), "2".into()),
        ("gqodb.quantity_decimals".into(), "0".into()),
    ]);
    let mut columns: Vec<(&str, ArrayRef)> = vec![
        (
            "source_utc_ns",
            Arc::new(Int64Array::from(vec![Some(8), None, Some(10)])),
        ),
        (
            "received_utc_ns",
            Arc::new(Int64Array::from(vec![9, 10, 11])),
        ),
        (
            "clock_error_ns",
            Arc::new(Int64Array::from(vec![Some(2), None, Some(2)])),
        ),
        (
            "available_utc_ns",
            Arc::new(Int64Array::from(vec![10, 11, 12])),
        ),
        ("sequence", Arc::new(Int64Array::from(vec![1, 2, 3]))),
        (
            "symbol",
            Arc::new(LargeStringArray::from(vec!["SYNTH", "SYNTH", "SYNTH"])),
        ),
        ("price", Arc::new(Int64Array::from(vec![100, 100, 101]))),
        ("quantity", Arc::new(Int64Array::from(vec![2, 2, 3]))),
        ("side", Arc::new(Int8Array::from(vec![0, 0, 1]))),
    ];
    match family {
        Family::Tick => {
            metadata.insert("gqodb.subtype".into(), "trade".into());
        }
        Family::Orderbook => {
            metadata.insert("gqodb.depth".into(), "10".into());
            columns.push(("level", Arc::new(Int64Array::from(vec![1, 1, 1]))));
            columns.push(("action", Arc::new(Int8Array::from(vec![0, 1, 1]))));
        }
    }
    let fields = columns
        .iter()
        .map(|(name, a)| Field::new(*name, a.data_type().clone(), a.null_count() > 0))
        .collect::<Vec<_>>();
    RecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(fields, metadata)),
        columns.into_iter().map(|(_, a)| a).collect(),
    )
    .map_err(Into::into)
}
