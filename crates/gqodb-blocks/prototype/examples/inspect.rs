use anyhow::Result;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::File;
fn main() -> Result<()> {
    for path in std::env::args().skip(1) {
        let b = ParquetRecordBatchReaderBuilder::try_new(File::open(&path)?)?;
        println!(
            "{path}\n{:?}\nrows={} groups={}",
            b.schema(),
            b.metadata().file_metadata().num_rows(),
            b.metadata().num_row_groups()
        );
        println!(
            "{:?}",
            b.metadata()
                .row_group(0)
                .columns()
                .iter()
                .map(|c| (c.column_path().string(), c.compression()))
                .collect::<Vec<_>>()
        );
        match b.with_batch_size(1).build()?.next() {
            Some(Ok(batch)) => println!("first batch readable; {} columns", batch.num_columns()),
            Some(Err(e)) => println!("read error: {e}"),
            None => (),
        }
    }
    Ok(())
}
