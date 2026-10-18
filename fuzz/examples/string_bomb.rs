//! Writes the reproducer of the open finding "unchecked string expansion" to a file (it is NOT
//! decoded here: decoding it with the unchecked API requests rows x string-length bytes).
//! Usage: `cargo run --release --example string_bomb -- OUT_FILE [STRING_BYTES]`
//! Default 60000 bytes -> about 60 GB requested by `gqodb_blocks::real::decode(.., Mode::Gqodb)`.
//! `gqodb_codec::decode` and `Reader::read_block_bounded` refuse the same bytes.
use arrow_array::{ArrayRef, Int64Array, LargeStringArray, RecordBatch};
use gqodb_blocks::{
    data::MAX_ROWS,
    real::{self, Mode},
};
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
}
