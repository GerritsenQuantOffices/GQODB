//! Writes the reproducer of the (fixed) finding "unchecked string expansion" to a file. It is NOT
//! decoded here. Before the 128 MiB limit on decoded string bytes existed, decoding it with the
//! unchecked API requested rows x string-length bytes; now every reader refuses it with an error.
//! Usage: `cargo run --release --example string_bomb -- OUT_FILE [STRING_BYTES [OUT_SEGMENT]]`
//! Default 60000 bytes -> 62,914,560,000 decoded string bytes claimed by a 60 KB block.
//! With `OUT_SEGMENT` the same block is also written inside a one-block `.gqodb.ob` segment (an
//! honest index: one symbol, time 5) so that `ob_store verify|query|recover` can be pointed at it.
use arrow_array::{ArrayRef, Int64Array, LargeStringArray, RecordBatch};
use gqodb_blocks::{
    data::MAX_ROWS,
    real::{self, Mode},
};
use gqodb_fuzz::segment::frame;
use std::sync::Arc;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("output file");
    let extra: usize = args.next().map_or(60_000, |s| s.parse().expect("number"));
    let batch = RecordBatch::try_from_iter(vec![
        (
            "t",
            Arc::new(Int64Array::from(vec![5; MAX_ROWS])) as ArrayRef,
        ),
        (
            "sym",
            Arc::new(LargeStringArray::from(vec!["BTC"; MAX_ROWS])) as ArrayRef,
        ),
    ])
    .expect("batch");
    let mut block = real::encode(&batch, Mode::Gqodb).expect("encode");
    let n = block.len();
    block.resize(n + extra, 0);
    let size = gqodb_fuzz::fixup::grow_real_meta(&mut block, n, extra, 0);
    block.truncate(size);
    gqodb_fuzz::fixup::real(&mut block);
    std::fs::write(&out, &block).expect("write");
    println!("{out}: {} bytes", block.len());
    if let Some(path) = args.next() {
        let segment = segment_with(&batch, &block);
        std::fs::write(&path, &segment).expect("write segment");
        println!("{path}: {} bytes", segment.len());
    }
}

/// One-block native segment around `payload`, built from the published container layout.
fn segment_with(batch: &RecordBatch, payload: &[u8]) -> Vec<u8> {
    let header = serde_json::json!({
        "schema": serde_json::to_value(batch.schema().as_ref()).expect("schema json"),
        "time_column": "t", "symbol_column": "sym", "codec": "GQOREAL3/GQOADP02",
    });
    let mut out = frame(
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
    out.extend_from_slice(&frame(b"GQOBBLK1", &meta, payload));
    let index_offset = out.len() as u64;
    out.extend_from_slice(&frame(
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
