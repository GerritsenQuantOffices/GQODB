use gqodb_lang::ast::{Bound, ClockName, Command, Show, Step, Stream};
use gqodb_lang::{Code, Diagnostic, parse};

fn one(src: &str) -> Command {
    let script = parse(src).unwrap_or_else(|e| panic!("{src}: {e:?}"));
    assert_eq!(script.commands.len(), 1, "{src}");
    script.commands.into_iter().next().unwrap().node
}

fn show(src: &str) -> Show {
    match one(src) {
        Command::Show(s) => s,
        other => panic!("not a show: {other:?}"),
    }
}

fn err(src: &str) -> Diagnostic {
    let mut e = parse(src).expect_err(src);
    assert_eq!(e.len(), 1, "{src}: {e:?}");
    e.remove(0)
}

fn text(src: &str, d: &Diagnostic) -> String {
    src[d.span.start..d.span.end].to_string()
}

#[test]
fn the_full_positional_form() {
    let s =
        show("show trades EURUSD bybit 2026-03-01..2026-03-02 on received in ./data | take 100");
    assert_eq!(s.stream.node, Stream::Trades);
    assert_eq!(s.symbol.node, "EURUSD");
    assert_eq!(s.venue.as_ref().unwrap().node, "bybit");
    let r = s.range.as_ref().unwrap();
    assert_eq!(r.node.start.node, Bound::Value("2026-03-01".into()));
    assert_eq!(r.node.end.node, Bound::Value("2026-03-02".into()));
    assert_eq!(s.clock.as_ref().unwrap().node, ClockName::Received);
    assert_eq!(s.path.as_ref().unwrap().node, "./data");
    assert_eq!(s.steps.len(), 1);
    assert_eq!(s.steps[0].node, Step::Take(100));
}

#[test]
fn every_part_after_the_symbol_is_optional() {
    let s = show("show book BTC-USDT");
    assert!(
        s.venue.is_none()
            && s.range.is_none()
            && s.clock.is_none()
            && s.path.is_none()
            && s.steps.is_empty()
    );
    let s = show("show quotes EURUSD in ./data");
    assert!(s.venue.is_none() && s.range.is_none() && s.path.is_some());
    let s = show("SHOW Trades EURUSD IN ./data");
    assert_eq!(s.stream.node, Stream::Trades);
}

#[test]
fn keywords_are_not_venues_but_quotes_make_them_so() {
    assert!(show("show trades EURUSD in ./d").venue.is_none());
    assert_eq!(
        show("show trades EURUSD 'in' in ./d").venue.unwrap().node,
        "in"
    );
    assert_eq!(
        show("show trades EURUSD \"my venue\"").venue.unwrap().node,
        "my venue"
    );
}

#[test]
fn open_ranges_and_aliases() {
    let r = show("show trades EURUSD 2026-03-01.. on wall")
        .range
        .unwrap()
        .node;
    assert_eq!(r.end.node, Bound::Open);
    let r = show("show trades EURUSD ..2026-03-01 on wall")
        .range
        .unwrap()
        .node;
    assert_eq!(r.start.node, Bound::Open);
    let r = show("show trades EURUSD .. on wall").range.unwrap().node;
    assert_eq!((r.start.node, r.end.node), (Bound::Open, Bound::Open));
    for (word, clock) in [
        ("local", ClockName::Received),
        ("WALL", ClockName::Received),
        ("received", ClockName::Received),
        ("broker", ClockName::Source),
        ("source", ClockName::Source),
        ("available", ClockName::Available),
        ("sequence", ClockName::Sequence),
    ] {
        let s = show(&format!("show trades EURUSD 1..2 on {word}"));
        assert_eq!(s.clock.unwrap().node, clock, "{word}");
    }
}

