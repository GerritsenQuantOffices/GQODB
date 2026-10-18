//! The exact text people see. Changing a message is a deliberate act: update the expectation here too.
use gqodb_lang::{Session, parse, plan, render_all};

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

#[test]
fn at_most_twenty_diagnostics_are_shown_then_one_line_with_the_rest() {
    let shown = |k: usize| {
        let src = "x;".repeat(k);
        render_all(&parse(&src).unwrap_err(), &src)
    };
    let twenty = shown(20);
    assert_eq!(twenty.matches("error[").count(), 20);
    assert!(!twenty.contains("... and"), "{twenty}");
    let more = shown(21);
    assert_eq!(more.matches("error[").count(), 20);
    assert!(more.ends_with("... and 1 more\n"), "{more}");
    assert!(shown(1000).ends_with("... and 980 more\n"));
}

/// The two lines under `error[...]`: what is shown of the source line, and the caret line (both without the indent).
fn shown_and_carets(text: &str) -> (String, String) {
    let lines: Vec<&str> = text.lines().collect();
    (lines[1][2..].to_string(), lines[2][2..].to_string())
}

#[test]
fn a_long_line_is_cut_around_the_caret_without_splitting_a_character() {
    for unit in ["1", "é", "€", "𝄞"] {
        let first = unit.chars().next().unwrap();
        // the caret moves along a long line of spaces
        for pad in 0..260 {
            let src = format!("{}show trades {}", " ".repeat(pad), unit.repeat(7));
            let text = render_all(&parse(&src).unwrap_err(), &src);
            let (shown, carets) = shown_and_carets(&text);
            let column = carets.chars().take_while(|c| *c == ' ').count();
            assert!(shown.chars().count() <= 126, "{unit} {pad}: {shown}");
            assert!(carets.chars().count() <= 126 + 7, "{unit} {pad}");
            assert_eq!(
                shown.chars().nth(column),
                Some(first),
                "{unit} {pad}\n{text}"
            );
            let marks = carets.trim_start().chars().count();
            assert!(
                (1..=7).contains(&marks) && carets.trim().chars().all(|c| c == '^'),
                "{unit} {pad}"
            );
        }
        // the caret comes after many multibyte characters
        for pad in 1..260 {
            let src = format!("show trades '{}' bybit 1xx", unit.repeat(pad));
            let text = render_all(&parse(&src).unwrap_err(), &src);
            let (shown, carets) = shown_and_carets(&text);
            let column = carets.chars().take_while(|c| *c == ' ').count();
            assert!(shown.chars().count() <= 126, "{unit} {pad}: {shown}");
            assert_eq!(shown.chars().nth(column), Some('1'), "{unit} {pad}\n{text}");
        }
        // the caret stands just after the end of a long line
        for pad in 0..260 {
            let src = format!("{}show trades", unit.repeat(pad).replace(unit, " "));
            let text = render_all(&parse(&src).unwrap_err(), &src);
            let (shown, carets) = shown_and_carets(&text);
            let column = carets.chars().take_while(|c| *c == ' ').count();
            assert!(shown.chars().count() <= 126, "{unit} {pad}: {shown}");
            assert!(
                column <= shown.chars().count() && carets.contains('^'),
                "{unit} {pad}"
            );
        }
    }
}

#[test]
fn a_short_line_is_shown_whole_and_a_long_one_says_where_it_was_cut() {
    let src = format!("show trades {} {}", "1".repeat(200), "z".repeat(200));
    let text = render_all(&parse(&src).unwrap_err(), &src);
    let (shown, _) = shown_and_carets(&text);
    assert!(shown.ends_with("..."), "{shown}");
    let src = format!("{}show trades 1x", " ".repeat(300));
    let text = render_all(&parse(&src).unwrap_err(), &src);
    let (shown, _) = shown_and_carets(&text);
    assert!(
        shown.starts_with("...") && !shown.ends_with("..."),
        "{shown}"
    );
}
