//! `arbitrary`-driven generators of VALID data for the round-trip targets.
//! The distributions are deliberately biased towards the shapes the encoders
//! specialise on: constants, runs, linear ramps, tiny ranges, common divisors,
//! values needing more than 32 bits, and the i64 extremes.
use arbitrary::{Result, Unstructured};
use arrow_array::{
    ArrayRef, BooleanArray, Float64Array, Int8Array, Int64Array, LargeStringArray, RecordBatch,
    TimestampMillisecondArray,
};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use gqodb_blocks::data::{Columns, Kind};
use std::{collections::HashMap, sync::Arc};

const EDGE: [i64; 8] = [
    0,
    1,
    -1,
    2,
    i64::MAX,
    i64::MIN,
    i32::MAX as i64 + 1,
    u32::MAX as i64,
];

fn edge(u: &mut Unstructured) -> Result<i64> {
    Ok(match u.int_in_range(0..=3)? {
        0 => EDGE[u.int_in_range(0..=EDGE.len() - 1)?],
        1 => i64::from(u.int_in_range(-300i16..=300)?),
        2 => u.arbitrary::<i64>()?,
        _ => i64::from(u.arbitrary::<i32>()?),
    })
}

/// One i64 column of `rows` values drawn from one of several shapes.
pub fn column(u: &mut Unstructured, rows: usize) -> Result<Vec<i64>> {
    let mut v = Vec::with_capacity(rows);
    match u.int_in_range(0..=8)? {
        0 => v.resize(rows, edge(u)?),
        1 => {
            let (a, s) = (edge(u)?, edge(u)?);
            v.extend((0..rows).map(|i| a.wrapping_add(s.wrapping_mul(i as i64))));
        }
        2 => v.extend((0..rows).map(|_| u.arbitrary::<i64>().unwrap_or(0))),
        3 => {
            let (a, w) = (edge(u)?, u.int_in_range(1..=1000i64)?);
            v.extend((0..rows).map(|_| a.wrapping_add(u.int_in_range(0..=w).unwrap_or(0))));
        }
        4 => {
            let mut x = edge(u)?;
            while v.len() < rows {
                let n = u.int_in_range(1..=40)?.min(rows - v.len());
                v.extend(std::iter::repeat_n(x, n));
                x = edge(u)?;
            }
        }
        5 => v.extend((0..rows).map(|i| EDGE[(i + 3) % EDGE.len()])),
        6 => {
            let (a, g) = (edge(u)?, u.int_in_range(2..=1_000_000i64)?);
            v.extend(
                (0..rows)
                    .map(|_| a.wrapping_add(g.wrapping_mul(u.int_in_range(0..=50).unwrap_or(0)))),
            );
        }
        7 => {
            let mut x = edge(u)?;
            for _ in 0..rows {
                x = x.wrapping_add(i64::from(u.int_in_range(-5..=5i8).unwrap_or(0)));
                v.push(x);
            }
        }
        _ => {
            let big = u.int_in_range(32..=63u32)?;
            v.extend((0..rows).map(|_| (u.arbitrary::<u64>().unwrap_or(0) >> (64 - big)) as i64));
        }
    }
    Ok(v)
}

/// A schema-v1 `Columns` fixture (8 columns) of 0..=300 rows, which crosses the 128-row group boundary.
pub fn columns(u: &mut Unstructured) -> Result<Columns> {
    let rows = u.int_in_range(0..=300usize)?;
    let kind = Kind::ALL[u.int_in_range(0..=2usize)?];
    let mut values: Vec<Vec<i64>> = Vec::new();
    for c in 0..8 {
        // Sometimes copy an earlier column plus a constant: the Reference predictor.
        if c > 0 && u.ratio(1, 6)? {
            let from = u.int_in_range(0..=c - 1)?;
            let k = edge(u)?;
            let col: Vec<i64> = values[from].iter().map(|v| v.wrapping_add(k)).collect();
            values.push(col);
        } else {
            values.push(column(u, rows)?);
        }
    }
    Ok(Columns { kind, values })
}

