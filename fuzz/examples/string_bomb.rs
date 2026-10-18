//! Writes the reproducer of the (fixed) finding "unchecked string expansion" to a file. It is NOT
//! decoded here. Before the 128 MiB limit on decoded string bytes existed, decoding it with the
//! unchecked API requested rows x string-length bytes; now every reader refuses it with an error.
//! Usage: `cargo run --release --example string_bomb -- OUT_FILE [STRING_BYTES [OUT_SEGMENT]]`
//! Default 60000 bytes -> 62,914,560,000 decoded string bytes claimed by a 60 KB block.
//! With `OUT_SEGMENT` the same block is also written inside a one-block `.gqodb.ob` segment (an
//! honest index: one symbol, time 5) so that `ob_store verify|query|recover` can be pointed at it.
use gqodb_blocks::real::Mode;
use gqodb_fuzz::seeds::{segment_around, string_bomb_block};

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("output file");
    let extra: usize = args.next().map_or(60_000, |s| s.parse().expect("number"));
    let (batch, block) = string_bomb_block(Mode::Gqodb, extra);
    std::fs::write(&out, &block).expect("write");
    println!("{out}: {} bytes", block.len());
    if let Some(path) = args.next() {
        let segment = segment_around(&batch, Mode::Gqodb, &block);
        std::fs::write(&path, &segment).expect("write segment");
        println!("{path}: {} bytes", segment.len());
    }
}
