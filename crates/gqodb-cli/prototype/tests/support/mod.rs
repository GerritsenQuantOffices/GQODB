//! Builds small native files for the tests and runs plans against them.
#![allow(dead_code)]
use arrow_array::{ArrayRef, Int8Array, Int64Array, LargeStringArray, RecordBatch};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use gqodb_cli::exec::{Executor, FileExecutor, Options, Report};
use gqodb_cli::sink::Sink;
use gqodb_cli::value::Cell;
use gqodb_codec::{Family, write_segment_with_family};
use gqodb_lang::{Plan, Session, parse, plan};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Debug)]
pub struct Trade {
    pub symbol: &'static str,
    pub source: Option<i64>,
    pub received: Option<i64>,
    pub clock_error: Option<i64>,
    pub available: i64,
    pub sequence: i64,
    pub price: i64,
    pub quantity: i64,
    pub side: i8,
}

/// 2026-03-01T00:00:00Z in nanoseconds. Times in the tests are written as offsets from it.
pub const BASE: i64 = 1_772_323_200_000_000_000;

/// The text of `BASE + offset_ns` as the language writes times.
pub fn at(offset_ns: i64) -> String {
    gqodb_lang::time::format_instant(BASE + offset_ns)
}

/// `available` is `BASE + offset`, `source` is 5 ns earlier, `received` 1 ns earlier.
pub fn trade(symbol: &'static str, offset: i64, sequence: i64) -> Trade {
    let available = BASE + offset;
    Trade {
        symbol,
        source: Some(available - 5),
        received: Some(available - 1),
        clock_error: Some(3),
        available,
        sequence,
        price: 108_342,
        quantity: 250,
        side: 1,
    }
}

fn metadata(venue: Option<&str>, pairs: &[(&str, &str)]) -> HashMap<String, String> {
    let mut m: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    m.insert("gqodb.schema_version".into(), "1".into());
    m.insert("gqodb.source_sha256".into(), "ab".repeat(32));
    if let Some(v) = venue {
        m.insert("gqodb.venue".into(), v.to_string());
    }
    m
}

fn common_fields() -> Vec<Field> {
    vec![
        Field::new("symbol", DataType::LargeUtf8, false),
        Field::new("source_utc_ns", DataType::Int64, true),
        Field::new("received_utc_ns", DataType::Int64, true),
        Field::new("clock_error_ns", DataType::Int64, true),
        Field::new("available_utc_ns", DataType::Int64, false),
        Field::new("sequence", DataType::Int64, false),
    ]
}

pub fn trades_schema(venue: Option<&str>, price_decimals: u8, quantity_decimals: u8) -> SchemaRef {
    let mut fields = common_fields();
    fields.push(Field::new("price", DataType::Int64, false));
    fields.push(Field::new("quantity", DataType::Int64, false));
    fields.push(Field::new("side", DataType::Int8, false));
    let pd = price_decimals.to_string();
    let qd = quantity_decimals.to_string();
    let meta = metadata(
        venue,
        &[
            ("gqodb.family", "tick"),
            ("gqodb.subtype", "trade"),
            ("gqodb.price_decimals", &pd),
            ("gqodb.quantity_decimals", &qd),
        ],
    );
    Arc::new(Schema::new_with_metadata(fields, meta))
}

pub fn trades_batch(
    rows: &[Trade],
    venue: Option<&str>,
    price_decimals: u8,
    quantity_decimals: u8,
) -> RecordBatch {
    let col = |f: &dyn Fn(&Trade) -> Option<i64>| -> ArrayRef {
        Arc::new(Int64Array::from(rows.iter().map(f).collect::<Vec<_>>()))
    };
    let columns: Vec<ArrayRef> = vec![
        Arc::new(LargeStringArray::from(
            rows.iter().map(|r| r.symbol).collect::<Vec<_>>(),
        )),
        col(&|r| r.source),
        col(&|r| r.received),
        col(&|r| r.clock_error),
        col(&|r| Some(r.available)),
        col(&|r| Some(r.sequence)),
        col(&|r| Some(r.price)),
        col(&|r| Some(r.quantity)),
        Arc::new(Int8Array::from(
            rows.iter().map(|r| r.side).collect::<Vec<_>>(),
        )),
    ];
    RecordBatch::try_new(
        trades_schema(venue, price_decimals, quantity_decimals),
        columns,
    )
    .unwrap()
}

