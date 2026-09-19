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
    let builder = ParquetRecordBatchReaderBuilder::try_new(checked_file(input)?)?;
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
    let reader = builder.with_batch_size(rows.max(1) as usize).build()?;
    for batch in reader {
        let batch = batch?;
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
    let reader = ParquetRecordBatchReaderBuilder::try_new(checked_file(input)?)?;
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
