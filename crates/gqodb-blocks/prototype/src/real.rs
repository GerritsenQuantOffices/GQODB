//! B03: exact Arrow schema and values, with reversible numeric normalization.
use anyhow::{Result, bail, ensure};
use arrow_array::{
    Array, ArrayRef, BooleanArray, Float64Array, Int8Array, Int64Array, LargeStringArray,
    RecordBatch, TimestampMillisecondArray, builder::LargeStringBuilder,
};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use bytes::Bytes;
use parquet::{
    arrow::ArrowWriter,
    basic::{BrotliLevel, Compression, Encoding},
    file::metadata::KeyValue,
    file::properties::{EnabledStatistics, WriterProperties, WriterVersion},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

/// Upper bound on the decoded string bytes of one real block, summed over all `LargeUtf8`
/// columns: the sum, over the rows that are valid, of the length of the dictionary string the
/// row refers to. [`decode`] and [`Decoder::decode`] refuse a block above it before they allocate
/// any string memory, and [`encode`] refuses a batch above it so that everything the encoder
/// writes can be decoded again. It is the same 128 MiB as `gqodb_codec::MAX_DECODED_BYTES` and
/// the store's frame limit. Without it, a block of a few tens of KB whose dictionary holds one
/// long string and whose column is a long run of that string asked for `rows x string length`
/// bytes (62,914,560,000 for a 60 KB block).
pub const MAX_DECODED_STRING_BYTES: u64 = 128 * 1024 * 1024;

fn string_limit_error(limit: u64) -> anyhow::Error {
    anyhow::anyhow!(
        "decoded string bytes exceed the limit of {limit} bytes ({} MiB)",
        limit >> 20
    )
}

/// Decoded string bytes of one dictionary-coded column: the sum of the dictionary entry lengths
/// over the valid rows (all rows without a validity vector). Pure arithmetic, nothing is
/// allocated; saturating, so it cannot wrap; an error when the sum is above `budget`, when an id
/// is outside the dictionary or when the validity vector is shorter than the ids.
fn string_bytes(
    ids: &[i64],
    validity: Option<&[i64]>,
    dictionary: &[String],
    budget: u64,
) -> Result<u64> {
    let entry = |id: i64| -> Result<u64> {
        usize::try_from(id)
            .ok()
            .and_then(|id| dictionary.get(id))
            .map(|s| s.len() as u64)
            .ok_or_else(|| anyhow::anyhow!("bad dictionary index"))
    };
    let mut total = 0u64;
    match validity {
        None => {
            for &id in ids {
                total = total.saturating_add(entry(id)?);
            }
        }
        Some(validity) => {
            ensure!(validity.len() >= ids.len(), "invalid validity");
            for (&id, &valid) in ids.iter().zip(validity) {
                if valid == 1 {
                    total = total.saturating_add(entry(id)?);
                }
            }
        }
    }
    ensure!(total <= budget, string_limit_error(budget));
    Ok(total)
}

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

fn normalize(batch: &RecordBatch, string_limit: u64) -> Result<(Layout, Vec<Vec<i64>>)> {
    let mut values = Vec::new();
    let mut columns = Vec::new();
    // Decoded string bytes so far over all string columns (nulls count as empty strings).
    let mut string_total = 0u64;
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
                    // Checked before the string is copied into the dictionary.
                    string_total = string_total.saturating_add(s.len() as u64);
                    ensure!(
                        string_total <= string_limit,
                        string_limit_error(string_limit)
                    );
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
fn restore(layout: Layout, values: Vec<Vec<i64>>) -> Result<RecordBatch> {
    restore_with_limit(layout, values, MAX_DECODED_STRING_BYTES)
}
fn restore_with_limit(
    layout: Layout,
    mut values: Vec<Vec<i64>>,
    string_limit: u64,
) -> Result<RecordBatch> {
    ensure!(
        layout.columns.len() == layout.schema.fields().len(),
        "schema dimensions"
    );
    let mut arrays = Vec::new();
    // Decoded string bytes still allowed, shared by all string columns of the block.
    let mut string_budget = string_limit;
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
                // The decoded size is known before anything is allocated: refuse above the limit.
                let bytes = string_bytes(
                    v,
                    validity.map(|a| a.as_slice()),
                    &column.dictionary,
                    string_budget,
                )?;
                string_budget -= bytes;
                if validity.is_none() {
                    let bytes = usize::try_from(bytes)
                        .map_err(|_| anyhow::anyhow!("string capacity overflow"))?;
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
    // Encodings are keyed by column NAME. Arrow allows duplicate names, and a later column's
    // encoding would silently replace an earlier one's (e.g. DELTA_BINARY_PACKED applied to a
    // Float64 column makes the parquet crate panic). Refuse that instead.
    let mut chosen: HashMap<&str, Encoding> = HashMap::new();
    for field in batch.schema().fields() {
        let encoding = match field.data_type() {
            DataType::Int64 | DataType::Int8 | DataType::Timestamp(TimeUnit::Millisecond, _) => {
                Encoding::DELTA_BINARY_PACKED
            }
            DataType::Float64 => Encoding::BYTE_STREAM_SPLIT,
            DataType::LargeUtf8 => Encoding::DELTA_BYTE_ARRAY,
            _ => Encoding::PLAIN,
        };
        ensure!(
            chosen
                .insert(field.name(), encoding)
                .is_none_or(|e| e == encoding),
            "duplicate column name {:?} needs different Parquet encodings",
            field.name()
        );
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
            let (layout, values) = normalize(batch, MAX_DECODED_STRING_BYTES)?;
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
            let (layout, values) = normalize(batch, MAX_DECODED_STRING_BYTES)?;
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
        ensure!(end >= 12 && n <= end - 12, "bad schema length");
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
        let builder = crate::parquet::open(input.slice(..end))?;
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
        let mut reader =
            crate::parquet::contain(|| Ok(builder.with_batch_size(n as usize).build()?))?;
        let batch = crate::parquet::contain(|| Ok(reader.next().transpose()?))?
            .ok_or_else(|| anyhow::anyhow!("empty parquet"))?;
        ensure!(
            batch.num_rows() == n as usize
                && crate::parquet::contain(|| Ok(reader.next()))?.is_none(),
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

    fn strings(rows: &[&str]) -> RecordBatch {
        RecordBatch::try_from_iter(vec![(
            "s",
            Arc::new(LargeStringArray::from(rows.to_vec())) as ArrayRef,
        )])
        .unwrap()
    }

    #[test]
    fn string_bytes_counts_valid_rows_and_checks_the_boundary() {
        let dictionary = ["", "abc", "défg"].map(String::from); // lengths 0, 3, 5
        let ids = [1, 2, 2, 0, 1];
        // all rows valid: 3 + 5 + 5 + 0 + 3
        assert_eq!(string_bytes(&ids, None, &dictionary, 16).unwrap(), 16);
        assert!(string_bytes(&ids, None, &dictionary, 15).is_err());
        assert_eq!(string_bytes(&ids, None, &dictionary, u64::MAX).unwrap(), 16);
        // invalid rows decode to nothing and do not count
        let validity = [1, 0, 1, 1, 0];
        assert_eq!(
            string_bytes(&ids, Some(&validity), &dictionary, 8).unwrap(),
            8
        );
        assert!(string_bytes(&ids, Some(&validity), &dictionary, 7).is_err());
        assert_eq!(string_bytes(&[], None, &dictionary, 0).unwrap(), 0);
        // out-of-range ids and a short validity vector are errors, not panics
        assert!(string_bytes(&[3], None, &dictionary, 99).is_err());
        assert!(string_bytes(&[-1], None, &dictionary, 99).is_err());
        assert!(string_bytes(&ids, Some(&validity[..4]), &dictionary, 99).is_err());
    }

    #[test]
    fn string_bytes_sums_beyond_32_bits_exactly() {
        let dictionary = vec!["x".repeat(1 << 16)];
        let ids = vec![0i64; 1 << 16];
        assert_eq!(
            string_bytes(&ids, None, &dictionary, u64::MAX).unwrap(),
            1u64 << 32
        );
        assert!(string_bytes(&ids, None, &dictionary, (1u64 << 32) - 1).is_err());
    }

    #[test]
    fn decode_limit_matches_encode_limit_at_the_boundary() {
        // 3 + 5 + 3 = 11 string bytes in the first column, 2 + 2 in the second.
        let batch = RecordBatch::try_from_iter(vec![
            (
                "a",
                Arc::new(LargeStringArray::from(vec!["abc", "défg", "abc"])) as ArrayRef,
            ),
            (
                "b",
                Arc::new(LargeStringArray::from(vec![Some("hi"), None, Some("hi")])) as ArrayRef,
            ),
        ])
        .unwrap();
        let total = 11 + 4;
        // The limit is on the total over all string columns, valid rows only.
        for limit in [total - 1, total, total + 1, 0, u64::MAX] {
            let encoded = normalize(&batch, limit);
            let (layout, values) = normalize(&batch, u64::MAX).unwrap();
            let decoded = restore_with_limit(layout, values, limit);
            assert_eq!(encoded.is_ok(), limit >= total, "encode, limit {limit}");
            assert_eq!(decoded.is_ok(), limit >= total, "decode, limit {limit}");
            if let Ok(decoded) = decoded {
                assert!(equal(&decoded, &batch));
            }
        }
        // Whatever the encoder accepts under the real limit decodes again.
        for mode in [Mode::Gqodb, Mode::GqodbBook, Mode::ParquetNormalizedLz4] {
            let bytes = encode(&batch, mode).unwrap();
            assert!(equal(&decode(Bytes::from(bytes), mode).unwrap(), &batch));
        }
    }

    #[test]
    fn the_limit_is_shared_by_all_string_columns_of_a_block() {
        let layout = |dictionary: Vec<String>| Layout {
            schema: Schema::new(vec![
                Field::new("a", DataType::LargeUtf8, false),
                Field::new("b", DataType::LargeUtf8, false),
            ]),
            columns: vec![
                Column {
                    value: 0,
                    validity: None,
                    scale: None,
                    dictionary: dictionary.clone(),
                },
                Column {
                    value: 1,
                    validity: None,
                    scale: None,
                    dictionary,
                },
            ],
        };
        let values = || vec![vec![0i64; 10], vec![0i64; 10]]; // 10 rows of 5 bytes, twice
        let l = || layout(vec!["12345".into()]);
        assert!(restore_with_limit(l(), values(), 100).is_ok());
        assert!(restore_with_limit(l(), values(), 99).is_err());
        assert!(restore_with_limit(l(), values(), 50).is_err());
    }

    #[test]
    fn encoder_refuses_a_batch_that_could_not_be_decoded() {
        // Two rows of 64 MiB + 1 byte: 128 MiB + 2 bytes of decoded strings, two bytes over the
        // limit. The refusal comes at the second row, before its string would be copied.
        let big = "z".repeat((MAX_DECODED_STRING_BYTES / 2) as usize + 1);
        let mut builder = LargeStringBuilder::with_capacity(2, big.len() * 2);
        builder.append_value_n(&big, 2);
        let batch = RecordBatch::try_from_iter(vec![("s", Arc::new(builder.finish()) as ArrayRef)])
            .unwrap();
        for mode in [Mode::Gqodb, Mode::GqodbBook, Mode::ParquetNormalizedLz4] {
            let err = encode(&batch, mode).unwrap_err().to_string();
            assert!(err.contains("decoded string bytes exceed"), "{err}");
        }
        // One row alone is under the limit and still encodes.
        let one = strings(&[big.as_str()]);
        assert!(encode(&one, Mode::Gqodb).is_ok());
    }
}
