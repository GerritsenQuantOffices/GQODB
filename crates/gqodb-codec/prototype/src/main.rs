#![forbid(unsafe_code)]
use anyhow::{Result, bail};
use gqodb_codec::*;
use std::path::Path;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["inspect", file] => println!(
            "{}",
            serde_json::to_string_pretty(&inspect(Path::new(file), None)?)?
        ),
        ["validate", file, family] => println!(
            "{}",
            serde_json::to_string_pretty(&inspect(Path::new(file), Some(Family::parse(family)?))?)?
        ),
        ["demo", "book-segment", file] => {
            write_segment(Path::new(file), &[fixture(Family::Orderbook)?])?
        }
        ["demo", "tick-segment", file] => {
            write_segment_with_family(Path::new(file), &[fixture(Family::Tick)?], Family::Tick)?
        }
        ["demo", family, file] => {
            let family = Family::parse(family)?;
            write_block(Path::new(file), &fixture(family)?, family)?;
        }
        ["--help"] | ["help"] | [] => println!(
            "gqodb-codec: native magic dispatch; extension is not trusted\n  inspect FILE\n  validate FILE tick|orderbook\n  demo tick|orderbook|tick-segment|book-segment NEW_FILE"
        ),
        _ => bail!("use --help"),
    }
    Ok(())
}
