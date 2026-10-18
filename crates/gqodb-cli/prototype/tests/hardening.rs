mod support;

use gqodb_cli::cli::{Flags, repl, run_text};
use gqodb_cli::exec::{ExecError, Executor, FileExecutor, Options, SkipReason, discover};
use gqodb_lang::Session;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
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

/// The demo files the language reference uses: what `gqodb-codec demo tick-segment` and `demo book-segment` write.
fn demo_dir() -> TempDir {
    use gqodb_codec::{Family, fixture, write_segment, write_segment_with_family};
    let dir = TempDir::new();
    std::fs::create_dir(dir.path("demo")).unwrap();
    write_segment_with_family(
        &dir.path("demo/demo.gqodb.tick"),
        &[fixture(Family::Tick).unwrap()],
        Family::Tick,
    )
    .unwrap();
    write_segment(
        &dir.path("demo/demo.gqodb.ob"),
        &[fixture(Family::Orderbook).unwrap()],
    )
    .unwrap();
    dir
}

/// Splits a command line the way a shell would for the simple cases the page uses: words, `"..."` and `'...'`.
fn shell_words(line: &str) -> Vec<String> {
    let (mut words, mut word, mut quote, mut started) = (Vec::new(), String::new(), None, false);
    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            (None, c) => {
                word.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(word);
    }
    words
}

#[test]
fn every_output_block_in_the_language_reference_is_what_the_command_prints() {
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/LANGUAGE.md"
    ))
    .unwrap();
    let dir = demo_dir();
    let lines: Vec<&str> = doc.lines().collect();
    let command_of = |l: &str| -> Option<String> {
        let l = l.trim_start();
        l.strip_prefix("$ gqodb ")
            .or_else(|| l.strip_prefix("gqodb "))
            .map(str::to_string)
    };
    let mut compared = 0;
    let mut in_text_block = false;
    for (i, line) in lines.iter().enumerate() {
        if let Some(fence) = line.strip_prefix("```") {
            in_text_block = !in_text_block && fence == "text";
            continue;
        }
        // Only the `text` blocks hold examples with output; the `sh` blocks show invocations to copy.
        let Some(command) = command_of(line).filter(|_| in_text_block) else {
            continue;
        };
        let expected: Vec<&str> = lines[i + 1..]
            .iter()
            .take_while(|l| !l.starts_with("```") && command_of(l).is_none())
            .copied()
            .collect();
        // Commands without a block are only parsed (the other test); those on `./data` have no data here.
        if expected.is_empty() || command.contains("./data") {
            continue;
        }
        let args = shell_words(&command);
        let output = gqodb()
            .current_dir(&dir.0)
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let (stdout, stderr) = (
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        );
        // The page says which stream carries what: an error block is standard error; in the others the lines that
        // start with `-- ` (the footer) are standard error and everything else is standard output.
        let is_error = expected[0].starts_with("error[");
        let (mut want_out, mut want_err) = (Vec::new(), Vec::new());
        for l in &expected {
            if is_error || l.starts_with("-- ") {
                want_err.push(*l);
            } else {
                want_out.push(*l);
            }
        }
        let at = format!("LANGUAGE.md line {}: {command}", i + 1);
        assert_eq!(
            stdout.trim_end(),
            want_out.join("\n").trim_end(),
            "stdout, {at}"
        );
        // The three format blocks (--format) show standard output only, as the page says; the others show the footer too.
        if !args.iter().any(|a| a == "--format") {
            assert_eq!(
                stderr.trim_end(),
                want_err.join("\n").trim_end(),
                "stderr, {at}"
            );
        }
        // The page gives no exit code per example. It is derived from the output: an error block means 2, anything
        // else 0. An example that ends in an exit-1 error (E-PATH, E-FILE) would need this test extended.
        assert_eq!(
            output.status.code(),
            Some(if is_error { 2 } else { 0 }),
            "{at}"
        );
        compared += 1;
    }
    assert!(
        compared >= 15,
        "LANGUAGE.md has 15 runnable example blocks, found {compared}"
    );
}

// ---- the fix wave after the final review: group 1 ----

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

/// Runs the closure on a thread and fails (instead of hanging the suite) when it does not finish in time.
fn within<T: Send + 'static>(seconds: u64, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(Duration::from_secs(seconds))
        .expect("did not finish in time (a hang?)")
}

