//! Read-only B06/B11 market-data fixture projection, shared with storage benchmarks.
use anyhow::{Context, Result, ensure};
use arrow_array::{ArrayRef, Int64Array, LargeStringArray, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use flate2::read::MultiGzDecoder;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Read},
    sync::Arc,
};

pub fn hash(path: &str) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub fn decimal(s: &str) -> Result<i64> {
    let (whole, fraction) = s.split_once('.').unwrap_or((s, ""));
    ensure!(
        !whole.is_empty()
            && whole.bytes().all(|c| c.is_ascii_digit())
            && fraction.len() <= 8
            && fraction.bytes().all(|c| c.is_ascii_digit()),
        "non-exact positive decimal: {s}"
    );
    let whole: i64 = whole.parse()?;
    let fraction: i64 = if fraction.is_empty() {
        0
    } else {
        fraction.parse()?
    };
    whole
        .checked_mul(100_000_000)
        .and_then(|n| {
            n.checked_add(
                fraction * 10i64.pow(8 - s.split_once('.').map_or(0, |(_, f)| f.len()) as u32),
            )
        })
        .context("decimal overflow")
}

fn integer(v: &Value, key: &str) -> Result<i64> {
    v[key]
        .as_i64()
        .with_context(|| format!("missing integer {key}"))
}

pub fn book(path: &str, limit: usize, batch_rows: usize) -> Result<(Vec<RecordBatch>, Value)> {
    let names = [
        "event_index",
        "receipt_ns",
        "exchange_ms",
        "cts_ms",
        "update_id",
        "sequence",
        "side",
        "level_index",
        "price_e8",
        "quantity_e8",
        "snapshot",
    ];
    let mut fields: Vec<_> = names
        .iter()
        .map(|n| Field::new(*n, DataType::Int64, false))
        .collect();
    fields.push(Field::new("symbol", DataType::LargeUtf8, false));
    let schema = Arc::new(Schema::new_with_metadata(
        fields,
        HashMap::from([
            ("price_scale".into(), "100000000".into()),
            ("quantity_scale".into(), "100000000".into()),
        ]),
    ));
    let mut columns = vec![Vec::<i64>::new(); names.len()];
    let mut symbols = Vec::new();
    let mut batches = Vec::new();
    let mut levels = 0;
    let mut events = 0;
    let mut empty_events = 0;
    let reader = BufReader::new(MultiGzDecoder::new(fs::File::open(path)?));
    let flush = |columns: &mut Vec<Vec<i64>>, symbols: &mut Vec<String>| -> Result<RecordBatch> {
        let mut arrays: Vec<ArrayRef> = columns
            .iter_mut()
            .map(|v| Arc::new(Int64Array::from(std::mem::take(v))) as ArrayRef)
            .collect();
        arrays.push(Arc::new(LargeStringArray::from(std::mem::take(symbols))));
        Ok(RecordBatch::try_new(schema.clone(), arrays)?)
    };
    for line in reader.lines() {
        let line = line?;
        let envelope: Value = serde_json::from_str(&line)?;
        let raw = envelope["raw_payload"]
            .as_str()
            .context("raw payload missing")?;
        let raw_sha = format!("{:x}", Sha256::digest(raw.as_bytes()));
        ensure!(
            envelope["payload_sha256"].as_str() == Some(&raw_sha),
            "payload hash mismatch"
        );
        let payload: Value = serde_json::from_str(raw)?;
        let data = &payload["data"];
        let symbol = data["s"].as_str().context("symbol missing")?;
        let snapshot = match payload["type"].as_str() {
            Some("delta") => 0,
            Some("snapshot") => 1,
            _ => anyhow::bail!("unknown event type"),
        };
        let prefix = [
            events as i64,
            integer(&envelope, "receipt_utc_ns")?,
            integer(&payload, "ts")?,
            integer(&payload, "cts")?,
            integer(data, "u")?,
            integer(data, "seq")?,
        ];
        let mut count = 0;
        for (side, key) in ["b", "a"].iter().enumerate() {
            for (index, pair) in data[*key]
                .as_array()
                .context("missing levels")?
                .iter()
                .enumerate()
            {
                let pair = pair.as_array().context("level not array")?;
                ensure!(pair.len() == 2, "invalid level");
                let mut row = prefix.to_vec();
                row.extend([
                    side as i64,
                    index as i64,
                    decimal(pair[0].as_str().context("price")?)?,
                    decimal(pair[1].as_str().context("quantity")?)?,
                    snapshot,
                ]);
                for (column, value) in columns.iter_mut().zip(row) {
                    column.push(value);
                }
                symbols.push(symbol.to_owned());
                count += 1;
                levels += 1;
                if symbols.len() == batch_rows {
                    batches.push(flush(&mut columns, &mut symbols)?);
                }
            }
        }
        events += 1;
        empty_events += usize::from(count == 0);
        // Stop at event boundary, never silently discard half an event.
        if levels >= limit {
            break;
        }
    }
    if !symbols.is_empty() {
        batches.push(flush(&mut columns, &mut symbols)?);
    }
    Ok((
        batches,
        serde_json::json!({"events":events,"levels":levels,"empty_events_excluded":empty_events,"projection":"level rows, event timestamps/IDs/type/symbol; collector envelope and original JSON lexical form excluded; no replay correctness claim", "decimal_scale":100000000}),
    ))
}
