use gqodb_lang::ast::{ClockName, Stream};
use gqodb_lang::{Code, Diagnostic, Plan, Session, parse, plan};

fn run(src: &str, session: &Session) -> Result<Plan, Vec<Diagnostic>> {
    let script = parse(src).unwrap_or_else(|e| panic!("{src}: {e:?}"));
    plan(&script.commands[0], session)
}

fn ok(src: &str) -> Plan {
    run(src, &Session::default()).unwrap_or_else(|e| panic!("{src}: {e:?}"))
}

fn errs(src: &str, session: &Session) -> Vec<Diagnostic> {
    run(src, session).expect_err(src)
}

const DAY: i64 = 86_400 * 1_000_000_000;
const MARCH_1: i64 = 1_772_323_200 * 1_000_000_000;

#[test]
fn a_complete_plan() {
    let p = ok(
        "show trades EURUSD bybit 2026-03-01..2026-03-02 on received in ./data | skip 2 | take 5 | columns price,side | order on source desc",
    );
    assert!(!p.explain);
    assert_eq!(
        (
            p.stream,
            p.symbol.as_str(),
            p.venue.as_deref(),
            p.path.as_str()
        ),
        (Stream::Trades, "EURUSD", Some("bybit"), "./data")
    );
    let r = p.range.unwrap();
    assert_eq!(
        (r.clock, r.start, r.end),
        (ClockName::Received, Some(MARCH_1), Some(MARCH_1 + DAY))
    );
    assert_eq!((p.skip, p.take), (2, Some(5)));
    assert_eq!(p.columns.unwrap(), ["price", "side"]);
    let o = p.order.unwrap();
    assert_eq!((o.clock, o.desc), (ClockName::Source, true));
}

#[test]
fn without_a_range_nothing_needs_a_clock() {
    let p = ok("show trades EURUSD in ./data");
    assert!(
        p.range.is_none()
            && p.order.is_none()
            && p.columns.is_none()
            && p.skip == 0
            && p.take.is_none()
    );
}

#[test]
fn a_range_without_a_clock_is_an_error_that_names_the_choices() {
    let src = "show trades EURUSD 2026-03-01..2026-03-02 in ./data";
    let e = errs(src, &Session::default());
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].code, Code::Clock);
    assert_eq!(
        &src[e[0].span.start..e[0].span.end],
        "2026-03-01..2026-03-02"
    );
    assert_eq!(
        e[0].help.as_deref(),
        Some("add one of: on available | on received | on source | on sequence")
    );
}

#[test]
fn use_clock_and_use_path_fill_the_gaps() {
    let mut session = Session::default();
    for line in ["use clock broker", "use path ./data"] {
        let script = parse(line).unwrap();
        assert!(session.apply(&script.commands[0].node));
    }
    let p = run("show trades EURUSD 2026-03-01..", &session).unwrap();
    assert_eq!(p.range.unwrap().clock, ClockName::Source);
    assert_eq!(p.path, "./data");
    assert!(!session.apply(&parse("quit").unwrap().commands[0].node));
    // a clock given in the command wins over the session
    let p = run("show trades EURUSD 2026-03-01.. on received", &session).unwrap();
    assert_eq!(p.range.unwrap().clock, ClockName::Received);
}

#[test]
fn ranges_are_typed_by_their_clock() {
    let p = ok("show book X 100..200 on sequence in ./d");
    let r = p.range.unwrap();
    assert_eq!((r.start, r.end), (Some(100), Some(200)));
    let p = ok("show book X 100.. on sequence in ./d");
    assert_eq!((p.range.unwrap().end, p.take), (None, None));
    let p = ok("show quotes X ..2026-03-01T00:00:00.000000001Z on available in ./d");
    assert_eq!(p.range.unwrap().end, Some(MARCH_1 + 1));
}

