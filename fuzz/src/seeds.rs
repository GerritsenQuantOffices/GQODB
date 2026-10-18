//! Deterministic, synthetic seed corpora. Every byte comes from the repository's own
//! fixtures (`gqodb_blocks::data::generate`, `gqodb_codec::fixture`) or from the
//! `gen` module driven by a fixed PRNG. No market data is involved.
use crate::{arb, scratch, segment};
use arbitrary::Unstructured;
use arrow_array::RecordBatch;
use bytes::Bytes;
use gqodb_blocks::{
    Codec,
    data::{Kind, Regime, generate},
    real::{self, Mode},
};
use gqodb_codec::Family;

pub type Seeds = Vec<(String, Vec<u8>)>;

/// SplitMix64 byte stream; the same algorithm as the repository's fixtures.
pub fn prng_bytes(seed: u64, n: usize) -> Vec<u8> {
    let mut rng = gqodb_blocks::data::Rng::new(seed);
    let mut out = Vec::with_capacity(n + 8);
    while out.len() < n {
        out.extend_from_slice(&rng.draw().to_le_bytes());
    }
    out.truncate(n);
    out
}

fn batches(count: u64, min_rows: usize) -> Vec<RecordBatch> {
    (1..=count)
        .map(|s| {
            let bytes = prng_bytes(s, 6000);
            arb::batch(&mut Unstructured::new(&bytes), min_rows).expect("generated batch")
        })
        .collect()
}

fn columns_blocks(codecs: &[Codec]) -> Seeds {
    let mut out = Vec::new();
    for codec in codecs {
        for kind in Kind::ALL {
            for (regime, rows) in [
                (Regime::Quiet, 130),
                (Regime::Volatile, 2),
                (Regime::Volatile, 130),
            ] {
                let data = generate(kind, regime, rows, 7).expect("fixture");
                let bytes = codec.encode(&data).expect("encode");
                out.push((format!("{codec:?}-{kind:?}-{regime:?}-{rows}"), bytes));
            }
        }
    }
    out
}

pub fn block() -> Seeds {
    columns_blocks(&[
        Codec::ColumnLz4,
        Codec::DeltaVarintLz4,
        Codec::DeltaBitpackLz4,
    ])
}

pub fn adaptive() -> Seeds {
    let mut out = columns_blocks(&[Codec::AdaptiveSimd, Codec::AdaptiveSimdLz4]);
    let rows = gqodb_blocks::data::MAX_ROWS;
    let constant = gqodb_blocks::data::Columns {
        kind: Kind::Trade,
        values: vec![vec![9; rows]; 8],
    };
    out.push((
        "max-rows-const".into(),
        Codec::AdaptiveSimdLz4.encode(&constant).expect("encode"),
    ));
    out
}

fn real_seeds(mode: Mode, family_fixture: Family) -> Seeds {
    let mut inputs = batches(10, 0);
    inputs.push(gqodb_codec::fixture(family_fixture).expect("fixture"));
    // A long run-heavy block exercises the run-length path of the book codec.
    for (i, batch) in inputs.iter().enumerate() {
        if real::encode(batch, mode).is_err() {
            eprintln!("seed {i} skipped for {}", mode.name());
        }
    }
    inputs
        .iter()
        .enumerate()
        .filter_map(|(i, b)| Some((format!("{}-{i}", mode.name()), real::encode(b, mode).ok()?)))
        .collect()
}

/// Maximum-row batch whose columns are all constant: a few hundred bytes encoded, ~1M rows decoded.
/// Seeds like this let the mutator reach "tiny input, huge claimed expansion" shapes.
fn max_rows_batch() -> RecordBatch {
    use arrow_array::{ArrayRef, Int64Array, LargeStringArray};
    let rows = gqodb_blocks::data::MAX_ROWS;
    RecordBatch::try_from_iter(vec![
        (
            "t",
            std::sync::Arc::new(Int64Array::from(vec![5; rows])) as ArrayRef,
        ),
        (
            "sym",
            std::sync::Arc::new(LargeStringArray::from(vec!["BTC"; rows])) as ArrayRef,
        ),
        (
            "v",
            std::sync::Arc::new(Int64Array::from(vec![7; rows])) as ArrayRef,
        ),
    ])
    .expect("batch")
}

