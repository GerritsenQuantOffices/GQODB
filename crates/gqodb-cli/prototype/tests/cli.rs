mod support;

use std::io::Write;
use std::process::{Command, Output, Stdio};
use support::*;

fn gqodb(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gqodb"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn gqodb_stdin(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_gqodb"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn err(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).unwrap()
}

fn code(o: &Output) -> i32 {
    o.status.code().unwrap()
}

/// Three blocks of four rows; sequence 1..12; available = BASE + 100, 101, ...
fn data() -> TempDir {
    let dir = TempDir::new();
    let blocks: Vec<Vec<Trade>> = (0..3)
        .map(|b| {
            (0..4)
                .map(|i| trade("EURUSD", 100 * (b + 1) + i, 4 * b + i + 1))
                .collect()
        })
        .collect();
    write_trades(&dir.path("a.gqodb.tick"), &blocks, Some("bybit"), 5, 2);
    dir
}

#[test]
fn a_command_as_separate_words_prints_csv_when_not_on_a_terminal() {
    let d = data();
    let o = gqodb(&[
        "show",
        "trades",
        "EURUSD",
        "bybit",
        "1..3",
        "on",
        "sequence",
        "in",
        &d.s(),
        "|",
        "columns",
        "sequence,price,quantity",
    ]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert_eq!(
        out(&o),
        "sequence,price,quantity\n1,1.08342,2.50\n2,1.08342,2.50\n"
    );
    let e = err(&o);
    assert!(e.contains("-- clock sequence, range [1, 3)"), "{e}");
    assert!(
        e.contains("files: 1 read, 0 skipped") && e.contains("rows: 12 scanned, 2 out"),
        "{e}"
    );
}

#[test]
fn one_quoted_command_works_too_and_formats_can_be_chosen() {
    let d = data();
    let q = format!(
        "show trades EURUSD in {} | take 2 | columns sequence",
        d.s()
    );
    let o = gqodb(&["--format", "json", &q]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert_eq!(out(&o), "[\n{\"sequence\":1},\n{\"sequence\":2}\n]\n");
    let o = gqodb(&["--format=table", &q]);
    assert_eq!(out(&o), "sequence\n--------\n1\n2\n");
    let o = gqodb(&["--format", "ndjson", &q]);
    assert_eq!(out(&o), "{\"sequence\":1}\n{\"sequence\":2}\n");
}

#[test]
fn a_range_without_a_clock_exits_2_with_the_message() {
    let d = data();
    let o = gqodb(&[&format!(
        "show trades EURUSD 2026-03-01..2026-03-02 in {}",
        d.s()
    )]);
    assert_eq!(code(&o), 2);
    assert!(out(&o).is_empty());
    assert!(
        err(&o).starts_with("error[E-CLOCK]: a range needs a clock\n"),
        "{}",
        err(&o)
    );
    assert!(
        err(&o).contains("help: add one of: on available | on received | on source | on sequence")
    );
}

#[test]
fn paths_and_budgets_have_their_own_codes() {
    let o = gqodb(&["show trades EURUSD"]);
    assert_eq!(
        (code(&o), err(&o).starts_with("error[E-PATH]")),
        (2, true),
        "{}",
        err(&o)
    );
    let o = gqodb(&["show trades EURUSD in /no/such/dir"]);
    assert_eq!(
        (code(&o), err(&o).starts_with("error[E-PATH]")),
        (1, true),
        "{}",
        err(&o)
    );
    let d = data();
    let o = gqodb(&[
        "--max-rows",
        "0",
        &format!("show trades EURUSD in {}", d.s()),
    ]);
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("E-BUDGET"));
    assert_eq!(code(&gqodb(&["--wat", "show trades A"])), 2);
}

#[test]
fn truncation_is_reported_and_strict_turns_it_into_exit_3() {
    let d = data();
    let q = format!("show trades EURUSD in {}", d.s());
    let o = gqodb(&["--max-rows", "5", &q]);
    assert_eq!(code(&o), 0);
    assert_eq!(out(&o).lines().count(), 6, "header and 5 rows");
    assert!(
        err(&o).contains("-- truncated: output rows (limits: 5 output rows"),
        "{}",
        err(&o)
    );
    let o = gqodb(&["--max-rows", "5", "--strict", &q]);
    assert_eq!(code(&o), 3);
}

#[test]
fn out_never_overwrites_an_existing_file() {
    let d = data();
    let target = d.path("result.csv");
    let q = format!(
        "show trades EURUSD in {} | take 1 | columns sequence",
        d.s()
    );
    let o = gqodb(&["--out", &target.display().to_string(), &q]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(out(&o).is_empty());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "sequence\n1\n");
    std::fs::write(&target, "mine").unwrap();
    let o = gqodb(&["--out", &target.display().to_string(), &q]);
    assert_eq!(code(&o), 1);
    assert!(err(&o).contains("never overwritten"), "{}", err(&o));
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "mine");
}

#[test]
fn a_script_keeps_its_session_and_stops_at_the_first_failure() {
    let d = data();
    let script = d.path("q.gq");
    std::fs::write(&script, format!("# demo\r\nuse path {}\r\nuse clock sequence\r\nshow trades EURUSD 1.. | take 1 | columns sequence\r\n", d.s())).unwrap();
    let o = gqodb(&["--format", "csv", "run", &script.display().to_string()]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(out(&o).starts_with("sequence\n"), "{}", out(&o));
    std::fs::write(
        &script,
        "show trades A in /no/such/dir\nshow trades B in /still/not\n",
    )
    .unwrap();
    let o = gqodb(&["run", &script.display().to_string()]);
    assert_eq!(code(&o), 1);
    assert_eq!(
        err(&o).matches("error[E-PATH]").count(),
        1,
        "stops after the first failure"
    );
    assert_eq!(code(&gqodb(&["run", "/no/such/script"])), 1);
}

#[test]
fn the_repl_reads_commands_from_standard_input() {
    let d = data();
    let input = format!(
        "use path {}\n\nuse clock sequence\nshow trades EURUSD 1..3 | columns sequence\nquit\nshow trades NEVER\n",
        d.s()
    );
    let o = gqodb_stdin(&["--format", "csv"], &input);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert_eq!(out(&o), "sequence\n1\n2\n", "quit ends the session");
}

#[test]
fn help_lists_the_clocks_and_an_empty_result_keeps_its_header() {
    let o = gqodb(&["help"]);
    assert_eq!(code(&o), 0);
    assert!(
        out(&o)
            .contains("clocks:  available | received (local, wall) | source (broker) | sequence")
    );
    assert_eq!(out(&gqodb(&["-h"])), out(&o));
    let d = data();
    let o = gqodb(&[
        "--format",
        "csv",
        &format!(
            "show trades EURUSD 1000..2000 on sequence in {} | columns sequence",
            d.s()
        ),
    ]);
    assert_eq!((code(&o), out(&o)), (0, "sequence\n".to_string()));
}

#[test]
fn explain_prints_the_plan_and_reads_no_data() {
    let d = data();
    let o = gqodb(&[&format!(
        "explain show trades EURUSD {}..{} on available in {}",
        at(200),
        at(300),
        d.s()
    )]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(out(&o).contains("1 of 3 blocks to read"), "{}", out(&o));
}

#[test]
fn a_huge_input_is_refused_not_parsed() {
    let o = gqodb_stdin(
        &[],
        &format!("show trades {}\n", "A".repeat(2 * 1024 * 1024)),
    );
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("longer than"), "{}", err(&o));
}