#[test]
fn bad_ranges_are_errors_with_the_right_text_marked() {
    let s = Session::default();
    let src = "show trades X 2026-02-30..2026-03-02 on received in ./d";
    let e = errs(src, &s);
    assert_eq!(e[0].code, Code::Range);
    assert_eq!(&src[e[0].span.start..e[0].span.end], "2026-02-30");
    let src = "show trades X 100..2026-03-02 on received in ./d";
    assert_eq!(errs(src, &s)[0].code, Code::Range);
    let src = "show trades X 2026-03-02..2026-03-01 on received in ./d";
    assert!(
        errs(src, &s)[0]
            .message
            .contains("begin must be before the end")
    );
    let src = "show trades X 2026-03-01..2026-03-01 on received in ./d";
    assert_eq!(errs(src, &s)[0].code, Code::Range);
    assert_eq!(
        errs("show book X 5..2026-01-01 on sequence in ./d", &s)[0].code,
        Code::Range
    );
    assert_eq!(
        errs("show book X 9..2 on sequence in ./d", &s)[0].code,
        Code::Range
    );
    assert_eq!(
        errs("show book X 99999999999999999999..1 on sequence in ./d", &s)[0].code,
        Code::Range
    );
    assert_eq!(
        errs("show trades X 2300-01-01.. on received in ./d", &s)[0].code,
        Code::Range
    );
    let e = errs(
        "show trades X 2026-13-01..2026-14-01 on received in ./d",
        &s,
    );
    assert_eq!(e.len(), 2, "both sides are reported");
}

#[test]
fn a_path_is_required() {
    let e = errs("show trades EURUSD", &Session::default());
    assert_eq!(e[0].code, Code::Path);
    assert!(e[0].help.as_deref().unwrap().contains("use path"));
}

#[test]
fn columns_are_checked_against_the_stream() {
    let s = Session::default();
    assert!(run("show quotes X in ./d | columns bid,ask,bid_quantity", &s).is_ok());
    assert!(run("show book X in ./d | columns level,action", &s).is_ok());
    let e = errs("show trades X in ./d | columns price,bid", &s);
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].code, Code::Column);
    assert!(
        e[0].help
            .as_deref()
            .unwrap()
            .contains("price, quantity, side")
    );
    assert_eq!(
        errs("show trades X in ./d | columns price,price", &s)[0].code,
        Code::Column
    );
}

#[test]
fn each_step_once_and_order_needs_a_clock() {
    let s = Session::default();
    assert_eq!(
        errs("show trades X in ./d | take 1 | take 2", &s)[0].code,
        Code::Syntax
    );
    assert_eq!(
        errs("show trades X in ./d | skip 1 | skip 2", &s)[0].code,
        Code::Syntax
    );
    assert_eq!(
        errs("show trades X in ./d | columns price | columns side", &s)[0].code,
        Code::Syntax
    );
    assert_eq!(
        errs(
            "show trades X in ./d | order on source | order on received",
            &s
        )[0]
        .code,
        Code::Syntax
    );
    assert_eq!(
        errs("show trades X in ./d | order", &s)[0].code,
        Code::Clock
    );
    // order falls back on the clock of the range, then of the session
    let p = ok("show trades X 1..2 on sequence in ./d | order desc");
    assert_eq!(p.order.unwrap().clock, ClockName::Sequence);
    let session = Session {
        clock: Some(ClockName::Available),
        path: None,
    };
    assert_eq!(
        run("show trades X in ./d | order", &session)
            .unwrap()
            .order
            .unwrap()
            .clock,
        ClockName::Available
    );
}

#[test]
fn explain_marks_the_plan_and_other_commands_do_not_plan() {
    assert!(ok("explain show trades X in ./d").explain);
    let script = parse("quit").unwrap();
    assert_eq!(
        plan(&script.commands[0], &Session::default()).unwrap_err()[0].code,
        Code::Syntax
    );
}

#[test]
fn several_problems_in_one_command_are_all_reported() {
    let e = errs(
        "show trades X 2026-03-01..2026-03-02 | columns nope",
        &Session::default(),
    );
    let codes: Vec<Code> = e.iter().map(|d| d.code).collect();
    assert_eq!(codes, [Code::Clock, Code::Path, Code::Column]);
}
