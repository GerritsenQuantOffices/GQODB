//! B05: alternating, warmed B04/current comparison in one process.
#![forbid(unsafe_code)]
#[allow(dead_code)]
#[path = "support/b04_adaptive.rs"]
mod adaptive;
#[allow(dead_code)]
#[path = "support/b04_real.rs"]
mod baseline;
pub use gqodb_blocks::data;

use anyhow::{Result, ensure};
use arrow_array::RecordBatch;
use bytes::Bytes;
use gqodb_blocks::{
    batch::Options,
    real::{self, Decoder, Mode},
};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, hint::black_box, time::Instant};

#[derive(Serialize)]
struct Sample {
    round: usize,
    mode: &'static str,
    encode_ns: u128,
    decode_ns: u128,
    bytes: usize,
}

fn pass(
    batches: &[RecordBatch],
    mode: usize,
    decoder: &mut Decoder,
    round: usize,
    workers: usize,
) -> Result<Sample> {
    let start = Instant::now();
    let encoded: Vec<Vec<u8>> = match mode {
        0 => batches
            .iter()
            .map(|b| baseline::encode(b, baseline::Mode::Gqodb))
            .collect::<Result<_>>()?,
        1 => Options::default().encode(batches, Mode::Gqodb)?,
        2 => Options { workers }.encode(batches, Mode::Gqodb)?,
        _ => Options::default().encode(batches, Mode::ParquetNormalizedLz4)?,
    };
    let encode_ns = start.elapsed().as_nanos();
    let bytes = encoded.iter().map(Vec::len).sum();
    let encoded: Vec<_> = encoded.into_iter().map(Bytes::from).collect();
    let start = Instant::now();
    let decoded = match mode {
        0 => encoded
            .iter()
            .cloned()
            .map(|b| baseline::decode(b, baseline::Mode::Gqodb))
            .collect::<Result<Vec<_>>>()?,
        1 => encoded
            .iter()
            .cloned()
            .map(|b| decoder.decode(b, Mode::Gqodb))
            .collect::<Result<Vec<_>>>()?,
        2 => Options { workers }.decode(&encoded, Mode::Gqodb)?,
        _ => Options::default().decode(&encoded, Mode::ParquetNormalizedLz4)?,
    };
    let decode_ns = start.elapsed().as_nanos();
    // Validate after both timings, so comparison encoding cannot warm decode caches.
    if mode == 1 || mode == 2 {
        for (batch, block) in batches.iter().zip(&encoded) {
            ensure!(
                block.as_ref() == baseline::encode(batch, baseline::Mode::Gqodb)?,
                "format changed"
            );
        }
    }
    ensure!(
        decoded.len() == batches.len()
            && batches.iter().zip(&decoded).all(|(a, b)| real::equal(a, b)),
        "roundtrip"
    );
    black_box(decoded);
    Ok(Sample {
        round,
        mode: ["b04", "b05-reuse", "b05-workers", "parquet-normalized"][mode],
        encode_ns,
        decode_ns,
        bytes,
    })
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() == 6,
        "usage: paired_benchmark SOURCE OUTPUT.json BATCH_ROWS REPETITIONS WORKERS"
    );
    let batch_rows: usize = args[3].parse()?;
    let reps: usize = args[4].parse()?;
    let workers: usize = args[5].parse()?;
    ensure!(
        (1..=65536).contains(&batch_rows)
            && (1..=100).contains(&reps)
            && (1..=64).contains(&workers),
        "invalid settings"
    );
    ensure!(!std::path::Path::new(&args[2]).exists(), "output exists");
    let source = fs::read(&args[1])?;
    let sha = format!("{:x}", Sha256::digest(&source));
    let batches = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(source))?
        .with_batch_size(batch_rows)
        .build()?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    let mut decoder = Decoder::default();
    for round in 0..3 {
        for mode in 0..4 {
            pass(&batches, mode, &mut decoder, round, workers)?;
        }
    }
    let mut samples = Vec::new();
    for round in 0..reps {
        // Rotate and reverse order to distribute first/last effects across modes.
        for offset in 0..4 {
            let mode = if round % 2 == 0 {
                (round / 2 + offset) % 4
            } else {
                (round / 2 + 3 - offset) % 4
            };
            samples.push(pass(&batches, mode, &mut decoder, round, workers)?);
        }
    }
    for name in ["b04", "b05-reuse", "b05-workers", "parquet-normalized"] {
        let mut enc: Vec<_> = samples
            .iter()
            .filter(|s| s.mode == name)
            .map(|s| s.encode_ns)
            .collect();
        let mut dec: Vec<_> = samples
            .iter()
            .filter(|s| s.mode == name)
            .map(|s| s.decode_ns)
            .collect();
        enc.sort_unstable();
        dec.sort_unstable();
        println!(
            "{name}: median encode {:.3} ms, decode {:.3} ms",
            enc[enc.len() / 2] as f64 / 1e6,
            dec[dec.len() / 2] as f64 / 1e6
        );
    }
    let report = serde_json::json!({"experiment":"B05", "baseline_commit":"44785d7", "source":args[1], "sha256":sha, "rows":rows, "batch_rows":batch_rows, "repetitions":reps, "workers":workers, "warmup_rounds":3, "scratch_retained_bytes":decoder.retained_bytes(), "samples":samples});
    fs::write(&args[2], serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
