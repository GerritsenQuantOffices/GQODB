use gqodb_lang::time::{TimeError, format_instant, parse_instant};

#[test]
fn epoch_and_known_dates() {
    assert_eq!(parse_instant("1970-01-01"), Ok(0));
    assert_eq!(parse_instant("2026-03-01"), Ok(1_772_323_200_000_000_000));
    assert_eq!(
        parse_instant("2026-03-01T12:30:00Z"),
        Ok(1_772_323_200_000_000_000 + 45_000 * 1_000_000_000)
    );
    assert_eq!(
        parse_instant("2026-03-01T00:00:00.5Z"),
        Ok(1_772_323_200_500_000_000)
    );
    assert_eq!(
        parse_instant("2026-03-01T00:00:00.123456789Z"),
        Ok(1_772_323_200_123_456_789)
    );
}

#[test]
fn leap_years_and_calendar_errors() {
    assert!(parse_instant("2024-02-29").is_ok());
    assert_eq!(parse_instant("2026-02-29"), Err(TimeError::Calendar));
    assert_eq!(parse_instant("2100-02-29"), Err(TimeError::Calendar));
    assert!(parse_instant("2000-02-29").is_ok());
    assert_eq!(parse_instant("2026-13-01"), Err(TimeError::Calendar));
    assert_eq!(parse_instant("2026-00-10"), Err(TimeError::Calendar));
    assert_eq!(
        parse_instant("2026-03-01T24:00:00Z"),
        Err(TimeError::Calendar)
    );
    assert_eq!(
        parse_instant("2026-03-01T23:59:60Z"),
        Err(TimeError::Calendar)
    );
}

#[test]
fn syntax_errors() {
    for bad in [
        "",
        "2026",
        "2026-3-1",
        "2026-03-01T12:00",
        "2026-03-01T12:00:00",
        "2026-03-01T12:00:00+01:00",
        "2026-03-01T12:00:00.Z",
        "2026-03-01T12:00:00.1234567890Z",
        "2026-03-01 12:00:00Z",
        "x026-03-01",
    ] {
        assert_eq!(parse_instant(bad), Err(TimeError::Syntax), "{bad}");
    }
}

#[test]
fn the_edges_of_a_64_bit_nanosecond_count() {
    assert_eq!(
        parse_instant("1677-09-21T00:12:43.145224192Z"),
        Ok(i64::MIN)
    );
    assert_eq!(
        parse_instant("1677-09-21T00:12:43.145224191Z"),
        Err(TimeError::OutOfRange)
    );
    assert_eq!(
        parse_instant("2262-04-11T23:47:16.854775807Z"),
        Ok(i64::MAX)
    );
    assert_eq!(
        parse_instant("2262-04-11T23:47:16.854775808Z"),
        Err(TimeError::OutOfRange)
    );
    assert_eq!(parse_instant("2262-04-12"), Err(TimeError::OutOfRange));
}

#[test]
fn format_is_the_inverse_of_parse_for_every_i64_we_try() {
    for ns in [
        0_i64,
        1,
        -1,
        1_772_323_200_123_456_789,
        i64::MAX,
        i64::MIN,
        86_399_999_999_999,
    ] {
        let text = format_instant(ns);
        assert_eq!(parse_instant(&text), Ok(ns), "{text}");
    }
    assert_eq!(format_instant(0), "1970-01-01T00:00:00.000000000Z");
    assert_eq!(
        format_instant(1_772_323_200_123_456_789),
        "2026-03-01T00:00:00.123456789Z"
    );
}
