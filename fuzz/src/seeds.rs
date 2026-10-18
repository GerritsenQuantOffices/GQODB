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

pub fn real_tick() -> Seeds {
    let mut out = real_seeds(Mode::Gqodb, Family::Tick);
    out.push((
        "max-rows-const".into(),
        real::encode(&max_rows_batch(), Mode::Gqodb).expect("encode"),
    ));
    out
}

pub fn real_book() -> Seeds {
    let mut out = real_seeds(Mode::GqodbBook, Family::Orderbook);
    out.push((
        "max-rows-const".into(),
        real::encode(&max_rows_batch(), Mode::GqodbBook).expect("encode"),
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
