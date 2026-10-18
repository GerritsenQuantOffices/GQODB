//! Cells of a result and their exact text. Numbers never pass through a float.
use anyhow::{Context, Result, bail};
use arrow_array::{Array, Int8Array, Int64Array, LargeStringArray};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    Null,
    Int(i64),
    /// Text, including exact decimals ("1.08342") and UTC times.
    Text(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeFormat {
    /// `2026-03-01T00:00:00.123456789Z`
    Iso,
    /// whole nanoseconds since the epoch
    Ns,
}

/// What a column of the normalized schema holds. Decides how its cells are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Symbol,
    Time,
    Price,
    Quantity,
    Int,
}

pub fn kind_of(column: &str) -> Kind {
    match column {
        "symbol" => Kind::Symbol,
        "source_utc_ns" | "received_utc_ns" | "available_utc_ns" => Kind::Time,
        "price" | "bid" | "ask" => Kind::Price,
        "quantity" | "bid_quantity" | "ask_quantity" => Kind::Quantity,
        _ => Kind::Int,
    }
}

/// `value / 10^scale` written exactly with `scale` fraction digits: (108342, 5) is "1.08342", (-5, 1) is "-0.5".
pub fn format_scaled(value: i64, scale: u8) -> String {
    let digits = value.unsigned_abs().to_string();
    let scale = usize::from(scale);
    let body = if scale == 0 {
        digits
    } else {
        let padded = if digits.len() <= scale {
            format!("{}{}", "0".repeat(scale + 1 - digits.len()), digits)
        } else {
            digits
        };
        let (whole, fraction) = padded.split_at(padded.len() - scale);
        format!("{whole}.{fraction}")
    };
    if value < 0 { format!("-{body}") } else { body }
}

/// Reads row `row` of `array` as a cell. `scales` are the file's `gqodb.price_decimals` and `gqodb.quantity_decimals`.
pub fn cell(
    array: &dyn Array,
    row: usize,
    kind: Kind,
    scales: (u8, u8),
    time: TimeFormat,
) -> Result<Cell> {
    if array.is_null(row) {
        return Ok(Cell::Null);
    }
    if kind == Kind::Symbol {
        let text = array
            .as_any()
            .downcast_ref::<LargeStringArray>()
            .context("column is not text")?;
        return Ok(Cell::Text(text.value(row).to_string()));
    }
    let value = if let Some(a) = array.as_any().downcast_ref::<Int64Array>() {
        a.value(row)
    } else if let Some(a) = array.as_any().downcast_ref::<Int8Array>() {
        i64::from(a.value(row))
    } else {
        bail!("column is not an integer column");
    };
    Ok(match kind {
        Kind::Price => Cell::Text(format_scaled(value, scales.0)),
        Kind::Quantity => Cell::Text(format_scaled(value, scales.1)),
        Kind::Time if time == TimeFormat::Iso => {
            Cell::Text(gqodb_lang::time::format_instant(value))
        }
        _ => Cell::Int(value),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_decimals_are_exact() {
        assert_eq!(format_scaled(108_342, 5), "1.08342");
        assert_eq!(format_scaled(5, 0), "5");
        assert_eq!(format_scaled(5, 3), "0.005");
        assert_eq!(format_scaled(-5, 1), "-0.5");
        assert_eq!(format_scaled(0, 3), "0.000");
        assert_eq!(format_scaled(-100, 2), "-1.00");
        assert_eq!(format_scaled(100, 2), "1.00");
        assert_eq!(format_scaled(i64::MAX, 0), "9223372036854775807");
        assert_eq!(format_scaled(i64::MIN, 0), "-9223372036854775808");
        assert_eq!(format_scaled(i64::MIN, 12), "-9223372.036854775808");
        assert_eq!(format_scaled(i64::MAX, 12), "9223372.036854775807");
        for scale in 0..=12 {
            let s = format_scaled(1, scale);
            assert_eq!(s.matches('.').count(), usize::from(scale > 0));
            assert_eq!(
                s.len(),
                if scale == 0 {
                    1
                } else {
                    usize::from(scale) + 2
                }
            );
        }
    }

    #[test]
    fn columns_have_a_kind() {
        assert_eq!(kind_of("symbol"), Kind::Symbol);
        assert_eq!(kind_of("received_utc_ns"), Kind::Time);
        assert_eq!(kind_of("clock_error_ns"), Kind::Int);
        assert_eq!(kind_of("bid"), Kind::Price);
        assert_eq!(kind_of("ask_quantity"), Kind::Quantity);
        assert_eq!(kind_of("side"), Kind::Int);
        assert_eq!(kind_of("sequence"), Kind::Int);
    }

    #[test]
    fn cells_read_nulls_times_and_scaled_numbers() {
        let ns = Int64Array::from(vec![Some(1_772_323_200_000_000_001), None]);
        assert_eq!(
            cell(&ns, 0, Kind::Time, (2, 2), TimeFormat::Iso).unwrap(),
            Cell::Text("2026-03-01T00:00:00.000000001Z".into())
        );
        assert_eq!(
            cell(&ns, 0, Kind::Time, (2, 2), TimeFormat::Ns).unwrap(),
            Cell::Int(1_772_323_200_000_000_001)
        );
        assert_eq!(
            cell(&ns, 1, Kind::Time, (2, 2), TimeFormat::Iso).unwrap(),
            Cell::Null
        );
        let price = Int64Array::from(vec![12_345]);
        assert_eq!(
            cell(&price, 0, Kind::Price, (2, 5), TimeFormat::Iso).unwrap(),
            Cell::Text("123.45".into())
        );
        assert_eq!(
            cell(&price, 0, Kind::Quantity, (2, 5), TimeFormat::Iso).unwrap(),
            Cell::Text("0.12345".into())
        );
        let side = Int8Array::from(vec![-1_i8]);
        assert_eq!(
            cell(&side, 0, Kind::Int, (2, 2), TimeFormat::Iso).unwrap(),
            Cell::Int(-1)
        );
        let symbol = LargeStringArray::from(vec!["EURUSD"]);
        assert_eq!(
            cell(&symbol, 0, Kind::Symbol, (2, 2), TimeFormat::Iso).unwrap(),
            Cell::Text("EURUSD".into())
        );
        assert!(cell(&symbol, 0, Kind::Int, (2, 2), TimeFormat::Iso).is_err());
    }
}
