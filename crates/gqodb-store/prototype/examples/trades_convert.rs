//! B17 converter: Binance aggTrades CSV (or a synthetic edge table) to the typed Parquet
//! source of docs/B17_PUBLIC_TRADES_PROTOCOL.md, plus the reference digests.
//!
//!   trades_convert csv --csv FILE|- --variant v1|v2 --rows N|all --out SOURCE.parquet
//!                      --report REPORT.json [--symbol S]
//!   trades_convert synthetic-edge --variant v1|v2 --out SOURCE.parquet --report REPORT.json
//!   trades_convert digest SOURCE.parquet --variant v1|v2
#![forbid(unsafe_code)]

#[path = "support/public_trades.rs"]
mod public_trades;

use anyhow::{Context, Result, bail, ensure};
use arrow_array::RecordBatch;
use parquet::{
    arrow::ArrowWriter,
    basic::{Compression, Encoding},
    file::{
        metadata::KeyValue,
        properties::{EnabledStatistics, WriterProperties},
    },
};
use public_trades::{BLOCK_ROWS, SYMBOL, TableBuilder, Variant};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read},
    path::Path,
};

fn option<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn required<'a>(args: &'a [String], name: &str) -> Result<&'a str> {
    option(args, name).with_context(|| format!("missing {name}"))
}

/// The source file is deliberately plain: uncompressed, PLAIN encoding, no dictionary,
/// so that every tool decodes it with the least possible work.
pub fn write_source(path: &Path, batches: &[RecordBatch]) -> Result<()> {
    let schema = batches[0].schema();
    let metadata = schema
        .metadata()
        .iter()
        .map(|(k, v)| KeyValue::new(k.clone(), v.clone()))
        .collect::<Vec<_>>();
    let props = WriterProperties::builder()
        .set_max_row_group_row_count(Some(BLOCK_ROWS))
        .set_dictionary_enabled(false)
        .set_encoding(Encoding::PLAIN)
        .set_compression(Compression::UNCOMPRESSED)
        .set_statistics_enabled(EnabledStatistics::Chunk)
        .set_key_value_metadata(Some(metadata))
        .build();
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
    for batch in batches {
        writer.write(batch)?;
        writer.flush()?;
    }
    let file = writer.into_inner()?;
    file.sync_all()?;
    Ok(())
}

fn report(variant: Variant, batches: &[RecordBatch], source: &Path, extra: Value) -> Result<Value> {
    let rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    let digest = public_trades::digest_batches(batches)?;
    let (start, end) = public_trades::window_bounds(batches);
    let window = public_trades::select_window(batches, SYMBOL, start, end)?;
    let window_rows: usize = window.iter().map(RecordBatch::num_rows).sum();
    ensure!(window_rows >= 1, "the time-range window is empty");
    let first = &batches[0];
    let last = &batches[batches.len() - 1];
    let agg = |b: &RecordBatch, i: usize| {
        b.column(0)
            .as_any()
            .downcast_ref::<arrow_array::Int64Array>()
            .unwrap()
            .value(i)
    };
    let times = |b: &RecordBatch, i: usize| public_trades::time_column(b).value(i);
    Ok(json!({
        "variant": variant.name(),
        "rows": rows,
        "blocks": batches.len(),
        "block_rows": BLOCK_ROWS,
        "symbol": SYMBOL,
        "digest": digest,
        "first_agg_id": agg(first, 0),
        "last_agg_id": agg(last, last.num_rows() - 1),
        "first_time": times(first, 0),
        "last_time": times(last, last.num_rows() - 1),
        "window": {
            "start": start,
            "end": end,
            "rows": window_rows,
            "digest": public_trades::digest_batches(&window)?,
        },
        "source_bytes": fs::metadata(source)?.len(),
        "source_sha256": public_trades::sha256_file(source)?,
        "details": extra,
    }))
}

fn finish(variant: Variant, batches: &[RecordBatch], out: &Path, extra: Value) -> Result<Value> {
    ensure!(!out.exists(), "output exists: {}", out.display());
    write_source(out, batches)?;
    // The file must read back to the identical digest (the source is itself verified).
    let reread = public_trades::read_parquet(out, variant)?;
    ensure!(
        public_trades::digest_batches(&reread)? == public_trades::digest_batches(batches)?,
        "source Parquet does not read back bit-exactly"
    );
    report(variant, &reread, out, extra)
}

fn from_csv(args: &[String]) -> Result<Value> {
    let variant = Variant::parse(required(args, "--variant")?)?;
    let symbol = option(args, "--symbol").unwrap_or(SYMBOL);
    ensure!(symbol == SYMBOL, "only {SYMBOL} is registered");
    let limit: Option<usize> = match required(args, "--rows")? {
        "all" => None,
        n => Some(n.parse()?),
    };
    let out = Path::new(required(args, "--out")?);
    let input: Box<dyn Read> = match required(args, "--csv")? {
        "-" => Box::new(std::io::stdin()),
        path => Box::new(File::open(path)?),
    };
    let mut reader = BufReader::with_capacity(1 << 20, input);
    let mut builder = TableBuilder::new(variant, symbol);
    let mut hasher = Sha256::new();
    let mut consumed = 0u64;
    let mut line = Vec::new();
    while limit.is_none_or(|n| builder.rows() < n) {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        ensure!(line.last() == Some(&b'\n'), "last CSV line has no newline");
        hasher.update(&line);
        consumed += line.len() as u64;
        let text = std::str::from_utf8(&line[..line.len() - 1]).context("CSV is not UTF-8")?;
        builder
            .push_line(text)
            .with_context(|| format!("CSV row {}", builder.rows() + 1))?;
    }
    if let Some(n) = limit {
        ensure!(
            builder.rows() == n,
            "CSV has only {} rows, {n} requested",
            builder.rows()
        );
    }
    let batches = builder.into_batches()?;
    finish(
        variant,
        &batches,
        out,
        json!({
            "input": "csv",
            "csv_bytes_consumed": consumed,
            "csv_consumed_sha256": format!("{:x}", hasher.finalize()),
        }),
    )
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("");
    let result = match command {
        "csv" => from_csv(&args)?,
        "synthetic-edge" => {
            let variant = Variant::parse(required(&args, "--variant")?)?;
            finish(
                variant,
                &public_trades::synthetic_edge(variant)?,
                Path::new(required(&args, "--out")?),
                json!({"input": "synthetic-edge"}),
            )?
        }
        "digest" => {
            let source = args
                .get(1)
                .context("usage: digest SOURCE.parquet --variant v1|v2")?;
            let variant = Variant::parse(required(&args, "--variant")?)?;
            let batches = public_trades::read_parquet(Path::new(source), variant)?;
            report(
                variant,
                &batches,
                Path::new(source),
                json!({"input": "parquet"}),
            )?
        }
        _ => bail!(
            "usage: trades_convert csv|synthetic-edge|digest ... (see the header of this file)"
        ),
    };
    if let Some(path) = option(&args, "--report") {
        ensure!(!Path::new(path).exists(), "report exists: {path}");
        fs::write(path, serde_json::to_vec_pretty(&result)?)?;
    }
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
