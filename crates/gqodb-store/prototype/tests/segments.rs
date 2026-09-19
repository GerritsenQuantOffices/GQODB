use arrow_array::{ArrayRef, Int64Array, LargeStringArray, RecordBatch};
use gqodb_blocks::real;
use gqodb_store::{Reader, Writer};
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "gqodb-store-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn batch(times: Vec<i64>, symbols: Vec<&str>) -> RecordBatch {
    let n = times.len();
    RecordBatch::try_from_iter(vec![
        ("receipt_ns", Arc::new(Int64Array::from(times)) as ArrayRef),
        (
            "symbol",
            Arc::new(LargeStringArray::from(symbols)) as ArrayRef,
        ),
        (
            "price_e8",
            Arc::new(Int64Array::from(vec![123456789; n])) as ArrayRef,
        ),
    ])
    .unwrap()
}

#[test]
fn indexed_ranges_preserve_late_rows_and_skip_other_blocks() {
    let temp = Temp::new();
    let path = temp.path("market.gqodb.ob");
    let a = batch(vec![10, 12, 11], vec!["BTC", "ETH", "BTC"]);
    let b = batch(vec![100, 102], vec!["ETH", "ETH"]);
    let c = batch(vec![9, 110], vec!["BTC", "BTC"]);
    let mut writer = Writer::create(&path, a.schema(), "receipt_ns", "symbol").unwrap();
    for input in [&a, &b, &c] {
        writer.append(input).unwrap();
    }
    writer.finish().unwrap();
    assert!(Writer::create(&path, a.schema(), "receipt_ns", "symbol").is_err());
    let mut reader = Reader::open(&path).unwrap();
    let all = reader.read_all().unwrap();
    assert!(all.iter().zip([&a, &b, &c]).all(|(x, y)| real::equal(x, y)));
    let selected = reader.query("BTC", 9, 11).unwrap();
    assert_eq!(selected.stats.blocks_read, 2);
    assert!(real::equal(
        &selected.batches[0],
        &batch(vec![10, 11], vec!["BTC", "BTC"])
    ));
    assert!(real::equal(
        &selected.batches[1],
        &batch(vec![9], vec!["BTC"])
    ));
    assert_eq!(
        reader
            .query("absent", i64::MIN, i64::MAX)
            .unwrap()
            .stats
            .blocks_read,
        0
    );
    assert!(reader.query("BTC", 12, 11).is_err());
}

#[test]
fn every_truncated_tail_recovers_exact_complete_prefix() {
    let temp = Temp::new();
    let path = temp.path("sealed.gqodb.ob");
    let input = batch(vec![1, 2], vec!["BTC", "ETH"]);
    let mut writer = Writer::create(&path, input.schema(), "receipt_ns", "symbol").unwrap();
    writer.append(&input).unwrap();
    writer.checkpoint().unwrap();
    writer.append(&input).unwrap();
    writer.finish().unwrap();
    let reader = Reader::open(&path).unwrap();
    let blocks = reader.blocks().to_vec();
    let raw = fs::read(&path).unwrap();
    let cut = temp.path("cut.gqodb.ob");
    for end in 0..raw.len() {
        fs::write(&cut, &raw[..end]).unwrap();
        assert!(Reader::open(&cut).is_err(), "cut {end}");
        if end < blocks[0].offset as usize {
            assert!(Reader::recover(&cut).is_err());
            continue;
        }
        let mut recovered = Reader::recover(&cut).unwrap_or_else(|e| panic!("cut {end}: {e}"));
        let count = blocks
            .iter()
            .filter(|b| b.offset + b.length <= end as u64)
            .count();
        assert_eq!(recovered.blocks().len(), count, "cut {end}");
        assert!(
            recovered
                .read_all()
                .unwrap()
                .iter()
                .all(|b| real::equal(b, &input))
        );
        assert_eq!(fs::metadata(&cut).unwrap().len(), end as u64);
    }
}

