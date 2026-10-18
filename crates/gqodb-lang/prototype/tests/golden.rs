//! The exact text people see. Changing a message is a deliberate act: update the expectation here too.
use gqodb_lang::{Session, parse, plan};

fn rendered(src: &str) -> String {
    let script = parse(src);
    let diags = match script {
        Err(d) => d,
        Ok(s) => plan(&s.commands[0], &Session::default()).expect_err("expected an error"),
    };
    diags
        .iter()
        .map(|d| d.render(src))
        .collect::<Vec<_>>()
        .join("")
}

#[test]
fn clock_missing() {
    assert_eq!(
        rendered("show trades EURUSD 2026-03-01..2026-03-02 in ./data"),
        "error[E-CLOCK]: a range needs a clock\n  show trades EURUSD 2026-03-01..2026-03-02 in ./data\n                     ^^^^^^^^^^^^^^^^^^^^^^\n  help: add one of: on available | on received | on source | on sequence\n"
    );
}

#[test]
fn unquoted_symbol() {
    assert_eq!(
        rendered("show trades 1INCHUSDT"),
        "error[E-SYNTAX]: `1INCHUSDT` cannot be written without quotes\n  show trades 1INCHUSDT\n              ^^^^^^^^^\n  help: an unquoted name starts with a letter and holds letters, digits and . _ : / - ; otherwise put it in quotes, e.g. '1INCHUSDT'\n"
    );
}

#[test]
fn needs_the_engine() {
    assert_eq!(
        rendered("show trades EURUSD as of 2026-03-05"),
        "error[E-ENGINE]: `as` needs the GQODB engine\n  show trades EURUSD as of 2026-03-05\n                     ^^\n  help: this construct belongs to the paid engine; the free executor does not run it\n"
    );
}

#[test]
fn unknown_stream_and_a_calendar_error() {
    assert_eq!(
        rendered("show tradez EURUSD"),
        "error[E-STREAM]: unknown stream `tradez`\n  show tradez EURUSD\n       ^^^^^^\n  help: streams: trades, quotes, book\n"
    );
    assert_eq!(
        rendered("show trades X 2026-02-30..2026-03-02 on received in ./d"),
        "error[E-RANGE]: `2026-02-30` is not a real date or time\n  show trades X 2026-02-30..2026-03-02 on received in ./d\n                ^^^^^^^^^^\n  help: check month, day, hour, minute and second\n"
    );
}

#[test]
fn the_caret_follows_characters_not_bytes() {
    assert_eq!(
        rendered("show 'ÅÅ' x"),
        "error[E-SYNTAX]: unexpected `ÅÅ`\n  show 'ÅÅ' x\n       ^^^^\n"
    );
}