/// Runs `gqodb`, keeps at most `limit` bytes of standard error and kills the process when there is more.
/// Returns (exit code or None when killed, standard error, whether it had more than `limit` bytes).
fn run_capped(args: &[String], limit: usize) -> (Option<i32>, Vec<u8>, bool) {
    let mut child = gqodb()
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut buf = Vec::new();
    child
        .stderr
        .take()
        .unwrap()
        .take(limit as u64 + 1)
        .read_to_end(&mut buf)
        .unwrap();
    let exceeded = buf.len() > limit;
    if exceeded {
        let _ = child.kill();
    }
    let status = child.wait().unwrap();
    (status.code(), buf, exceeded)
}

#[test]
fn out_defaults_to_csv_even_when_standard_output_is_a_terminal() {
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &[rows(3)], None, 5, 2);
    let target = dir.path("res.csv");
    let flags = Flags {
        out: Some(target.display().to_string()),
        ..Flags::default()
    };
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let tty = true;
    let code = run_text(
        &format!(
            "show trades EURUSD in {} | take 2 | columns sequence",
            dir.s()
        ),
        &mut Session::default(),
        &flags,
        tty,
        &mut o,
        &mut e,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&e));
    assert_eq!(
        read(&target),
        "sequence\n1\n2\n",
        "a file is CSV, not a table"
    );
    assert!(o.is_empty());
}

#[test]
fn a_failed_run_leaves_no_out_file_and_a_repeat_works() {
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &[rows(4)], None, 5, 2);
    let target = dir.path("res.csv");
    let t = target.display().to_string();
    let o = gqodb()
        .args(["--out", &t, "show trades EURUSD in /no/such/dir"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(!target.exists(), "a failed run leaves no file behind");
    let q = format!("show trades EURUSD in {} | columns sequence", dir.s());
    let o = gqodb().args(["--out", &t, &q]).output().unwrap();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert_eq!(read(&target), "sequence\n1\n2\n3\n4\n");
}

#[test]
fn a_damaged_file_leaves_no_out_file_unless_skip_bad_or_strict_made_it_a_result() {
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &[rows(4)], None, 5, 2);
    write_trades(&dir.path("b.gqodb.tick"), &[rows(4)], None, 5, 2);
    let bytes = std::fs::read(dir.path("b.gqodb.tick")).unwrap();
    std::fs::write(dir.path("b.gqodb.tick"), &bytes[..bytes.len() - 7]).unwrap();
    let q = format!("show trades EURUSD in {} | columns sequence", dir.s());
    let target = dir.path("res.csv");
    let t = target.display().to_string();
    let o = gqodb().args(["--out", &t, &q]).output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(
        !target.exists(),
        "rows of a were written before b failed: that file must not stay"
    );
    let o = gqodb()
        .args(["--skip-bad", "--out", &t, &q])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(
        read(&target),
        "sequence\n1\n2\n3\n4\n",
        "with --skip-bad the result stays, and exit 1 plus the notice say b was skipped"
    );
    std::fs::remove_file(&target).unwrap();
    let o = gqodb()
        .args(["--skip-bad", "--max-rows", "2", "--strict", "--out", &t, &q])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    assert_eq!(
        read(&target),
        "sequence\n1\n2\n",
        "a truncated result stays, exit 3"
    );
}

fn gqodb_stdin_bytes(args: &[&str], input: &[u8]) -> std::process::Output {
    use std::io::Write;
    let mut child = gqodb()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let _ = stdin.write_all(input); // the child may stop reading before the end
    drop(stdin);
    child.wait_with_output().unwrap()
}

#[test]
fn a_read_error_on_standard_input_is_an_error_not_a_quiet_stop() {
    let o = gqodb_stdin_bytes(&[], b"use clock sequence\n\xff\nhelp\n");
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.starts_with("error[E-FILE]: cannot read standard input: "),
        "{stderr}"
    );
    assert!(
        o.stdout.is_empty(),
        "the line after the bad one must not run"
    );
}

