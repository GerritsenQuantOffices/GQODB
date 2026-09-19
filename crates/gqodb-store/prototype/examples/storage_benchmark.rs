//! B12: durable file writes, warm reads and indexed queries on identical book rows.
#![forbid(unsafe_code)]
use anyhow::{Result, ensure};
use arrow_array::{Array, BooleanArray, Int64Array, LargeStringArray, RecordBatch};
use arrow_select::{concat::concat_batches, filter::filter_record_batch};
use gqodb_blocks::{market, real};
use gqodb_store::{Reader, Writer};
use parquet::{
    arrow::{ArrowWriter, arrow_reader::ParquetRecordBatchReaderBuilder},
    basic::{Compression, Encoding},
    file::{
        metadata::KeyValue,
        properties::{EnabledStatistics, WriterProperties, WriterVersion},
        statistics::Statistics,
    },
};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    path::Path,
    sync::Arc,
    time::Instant,
};

#[derive(Clone, Copy, Serialize)]
enum Mode {
    Gqodb,
    DictionaryLz4,
    DeltaLz4,
}
impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::Gqodb => "gqodb",
            Self::DictionaryLz4 => "parquet-dictionary-lz4",
            Self::DeltaLz4 => "parquet-delta-lz4",
        }
    }
}
#[derive(Clone, Serialize)]
struct Query {
    name: String,
    symbol: String,
    start: i64,
    end: i64,
}

fn filtered(batches: &[RecordBatch], q: &Query) -> Result<Vec<RecordBatch>> {
    let mut out = Vec::new();
    for b in batches {
        let t = b
            .column_by_name("receipt_ns")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        let s = b
            .column_by_name("symbol")
            .unwrap()
            .as_any()
            .downcast_ref::<LargeStringArray>()
            .unwrap();
        let mask = BooleanArray::from(
            (0..b.num_rows())
                .map(|i| t.value(i) >= q.start && t.value(i) <= q.end && s.value(i) == q.symbol)
                .collect::<Vec<_>>(),
        );
        let b = filter_record_batch(b, &mask)?;
        if b.num_rows() > 0 {
            out.push(b);
        }
    }
    Ok(out)
}

fn write(path: &Path, input: &[RecordBatch], mode: Mode) -> Result<()> {
    if matches!(mode, Mode::Gqodb) {
        let mut writer = Writer::create(path, input[0].schema(), "receipt_ns", "symbol")?;
        for b in input {
            writer.append(b)?;
        }
        writer.finish()
    } else {
        let props = WriterProperties::builder()
            .set_writer_version(WriterVersion::PARQUET_2_0)
            .set_max_row_group_row_count(Some(32768))
            .set_data_page_row_count_limit(65536)
            .set_data_page_size_limit(1048576)
            .set_statistics_enabled(EnabledStatistics::Page)
            .set_dictionary_enabled(matches!(mode, Mode::DictionaryLz4))
            .set_compression(Compression::LZ4_RAW);
        let mut props = props;
        let metadata = input[0]
            .schema()
            .metadata()
            .iter()
            .map(|(k, v)| KeyValue::new(k.clone(), v.clone()))
            .collect::<Vec<_>>();
        props = props.set_key_value_metadata(Some(metadata));
        for field in input[0].schema().fields() {
            let encoding = if field.name() == "symbol" {
                Encoding::DELTA_BYTE_ARRAY
            } else {
                Encoding::DELTA_BINARY_PACKED
            };
            props = props.set_column_encoding(field.name().as_str().into(), encoding);
        }
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        let mut writer = ArrowWriter::try_new(file, input[0].schema(), Some(props.build()))?;
        for b in input {
            writer.write(b)?;
            writer.flush()?;
        }
        let file = writer.into_inner()?;
        file.sync_all()?;
        Ok(())
    }
}

