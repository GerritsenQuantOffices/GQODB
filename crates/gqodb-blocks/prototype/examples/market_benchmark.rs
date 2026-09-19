//! B06 real trade rows / projected order-book levels, bounded resident input.
//! B11 adds the dedicated book codec through GQODB_BOOK_CODEC=1.
#![forbid(unsafe_code)]

use anyhow::{Result, ensure};
use arrow_array::RecordBatch;
use bytes::Bytes;
use gqodb_blocks::{
    market::{book, hash},
    real::{self, Decoder, Mode},
};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::Serialize;
use std::{fs, hint::black_box, time::Instant};

#[derive(Serialize)]
struct Sample {
    round: usize,
    mode: String,
    encode_ns: u128,
    decode_ns: u128,
    bytes: usize,
}

fn pass(
    batches: &[RecordBatch],
    mode: Mode,
    round: usize,
    decoder: &mut Decoder,
) -> Result<Sample> {
    let mut sample = Sample {
        round,
        mode: mode.name().into(),
        encode_ns: 0,
        decode_ns: 0,
        bytes: 0,
    };
    for batch in batches {
        let start = Instant::now();
        let encoded = real::encode(batch, mode)?;
        sample.encode_ns += start.elapsed().as_nanos();
        sample.bytes += encoded.len();

        let start = Instant::now();
        let decoded = decoder.decode(Bytes::from(encoded), mode)?;
        sample.decode_ns += start.elapsed().as_nanos();
        ensure!(
            real::equal(batch, &decoded),
            "roundtrip failed for {}; schema_equal={}; mismatched_columns={:?}",
            mode.name(),
            batch.schema() == decoded.schema(),
            batch
                .columns()
                .iter()
                .zip(decoded.columns())
                .enumerate()
                .filter(|(_, (a, b))| a.to_data() != b.to_data())
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        );
        black_box(decoded);
    }
    Ok(sample)
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() == 9,
        "usage: market_benchmark trades|book SOURCE OUTPUT.json ROWS BATCH_ROWS REPS ROW_GROUP START_BATCH"
    );
    let limit: usize = args[4].parse()?;
    let batch_rows: usize = args[5].parse()?;
    let reps: usize = args[6].parse()?;
    let group: usize = args[7].parse()?;
    let skip: usize = args[8].parse()?;
    ensure!(
        (1..=1_048_576).contains(&limit)
            && (128..=32_768).contains(&batch_rows)
            && (3..=30).contains(&reps),
        "invalid limits"
    );
    ensure!(!std::path::Path::new(&args[3]).exists(), "output exists");

    let source_sha256 = hash(&args[2])?;
    let (batches, provenance) = if args[1] == "book" {
        ensure!(group == 0 && skip == 0, "book offset unsupported");
        let expected = fs::read_to_string(format!("{}.sha256", args[2]))?;
        ensure!(
            expected.split_whitespace().next() == Some(source_sha256.as_str()),
            "source SHA mismatch"
        );
        book(&args[2], limit, batch_rows)?
    } else {
        ensure!(args[1] == "trades", "unknown source kind");
        let builder = ParquetRecordBatchReaderBuilder::try_new(fs::File::open(&args[2])?)?;
        let source_rows = builder.metadata().file_metadata().num_rows();
        ensure!(
            group < builder.metadata().num_row_groups(),
            "invalid row group"
        );
        let reader = builder
            .with_row_groups(vec![group])
            .with_batch_size(batch_rows)
            .build()?;
        let mut batches = Vec::new();
        let mut rows = 0;
        for batch in reader.skip(skip) {
            let batch = batch?;
            let take = batch.num_rows().min(limit - rows);
            batches.push(batch.slice(0, take));
            rows += take;
            if rows == limit {
                break;
            }
        }
        ensure!(
            rows == limit,
            "requested {limit} trade rows but loaded {rows}"
        );
        (
            batches,
            serde_json::json!({
                "source_rows": source_rows,
                "row_group": group,
                "skip_batches": skip,
                "projection": "all source columns, original Arrow schema, nulls and bitwise float values"
            }),
        )
    };

    let rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    ensure!(rows > 0, "empty source");
    let arrow_input_bytes: usize = batches.iter().map(RecordBatch::get_array_memory_size).sum();
    ensure!(
        arrow_input_bytes <= 256 * 1024 * 1024,
        "resident Arrow input exceeds 256 MiB"
    );
    println!(
        "loaded {rows} rows, {} blocks, {arrow_input_bytes} Arrow bytes",
        batches.len()
    );

    let book_codec = std::env::var("GQODB_BOOK_CODEC").as_deref() == Ok("1");
    let mut modes = Mode::ALL.to_vec();
    if book_codec {
        modes.push(Mode::GqodbBook);
    }
    let mut decoders: Vec<_> = modes.iter().map(|_| Decoder::default()).collect();
    for (index, mode) in modes.iter().enumerate() {
        pass(&batches, *mode, usize::MAX, &mut decoders[index])?;
    }

    let mut samples = Vec::new();
    for round in 0..reps {
        for offset in 0..modes.len() {
            let index = if round % 2 == 0 {
                (round / 2 + offset) % modes.len()
            } else {
                (round / 2 + modes.len() - 1 - offset) % modes.len()
            };
            samples.push(pass(&batches, modes[index], round, &mut decoders[index])?);
        }
        println!("round {}/{} done", round + 1, reps);
    }

    let mut summary = Vec::new();
    for mode in modes {
        let selected: Vec<_> = samples.iter().filter(|s| s.mode == mode.name()).collect();
        ensure!(
            selected.iter().all(|s| s.bytes == selected[0].bytes),
            "nondeterministic size"
        );
        let mut encode: Vec<_> = selected.iter().map(|s| s.encode_ns).collect();
        encode.sort_unstable();
        let mut decode: Vec<_> = selected.iter().map(|s| s.decode_ns).collect();
        decode.sort_unstable();
        let p95 = (reps * 95).div_ceil(100) - 1;
        println!(
            "{}: bytes {}, encode median {:.3}ms, decode median {:.3}ms",
            mode.name(),
            selected[0].bytes,
            encode[reps / 2] as f64 / 1e6,
            decode[reps / 2] as f64 / 1e6
        );
        summary.push(serde_json::json!({
            "mode": mode.name(),
            "bytes": selected[0].bytes,
            "encode_median_ns": encode[reps / 2],
            "decode_median_ns": decode[reps / 2],
            "encode_p95_ns": encode[p95],
            "decode_p95_ns": decode[p95]
        }));
    }

    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    let report = serde_json::json!({
        "experiment": if book_codec { "B11" } else { "B06" },
        "args": args,
        "source_sha256": source_sha256,
        "provenance": provenance,
        "rows": rows,
        "blocks": batches.len(),
        "arrow_input_bytes": arrow_input_bytes,
        "warmup_passes": 1,
        "repetitions": reps,
        "samples": samples,
        "summary": summary,
        "scratch_bytes": decoders.iter().map(Decoder::retained_bytes).collect::<Vec<_>>(),
        "process_memory": status
            .lines()
            .filter(|line| line.starts_with("VmHWM:") || line.starts_with("VmRSS:"))
            .collect::<Vec<_>>()
    });
    fs::write(&args[3], serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use gqodb_blocks::market::decimal;

    #[test]
    fn exact_decimal() {
        assert_eq!(decimal("123.45").unwrap(), 12_345_000_000);
        assert_eq!(decimal("0.00000001").unwrap(), 1);
        for value in [
            "0.000000001",
            "NaN",
            "1e3",
            "-1",
            "9223372036854775807",
            "x",
        ] {
            assert!(decimal(value).is_err());
        }
    }
}