#[test]
fn quoted_symbols_for_names_that_need_it() {
    assert_eq!(show("show trades '1INCHUSDT'").symbol.node, "1INCHUSDT");
    assert_eq!(show("show trades \"EUR/USD\"").symbol.node, "EUR/USD");
    assert_eq!(show("show trades BRK.B").symbol.node, "BRK.B");
    assert_eq!(
        show("show trades eurusd").symbol.node,
        "eurusd",
        "symbols are case sensitive and kept as typed"
    );
}

#[test]
fn the_named_form_in_any_order() {
    let s = show(
        "show clock=broker path=./data symbol=EURUSD stream=trades range=2026-03-01..2026-03-02 venue=bybit | skip 2 | take 3",
    );
    assert_eq!(s.stream.node, Stream::Trades);
    assert_eq!(s.clock.unwrap().node, ClockName::Source);
    assert!(s.range.is_some() && s.venue.is_some());
    assert_eq!(s.steps.len(), 2);
    let s = show("show stream=book symbol='A B' clock=sequence");
    assert_eq!(s.symbol.node, "A B");
    assert!(s.range.is_none() && s.clock.is_some());
}

#[test]
fn steps() {
    let s = show(
        "show trades EURUSD | skip 5 | take 10 | columns price, quantity,side | order on source desc",
    );
    assert_eq!(s.steps[0].node, Step::Skip(5));
    assert_eq!(s.steps[1].node, Step::Take(10));
    match &s.steps[2].node {
        Step::Columns(c) => assert_eq!(
            c.iter().map(|x| x.node.as_str()).collect::<Vec<_>>(),
            ["price", "quantity", "side"]
        ),
        other => panic!("{other:?}"),
    }
    match &s.steps[3].node {
        Step::Order { clock, desc } => {
            assert!(clock.as_ref().unwrap().node == ClockName::Source && *desc)
        }
        other => panic!("{other:?}"),
    }
    match &show("show trades EURUSD | order").steps[0].node {
        Step::Order { clock, desc } => assert!(clock.is_none() && !*desc),
        other => panic!("{other:?}"),
    }
}

#[test]
fn use_help_quit_and_explain() {
    assert_eq!(
        one("use clock broker"),
        Command::UseClock(ClockName::Source)
    );
    assert_eq!(one("use path ./data"), Command::UsePath("./data".into()));
    assert_eq!(
        one("use path 'my data'"),
        Command::UsePath("my data".into())
    );
    assert_eq!(one("help"), Command::Help(None));
    assert_eq!(one("help clocks"), Command::Help(Some("clocks".into())));
    assert_eq!(one("quit"), Command::Quit);
    assert_eq!(one("exit"), Command::Quit);
    assert!(matches!(
        one("explain show trades EURUSD in ./d"),
        Command::Explain(_)
    ));
}

#[test]
fn a_script_has_several_commands_and_reports_every_broken_one() {
    let script =
        parse("use clock received\n# note\nshow trades A in ./d;show book B in ./d\n\n").unwrap();
    assert_eq!(script.commands.len(), 3);
    let errs = parse("show trades A in ./d\nshow wat B\nshow trades\nquit").unwrap_err();
    assert_eq!(errs.len(), 2);
    assert_eq!((errs[0].code, errs[1].code), (Code::Stream, Code::Syntax));
}

#[test]
fn paid_constructs_are_recognised_not_syntax_errors() {
    let src = "show trades EURUSD as of 2026-03-05";
    let d = err(src);
    assert_eq!(d.code, Code::Engine);
    assert_eq!(text(src, &d), "as");
    let src = "show trades EURUSD 2026-03-01..2026-03-02 on received as of 2026-03-05";
    assert_eq!(err(src).code, Code::Engine);
    let src = "show trades EURUSD in ./d | join quotes";
    assert_eq!(err(src).code, Code::Engine);
    for verb in ["import x", "capture x", "ledger", "join a b", "asof a"] {
        assert_eq!(err(verb).code, Code::Engine, "{verb}");
    }
    assert_eq!(
        show("show trades EURUSD 'as' in ./d").venue.unwrap().node,
        "as",
        "quoted, so a venue"
    );
}