fn read(path: &Path, mode: Mode, query: Option<&Query>) -> Result<(Vec<RecordBatch>, usize)> {
    if matches!(mode, Mode::Gqodb) {
        let mut reader = Reader::open(path)?;
        if let Some(q) = query {
            let result = reader.query(&q.symbol, q.start, q.end)?;
            Ok((result.batches, result.stats.blocks_read))
        } else {
            let n = reader.blocks().len();
            Ok((reader.read_all()?, n))
        }
    } else {
        let builder = ParquetRecordBatchReaderBuilder::try_new(File::open(path)?)?;
        let schema = builder.schema().clone();
        let metadata: HashMap<String, String> = builder
            .metadata()
            .file_metadata()
            .key_value_metadata()
            .into_iter()
            .flatten()
            .filter(|kv| kv.key != "ARROW:schema")
            .filter_map(|kv| kv.value.as_ref().map(|v| (kv.key.clone(), v.clone())))
            .collect();
        let restored_schema = Arc::new(arrow_schema::Schema::new_with_metadata(
            schema.fields().clone(),
            metadata,
        ));
        let ti = schema.index_of("receipt_ns")?;
        let si = schema.index_of("symbol")?;
        let groups: Vec<_> = builder
            .metadata()
            .row_groups()
            .iter()
            .enumerate()
            .filter_map(|(i, group)| {
                if let Some(q) = query {
                    if let Some(Statistics::Int64(s)) = group.column(ti).statistics() {
                        if s.min_opt().is_some_and(|v| *v > q.end)
                            || s.max_opt().is_some_and(|v| *v < q.start)
                        {
                            return None;
                        }
                    }
                    if let Some(Statistics::ByteArray(s)) = group.column(si).statistics() {
                        if s.min_opt().is_some_and(|v| v.data() > q.symbol.as_bytes())
                            || s.max_opt().is_some_and(|v| v.data() < q.symbol.as_bytes())
                        {
                            return None;
                        }
                    }
                }
                Some(i)
            })
            .collect();
        let count = groups.len();
        let batches = builder
            .with_row_groups(groups)
            .with_batch_size(32768)
            .build()?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let batches = batches
            .into_iter()
            .map(|b| RecordBatch::try_new(restored_schema.clone(), b.columns().to_vec()))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let batches = if let Some(q) = query {
            filtered(&batches, q)?
        } else {
            batches
        };
        Ok((batches, count))
    }
}