fn small_string(u: &mut Unstructured) -> Result<String> {
    const POOL: [&str; 6] = ["", "BTCUSDT", "ETHUSDT", "β/日本", "x", "UNIUSDT"];
    Ok(if u.ratio(3, 4)? {
        POOL[u.int_in_range(0..=POOL.len() - 1)?].to_string()
    } else {
        let s: String = u.arbitrary()?;
        s.chars().take(12).collect()
    })
}

fn float(u: &mut Unstructured) -> Result<f64> {
    Ok(match u.int_in_range(0..=6)? {
        0 => f64::from(u.int_in_range(-100_000..=100_000i32)?) / 100.0,
        1 => f64::from(u.int_in_range(0..=1_000_000i32)?) / 1e8,
        2 => f64::from_bits(u.arbitrary::<u64>()?),
        3 => [
            0.0,
            -0.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
            f64::MIN_POSITIVE,
        ][u.int_in_range(0..=6usize)?],
        4 => u.arbitrary::<i32>()? as f64,
        5 => (edge(u)? as f64) / 10f64.powi(u.int_in_range(0..=9)?),
        _ => f64::from_bits(0x7ff8_0000_0000_0000 | u.int_in_range(0..=0xffffu64)?),
    })
}

#[derive(Clone, Copy)]
enum Ty {
    I8,
    Ts(Option<&'static str>),
    I64,
    F64,
    Bool,
    Str,
}

fn tys(u: &mut Unstructured, n: usize) -> Result<Vec<Ty>> {
    (0..n)
        .map(|_| {
            Ok(match u.int_in_range(0..=6)? {
                0 => Ty::I8,
                1 => Ty::Ts(None),
                2 => Ty::Ts(Some(["UTC", "+01:00"][u.int_in_range(0..=1usize)?])),
                3 | 4 => Ty::I64,
                5 => Ty::F64,
                _ => [Ty::Bool, Ty::Str][u.int_in_range(0..=1usize)?],
            })
        })
        .collect()
}

fn array(u: &mut Unstructured, ty: Ty, rows: usize, nullable: bool) -> Result<ArrayRef> {
    let nulls: Vec<bool> = (0..rows)
        .map(|_| Ok(nullable && u.ratio(1, 4)?))
        .collect::<Result<_>>()?;
    let opt = |i: usize| !nulls[i];
    Ok(match ty {
        Ty::I8 => Arc::new(Int8Array::from(
            (0..rows)
                .map(|i| Ok(opt(i).then_some(u.arbitrary::<i8>()?)))
                .collect::<Result<Vec<_>>>()?,
        )),
        Ty::Ts(tz) => {
            let a = TimestampMillisecondArray::from(
                (0..rows)
                    .map(|i| Ok(opt(i).then_some(edge(u)?)))
                    .collect::<Result<Vec<_>>>()?,
            );
            Arc::new(a.with_timezone_opt(tz))
        }
        Ty::I64 => {
            let col = column(u, rows)?;
            Arc::new(Int64Array::from(
                col.into_iter()
                    .enumerate()
                    .map(|(i, v)| opt(i).then_some(v))
                    .collect::<Vec<_>>(),
            ))
        }
        Ty::F64 => Arc::new(Float64Array::from(
            (0..rows)
                .map(|i| Ok(opt(i).then_some(float(u)?)))
                .collect::<Result<Vec<_>>>()?,
        )),
        Ty::Bool => Arc::new(BooleanArray::from(
            (0..rows)
                .map(|i| Ok(opt(i).then_some(u.arbitrary::<bool>()?)))
                .collect::<Result<Vec<_>>>()?,
        )),
        Ty::Str => {
            // Runs of the same string are what the dictionary + run path specialises on.
            let mut v: Vec<Option<String>> = Vec::with_capacity(rows);
            let mut cur = small_string(u)?;
            for i in 0..rows {
                if u.ratio(1, 5)? {
                    cur = small_string(u)?;
                }
                v.push(opt(i).then(|| cur.clone()));
            }
            Arc::new(LargeStringArray::from(v))
        }
    })
}

fn metadata(u: &mut Unstructured) -> Result<HashMap<String, String>> {
    let mut m = HashMap::new();
    for _ in 0..u.int_in_range(0..=3)? {
        m.insert(small_string(u)?, small_string(u)?);
    }
    Ok(m)
}

/// A generic multi-type batch (every type `real::encode` supports, with nulls).
/// `min_rows` is 1 for the Parquet modes, which refuse an empty block.
pub fn batch(u: &mut Unstructured, min_rows: usize) -> Result<RecordBatch> {
    let rows = u.int_in_range(min_rows..=200usize)?;
    let n = u.int_in_range(1..=6usize)?;
    let types = tys(u, n)?;
    let mut fields = Vec::new();
    let mut arrays = Vec::new();
    for (i, ty) in types.into_iter().enumerate() {
        let nullable = u.arbitrary::<bool>()?;
        let a = array(u, ty, rows, nullable)?;
        let nullable = nullable || a.null_count() > 0;
        fields.push(Field::new(format!("c{i}"), a.data_type().clone(), nullable));
        arrays.push(a);
    }
    let schema = Arc::new(Schema::new_with_metadata(fields, metadata(u)?));
    Ok(RecordBatch::try_new(schema, arrays).expect("generated batch is consistent"))
}

/// Time column spec for the store: ascending with late events, duplicates and extremes.
fn times(u: &mut Unstructured, rows: usize) -> Result<Vec<i64>> {
    let mut t = edge(u)?;
    (0..rows)
        .map(|_| {
            t = match u.int_in_range(0..=5)? {
                0 => t.wrapping_sub(i64::from(u.int_in_range(0..=50u8)?)),
                1 => t,
                2 => edge(u)?,
                _ => t.wrapping_add(i64::from(u.int_in_range(0..=1000u16)?)),
            };
            Ok(t)
        })
        .collect()
}

pub const SYMBOLS: [&str; 4] = ["BTC", "ETH", "", "β"];

/// Store batches sharing one schema: `t` (Int64), `sym` (LargeUtf8), `idx` (row id), extras.
pub fn store_batches(u: &mut Unstructured) -> Result<Vec<RecordBatch>> {
    let extra = u.int_in_range(0..=2usize)?;
    let types = tys(u, extra)?;
    let nullable: Vec<bool> = (0..extra).map(|_| u.arbitrary()).collect::<Result<_>>()?;
    let mut fields = vec![
        Field::new("t", DataType::Int64, false),
        Field::new("sym", DataType::LargeUtf8, false),
        Field::new("idx", DataType::Int64, false),
    ];
    let mut shapes = Vec::new();
    for (i, ty) in types.into_iter().enumerate() {
        let dt = match ty {
            Ty::I8 => DataType::Int8,
            Ty::Ts(tz) => DataType::Timestamp(TimeUnit::Millisecond, tz.map(Into::into)),
            Ty::I64 => DataType::Int64,
            Ty::F64 => DataType::Float64,
            Ty::Bool => DataType::Boolean,
            Ty::Str => DataType::LargeUtf8,
        };
        fields.push(Field::new(format!("x{i}"), dt, nullable[i]));
        shapes.push((ty, nullable[i]));
    }
    let schema = Arc::new(Schema::new_with_metadata(fields, metadata(u)?));
    let mut out = Vec::new();
    let mut next_idx = 0i64;
    for _ in 0..u.int_in_range(1..=3usize)? {
        let rows = u.int_in_range(1..=40usize)?;
        let syms: Vec<&str> = (0..rows)
            .map(|_| Ok(SYMBOLS[u.int_in_range(0..=SYMBOLS.len() - 1)?]))
            .collect::<Result<_>>()?;
        let mut arrays: Vec<ArrayRef> = vec![
            Arc::new(Int64Array::from(times(u, rows)?)),
            Arc::new(LargeStringArray::from(syms)),
            Arc::new(Int64Array::from(
                (0..rows as i64).map(|i| next_idx + i).collect::<Vec<_>>(),
            )),
        ];
        next_idx += rows as i64;
        for (ty, nullable) in &shapes {
            arrays.push(array(u, *ty, rows, *nullable)?);
        }
        out.push(RecordBatch::try_new(schema.clone(), arrays).expect("consistent"));
    }
    Ok(out)
}

/// A batch satisfying the normalized contract of `gqodb-codec` for `family`.
pub fn normalized(u: &mut Unstructured, orderbook: bool, quote: bool) -> Result<RecordBatch> {
    let rows = u.int_in_range(1..=60usize)?;
    let sha: String = (0..64)
        .map(|_| {
            Ok(char::from(
                b"0123456789abcdef"[u.int_in_range(0..=15usize)?],
            ))
        })
        .collect::<Result<_>>()?;
    let sha = if sha.bytes().all(|b| b == b'0') {
        "1".repeat(64)
    } else {
        sha
    };
    let mut meta: HashMap<String, String> = HashMap::from([
        (
            "gqodb.family".into(),
            if orderbook { "orderbook" } else { "tick" }.into(),
        ),
        ("gqodb.schema_version".into(), "1".into()),
        ("gqodb.source_sha256".into(), sha),
        (
            "gqodb.price_decimals".into(),
            u.int_in_range(0..=12u8)?.to_string(),
        ),
        (
            "gqodb.quantity_decimals".into(),
            u.int_in_range(0..=12u8)?.to_string(),
        ),
    ]);
    let mut cols: Vec<(String, ArrayRef, bool)> = Vec::new();
    let opt_i64 = |u: &mut Unstructured| -> Result<ArrayRef> {
        Ok(Arc::new(Int64Array::from(
            (0..rows)
                .map(|_| Ok(if u.ratio(1, 4)? { None } else { Some(edge(u)?) }))
                .collect::<Result<Vec<_>>>()?,
        )))
    };
    for n in ["source_utc_ns", "received_utc_ns", "clock_error_ns"] {
        cols.push((n.into(), opt_i64(u)?, true));
    }
    for n in ["available_utc_ns", "sequence"] {
        cols.push((
            n.into(),
            Arc::new(Int64Array::from(column(u, rows)?)),
            false,
        ));
    }
    let syms: Vec<String> = (0..rows).map(|_| small_string(u)).collect::<Result<_>>()?;
    cols.push((
        "symbol".into(),
        Arc::new(LargeStringArray::from(syms)),
        false,
    ));
    let int64 = |u: &mut Unstructured| -> Result<ArrayRef> {
        Ok(Arc::new(Int64Array::from(column(u, rows)?)))
    };
    let int8 = |u: &mut Unstructured| -> Result<ArrayRef> {
        Ok(Arc::new(Int8Array::from(
            (0..rows)
                .map(|_| u.arbitrary::<i8>())
                .collect::<Result<Vec<_>>>()?,
        )))
    };
    if orderbook {
        meta.insert(
            "gqodb.depth".into(),
            u.int_in_range(1..=100_000u32)?.to_string(),
        );
        for n in ["price", "quantity", "level"] {
            cols.push((n.into(), int64(u)?, false));
        }
        for n in ["side", "action"] {
            cols.push((n.into(), int8(u)?, false));
        }
    } else if quote {
        meta.insert("gqodb.subtype".into(), "quote".into());
        for n in ["bid", "ask", "bid_quantity", "ask_quantity"] {
            cols.push((n.into(), int64(u)?, false));
        }
    } else {
        meta.insert("gqodb.subtype".into(), "trade".into());
        for n in ["price", "quantity"] {
            cols.push((n.into(), int64(u)?, false));
        }
        cols.push(("side".into(), int8(u)?, false));
    }
    let fields: Vec<Field> = cols
        .iter()
        .map(|(n, a, nullable)| Field::new(n, a.data_type().clone(), *nullable))
        .collect();
    Ok(RecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(fields, meta)),
        cols.into_iter().map(|c| c.1).collect(),
    )
    .expect("consistent"))
}