/// The "string expansion" reproducer: a maximum-row block (about 60 KB with the default `extra`)
/// whose only dictionary string was grown to `extra` bytes with all lengths and checksums kept
/// consistent, so it claims rows x `extra` decoded string bytes (62,914,560,000 for 60,000).
/// Returns the batch the block was grown from (for its schema) and the block.
pub fn string_bomb_block(mode: Mode, extra: usize) -> (RecordBatch, Vec<u8>) {
    let batch = max_rows_batch();
    let batch = batch.project(&[0, 1]).expect("projection");
    let mut block = real::encode(&batch, mode).expect("encode");
    let n = block.len();
    block.resize(n + extra, 0);
    let size = crate::fixup::grow_real_meta(&mut block, n, extra, 0);
    block.truncate(size);
    crate::fixup::real(&mut block);
    (batch, block)
}

/// A one-block native segment around `payload` with an honest index (one symbol, time 5), built
/// from the published container layout, so that the store readers and `ob_store` reach the block.
pub fn segment_around(batch: &RecordBatch, mode: Mode, payload: &[u8]) -> Vec<u8> {
    let codec = if matches!(mode, Mode::GqodbBook) {
        "GQOREAL4/GQOBOOK1"
    } else {
        "GQOREAL3/GQOADP02"
    };
    let header = serde_json::json!({
        "schema": serde_json::to_value(batch.schema().as_ref()).expect("schema json"),
        "time_column": "t", "symbol_column": "sym", "codec": codec,
    });
    let mut out = segment::frame(
        b"GQOBHDR1",
        &serde_json::to_vec(&header).expect("json"),
        &[],
    );
    let offset = out.len() as u64;
    let mut entry = serde_json::json!({
        "offset": offset, "length": 0, "rows": batch.num_rows(),
        "min_time": 5, "max_time": 5, "symbols": ["BTC"],
    });
    let meta = serde_json::to_vec(&entry).expect("json");
    entry["length"] = serde_json::json!(24 + meta.len() as u64 + payload.len() as u64);
    out.extend_from_slice(&segment::frame(b"GQOBBLK1", &meta, payload));
    let index_offset = out.len() as u64;
    out.extend_from_slice(&segment::frame(
        b"GQOBIDX1",
        &[],
        &serde_json::to_vec(&[entry]).expect("json"),
    ));
    let mut trailer = b"GQOBEND1".to_vec();
    trailer.extend_from_slice(&index_offset.to_le_bytes());
    trailer.extend_from_slice(&crc32fast::hash(&trailer).to_le_bytes());
    out.extend_from_slice(&trailer);
    out
}

pub fn real_tick() -> Seeds {
    let mut out = real_seeds(Mode::Gqodb, Family::Tick);
    out.push((
        "max-rows-const".into(),
        real::encode(&max_rows_batch(), Mode::Gqodb).expect("encode"),
    ));
    // Refused by the decoded-string limit; keeps the mutator next to that boundary.
    out.push((
        "string-bomb-60kb".into(),
        string_bomb_block(Mode::Gqodb, 60_000).1,
    ));
    out
}

pub fn real_book() -> Seeds {
    let mut out = real_seeds(Mode::GqodbBook, Family::Orderbook);
    out.push((
        "max-rows-const".into(),
        real::encode(&max_rows_batch(), Mode::GqodbBook).expect("encode"),
    ));
    out.push((
        "string-bomb-60kb".into(),
        string_bomb_block(Mode::GqodbBook, 60_000).1,
    ));
    out
}

pub fn parquet() -> Seeds {
    let mut out = Vec::new();
    for codec in Codec::ALL.into_iter().filter(|c| c.is_parquet()) {
        for kind in Kind::ALL {
            let data = generate(kind, Regime::Volatile, 40, 3).expect("fixture");
            out.push((
                format!("{codec:?}-{kind:?}"),
                codec.encode(&data).expect("encode"),
            ));
        }
    }
    let inputs = batches(4, 1);
    for mode in [
        Mode::ParquetLz4,
        Mode::ParquetDictionaryLz4,
        Mode::ParquetSnappy,
        Mode::ParquetBrotli,
        Mode::ParquetNormalizedLz4,
    ] {
        for (i, b) in inputs.iter().enumerate() {
            if let Ok(bytes) = real::encode(b, mode) {
                out.push((format!("real-{}-{i}", mode.name()), bytes));
            }
        }
    }
    out
}