#[test]
fn a_flood_of_bad_commands_gives_a_short_report() {
    let src = "x;".repeat(60_000);
    let started = Instant::now();
    let (code, stderr, exceeded) = run_capped(&[src], 64 * 1024);
    assert!(!exceeded, "more than 64 KiB of diagnostics");
    assert_eq!(code, Some(2));
    let stderr = String::from_utf8(stderr).unwrap();
    assert_eq!(stderr.matches("error[E-SYNTAX]").count(), 20, "{stderr}");
    assert!(stderr.ends_with("... and 59980 more\n"), "{stderr}");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_one_mebibyte_script_of_bad_commands_gives_a_short_report_quickly() {
    let dir = TempDir::new();
    std::fs::write(dir.path("s.gq"), "x;".repeat(512 * 1024)).unwrap();
    let started = Instant::now();
    let (code, stderr, exceeded) = run_capped(
        &["run".to_string(), dir.path("s.gq").display().to_string()],
        64 * 1024,
    );
    assert!(!exceeded);
    assert_eq!(code, Some(2));
    assert!(
        String::from_utf8(stderr)
            .unwrap()
            .contains("... and 524268 more")
    );
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn quit_ends_a_pipe_whatever_it_looks_like_and_nothing_after_it_runs() {
    for input in [
        "quit;\nshow trades A in /no/such/dir\n",
        "use clock sequence; quit\nshow trades A in /no/such/dir\n",
        "exit # goodbye\nshow trades A in /no/such/dir\n",
        "QUIT\nshow trades A in /no/such/dir\n",
    ] {
        let o = gqodb_stdin_bytes(&[], input.as_bytes());
        assert_eq!(
            o.status.code(),
            Some(0),
            "{input:?}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(o.stderr.is_empty(), "{input:?}");
        // the same text in a script file behaves the same
        let dir = TempDir::new();
        std::fs::write(dir.path("s.gq"), input).unwrap();
        let o = gqodb()
            .args(["run", &dir.path("s.gq").display().to_string()])
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(0), "run {input:?}");
        assert!(o.stderr.is_empty(), "run {input:?}");
    }
}

#[test]
fn the_prompt_goes_to_standard_error_and_only_when_standard_input_is_a_terminal() {
    let mut input = std::io::Cursor::new(b"help clocks\n".to_vec());
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let code = repl(&mut input, true, false, &Flags::default(), &mut o, &mut e);
    assert_eq!(code, 0);
    let (out, err) = (String::from_utf8(o).unwrap(), String::from_utf8(e).unwrap());
    assert!(
        !out.contains("gqodb>"),
        "the result stream stays clean: {out:?}"
    );
    assert!(out.contains("clocks"), "{out:?}");
    assert!(err.contains("gqodb> "), "{err:?}");
    let mut input = std::io::Cursor::new(b"help clocks\n".to_vec());
    let (mut o, mut e) = (Vec::new(), Vec::new());
    repl(&mut input, false, false, &Flags::default(), &mut o, &mut e);
    assert!(!String::from_utf8_lossy(&o).contains("gqodb>"));
    assert!(e.is_empty(), "no prompt for a pipe");
}

/// `gqodb` under a virtual-memory limit, so that unbounded reading ends in an error instead of eating the host.
#[cfg(unix)]
fn limited(args: &[&str], stdin: Stdio) -> std::process::Output {
    Command::new("sh")
        .args(["-c", "ulimit -v 1000000; exec \"$0\" \"$@\""])
        .arg(env!("CARGO_BIN_EXE_gqodb"))
        .args(args)
        .stdin(stdin)
        .output()
        .unwrap()
}

#[cfg(unix)]
#[test]
fn reading_a_script_or_a_line_is_bounded() {
    let stderr_of = |o: &std::process::Output| String::from_utf8_lossy(&o.stderr).into_owned();
    let o = limited(&["run", "/dev/zero"], Stdio::null());
    assert_eq!(o.status.code(), Some(2), "{}", stderr_of(&o));
    assert!(
        stderr_of(&o).contains("error[E-SYNTAX]: the input is longer than 1048576 bytes"),
        "{}",
        stderr_of(&o)
    );
    let o = limited(&[], Stdio::from(std::fs::File::open("/dev/zero").unwrap()));
    assert_eq!(o.status.code(), Some(2), "{}", stderr_of(&o));
    assert!(
        stderr_of(&o).contains("error[E-SYNTAX]: the input is longer than 1048576 bytes"),
        "{}",
        stderr_of(&o)
    );
    let o = gqodb_stdin_bytes(&[], "A".repeat(2 * 1024 * 1024).as_bytes());
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).contains("longer than 1048576 bytes"));
}

