//! B03: exact Arrow schema and values, with reversible numeric normalization.
use anyhow::{Result, bail, ensure};
use arrow_array::{
    Array, ArrayRef, BooleanArray, Float64Array, Int8Array, Int64Array, LargeStringArray,
    RecordBatch, TimestampMillisecondArray, builder::LargeStringBuilder,
};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use bytes::Bytes;
use parquet::{
    arrow::{ArrowWriter, arrow_reader::ParquetRecordBatchReaderBuilder},
    basic::{BrotliLevel, Compression, Encoding},
    file::metadata::KeyValue,
    file::properties::{EnabledStatistics, WriterProperties, WriterVersion},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Mode {
    Gqodb,
    GqodbBook,
    ParquetLz4,
    ParquetDictionaryLz4,
    ParquetSnappy,
    ParquetBrotli,
    ParquetNormalizedLz4,
}
impl Mode {
    pub const ALL: [Self; 6] = [
        Self::Gqodb,
        Self::ParquetLz4,
        Self::ParquetDictionaryLz4,
        Self::ParquetSnappy,
        Self::ParquetBrotli,
        Self::ParquetNormalizedLz4,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Gqodb => "gqodb-adaptive-lz4",
            Self::GqodbBook => "gqodb-book-runs-planes-lz4",
            Self::ParquetLz4 => "parquet-delta-bss-lz4",
            Self::ParquetDictionaryLz4 => "parquet-dictionary-lz4",
            Self::ParquetSnappy => "parquet-delta-bss-snappy",
            Self::ParquetBrotli => "parquet-delta-bss-brotli5",
            Self::ParquetNormalizedLz4 => "parquet-normalized-lz4",
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Column {
    value: usize,
    validity: Option<usize>,
    scale: Option<f64>,
    dictionary: Vec<String>,
}
#[derive(Serialize, Deserialize)]
struct Layout {
    schema: Schema,
    columns: Vec<Column>,
}

fn normalize(batch: &RecordBatch) -> Result<(Layout, Vec<Vec<i64>>)> {
    let mut values = Vec::new();
    let mut columns = Vec::new();
    for array in batch.columns() {
        let mut column = Column {
            value: values.len(),
            validity: None,
            scale: None,
            dictionary: Vec::new(),
        };
        let v = match array.data_type() {
            DataType::Int8 => array
                .as_any()
                .downcast_ref::<Int8Array>()
                .unwrap()
                .iter()
                .map(|v| i64::from(v.unwrap_or(0)))
                .collect(),
            DataType::Timestamp(TimeUnit::Millisecond, _) => array
                .as_any()
                .downcast_ref::<TimestampMillisecondArray>()
                .unwrap()
                .iter()
                .map(|v| v.unwrap_or(0))
                .collect(),
            DataType::Int64 => array
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap()
                .iter()
                .map(|v| v.unwrap_or(0))
                .collect(),
            DataType::Float64 => {
                let a = array.as_any().downcast_ref::<Float64Array>().unwrap();
                // Select from a prefix, then prove bit-exact reversibility on EVERY nonnull value.
                let scale = (0..=9).map(|n| 10f64.powi(n)).find(|&s| {
                    a.iter()
                        .take(1024)
                        .flatten()
                        .all(|v| exact_scaled(v, s).is_some())
                });
                if let Some(s) = scale {
                    let scaled: Option<Vec<i64>> = if a.null_count() == 0 {
                        a.values().iter().map(|&v| exact_scaled(v, s)).collect()
                    } else {
                        a.iter()
                            .map(|v| match v {
                                None => Some(0),
                                Some(v) => exact_scaled(v, s),
                            })
                            .collect()
                    };
                    if let Some(v) = scaled {
                        column.scale = Some(s);
                        v
                    } else {
                        a.iter()
                            .map(|v| v.unwrap_or(0.0).to_bits() as i64)
                            .collect()
                    }
                } else {
                    a.iter()
                        .map(|v| v.unwrap_or(0.0).to_bits() as i64)
                        .collect()
                }
            }
            DataType::Boolean => array
                .as_any()
                .downcast_ref::<BooleanArray>()
                .unwrap()
                .iter()
                .map(|v| i64::from(v.unwrap_or(false)))
                .collect(),
            DataType::LargeUtf8 => {
                let a = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
                let mut ids = HashMap::new();
                let mut v = Vec::with_capacity(a.len());
                for s in a.iter() {
                    let s = s.unwrap_or("");
                    let id = *ids.entry(s).or_insert_with(|| {
                        let id = column.dictionary.len() as i64;
                        column.dictionary.push(s.to_owned());
                        id
                    });
                    v.push(id);
                }
                v
            }
            t => bail!("unsupported real source type: {t:?}; no columns are silently dropped"),
        };
        values.push(v);
        if array.null_count() > 0 {
            column.validity = Some(values.len());
            values.push(
                (0..array.len())
                    .map(|i| i64::from(array.is_valid(i)))
                    .collect(),
            );
        }
        columns.push(column);
    }
    Ok((
        Layout {
            schema: batch.schema().as_ref().clone(),
            columns,
        },
        values,
    ))
}
#[inline]
fn exact_scaled(v: f64, scale: f64) -> Option<i64> {
    let scaled = (v * scale).round();
    if !scaled.is_finite() || scaled < i64::MIN as f64 || scaled >= i64::MAX as f64 {
        return None;
    }
    let n = scaled as i64;
    ((n as f64 / scale).to_bits() == v.to_bits()).then_some(n)
}
fn restore(layout: Layout, mut values: Vec<Vec<i64>>) -> Result<RecordBatch> {
    ensure!(
        layout.columns.len() == layout.schema.fields().len(),
        "schema dimensions"
    );
    let mut arrays = Vec::new();
    // Metadata may alias normalized columns (including validity). Move a vector
    // only on its last use; older/aliased layouts retain their original meaning.
    let mut uses = vec![0usize; values.len()];
    for column in &layout.columns {
        *uses
            .get_mut(column.value)
            .ok_or_else(|| anyhow::anyhow!("bad column"))? += 1;
        if let Some(i) = column.validity {
            *uses
                .get_mut(i)
                .ok_or_else(|| anyhow::anyhow!("bad validity"))? += 1;
        }
    }
    for (field, column) in layout.schema.fields().iter().zip(layout.columns) {
        if field.data_type() == &DataType::Int64
            && column.validity.is_none()
            && uses[column.value] == 1
        {
            arrays.push(
                Arc::new(Int64Array::from(std::mem::take(&mut values[column.value]))) as ArrayRef,
            );
            uses[column.value] -= 1;
            continue;
        }
        let v = values
            .get(column.value)
            .ok_or_else(|| anyhow::anyhow!("bad column"))?;
        let validity = column
            .validity
            .map(|i| values.get(i).ok_or_else(|| anyhow::anyhow!("bad validity")))
            .transpose()?;
        if let Some(validity) = validity {
            ensure!(
                validity.len() == v.len() && validity.iter().all(|&v| v == 0 || v == 1),
                "invalid validity"
            );
        }
        let valid = |i: usize| validity.is_none_or(|a| a[i] == 1);
        let a: ArrayRef = match field.data_type() {
            DataType::Int8 => {
                ensure!(v.iter().all(|&v| i8::try_from(v).is_ok()), "invalid int8");
                Arc::new(Int8Array::from_iter(
                    v.iter()
                        .enumerate()
                        .map(|(i, &v)| valid(i).then_some(v as i8)),
                ))
            }
            DataType::Timestamp(TimeUnit::Millisecond, tz) => Arc::new(
                TimestampMillisecondArray::from_iter(
                    v.iter().enumerate().map(|(i, &v)| valid(i).then_some(v)),
                )
                .with_timezone_opt(tz.clone()),
            ),
            DataType::Int64 if validity.is_none() => Arc::new(Int64Array::from(v.clone())),
            DataType::Float64 if validity.is_none() => {
                let floats: Vec<f64> = match column.scale {
                    Some(s) => v.iter().map(|&v| v as f64 / s).collect(),
                    None => v.iter().map(|&v| f64::from_bits(v as u64)).collect(),
                };
                Arc::new(Float64Array::from(floats))
            }
            DataType::Int64 => Arc::new(Int64Array::from_iter(
                v.iter().enumerate().map(|(i, &v)| valid(i).then_some(v)),
            )),
            DataType::Float64 => Arc::new(Float64Array::from_iter(v.iter().enumerate().map(
                |(i, &v)| {
                    valid(i).then(|| {
                        column
                            .scale
                            .map_or_else(|| f64::from_bits(v as u64), |s| v as f64 / s)
                    })
                },
            ))),
            DataType::Boolean => {
                ensure!(v.iter().all(|&v| v == 0 || v == 1), "invalid bool");
                Arc::new(BooleanArray::from_iter(
                    v.iter()
                        .enumerate()
                        .map(|(i, &v)| valid(i).then_some(v != 0)),
                ))
            }
            DataType::LargeUtf8 => {
                ensure!(
                    v.iter()
                        .all(|&v| v >= 0 && (v as usize) < column.dictionary.len()),
                    "bad dictionary index"
                );
                if validity.is_none() {
                    let bytes = v.iter().try_fold(0usize, |n, &id| {
                        n.checked_add(column.dictionary[id as usize].len())
                            .ok_or_else(|| anyhow::anyhow!("string capacity overflow"))
                    })?;
                    let mut builder = LargeStringBuilder::with_capacity(v.len(), bytes);
                    let mut start = 0;
                    while start < v.len() {
                        let mut end = start + 1;
                        while end < v.len() && v[end] == v[start] {
                            end += 1;
                        }
                        builder.append_value_n(&column.dictionary[v[start] as usize], end - start);
                        start = end;
                    }
                    Arc::new(builder.finish())
                } else {
                    Arc::new(LargeStringArray::from_iter(v.iter().enumerate().map(
                        |(i, &v)| valid(i).then(|| column.dictionary[v as usize].as_str()),
                    )))
                }
            }
            t => bail!("unsupported {t:?}"),
        };
        arrays.push(a);
        uses[column.value] -= 1;
        if let Some(i) = column.validity {
            uses[i] -= 1;
        }
    }
    Ok(RecordBatch::try_new(Arc::new(layout.schema), arrays)?)
}

fn parquet_encode(batch: &RecordBatch, mode: Mode) -> Result<Vec<u8>> {
    let compression = match mode {
        Mode::ParquetSnappy => Compression::SNAPPY,
        Mode::ParquetBrotli => Compression::BROTLI(BrotliLevel::try_new(5)?),
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
        .set_max_row_group_row_count(Some(batch.num_rows().max(1)))
        .set_data_page_row_count_limit(65536)
        .set_data_page_size_limit(1048576)
        .set_statistics_enabled(EnabledStatistics::Page)
        .set_dictionary_enabled(matches!(mode, Mode::ParquetDictionaryLz4))
        .set_compression(compression)
        .set_key_value_metadata((!metadata.is_empty()).then_some(metadata));
    for field in batch.schema().fields() {
        let encoding = match field.data_type() {
            DataType::Int64 | DataType::Int8 | DataType::Timestamp(TimeUnit::Millisecond, _) => {
                Encoding::DELTA_BINARY_PACKED
            }
            DataType::Float64 => Encoding::BYTE_STREAM_SPLIT,
            DataType::LargeUtf8 => Encoding::DELTA_BYTE_ARRAY,
            _ => Encoding::PLAIN,
        };
        props = props.set_column_encoding(field.name().as_str().into(), encoding);
    }
    let mut out = Vec::new();
    let mut writer = ArrowWriter::try_new(&mut out, batch.schema(), Some(props.build()))?;
    writer.write(batch)?;
    writer.close()?;
    Ok(out)
}
pub fn encode(batch: &RecordBatch, mode: Mode) -> Result<Vec<u8>> {
    let mut out = match mode {
        Mode::Gqodb | Mode::GqodbBook => {
            let (layout, values) = normalize(batch)?;
            let meta = serde_json::to_vec(&layout)?;
            let payload = if matches!(mode, Mode::GqodbBook) {
                crate::book::encode(&values)?
            } else {
                crate::adaptive::encode_values(&values, true, 0)?
            };
            let mut out = if matches!(mode, Mode::GqodbBook) {
                b"GQOREAL4"
            } else {
                b"GQOREAL3"
            }
            .to_vec();
            out.extend_from_slice(&(meta.len() as u32).to_le_bytes());
            out.extend_from_slice(&meta);
            out.extend_from_slice(&payload);
            out
        }
        Mode::ParquetNormalizedLz4 => {
            let (layout, values) = normalize(batch)?;
            let fields: Vec<_> = (0..values.len())
                .map(|i| Field::new(format!("c{i}"), DataType::Int64, false))
                .collect();
            let schema = Arc::new(Schema::new_with_metadata(
                fields,
                HashMap::from([(
                    "gqodb.normalization".into(),
                    serde_json::to_string(&layout)?,
                )]),
            ));
            let arrays = values
                .into_iter()
                .map(|v| Arc::new(Int64Array::from(v)) as ArrayRef)
                .collect();
            parquet_encode(&RecordBatch::try_new(schema, arrays)?, mode)?
        }
        _ => parquet_encode(batch, mode)?,
    };
    out.extend_from_slice(&crc32fast::hash(&out).to_le_bytes());
    Ok(out)
}
pub fn decode(input: Bytes, mode: Mode) -> Result<RecordBatch> {
    Decoder::default().decode(input, mode)
}

/// Reusable per-worker LZ4 workspace. Returned batches own their data.
#[derive(Default)]
pub struct Decoder {
    scratch: Vec<u8>,
}

impl Decoder {
    /// Bytes retained for the largest decoded packed column so far.
    pub fn retained_bytes(&self) -> usize {
        self.scratch.capacity()
    }

    /// Release the workspace when a stream becomes idle.
    pub fn release(&mut self) {
        self.scratch = Vec::new();
    }

    pub fn decode(&mut self, input: Bytes, mode: Mode) -> Result<RecordBatch> {
        decode_with_scratch(input, mode, &mut self.scratch)
    }
}

fn decode_with_scratch(input: Bytes, mode: Mode, scratch: &mut Vec<u8>) -> Result<RecordBatch> {
    ensure!(
        input.len() >= 12 && input.len() <= 128 * 1024 * 1024,
        "invalid real block length"
    );
    let end = input.len() - 4;
    ensure!(
        crc32fast::hash(&input[..end]) == u32::from_le_bytes(input[end..].try_into().unwrap()),
        "real block checksum"
    );
    if matches!(mode, Mode::Gqodb | Mode::GqodbBook) {
        let book = matches!(mode, Mode::GqodbBook);
        ensure!(
            &input[..8] == if book { b"GQOREAL4" } else { b"GQOREAL3" },
            "real block magic"
        );
        let n = u32::from_le_bytes(input[8..12].try_into().unwrap()) as usize;
        ensure!(n <= end - 12, "bad schema length");
        let layout = serde_json::from_slice(&input[12..12 + n])?;
        let values = if book {
            crate::book::decode(&input[12 + n..end], scratch)?
        } else {
            let (kind, values) =
                crate::adaptive::decode_values_with_scratch(&input[12 + n..end], scratch)?;
            ensure!(kind == 0, "real block kind");
            values
        };
        restore(layout, values)
    } else {
        let builder = ParquetRecordBatchReaderBuilder::try_new(input.slice(..end))?;
        let n = builder.metadata().file_metadata().num_rows();
        ensure!(
            n > 0 && n <= crate::data::MAX_ROWS as i64,
            "invalid real rows"
        );
        let normalization = builder
            .metadata()
            .file_metadata()
            .key_value_metadata()
            .and_then(|items| {
                items
                    .iter()
                    .find(|kv| kv.key == "gqodb.normalization")
                    .and_then(|kv| kv.value.clone())
            });
        // The reader's batch schema can omit custom file metadata. Recover the
        // metadata explicitly written by parquet_encode; ARROW:schema is internal.
        let metadata: HashMap<String, String> = builder
            .metadata()
            .file_metadata()
            .key_value_metadata()
            .into_iter()
            .flatten()
            .filter(|kv| kv.key != "ARROW:schema")
            .filter_map(|kv| kv.value.as_ref().map(|v| (kv.key.clone(), v.clone())))
            .collect();
        let mut reader = builder.with_batch_size(n as usize).build()?;
        let batch = reader
            .next()
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("empty parquet"))?;
        ensure!(
            batch.num_rows() == n as usize && reader.next().is_none(),
            "unexpected batch count"
        );
        if matches!(mode, Mode::ParquetNormalizedLz4) {
            let meta = normalization
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("missing normalization"))?;
            let values = batch
                .columns()
                .iter()
                .map(|a| {
                    a.as_any()
                        .downcast_ref::<Int64Array>()
                        .ok_or_else(|| anyhow::anyhow!("bad normalized type"))
                        .map(|a| a.values().to_vec())
                })
                .collect::<Result<Vec<_>>>()?;
            restore(serde_json::from_str(meta)?, values)
        } else {
            Ok(RecordBatch::try_new(
                Arc::new(Schema::new_with_metadata(
                    batch.schema().fields().clone(),
                    metadata,
                )),
                batch.columns().to_vec(),
            )?)
        }
    }
}
/// Bitwise float equality (including signed zero and NaN payloads), schema, validity and strings.
pub fn equal(a: &RecordBatch, b: &RecordBatch) -> bool {
    if a.schema() != b.schema() || a.num_rows() != b.num_rows() {
        return false;
    }
    a.columns().iter().zip(b.columns()).all(|(a, b)| {
        if a.data_type() == &DataType::Float64 {
            let a = a.as_any().downcast_ref::<Float64Array>().unwrap();
            let b = b.as_any().downcast_ref::<Float64Array>().unwrap();
            a.iter()
                .zip(b.iter())
                .all(|(a, b)| a.map(f64::to_bits) == b.map(f64::to_bits))
        } else {
            a.to_data() == b.to_data()
        }
    })
}

#[cfg(test)]
mod restore_tests {
    use super::*;

    #[test]
    fn shared_value_and_validity_columns_survive_ownership_transfer() -> Result<()> {
        let schema = Schema::new(vec![
            Field::new("a", DataType::Int64, false),
            Field::new("b", DataType::Int64, true),
            Field::new("c", DataType::Int64, false),
        ]);
        let column = |value, validity| Column {
            value,
            validity,
            scale: None,
            dictionary: vec![],
        };
        let layout = Layout {
            schema: schema.clone(),
            columns: vec![column(0, None), column(1, Some(0)), column(0, None)],
        };
        let actual = restore(layout, vec![vec![1, 0, 1], vec![10, 20, 30]])?;
        let expected = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(Int64Array::from(vec![1, 0, 1])),
                Arc::new(Int64Array::from(vec![Some(10), None, Some(30)])),
                Arc::new(Int64Array::from(vec![1, 0, 1])),
            ],
        )?;
        assert!(equal(&actual, &expected));
        Ok(())
    }
}
