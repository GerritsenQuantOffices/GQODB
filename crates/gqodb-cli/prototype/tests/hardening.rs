mod support;

use gqodb_cli::exec::{ExecError, Executor, FileExecutor, Options, SkipReason, discover};
use std::process::{Command, Stdio};
use support::*;

fn rows(n: i64) -> Vec<Trade> {
    (0..n).map(|i| trade("EURUSD", i, i + 1)).collect()
}

fn gqodb() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gqodb"))
}

#[cfg(unix)]
#[test]
fn a_closed_output_pipe_ends_quietly_with_exit_0() {
    use std::io::Read;
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &[rows(20_000)], None, 5, 2);
    let mut child = gqodb()
        .args([
            "--max-rows",
            "20000",
            "--format",
            "csv",
            &format!("show trades EURUSD in {}", dir.s()),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = [0_u8; 16];
    child.stdout.take().unwrap().read_exact(&mut first).unwrap(); // read a little, then close our end
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("error["),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
#[test]
fn an_argument_that_is_not_utf8_is_never_a_panic() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let output = gqodb()
        .arg(OsStr::from_bytes(b"show trades EURUSD in /no/\xff/such"))
        .output()
        .unwrap();
    let code = output
        .status
        .code()
        .expect("exited normally, not by a signal");
    assert!(code == 1 || code == 2, "{code}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("error["));
}

#[test]
fn skip_bad_reads_the_good_files_reports_the_bad_one_and_exits_1() {
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &[rows(4)], None, 5, 2);
    write_trades(&dir.path("b.gqodb.tick"), &[rows(4)], None, 5, 2);
    let bytes = std::fs::read(dir.path("b.gqodb.tick")).unwrap();
    std::fs::write(dir.path("b.gqodb.tick"), &bytes[..bytes.len() - 7]).unwrap();
    let q = format!("show trades EURUSD in {} | columns sequence", dir.s());
    let output = gqodb()
        .args(["--skip-bad", "--format", "csv", &q])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "sequence\n1\n2\n3\n4\n"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("-- could not read b.gqodb.tick"));
    let output = gqodb().args(["--format", "csv", &q]).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "without --skip-bad it is an error"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("error[E-FILE]: b.gqodb.tick"));
}

#[test]
fn more_than_ten_thousand_files_in_one_directory_is_a_budget_error() {
    let dir = TempDir::new();
    for i in 0..10_001 {
        std::fs::write(dir.path(&format!("f{i:05}.gqodb.tick")), b"").unwrap();
    }
    assert!(matches!(discover(&dir.s()), Err(ExecError::Budget(_))));
    let dir = TempDir::new();
    for i in 0..10_000 {
        std::fs::write(dir.path(&format!("f{i:05}.gqodb.tick")), b"").unwrap();
    }
    assert_eq!(
        discover(&dir.s()).unwrap().len(),
        10_000,
        "exactly the limit is allowed"
    );
}

#[test]
fn a_plain_store_file_without_the_normalized_metadata_is_skipped_as_not_normalized() {
    use arrow_array::{ArrayRef, Int64Array, LargeStringArray, RecordBatch};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;
    let dir = TempDir::new();
    let schema = Arc::new(Schema::new(vec![
        Field::new("symbol", DataType::LargeUtf8, false),
        Field::new("t", DataType::Int64, false),
    ]));
    let columns: Vec<ArrayRef> = vec![
        Arc::new(LargeStringArray::from(vec!["EURUSD"])),
        Arc::new(Int64Array::from(vec![1_i64])),
    ];
    let batch = RecordBatch::try_new(schema.clone(), columns).unwrap();
    let mut writer = gqodb_store::Writer::create_with_codec(
        dir.path("plain.gqodb.tick"),
        schema,
        "t",
        "symbol",
        gqodb_blocks::real::Mode::Gqodb,
    )
    .unwrap();
    writer.append(&batch).unwrap();
    writer.finish().unwrap();
    write_trades(&dir.path("real.gqodb.tick"), &[rows(2)], None, 5, 2);
    let (out, report) = run(&format!("show trades EURUSD in {}", dir.s()));
    assert_eq!(out.rows.len(), 2);
    assert_eq!(
        report.skipped,
        vec![("plain.gqodb.tick".to_string(), SkipReason::NotNormalized)]
    );
}

#[test]
fn two_files_with_different_scales_stay_exact_per_row_even_when_sorted_together() {
    let dir = TempDir::new();
    let mut a = trade("EURUSD", 10, 1);
    a.price = 108_342; // scale 5 -> 1.08342
    let mut b = trade("EURUSD", 20, 2);
    b.price = 150; // scale 2 -> 1.50
    write_trades(&dir.path("a.gqodb.tick"), &[vec![a]], None, 5, 2);
    write_trades(&dir.path("b.gqodb.tick"), &[vec![b]], None, 2, 2);
    let (out, _) = run(&format!(
        "show trades EURUSD in {} | order on available desc | columns sequence, price",
        dir.s()
    ));
    assert_eq!(out.ints("sequence"), [2, 1]);
    assert_eq!(out.texts("price"), ["1.50", "1.08342"]);
}

#[test]
fn every_clock_empty_means_an_empty_answer_with_a_count() {
    let dir = TempDir::new();
    let mut r = rows(3);
    for row in &mut r {
        row.source = None;
    }
    write_trades(&dir.path("a.gqodb.tick"), &[r], None, 5, 2);
    let (out, report) = run(&format!(
        "show trades EURUSD {}..{} on source in {}",
        at(0),
        at(10),
        dir.s()
    ));
    assert_eq!((out.rows.len(), report.null_clock_excluded), (0, 3));
}