#[derive(Serialize)]
struct QuerySample {
    name: String,
    elapsed_ns: u128,
    blocks_read: usize,
    rows: usize,
}
#[derive(Serialize)]
struct Sample {
    round: usize,
    mode: Mode,
    path: String,
    bytes: u64,
    write_sync_ns: u128,
    read_ns: u128,
    queries: Vec<QuerySample>,
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() == 4 || (args.len() == 5 && args[4] == "--parquet-projection"),
        "usage: storage_benchmark SOURCE NEW_ARTIFACT_DIRECTORY NEW_REPORT.json [--parquet-projection]"
    );
    ensure!(
        !Path::new(&args[2]).exists() && !Path::new(&args[3]).exists(),
        "output exists"
    );
    let sha = market::hash(&args[1])?;
    let (input, provenance) = if args.len() == 5 {
        ensure!(
            fs::metadata(&args[1])?.len() <= 256 * 1024 * 1024,
            "projection file budget"
        );
        let builder = ParquetRecordBatchReaderBuilder::try_new(File::open(&args[1])?)?;
        ensure!(
            (1..=1_048_576).contains(&builder.metadata().file_metadata().num_rows()),
            "projection row budget"
        );
        drop(builder);
        let (batches, _) = read(Path::new(&args[1]), Mode::DictionaryLz4, None)?;
        ensure!(
            batches
                .iter()
                .map(RecordBatch::get_array_memory_size)
                .sum::<usize>()
                <= 256 * 1024 * 1024,
            "projection resident budget"
        );
        (
            batches,
            serde_json::json!({"projection":"all supplied Parquet columns/schema, receipt_ns/symbol indexed; source raw not reread", "input_kind":"materialized_market_projection", "source_identity":"SHA256 of the exact supplied Parquet file; attach originating dataset manifest separately"}),
        )
    } else {
        let expected = fs::read_to_string(format!("{}.sha256", args[1]))?;
        ensure!(
            expected.split_whitespace().next() == Some(sha.as_str()),
            "source SHA mismatch"
        );
        market::book(&args[1], 1_048_576, 32768)?
    };
    let schema = input[0].schema();
    let reference = concat_batches(&schema, &input)?;
    let t = reference
        .column_by_name("receipt_ns")
        .unwrap()
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap();
    let s = reference
        .column_by_name("symbol")
        .unwrap()
        .as_any()
        .downcast_ref::<LargeStringArray>()
        .unwrap();
    let mut queries = Vec::new();
    for percent in [25, 50, 75] {
        let i = reference.num_rows() * percent / 100;
        let start = t.value(i);
        queries.push(Query {
            name: format!("window-{percent}-10ms"),
            symbol: s.value(i).into(),
            start,
            end: start.checked_add(10_000_000).unwrap(),
        });
    }
    queries.push(Query {
        name: "full-symbol".into(),
        symbol: s.value(0).into(),
        start: i64::MIN,
        end: i64::MAX,
    });
    queries.push(Query {
        name: "absent-symbol".into(),
        symbol: "ZZZ_NOT_PRESENT".into(),
        start: i64::MIN,
        end: i64::MAX,
    });
    let expected_queries = queries
        .iter()
        .map(|q| concat_batches(&schema, &filtered(&input, q)?).map_err(Into::into))
        .collect::<Result<Vec<_>>>()?;
    fs::create_dir(&args[2])?;
    let read_only = std::env::var("GQODB_READ_ONLY").ok();
    let mut profiles = Vec::new();
    if let Some(directory) = &read_only {
        if std::env::var("GQODB_READ_PROFILE").as_deref() == Ok("1") {
            for _ in 0..3 {
                let (batches, profile) =
                    Reader::open(Path::new(directory).join("gqodb-6.gqodb.ob"))?
                        .profile_read_all()?;
                ensure!(
                    batches.iter().all(|b| b.schema() == schema)
                        && real::equal(&reference, &concat_batches(&schema, &batches)?),
                    "profile mismatch"
                );
                profiles.push(profile);
            }
        }
    }
    let modes = [Mode::Gqodb, Mode::DictionaryLz4, Mode::DeltaLz4];
    let mut samples = Vec::new();
    // Round zero is a separate warmup; six measured rounds balance order positions.
    for round in 0..=6 {
        for offset in 0..3 {
            let index = if round % 2 == 0 {
                (round / 2 + offset) % 3
            } else {
                (round / 2 + 2 - offset) % 3
            };
            let mode = modes[index];
            let extension = if matches!(mode, Mode::Gqodb) {
                "gqodb.ob"
            } else {
                "parquet"
            };
            let path = if let Some(directory) = &read_only {
                Path::new(directory).join(format!("{}-6.{extension}", mode.name()))
            } else {
                Path::new(&args[2]).join(format!("{}-{round}.{extension}", mode.name()))
            };
            let begin = Instant::now();
            if read_only.is_none() {
                write(&path, &input, mode)?;
            }
            let write_sync_ns = if read_only.is_some() {
                0
            } else {
                begin.elapsed().as_nanos()
            };
            let bytes = fs::metadata(&path)?.len();
            let begin = Instant::now();
            let (decoded, _) = read(&path, mode, None)?;
            let read_ns = begin.elapsed().as_nanos();
            ensure!(
                decoded.iter().all(|b| b.schema() == schema),
                "full read schema mismatch"
            );
            ensure!(
                real::equal(&reference, &concat_batches(&schema, &decoded)?),
                "full read mismatch"
            );
            let mut results = Vec::new();
            for (q, expected) in queries.iter().zip(&expected_queries) {
                let begin = Instant::now();
                let (actual, blocks) = read(&path, mode, Some(q))?;
                let elapsed_ns = begin.elapsed().as_nanos();
                ensure!(
                    actual.iter().all(|b| b.schema() == schema),
                    "query schema mismatch"
                );
                let actual = concat_batches(&schema, &actual)?;
                ensure!(real::equal(expected, &actual), "query mismatch: {}", q.name);
                results.push(QuerySample {
                    name: q.name.clone(),
                    elapsed_ns,
                    blocks_read: blocks,
                    rows: actual.num_rows(),
                });
            }
            println!(
                "round {round} {}: {bytes} bytes, write+sync {:.2} ms, read {:.2} ms",
                mode.name(),
                write_sync_ns as f64 / 1e6,
                read_ns as f64 / 1e6
            );
            samples.push(Sample {
                round,
                mode,
                path: path.display().to_string(),
                bytes,
                write_sync_ns,
                read_ns,
                queries: results,
            });
        }
    }
    let report = serde_json::json!({"experiment":if read_only.is_some() { "B13" } else { "B12" }, "args":args, "source_sha256":sha,
        "read_only_files":read_only,"diagnostic_profiles":profiles,
        "provenance":provenance, "rows":reference.num_rows(), "queries":queries, "samples":samples,
        "warmup_round":0, "measured_rounds":6, "cache":"warm/page-cache; no cache eviction",
        "durability":"file sync_all at end for every codec; directory sync and atomic publication excluded",
        "comparison":"Parquet dictionary-LZ4 and delta-LZ4; row-group min/max pruning, no page-level filtering",
        "memory":fs::read_to_string("/proc/self/status")?.lines().filter(|l|l.starts_with("VmHWM:")).collect::<Vec<_>>()});
    fs::write(&args[3], serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parquet_file_control_preserves_schema_metadata() -> Result<()> {
        let schema = Arc::new(arrow_schema::Schema::new_with_metadata(
            vec![
                arrow_schema::Field::new("receipt_ns", arrow_schema::DataType::Int64, false),
                arrow_schema::Field::new("symbol", arrow_schema::DataType::LargeUtf8, false),
            ],
            HashMap::from([("price_scale".into(), "100000000".into())]),
        ));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(vec![1, 2])),
                Arc::new(LargeStringArray::from(vec!["BTC", "ETH"])),
            ],
        )?;
        for mode in [Mode::DictionaryLz4, Mode::DeltaLz4] {
            let path = std::env::temp_dir().join(format!(
                "gqodb-parquet-control-{}-{}.parquet",
                std::process::id(),
                mode.name()
            ));
            write(&path, std::slice::from_ref(&batch), mode)?;
            let (decoded, _) = read(&path, mode, None)?;
            fs::remove_file(path)?;
            ensure!(
                decoded[0].schema() == schema && real::equal(&decoded[0], &batch),
                "metadata lost"
            );
        }
        Ok(())
    }
}