#[test]
fn complete_corruption_is_not_silently_treated_as_a_crash_tail() {
    let temp = Temp::new();
    let path = temp.path("market.gqodb.ob");
    let input = batch(vec![1, 2], vec!["BTC", "BTC"]);
    let mut writer = Writer::create(&path, input.schema(), "receipt_ns", "symbol").unwrap();
    writer.append(&input).unwrap();
    writer.finish().unwrap();
    let index = Reader::open(&path).unwrap().blocks()[0].clone();
    let mut raw = fs::read(&path).unwrap();
    raw[(index.offset + index.length - 1) as usize] ^= 1;
    fs::write(&path, raw).unwrap();
    assert!(Reader::open(&path).unwrap().read_all().is_err());
    assert!(Reader::recover(&path).is_err());
}

#[test]
fn unsealed_checkpoint_and_schema_rejection() {
    let temp = Temp::new();
    let path = temp.path("partial.gqodb.ob");
    let a = batch(vec![1], vec!["BTC"]);
    let mut writer = Writer::create(&path, a.schema(), "receipt_ns", "symbol").unwrap();
    writer.append(&a).unwrap();
    writer.checkpoint().unwrap();
    let wrong =
        RecordBatch::try_from_iter(vec![("x", Arc::new(Int64Array::from(vec![1])) as ArrayRef)])
            .unwrap();
    assert!(writer.append(&wrong).is_err());
    drop(writer);
    assert!(Reader::open(&path).is_err());
    assert!(real::equal(
        &Reader::recover(&path).unwrap().read_all().unwrap()[0],
        &a
    ));
    let before = fs::read(&path).unwrap();
    let target = temp.path("recovered.gqodb.ob");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ob_store"))
        .arg("recover")
        .arg(&path)
        .arg(&target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(real::equal(
        &Reader::open(&target).unwrap().read_all().unwrap()[0],
        &a
    ));
}

#[test]
fn tick_codec_segment_and_recovery_cli_preserve_native_profile() {
    let temp = Temp::new();
    let source = temp.path("source.gqodb.tick");
    let output = temp.path("recovered.gqodb.tick");
    let input = batch(vec![10, 12, 11], vec!["BTC", "ETH", "BTC"]);
    let mut writer = Writer::create_with_codec(
        &source,
        input.schema(),
        "receipt_ns",
        "symbol",
        real::Mode::Gqodb,
    )
    .unwrap();
    writer.append(&input).unwrap();
    writer.checkpoint().unwrap();
    drop(writer);
    assert!(Reader::open(&source).is_err());
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_ob_store"))
        .arg("recover")
        .arg(&source)
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let mut reader = Reader::open(&output).unwrap();
    assert!(matches!(reader.codec(), real::Mode::Gqodb));
    assert!(real::equal(&reader.read_all().unwrap()[0], &input));
    let bad = temp.path("unsupported");
    assert!(
        Writer::create_with_codec(
            &bad,
            input.schema(),
            "receipt_ns",
            "symbol",
            real::Mode::ParquetLz4
        )
        .is_err()
    );
    assert!(!bad.exists());
}

#[test]
fn bounded_single_block_access_checks_index_rows_and_expansion_budget() {
    let temp = Temp::new();
    let path = temp.path("bounded.gqodb.tick");
    let input = batch(vec![1, 2, 3], vec!["BTC", "BTC", "BTC"]);
    let mut writer = Writer::create_with_codec(
        &path,
        input.schema(),
        "receipt_ns",
        "symbol",
        real::Mode::Gqodb,
    )
    .unwrap();
    writer.append(&input).unwrap();
    writer.finish().unwrap();
    let mut reader = Reader::open(&path).unwrap();
    assert!(reader.read_block(1).is_err());
    assert!(reader.read_block_bounded(0, 2, 1024).is_err());
    assert!(reader.read_block_bounded(0, 3, 8).is_err());
    assert!(real::equal(
        &reader.read_block_bounded(0, 3, 1024).unwrap(),
        &input
    ));
}
