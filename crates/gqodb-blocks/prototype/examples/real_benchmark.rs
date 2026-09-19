//! B03: compare gqodb and Parquet on a real local market-data file.
//! All ingestion, timing, checksum verification and reporting is Rust.
#![forbid(unsafe_code)]

use anyhow::{Result, ensure};
use arrow_array::RecordBatch;
use bytes::Bytes;
use gqodb_blocks::real::{self, Mode};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Default, Serialize)]
struct ModeResult {
    mode: String,
    bytes: u64,
    encode_ns: u128,
    decode_ns: u128,
    blocks: usize,
    rows: u64,
    exact_roundtrips: u64,
}

#[derive(Serialize)]
struct Report {
    experiment: &'static str,
    source: String,
    source_bytes: u64,
    source_sha256: String,
    source_rows: i64,
    source_row_groups: usize,
    batch_rows: usize,
    repetitions: usize,
    modes: Vec<ModeResult>,
    caveats: Vec<&'static str>,
}

fn hash_file(path: &Path) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn load_batches(path: &Path, batch_rows: usize) -> Result<(Vec<RecordBatch>, i64, usize)> {
    let file = fs::File::open(path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    let rows = builder.metadata().file_metadata().num_rows();
    let groups = builder.metadata().num_row_groups();
    let reader = builder.with_batch_size(batch_rows).build()?;
    let mut batches = Vec::new();
    for batch in reader {
        batches.push(batch?);
    }
    ensure!(
        !batches.is_empty()
            && batches.iter().map(RecordBatch::num_rows).sum::<usize>() == rows as usize,
        "source row count mismatch"
    );
    Ok((batches, rows, groups))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help") || args.len() < 3 {
        eprintln!(
            "usage: real_benchmark SOURCE.parquet OUTPUT_DIR [--batch-rows N] [--repetitions N]"
        );
        std::process::exit(2);
    }
    let source = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let batch_rows = args
        .windows(2)
        .find(|w| w[0] == "--batch-rows")
        .map(|w| w[1].parse())
        .transpose()?
        .unwrap_or(32_768);
    let repetitions = args
        .windows(2)
        .find(|w| w[0] == "--repetitions")
        .map(|w| w[1].parse())
        .transpose()?
        .unwrap_or(5);
    ensure!(
        source.is_file(),
        "source is not a file: {}",
        source.display()
    );
    ensure!(
        (1..=100).contains(&repetitions) && (1..=1_048_576).contains(&batch_rows),
        "invalid benchmark parameters"
    );
    ensure!(
        !output.exists(),
        "output already exists: {}",
        output.display()
    );
    fs::create_dir_all(&output)?;
    let (batches, source_rows, source_groups) = load_batches(&source, batch_rows)?;
    let source_bytes = fs::metadata(&source)?.len();
    let source_sha256 = hash_file(&source)?;
    let mut results = Vec::new();
    for mode in Mode::ALL {
        let mut result = ModeResult {
            mode: mode.name().to_owned(),
            ..Default::default()
        };
        for _rep in 0..repetitions {
            for batch in &batches {
                let start = Instant::now();
                let encoded = real::encode(batch, mode)?;
                result.encode_ns += start.elapsed().as_nanos();
                result.bytes += encoded.len() as u64;
                let start = Instant::now();
                let decoded = real::decode(Bytes::from(encoded), mode)?;
                result.decode_ns += start.elapsed().as_nanos();
                ensure!(
                    real::equal(batch, &decoded),
                    "exact roundtrip failed for {}",
                    mode.name()
                );
                black_box(decoded);
                result.exact_roundtrips += 1;
                result.rows += batch.num_rows() as u64;
                result.blocks += 1;
            }
        }
        // Keep a single-repetition byte count; bytes are identical across repetitions.
        result.bytes /= repetitions as u64;
        result.rows /= repetitions as u64;
        result.blocks /= repetitions;
        results.push(result);
    }
    let mut md = format!(
        "# B03 real market-data comparison\n\nSource: `{}`\n\n- source bytes: {}\n- source SHA-256: `{}`\n- rows: {}\n- row groups: {}\n- benchmark batch size: {} rows\n- repetitions: {}\n\nThe source is a local Dukascopy FRAIDXEUR market-data Parquet file. Each candidate\nreads the same Arrow batches and must reproduce the exact schema, nulls, strings and\nbitwise float values. Bytes are the sum of independent encoded blocks for one pass.\nTimes include allocation, normalization/encoding, checksums, decoding and restoring\nthe original Arrow representation; source generation and the initial source read are\noutside the timed section.\n\n| Mode | Encoded bytes | Bytes/row | Encode ms/pass | Decode ms/pass | Encode Mrow/s | Decode Mrow/s | Exact blocks |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
        source.display(),
        source_bytes,
        source_sha256,
        source_rows,
        source_groups,
        batch_rows,
        repetitions
    );
    for result in &results {
        let enc_ms = result.encode_ns as f64 / repetitions as f64 / 1e6;
        let dec_ms = result.decode_ns as f64 / repetitions as f64 / 1e6;
        let rows = result.rows as f64;
        md.push_str(&format!(
            "| {} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {} |\n",
            result.mode,
            result.bytes,
            result.bytes as f64 / rows,
            enc_ms,
            dec_ms,
            rows / (enc_ms * 1000.0),
            rows / (dec_ms * 1000.0),
            result.exact_roundtrips
        ));
    }
    md.push_str("\n## Interpretation\n\nThe existing source file is included as provenance, not as a controlled baseline: it has its own partitioning, row-group and codec settings. The controlled comparison is between the modes above, all written from identical decoded batches. `gqodb-adaptive-lz4` is a custom format; the other modes are Parquet configurations. No result here establishes order-book correctness, persistence, partial reads, live ingestion or market-wide superiority.\n\nThe test uses at most `batch_rows` records in each processing batch; source batches are retained so every mode sees identical input. The output is a research artifact, not a production data copy.\n");
    fs::write(output.join("REPORT.md"), md)?;
    let report = Report {
        experiment: "B03",
        source: source.display().to_string(),
        source_bytes,
        source_sha256,
        source_rows,
        source_row_groups: source_groups,
        batch_rows,
        repetitions,
        modes: results,
        caveats: vec![
            "local real market data only",
            "controlled in-memory batch comparison",
            "no order-book stream in this source",
            "no disk/durability/query benchmark",
            "no paid data downloaded",
        ],
    };
    fs::write(
        output.join("results.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("wrote {}", output.display());
    Ok(())
}