// Entries with a fitting name that are not plain files: reported, never dropped quietly, never opened when they could hang.

fn skipped_of(report: &gqodb_cli::exec::Report) -> Vec<(String, String)> {
    report
        .skipped
        .iter()
        .map(|(n, r)| (n.clone(), r.to_string()))
        .collect()
}

#[cfg(unix)]
#[test]
fn a_link_to_outside_the_directory_is_reported_as_skipped() {
    let dir = TempDir::new();
    let (inside, outside) = (dir.path("data"), dir.path("elsewhere"));
    std::fs::create_dir(&inside).unwrap();
    std::fs::create_dir(&outside).unwrap();
    write_trades(&inside.join("a.gqodb.tick"), &[rows(2)], None, 5, 2);
    write_trades(&outside.join("secret.gqodb.tick"), &[rows(2)], None, 5, 2);
    std::os::unix::fs::symlink(
        outside.join("secret.gqodb.tick"),
        inside.join("b.gqodb.tick"),
    )
    .unwrap();
    let q = format!("show trades EURUSD in {}", inside.display());
    let (out, report) = run(&q);
    assert_eq!(out.rows.len(), 2, "only a is read");
    assert_eq!(
        skipped_of(&report),
        [(
            "b.gqodb.tick".to_string(),
            "symbolic link to outside the directory".to_string()
        )]
    );
    let lines = FileExecutor
        .explain(&plan_of(&format!("explain {q}")), &Options::default())
        .unwrap();
    assert!(
        lines
            .iter()
            .any(|l| l == "b.gqodb.tick: skipped (symbolic link to outside the directory)"),
        "{lines:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_broken_link_is_reported_as_skipped() {
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &[rows(2)], None, 5, 2);
    std::os::unix::fs::symlink("/no/such/target.gqodb.tick", dir.path("b.gqodb.tick")).unwrap();
    let (out, report) = run(&format!("show trades EURUSD in {}", dir.s()));
    assert_eq!(out.rows.len(), 2);
    assert_eq!(
        skipped_of(&report),
        [(
            "b.gqodb.tick".to_string(),
            "broken symbolic link".to_string()
        )]
    );
}

#[cfg(unix)]
#[test]
fn a_pipe_a_directory_and_a_link_to_them_are_reported_and_never_opened() {
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &[rows(2)], None, 5, 2);
    let fifo = dir.path("b.gqodb.tick");
    let made = Command::new("mkfifo").arg(&fifo).status().unwrap();
    assert!(made.success());
    std::os::unix::fs::symlink(&fifo, dir.path("c.gqodb.tick")).unwrap();
    std::fs::create_dir(dir.path("d.gqodb.tick")).unwrap();
    let q = format!("show trades EURUSD in {}", dir.s());
    let (out, report) = within(20, move || run(&q));
    assert_eq!(out.rows.len(), 2);
    let not_regular = "not a regular file".to_string();
    assert_eq!(
        skipped_of(&report),
        [
            ("b.gqodb.tick".to_string(), not_regular.clone()),
            ("c.gqodb.tick".to_string(), not_regular.clone()),
            ("d.gqodb.tick".to_string(), not_regular),
        ]
    );
}

