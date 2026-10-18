use gqodb_lang::Code;
use gqodb_lang::lexer::{Tok, lex};

fn kinds(src: &str) -> Vec<Tok> {
    lex(src).unwrap().into_iter().map(|t| t.tok).collect()
}

fn atom(s: &str) -> Tok {
    Tok::Atom(s.to_string())
}

#[test]
fn words_pipes_commas_and_equals() {
    assert_eq!(
        kinds("show trades EURUSD | take 5"),
        vec![
            atom("show"),
            atom("trades"),
            atom("EURUSD"),
            Tok::Pipe,
            atom("take"),
            atom("5")
        ]
    );
    assert_eq!(
        kinds("columns a,b"),
        vec![atom("columns"), atom("a"), Tok::Comma, atom("b")]
    );
    assert_eq!(
        kinds("symbol=EURUSD"),
        vec![atom("symbol"), Tok::Eq, atom("EURUSD")]
    );
}

#[test]
fn a_range_is_one_atom() {
    assert_eq!(
        kinds("2026-03-01..2026-03-02"),
        vec![atom("2026-03-01..2026-03-02")]
    );
    assert_eq!(
        kinds("2026-03-01T12:30:00.5Z.."),
        vec![atom("2026-03-01T12:30:00.5Z..")]
    );
    assert_eq!(kinds("..2026-03-02"), vec![atom("..2026-03-02")]);
}

#[test]
fn quotes_keep_their_text_and_do_not_split() {
    assert_eq!(kinds("'1INCHUSDT'"), vec![Tok::Quoted("1INCHUSDT".into())]);
    assert_eq!(kinds("\"EUR/USD\""), vec![Tok::Quoted("EUR/USD".into())]);
    assert_eq!(
        kinds("'a | b, c = d'"),
        vec![Tok::Quoted("a | b, c = d".into())]
    );
    assert_eq!(kinds("''"), vec![Tok::Quoted(String::new())]);
}

#[test]
fn separators_and_comments() {
    assert_eq!(
        kinds("show a # a comment | not lexed\nshow b;show c"),
        vec![
            atom("show"),
            atom("a"),
            Tok::Sep,
            atom("show"),
            atom("b"),
            Tok::Sep,
            atom("show"),
            atom("c")
        ]
    );
    assert_eq!(kinds("# only a comment"), vec![]);
    assert_eq!(kinds("   \t "), vec![]);
}

#[test]
fn spans_are_byte_offsets_also_after_multibyte_text() {
    let toks = lex("show 'Åx' z").unwrap();
    assert_eq!((toks[1].span.start, toks[1].span.end), (5, 10)); // 'Åx' is 5 bytes: quote, 2-byte letter, x, quote
    assert_eq!((toks[2].span.start, toks[2].span.end), (11, 12));
}

#[test]
fn unclosed_quotes_are_errors_with_a_position() {
    let e = lex("show 'abc").unwrap_err();
    assert_eq!(e.code, Code::Syntax);
    assert_eq!((e.span.start, e.span.end), (5, 9));
    let e = lex("show 'ab\nc'").unwrap_err();
    assert_eq!(e.code, Code::Syntax);
    assert!(e.message.contains("same line"));
}
