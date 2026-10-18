//! Regenerates the committed seed corpora under `fuzz/corpus/<target>/`.
//! Usage: `cargo run --release --example seedgen -- <corpus-dir>` (from `fuzz/`, stable toolchain).
use gqodb_fuzz::seeds;
use std::{fs, path::Path};

fn write(root: &Path, targets: &[&str], items: seeds::Seeds) {
    for target in targets {
        let dir = root.join(target);
        fs::create_dir_all(&dir).expect("create corpus dir");
        for (name, bytes) in &items {
            fs::write(dir.join(name.replace(['/', ' '], "_")), bytes).expect("write seed");
        }
        println!(
            "{target}: {} seeds, {} bytes",
            items.len(),
            items.iter().map(|i| i.1.len()).sum::<usize>()
        );
    }
}

fn main() {
    let root = std::env::args().nth(1).expect("corpus directory argument");
    let root = Path::new(&root);
    seeds::self_check();
    write(root, &["block_decode"], seeds::block());
    write(root, &["adaptive_decode"], seeds::adaptive());
    write(root, &["real_tick_decode"], seeds::real_tick());
    write(root, &["real_book_decode"], seeds::real_book());
    write(root, &["parquet_decode"], seeds::parquet());
    let segs = seeds::segments();
    write(root, &["store_reader", "store_recover"], segs);
    write(root, &["codec_facade"], seeds::codec_all());
    write(root, &["codec_schema"], seeds::schemas());
    write(root, &["types_parse"], seeds::types());
    write(
        root,
        &[
            "store_structured",
            "roundtrip_columns",
            "roundtrip_real",
            "roundtrip_codec",
            "roundtrip_store",
        ],
        seeds::structured(),
    );
    println!("seed self-check passed");
}