#[cfg(unix)]
#[test]
fn a_pipe_named_as_the_path_is_neither_a_file_nor_a_directory() {
    let dir = TempDir::new();
    let fifo = dir.path("p");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let path = fifo.display().to_string();
    match within(20, move || discover(&path)) {
        Err(ExecError::Path(m)) => assert!(m.contains("is neither a file nor a directory"), "{m}"),
        other => panic!("{other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn a_link_to_a_file_in_the_same_directory_is_not_read_twice() {
    let dir = TempDir::new();
    write_trades(&dir.path("b.gqodb.tick"), &[rows(2)], None, 5, 2);
    // sorted before its target, and one more link to a file that has no fitting name itself
    std::os::unix::fs::symlink(dir.path("b.gqodb.tick"), dir.path("a.gqodb.tick")).unwrap();
    write_trades(&dir.path("plain.bin"), &[rows(2)], None, 5, 2);
    std::os::unix::fs::symlink(dir.path("plain.bin"), dir.path("c.gqodb.tick")).unwrap();
    std::os::unix::fs::symlink(dir.path("plain.bin"), dir.path("d.gqodb.tick")).unwrap();
    let (out, report) = run(&format!("show trades EURUSD in {}", dir.s()));
    assert_eq!(out.rows.len(), 4, "b once and plain.bin once");
    assert_eq!(report.files_read, 2);
    assert_eq!(
        skipped_of(&report),
        [
            (
                "a.gqodb.tick".to_string(),
                "duplicate of b.gqodb.tick".to_string()
            ),
            (
                "d.gqodb.tick".to_string(),
                "duplicate of c.gqodb.tick".to_string()
            ),
        ]
    );
}

fn hand_written(path: &Path, price_decimals: &str, received: arrow_schema::DataType) {
    use arrow_array::{ArrayRef, Int8Array, Int64Array, LargeStringArray, RecordBatch};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;
    let mut meta = std::collections::HashMap::new();
    for (k, v) in [
        ("gqodb.family", "tick"),
        ("gqodb.subtype", "trade"),
        ("gqodb.price_decimals", price_decimals),
        ("gqodb.quantity_decimals", "2"),
    ] {
        meta.insert(k.to_string(), v.to_string());
    }
    let schema = Arc::new(Schema::new_with_metadata(
        vec![
            Field::new("symbol", DataType::LargeUtf8, false),
            Field::new("available_utc_ns", DataType::Int64, false),
            Field::new("received_utc_ns", received.clone(), true),
            Field::new("sequence", DataType::Int64, false),
            Field::new("price", DataType::Int64, false),
        ],
        meta,
    ));
    let received_column: ArrayRef = if received == DataType::Int8 {
        Arc::new(Int8Array::from(vec![Some(5)]))
    } else {
        Arc::new(Int64Array::from(vec![Some(5_i64)]))
    };
    let columns: Vec<ArrayRef> = vec![
        Arc::new(LargeStringArray::from(vec!["EURUSD"])),
        Arc::new(Int64Array::from(vec![10_i64])),
        received_column,
        Arc::new(Int64Array::from(vec![1_i64])),
        Arc::new(Int64Array::from(vec![123_i64])),
    ];
    let batch = RecordBatch::try_new(schema.clone(), columns).unwrap();
    let mut writer = gqodb_store::Writer::create_with_codec(
        path,
        schema,
        "available_utc_ns",
        "symbol",
        gqodb_blocks::real::Mode::Gqodb,
    )
    .unwrap();
    writer.append(&batch).unwrap();
    writer.finish().unwrap();
}

#[test]
fn a_file_with_a_wrong_scale_or_a_wrong_clock_type_is_a_broken_file_not_a_wrong_answer() {
    use arrow_schema::DataType;
    let q = |dir: &TempDir| {
        plan_of(&format!(
            "show trades EURUSD in {} | columns sequence, price",
            dir.s()
        ))
    };
    for (price_decimals, received, what) in [
        ("13", DataType::Int64, "scale"),
        ("5", DataType::Int8, "clock type"),
    ] {
        let dir = TempDir::new();
        hand_written(&dir.path("h.gqodb.tick"), price_decimals, received);
        let mut sink = Collect::default();
        let err = FileExecutor
            .execute(&q(&dir), &Options::default(), &mut sink)
            .expect_err(what);
        assert!(
            matches!(&err, ExecError::File { path, .. } if path == "h.gqodb.tick"),
            "{what}: {err}"
        );
        let mut sink = Collect::default();
        let report = FileExecutor
            .execute(
                &q(&dir),
                &Options {
                    skip_bad: true,
                    ..Options::default()
                },
                &mut sink,
            )
            .unwrap();
        assert_eq!((report.bad_files.len(), sink.rows.len()), (1, 0), "{what}");
    }
    // the same file with a good scale and clock type is read
    let dir = TempDir::new();
    hand_written(&dir.path("h.gqodb.tick"), "5", DataType::Int64);
    let mut sink = Collect::default();
    FileExecutor
        .execute(&q(&dir), &Options::default(), &mut sink)
        .unwrap();
    assert_eq!(sink.texts("price"), ["0.00123"]);
}
