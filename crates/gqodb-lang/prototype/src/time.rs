//! UTC instants as whole nanoseconds since 1970-01-01T00:00:00Z. No dependencies, no floats.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeError {
    /// Not of the form YYYY-MM-DD or YYYY-MM-DDTHH:MM:SS[.fraction]Z.
    Syntax,
    /// A field is outside its calendar range (month 13, 30 February, hour 24, ...).
    Calendar,
    /// The instant does not fit in an i64 of nanoseconds (before 1677-09-21 or after 2262-04-11).
    OutOfRange,
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = (y - era * 400) as u32;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        _ => 28,
    }
}

fn digits(s: &str, n: usize) -> Option<u32> {
    if s.len() != n || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Parses `YYYY-MM-DD` (midnight UTC) or `YYYY-MM-DDTHH:MM:SS[.f]Z` with 1 to 9 fraction digits.
pub fn parse_instant(s: &str) -> Result<i64, TimeError> {
    let (date, time) = match s.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (s, None),
    };
    let mut parts = date.split('-');
    let (y, m, d) = match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(y), Some(m), Some(d), None) => (y, m, d),
        _ => return Err(TimeError::Syntax),
    };
    let year = digits(y, 4).ok_or(TimeError::Syntax)? as i64;
    let month = digits(m, 2).ok_or(TimeError::Syntax)?;
    let day = digits(d, 2).ok_or(TimeError::Syntax)?;
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return Err(TimeError::Calendar);
    }
    let (mut secs_of_day, mut frac_ns) = (0_i64, 0_i64);
    if let Some(t) = time {
        let t = t.strip_suffix('Z').ok_or(TimeError::Syntax)?;
        let (hms, frac) = match t.split_once('.') {
            Some((h, f)) => (h, Some(f)),
            None => (t, None),
        };
        let mut p = hms.split(':');
        let (h, mi, se) = match (p.next(), p.next(), p.next(), p.next()) {
            (Some(h), Some(mi), Some(se), None) => (h, mi, se),
            _ => return Err(TimeError::Syntax),
        };
        let (h, mi, se) = (
            digits(h, 2).ok_or(TimeError::Syntax)?,
            digits(mi, 2).ok_or(TimeError::Syntax)?,
            digits(se, 2).ok_or(TimeError::Syntax)?,
        );
        if h > 23 || mi > 59 || se > 59 {
            return Err(TimeError::Calendar);
        }
        secs_of_day = (h * 3600 + mi * 60 + se) as i64;
        if let Some(f) = frac {
            if f.is_empty() || f.len() > 9 || !f.bytes().all(|b| b.is_ascii_digit()) {
                return Err(TimeError::Syntax);
            }
            let padded = format!("{f:0<9}");
            frac_ns = padded.parse::<i64>().map_err(|_| TimeError::Syntax)?;
        }
    }
    let days = days_from_civil(year, month, day);
    // i128 so that the extreme representable instants (i64::MIN, i64::MAX) do not overflow half-way.
    let total =
        (i128::from(days) * 86_400 + i128::from(secs_of_day)) * 1_000_000_000 + i128::from(frac_ns);
    i64::try_from(total).map_err(|_| TimeError::OutOfRange)
}

/// `YYYY-MM-DDTHH:MM:SS.nnnnnnnnnZ`, always nine fraction digits. Inverse of `parse_instant` for every i64.
pub fn format_instant(ns: i64) -> String {
    let secs = ns.div_euclid(1_000_000_000);
    let frac = ns.rem_euclid(1_000_000_000);
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{frac:09}Z",
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}