#[test]
fn begin_equal_to_end_and_take_zero() {
    let p =
        gqodb_lang::parse("show trades EURUSD 2026-03-01..2026-03-01 on received in ./d").unwrap();
    assert_eq!(
        gqodb_lang::plan(&p.commands[0], &gqodb_lang::Session::default()).unwrap_err()[0].code,
        gqodb_lang::Code::Range
    );
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &[rows(3)], None, 5, 2);
    let (out, report) = run(&format!("show trades EURUSD in {} | take 0", dir.s()));
    assert_eq!((out.rows.len(), report.truncated), (0, None));
}

#[test]
fn help_has_topics() {
    for topic in ["clocks", "ranges", "steps", "flags", "errors"] {
        assert!(
            gqodb_cli::help::topic(topic).is_some_and(|t| !t.is_empty()),
            "{topic}"
        );
    }
    assert!(gqodb_cli::help::topic("nonsense").is_none());
    let output = gqodb().args(["help clocks"]).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("received"));
    let output = gqodb().args(["help nonsense"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("topics: clocks, ranges, steps, flags, errors")
    );
}

#[test]
fn the_examples_in_help_parse() {
    let mut checked = 0;
    for line in gqodb_cli::cli::HELP.lines() {
        let Some(rest) = line.trim_start().strip_prefix("gqodb \"") else {
            continue;
        };
        let command = rest.trim_end().trim_end_matches('"');
        gqodb_lang::parse(command).unwrap_or_else(|e| panic!("{command}: {e:?}"));
        checked += 1;
    }
    assert!(checked >= 2, "the help shows at least two examples");
}

#[test]
fn explain_of_a_file_from_another_stream_says_why() {
    let dir = TempDir::new();
    write_book(&dir.path("b.gqodb.ob"));
    let p = plan_of(&format!("explain show trades EURUSD in {}", dir.s()));
    let lines = FileExecutor.explain(&p, &Options::default()).unwrap();
    assert!(
        lines
            .iter()
            .any(|l| l.contains("b.gqodb.ob: skipped (other stream)")),
        "{lines:?}"
    );
}

/// Runs `gqodb` with `args`, reads 16 bytes of stdout, closes it, and returns the output.
#[cfg(unix)]
fn run_and_close_after_16_bytes(args: &[String]) -> std::process::Output {
    use std::io::Read;
    let mut child = gqodb()
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = [0_u8; 16];
    child.stdout.take().unwrap().read_exact(&mut first).unwrap();
    child.wait_with_output().unwrap()
}

#[cfg(unix)]
#[test]
fn a_closed_pipe_keeps_the_skip_bad_notice_and_exit_1() {
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &[rows(4)], None, 5, 2);
    let bytes = std::fs::read(dir.path("a.gqodb.tick")).unwrap();
    std::fs::write(dir.path("a.gqodb.tick"), &bytes[..bytes.len() - 7]).unwrap(); // bad, met first
    write_trades(&dir.path("b.gqodb.tick"), &[rows(20_000)], None, 5, 2);
    let q = format!("show trades EURUSD in {}", dir.s());
    let args = ["--skip-bad", "--max-rows", "20000", "--format", "csv", &q].map(String::from);
    let output = run_and_close_after_16_bytes(&args);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("-- could not read a.gqodb.tick"),
        "{stderr}"
    );
    assert!(!stderr.contains("error["), "{stderr}");
}

#[cfg(unix)]
#[test]
fn a_closed_pipe_ends_the_whole_script() {
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &[rows(20_000)], None, 5, 2);
    let script = dir.path("s.gq");
    std::fs::write(
        &script,
        format!(
            "show trades EURUSD in {}\nshow trades EURUSD in /no/such/dir\n",
            dir.s()
        ),
    )
    .unwrap();
    let args = [
        "--max-rows",
        "20000",
        "--format",
        "csv",
        "run",
        &script.to_string_lossy(),
    ]
    .map(String::from);
    let output = run_and_close_after_16_bytes(&args);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_eq!(stderr, "", "later commands must not run");
}

#[cfg(unix)]
#[test]
fn a_closed_pipe_ends_the_repl() {
    use std::io::{Read, Write};
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &[rows(20_000)], None, 5, 2);
    let mut child = gqodb()
        .args(["--max-rows", "20000", "--format", "csv"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let _ = write!(
        stdin,
        "show trades EURUSD in {}\nshow trades EURUSD in /no/such/dir\n",
        dir.s()
    );
    drop(stdin);
    let mut first = [0_u8; 16];
    child.stdout.take().unwrap().read_exact(&mut first).unwrap();
    let output = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_eq!(stderr, "", "later lines must not run");
}

#[test]
fn every_command_in_the_language_reference_parses() {
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/LANGUAGE.md"
    ))
    .unwrap();
    let mut checked = 0;
    for line in doc
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix("$ gqodb "))
    {
        let command = line.trim().trim_matches('"');
        gqodb_lang::parse(command).unwrap_or_else(|e| panic!("{command}: {e:?}"));
        checked += 1;
    }
    assert!(
        checked >= 8,
        "LANGUAGE.md shows at least eight example commands (lines that start with `$ gqodb `)"
    );
}
