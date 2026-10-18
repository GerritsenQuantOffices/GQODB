//! Which rows of a decoded block belong to the answer. Pure and independent of files, so it can be tested alone.
use anyhow::{Context, Result};
use arrow_array::{Array, Int64Array, LargeStringArray, RecordBatch};
use gqodb_lang::plan::ResolvedRange;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Picked {
    /// Row numbers in the block, in block order.
    pub rows: Vec<usize>,
    /// Rows of the symbol that were left out because the chosen clock is empty for them.
    pub null_clock: u64,
}

/// Rows with `symbol` equal to `symbol` (exact, case sensitive) and, if a range is given, a non-empty value on the
/// range's clock inside the half-open range `[start, end)`. A row with an empty clock value is never inside any range.
pub fn pick_rows(
    batch: &RecordBatch,
    symbol: &str,
    range: Option<&ResolvedRange>,
) -> Result<Picked> {
    let symbols = batch
        .column_by_name("symbol")
        .and_then(|c| c.as_any().downcast_ref::<LargeStringArray>())
        .context("the block has no text column `symbol`")?;
    let clock = match range {
        Some(r) => Some(
            batch
                .column_by_name(r.clock.column())
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
                .with_context(|| {
                    format!("the block has no integer column `{}`", r.clock.column())
                })?,
        ),
        None => None,
    };
    let mut picked = Picked::default();
    for row in 0..batch.num_rows() {
        if symbols.is_null(row) || symbols.value(row) != symbol {
            continue;
        }
        if let (Some(r), Some(col)) = (range, clock) {
            if col.is_null(row) {
                picked.null_clock += 1;
                continue;
            }
            let v = col.value(row);
            if r.start.is_some_and(|s| v < s) || r.end.is_some_and(|e| v >= e) {
                continue;
            }
        }
        picked.rows.push(row);
    }
    Ok(picked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_schema::{DataType, Field, Schema};
    use gqodb_lang::ClockName;
    use std::sync::Arc;

    fn batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("symbol", DataType::LargeUtf8, false),
            Field::new("received_utc_ns", DataType::Int64, true),
            Field::new("available_utc_ns", DataType::Int64, false),
        ]));
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(LargeStringArray::from(vec!["A", "A", "B", "A", "A", "a"])),
                Arc::new(Int64Array::from(vec![
                    Some(10),
                    None,
                    Some(10),
                    Some(20),
                    Some(30),
                    Some(10),
                ])),
                Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5, 6])),
            ],
        )
        .unwrap()
    }

    fn range(clock: ClockName, start: Option<i64>, end: Option<i64>) -> ResolvedRange {
        ResolvedRange { clock, start, end }
    }

    #[test]
    fn symbol_only() {
        let p = pick_rows(&batch(), "A", None).unwrap();
        assert_eq!((p.rows, p.null_clock), (vec![0, 1, 3, 4], 0));
        assert!(pick_rows(&batch(), "Z", None).unwrap().rows.is_empty());
    }

    #[test]
    fn symbols_are_case_sensitive() {
        assert_eq!(pick_rows(&batch(), "a", None).unwrap().rows, vec![5]);
    }

    #[test]
    fn a_range_is_half_open_and_empty_clocks_are_left_out_and_counted() {
        let r = range(ClockName::Received, Some(10), Some(30));
        let p = pick_rows(&batch(), "A", Some(&r)).unwrap();
        assert_eq!(p.rows, vec![0, 3], "10 is in, 30 is out");
        assert_eq!(p.null_clock, 1, "row 1 has no received time");
        let r = range(ClockName::Received, None, None);
        let p = pick_rows(&batch(), "A", Some(&r)).unwrap();
        assert_eq!(
            (p.rows, p.null_clock),
            (vec![0, 3, 4], 1),
            "even an open range leaves out empty clocks"
        );
        let r = range(ClockName::Received, Some(31), None);
        assert!(pick_rows(&batch(), "A", Some(&r)).unwrap().rows.is_empty());
    }

    #[test]
    fn a_clock_without_empty_values_counts_nothing() {
        let r = range(ClockName::Available, Some(2), Some(5));
        let p = pick_rows(&batch(), "A", Some(&r)).unwrap();
        assert_eq!((p.rows, p.null_clock), (vec![1, 3], 0));
    }

    #[test]
    fn a_missing_column_is_an_error() {
        let r = range(ClockName::Source, None, None);
        assert!(pick_rows(&batch(), "A", Some(&r)).is_err());
    }
}