#[test]
fn errors_point_at_the_problem() {
    let src = "show tradez EURUSD";
    let d = err(src);
    assert_eq!(d.code, Code::Stream);
    assert_eq!(text(src, &d), "tradez");
    assert!(d.help.unwrap().contains("trades, quotes, book"));

    let src = "show trades EURUSD 2026-03-01..2026-03-02 on watch";
    let d = err(src);
    assert_eq!(d.code, Code::Clock);
    assert_eq!(text(src, &d), "watch");
    assert!(
        d.help
            .unwrap()
            .contains("available | received | source | sequence")
    );

    let src = "show trades EURUSD 2026-03-01";
    let d = err(src);
    assert_eq!(d.code, Code::Range);
    assert_eq!(text(src, &d), "2026-03-01");

    let src = "show trades EURUSD 1..2..3 on sequence";
    assert_eq!(err(src).code, Code::Range);

    let src = "show trades 1INCHUSDT";
    let d = err(src);
    assert_eq!(d.code, Code::Syntax);
    assert!(d.help.unwrap().contains("quotes"));

    let src = "show trades EURUSD on received";
    let d = err(src);
    assert_eq!(d.code, Code::Syntax);
    assert!(d.message.contains("follows a range"));

    assert_eq!(err("show trades").code, Code::Syntax);
    assert_eq!(err("show").code, Code::Syntax);
    assert_eq!(err("fetch x").code, Code::Syntax);
    assert_eq!(err("show trades EURUSD in").code, Code::Syntax);
    assert_eq!(err("show trades EURUSD | take").code, Code::Syntax);
    assert_eq!(err("show trades EURUSD | take x").code, Code::Syntax);
    assert_eq!(
        err("show trades EURUSD | take 99999999999999999999999").code,
        Code::Syntax
    );
    assert_eq!(err("show trades EURUSD | columns a,").code, Code::Syntax);
    assert_eq!(err("show trades EURUSD | wat").code, Code::Syntax);
    assert_eq!(err("show trades EURUSD |").code, Code::Syntax);
    assert_eq!(err("show trades EURUSD in ./d extra").code, Code::Syntax);
    assert_eq!(err("explain trades").code, Code::Syntax);
    assert_eq!(err("use colour red").code, Code::Syntax);
    assert_eq!(err("use clock").code, Code::Syntax);
    assert_eq!(err("show stream=trades").code, Code::Syntax);
    assert_eq!(
        err("show symbol=A stream=trades symbol=B").code,
        Code::Syntax
    );
    assert_eq!(
        err("show stream=trades symbol=A speed=3").code,
        Code::Syntax
    );
    assert_eq!(
        err("show stream=trades symbol=A clock=watch").code,
        Code::Clock
    );
    assert_eq!(err("show stream=wat symbol=A").code, Code::Stream);
}

#[test]
fn keyword_range_must_be_printable_positionally() {
    // Found by fuzzing (lang_parse): `range=sy6..2026` parsed, but the canonical text writes the
    // range positionally, where it must start with a digit or a dot, so the text did not parse again.
    let src = "show stream=trades symbol=A range=sy6-03-01..2026-04/02 clock=source";
    assert_eq!(err(src).code, Code::Range);
    let src = "show stream=trades symbol=A range=x..y";
    assert_eq!(err(src).code, Code::Range);
    // The accepted forms keep working and print to text that parses again.
    for ok in [
        "show stream=trades symbol=A range=2026-03-01..2026-03-02 clock=source",
        "show stream=trades symbol=A range=..2026-03-02 clock=source",
        "show stream=trades symbol=A range=.. clock=source",
    ] {
        let script = parse(ok).unwrap_or_else(|e| panic!("{ok}: {e:?}"));
        let canonical = gqodb_lang::to_canonical(&script);
        let again = parse(&canonical).unwrap_or_else(|e| panic!("{canonical}: {e:?}"));
        assert_eq!(gqodb_lang::to_canonical(&again), canonical);
    }
}