pub fn codec_blocks() -> Seeds {
    let mut out = Vec::new();
    for (family, name) in [(Family::Tick, "tick"), (Family::Orderbook, "book")] {
        let batch = gqodb_codec::fixture(family).expect("fixture");
        out.push((
            format!("block-{name}"),
            gqodb_codec::encode(&batch, family).expect("encode"),
        ));
    }
    out
}

fn segment_bytes(write: impl FnOnce(&std::path::Path)) -> Vec<u8> {
    let path = scratch::fresh("seed.gqodb.ob");
    write(&path);
    let bytes = std::fs::read(&path).expect("read segment");
    let _ = std::fs::remove_file(&path);
    bytes
}

pub fn segments() -> Seeds {
    let mut out = Vec::new();
    for (family, name) in [(Family::Tick, "tick"), (Family::Orderbook, "book")] {
        let fixture = gqodb_codec::fixture(family).expect("fixture");
        let one = segment_bytes(|p| {
            gqodb_codec::write_segment_with_family(p, std::slice::from_ref(&fixture), family)
                .unwrap()
        });
        let three = segment_bytes(|p| {
            gqodb_codec::write_segment_with_family(
                p,
                &[fixture.clone(), fixture.clone(), fixture],
                family,
            )
            .unwrap()
        });
        out.push((format!("segment-{name}-1"), one));
        out.push((format!("segment-{name}-3"), three));
    }
    for (mode, name) in [(Mode::Gqodb, "tick"), (Mode::GqodbBook, "book")] {
        let batch = max_rows_batch();
        let bytes = segment_bytes(|p| {
            let mut w = gqodb_store::Writer::create_with_codec(p, batch.schema(), "t", "sym", mode)
                .unwrap();
            w.append(&batch).unwrap();
            w.finish().unwrap();
        });
        out.push((format!("segment-{name}-max-rows"), bytes));
        let (batch, block) = string_bomb_block(mode, 60_000);
        out.push((
            format!("segment-{name}-string-bomb"),
            segment_around(&batch, mode, &block),
        ));
    }
    // Generic-schema segments built by the structure-aware builder, honest only.
    for seed in 1..=40u64 {
        let bytes = prng_bytes(seed * 7919, 4000);
        if let Ok(built) = segment::build(&mut Unstructured::new(&bytes))
            && built.honest
            && out.len() < 16
        {
            out.push((
                format!(
                    "generic-{seed}-{}",
                    if built.book { "book" } else { "tick" }
                ),
                built.bytes,
            ));
        }
    }
    out
}

pub fn codec_all() -> Seeds {
    let mut out = codec_blocks();
    out.extend(segments());
    out
}

pub fn schemas() -> Seeds {
    let mut out = Vec::new();
    for (family, name) in [(Family::Tick, "tick"), (Family::Orderbook, "book")] {
        let batch = gqodb_codec::fixture(family).expect("fixture");
        let json = serde_json::to_vec(batch.schema().as_ref()).expect("schema json");
        out.push((format!("schema-{name}"), json));
    }
    out
}

pub fn types() -> Seeds {
    crate::typesfuzz::seeds()
}

pub fn structured() -> Seeds {
    (1..=6u64)
        .map(|s| (format!("prng-{s}"), prng_bytes(s, 512 * s as usize)))
        .collect()
}

/// Decode helper used by the seedgen self-check: every committed seed must be valid.
pub fn self_check() {
    for (name, bytes) in block() {
        gqodb_blocks::block::decode(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    for (name, bytes) in adaptive() {
        gqodb_blocks::adaptive::decode(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    for (name, bytes) in codec_all() {
        if name.starts_with("block") {
            gqodb_codec::decode(Bytes::from(bytes), None).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }
}
