//! Minimal local inspection/recovery CLI; no live services or source mutation.
#![forbid(unsafe_code)]
use anyhow::{Result, ensure};
use gqodb_store::{Reader, Writer};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() >= 2,
        "usage: ob_store inspect|verify FILE | recover SOURCE NEW_FILE | query FILE SYMBOL START_NS END_NS"
    );
    match args[0].as_str() {
        "inspect" if args.len() == 2 => {
            let reader = Reader::open(&args[1])?;
            println!("{}", serde_json::to_string_pretty(reader.blocks())?);
        }
        "verify" if args.len() == 2 => {
            let mut reader = Reader::open(&args[1])?;
            let mut rows = 0;
            reader.visit_blocks(|b| {
                rows += b.num_rows();
                Ok(())
            })?;
            println!("verified {} blocks, {rows} rows", reader.blocks().len());
        }
        "recover" if args.len() == 3 => {
            let mut reader = Reader::recover(&args[1])?;
            let (time, symbol) = reader.index_columns();
            let mut writer =
                Writer::create_with_codec(&args[2], reader.schema(), time, symbol, reader.codec())?;
            reader.visit_blocks(|b| writer.append(b))?;
            writer.finish()?;
            println!(
                "recovered {} complete blocks to {}; source unchanged",
                reader.blocks().len(),
                args[2]
            );
        }
        "query" if args.len() == 5 => {
            let result =
                Reader::open(&args[1])?.query(&args[2], args[3].parse()?, args[4].parse()?)?;
            let rows: usize = result.batches.iter().map(|b| b.num_rows()).sum();
            println!(
                "{rows} rows; {} blocks read; {} block bytes read",
                result.stats.blocks_read, result.stats.bytes_read
            );
        }
        _ => anyhow::bail!("invalid command/arguments"),
    }
    Ok(())
}