/// One block per inner vector.
pub fn write_trades(
    path: &Path,
    blocks: &[Vec<Trade>],
    venue: Option<&str>,
    price_decimals: u8,
    quantity_decimals: u8,
) {
    let batches: Vec<RecordBatch> = blocks
        .iter()
        .map(|b| trades_batch(b, venue, price_decimals, quantity_decimals))
        .collect();
    write_segment_with_family(path, &batches, Family::Tick).unwrap();
}

/// A small order-book file, to be skipped when trades are asked for.
pub fn write_book(path: &Path) {
    let mut fields = common_fields();
    fields.push(Field::new("price", DataType::Int64, false));
    fields.push(Field::new("quantity", DataType::Int64, false));
    fields.push(Field::new("level", DataType::Int64, false));
    fields.push(Field::new("side", DataType::Int8, false));
    fields.push(Field::new("action", DataType::Int8, false));
    let meta = metadata(
        None,
        &[
            ("gqodb.family", "orderbook"),
            ("gqodb.depth", "10"),
            ("gqodb.price_decimals", "2"),
            ("gqodb.quantity_decimals", "2"),
        ],
    );
    let schema: SchemaRef = Arc::new(Schema::new_with_metadata(fields, meta));
    let i = |v: i64| -> ArrayRef { Arc::new(Int64Array::from(vec![v])) };
    let columns: Vec<ArrayRef> = vec![
        Arc::new(LargeStringArray::from(vec!["EURUSD"])),
        Arc::new(Int64Array::from(vec![Some(5_i64)])),
        Arc::new(Int64Array::from(vec![Some(6_i64)])),
        Arc::new(Int64Array::from(vec![Some(1_i64)])),
        i(10),
        i(1),
        i(100),
        i(200),
        i(0),
        Arc::new(Int8Array::from(vec![1_i8])),
        Arc::new(Int8Array::from(vec![2_i8])),
    ];
    let batch = RecordBatch::try_new(schema, columns).unwrap();
    write_segment_with_family(path, &[batch], Family::Orderbook).unwrap();
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A fresh empty directory, removed when dropped.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("gqodb-cli-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
    pub fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    pub fn s(&self) -> String {
        self.0.display().to_string()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
pub struct Collect {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
    pub finished: bool,
}

impl Sink for Collect {
    fn begin(&mut self, columns: &[String]) -> std::io::Result<()> {
        self.columns = columns.to_vec();
        Ok(())
    }
    fn row(&mut self, cells: &[Cell]) -> std::io::Result<()> {
        self.rows.push(cells.to_vec());
        Ok(())
    }
    fn finish(&mut self) -> std::io::Result<()> {
        self.finished = true;
        Ok(())
    }
}

impl Collect {
    /// The cells of one column, as integers (panics on other cells).
    pub fn ints(&self, column: &str) -> Vec<i64> {
        let i = self
            .columns
            .iter()
            .position(|c| c == column)
            .unwrap_or_else(|| panic!("no column {column}"));
        self.rows
            .iter()
            .map(|r| match &r[i] {
                Cell::Int(v) => *v,
                other => panic!("{other:?}"),
            })
            .collect()
    }
    pub fn texts(&self, column: &str) -> Vec<String> {
        let i = self.columns.iter().position(|c| c == column).unwrap();
        self.rows
            .iter()
            .map(|r| match &r[i] {
                Cell::Text(v) => v.clone(),
                Cell::Null => String::new(),
                other => panic!("{other:?}"),
            })
            .collect()
    }
}

pub fn plan_of(text: &str) -> Plan {
    let script = parse(text).unwrap_or_else(|e| panic!("{text}: {e:?}"));
    plan(&script.commands[0], &Session::default()).unwrap_or_else(|e| panic!("{text}: {e:?}"))
}

pub fn run(text: &str) -> (Collect, Report) {
    run_with(text, &Options::default())
}

pub fn run_with(text: &str, opts: &Options) -> (Collect, Report) {
    let p = plan_of(text);
    let mut sink = Collect::default();
    let report = FileExecutor
        .execute(&p, opts, &mut sink)
        .unwrap_or_else(|e| panic!("{text}: {e}"));
    assert!(sink.finished);
    (sink, report)
}
