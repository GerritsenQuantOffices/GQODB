mod support;

use gqodb_cli::exec::{ExecError, Executor, FileExecutor, Options, SkipReason, Truncation};
use gqodb_cli::value::{Cell, TimeFormat};
use support::*;

type Getter = Box<dyn Fn(&Trade) -> Option<i64>>;

/// Three blocks. `available`: 100..103, 200..203, 300..303. `sequence`: 1..12.
fn three_blocks() -> Vec<Vec<Trade>> {
    (0..3)
        .map(|b| {
            (0..4)
                .map(|i| trade("EURUSD", 100 * (b + 1) + i, 4 * b + i + 1))
                .collect()
        })
        .collect()
}

fn one_file() -> (TempDir, String) {
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &three_blocks(), None, 5, 2);
    let path = dir.s();
    (dir, path)
}

#[test]
fn the_index_clock_skips_blocks_and_other_clocks_read_every_block_with_the_symbol() {
    let (_dir, path) = one_file();
    let (out, report) = run(&format!(
        "show trades EURUSD {}..{} on available in {path}",
        at(200),
        at(300)
    ));
    assert_eq!((report.blocks_total, report.blocks_read), (3, 1));
    assert_eq!(out.ints("sequence"), [5, 6, 7, 8]);
    assert_eq!(report.rows_scanned, 4);
    let (out, report) = run(&format!("show trades EURUSD 5..9 on sequence in {path}"));
    assert_eq!((report.blocks_total, report.blocks_read), (3, 3));
    assert_eq!(
        out.ints("sequence"),
        [5, 6, 7, 8],
        "the range is half open: 9 is out"
    );
    assert_eq!(report.null_clock_excluded, 0);
}

#[test]
fn numbers_are_exact_and_columns_follow_the_schema() {
    let (_dir, path) = one_file();
    let (out, _) = run(&format!("show trades EURUSD 1..2 on sequence in {path}"));
    assert_eq!(
        out.columns,
        [
            "symbol",
            "source_utc_ns",
            "received_utc_ns",
            "clock_error_ns",
            "available_utc_ns",
            "sequence",
            "price",
            "quantity",
            "side"
        ]
    );
    assert_eq!(out.texts("price"), ["1.08342"]);
    assert_eq!(out.texts("quantity"), ["2.50"]);
    assert_eq!(out.ints("side"), [1]);
    assert_eq!(
        out.texts("available_utc_ns"),
        ["2026-03-01T00:00:00.000000100Z"]
    );
    let (out, _) = run_with(
        &format!("show trades EURUSD 1..2 on sequence in {path}"),
        &Options {
            time: TimeFormat::Ns,
            ..Options::default()
        },
    );
    assert_eq!(out.ints("available_utc_ns"), [BASE + 100]);
    let (out, _) = run(&format!(
        "show trades EURUSD 1..3 on sequence in {path} | columns sequence, price"
    ));
    assert_eq!(out.columns, ["sequence", "price"]);
    assert_eq!(out.rows.len(), 2);
}

#[test]
fn rows_with_an_empty_clock_are_left_out_and_counted() {
    let dir = TempDir::new();
    let mut blocks = three_blocks();
    blocks[0][1].received = None; // sequence 2
    blocks[2][0].received = None; // sequence 9
    write_trades(&dir.path("a.gqodb.tick"), &blocks, None, 5, 2);
    let (out, report) = run(&format!(
        "show trades EURUSD {}..{} on received in {}",
        at(99),
        at(200),
        dir.s()
    ));
    assert_eq!(
        out.ints("sequence"),
        [1, 3, 4, 5],
        "received is available - 1: 99, none, 101, 102, then 199"
    );
    assert_eq!(
        report.null_clock_excluded, 2,
        "sequence 2 and 9 have no received time"
    );
    let (out, report) = run(&format!("show trades EURUSD .. on received in {}", dir.s()));
    assert_eq!(out.rows.len(), 10);
    assert_eq!(report.null_clock_excluded, 2);
}

#[test]
fn rows_come_back_in_storage_order_late_events_included() {
    let dir = TempDir::new();
    let rows: Vec<Trade> = [150, 120, 180, 110]
        .iter()
        .enumerate()
        .map(|(i, t)| trade("EURUSD", *t, i as i64 + 1))
        .collect();
    write_trades(&dir.path("a.gqodb.tick"), &[rows], None, 5, 2);
    let (out, _) = run_with(
        &format!(
            "show trades EURUSD {}..{} on available in {}",
            at(100),
            at(200),
            dir.s()
        ),
        &Options {
            time: TimeFormat::Ns,
            ..Options::default()
        },
    );
    assert_eq!(
        out.ints("available_utc_ns"),
        [BASE + 150, BASE + 120, BASE + 180, BASE + 110]
    );
}

#[test]
fn symbols_are_exact_and_other_symbols_are_left_out() {
    let dir = TempDir::new();
    let rows = vec![
        trade("EURUSD", 1, 1),
        trade("GBPUSD", 2, 2),
        trade("eurusd", 3, 3),
        trade("EURUSD", 4, 4),
    ];
    write_trades(&dir.path("a.gqodb.tick"), &[rows], None, 5, 2);
    let (out, _) = run(&format!("show trades EURUSD in {}", dir.s()));
    assert_eq!(out.ints("sequence"), [1, 4]);
    let (out, report) = run(&format!("show trades NOPE in {}", dir.s()));
    assert!(
        out.rows.is_empty() && out.finished && report.blocks_read == 0,
        "a symbol in no block reads no block"
    );
    assert!(
        !out.columns.is_empty(),
        "an empty answer still has its header"
    );
}

#[test]
fn files_are_taken_in_name_order_and_filtered_by_stream_and_venue() {
    let dir = TempDir::new();
    let one = |seq: i64| vec![vec![trade("EURUSD", seq * 10, seq)]];
    write_trades(&dir.path("c.gqodb.tick"), &one(3), Some("okx"), 5, 2);
    write_trades(&dir.path("a.gqodb.tick"), &one(1), Some("bybit"), 5, 2);
    write_trades(&dir.path("b.gqodb.tick"), &one(2), None, 5, 2);
    write_book(&dir.path("d.gqodb.ob"));
    std::fs::write(dir.path("notes.txt"), "ignored").unwrap();
    std::fs::create_dir(dir.path("sub")).unwrap();
    write_trades(&dir.path("sub").join("z.gqodb.tick"), &one(9), None, 5, 2);
    let (out, report) = run(&format!("show trades EURUSD in {}", dir.s()));
    assert_eq!(
        out.ints("sequence"),
        [1, 2, 3],
        "a, b, c by name; not recursive"
    );
    assert_eq!(report.files_read, 3);
    assert_eq!(
        report.skipped,
        vec![("d.gqodb.ob".to_string(), SkipReason::WrongStream)]
    );
    let (out, report) = run(&format!("show trades EURUSD bybit in {}", dir.s()));
    assert_eq!(out.ints("sequence"), [1]);
    assert_eq!(
        report.skipped,
        vec![
            ("b.gqodb.tick".to_string(), SkipReason::NoVenueLabel),
            ("c.gqodb.tick".to_string(), SkipReason::VenueMismatch),
            ("d.gqodb.ob".to_string(), SkipReason::WrongStream),
        ]
    );
    let (out, _) = run(&format!("show book EURUSD in {}", dir.s()));
    assert_eq!(out.rows.len(), 1);
    assert_eq!(
        out.texts("price"),
        ["1.00"],
        "book prices use the book file's scale"
    );
}

#[test]
fn order_skip_take_and_columns() {
    let dir = TempDir::new();
    let mut rows: Vec<Trade> = [30, 10, 20, 40]
        .iter()
        .enumerate()
        .map(|(i, t)| trade("EURUSD", *t, i as i64 + 1))
        .collect();
    rows[0].source = None;
    write_trades(&dir.path("a.gqodb.tick"), &[rows], None, 5, 2);
    let path = dir.s();
    let seqs = |steps: &str| {
        run(&format!("show trades EURUSD in {path} {steps}"))
            .0
            .ints("sequence")
    };
    assert_eq!(seqs("| order on available"), [2, 3, 1, 4]);
    assert_eq!(seqs("| order on available desc"), [4, 1, 3, 2]);
    assert_eq!(seqs("| order on sequence desc | skip 1 | take 2"), [3, 2]);
    assert_eq!(
        seqs("| order on source"),
        [2, 3, 4, 1],
        "an empty clock value comes last"
    );
    assert_eq!(
        seqs("| order on source desc"),
        [4, 3, 2, 1],
        "also last when descending"
    );
    assert_eq!(
        seqs("| skip 1 | take 2"),
        [2, 3],
        "without order: storage order"
    );
    assert_eq!(seqs("| skip 10"), Vec::<i64>::new());
}

#[test]
fn budgets_truncate_loudly() {
    let (_dir, path) = one_file();
    let q = format!("show trades EURUSD in {path}");
    let with = |opts: Options| run_with(&q, &opts);
    let (out, r) = with(Options {
        max_rows: 3,
        ..Options::default()
    });
    assert_eq!(
        (out.rows.len(), r.truncated),
        (3, Some(Truncation::OutputRows))
    );
    let (out, r) = with(Options {
        max_rows: 12,
        ..Options::default()
    });
    assert_eq!(
        (out.rows.len(), r.truncated),
        (12, None),
        "exactly the limit is not a truncation"
    );
    let (out, r) = run_with(
        &format!("{q} | take 3"),
        &Options {
            max_rows: 3,
            ..Options::default()
        },
    );
    assert_eq!(
        (out.rows.len(), r.truncated),
        (3, None),
        "the user asked for 3"
    );
    let (out, r) = run_with(
        &format!("{q} | take 5"),
        &Options {
            max_rows: 3,
            ..Options::default()
        },
    );
    assert_eq!(
        (out.rows.len(), r.truncated),
        (3, Some(Truncation::OutputRows)),
        "the user asked for more than the limit"
    );
    let (out, r) = run_with(&format!("{q} | take 0"), &Options::default());
    assert_eq!((out.rows.len(), r.truncated), (0, None));
    let (out, r) = with(Options {
        max_blocks: 2,
        ..Options::default()
    });
    assert_eq!(
        (out.rows.len(), r.blocks_read, r.truncated),
        (8, 2, Some(Truncation::Blocks))
    );
    let (out, r) = with(Options {
        max_scan_rows: 6,
        ..Options::default()
    });
    assert_eq!(
        (out.rows.len(), r.truncated),
        (4, Some(Truncation::ScanRows))
    );
    let (_, r) = with(Options {
        max_scan_rows: 12,
        ..Options::default()
    });
    assert_eq!(r.truncated, None);
}

#[test]
fn options_have_hard_limits() {
    assert!(Options::default().validate().is_ok());
    assert!(
        Options {
            max_rows: 0,
            ..Options::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        Options {
            max_rows: 1_000_001,
            ..Options::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        Options {
            max_scan_rows: 25_600_001,
            ..Options::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        Options {
            max_blocks: 100_001,
            ..Options::default()
        }
        .validate()
        .is_err()
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_out_of_the_directory_is_not_followed() {
    let dir = TempDir::new();
    let inside = dir.path("data");
    let outside = dir.path("elsewhere");
    std::fs::create_dir(&inside).unwrap();
    std::fs::create_dir(&outside).unwrap();
    let one = |seq: i64| vec![vec![trade("EURUSD", seq, seq)]];
    write_trades(&inside.join("a.gqodb.tick"), &one(1), None, 5, 2);
    write_trades(&outside.join("secret.gqodb.tick"), &one(2), None, 5, 2);
    std::os::unix::fs::symlink(
        outside.join("secret.gqodb.tick"),
        inside.join("b.gqodb.tick"),
    )
    .unwrap();
    std::os::unix::fs::symlink(inside.join("a.gqodb.tick"), inside.join("c.gqodb.tick")).unwrap();
    let (out, _) = run(&format!("show trades EURUSD in {}", inside.display()));
    assert_eq!(
        out.ints("sequence"),
        [1, 1],
        "a and the link to a; not the link out"
    );
}

#[test]
fn a_damaged_file_is_an_error_or_a_skip_with_a_notice_never_silent() {
    let dir = TempDir::new();
    write_trades(&dir.path("a.gqodb.tick"), &three_blocks(), None, 5, 2);
    write_trades(&dir.path("b.gqodb.tick"), &three_blocks(), None, 5, 2);
    let bytes = std::fs::read(dir.path("b.gqodb.tick")).unwrap();
    std::fs::write(dir.path("b.gqodb.tick"), &bytes[..bytes.len() - 7]).unwrap();
    let p = plan_of(&format!("show trades EURUSD in {}", dir.s()));
    let mut sink = Collect::default();
    let err = FileExecutor
        .execute(&p, &Options::default(), &mut sink)
        .unwrap_err();
    assert!(
        matches!(&err, ExecError::File { path, .. } if path == "b.gqodb.tick"),
        "{err}"
    );
    let (out, report) = run_with(
        &format!("show trades EURUSD in {}", dir.s()),
        &Options {
            skip_bad: true,
            ..Options::default()
        },
    );
    assert_eq!(out.rows.len(), 12, "a is read completely");
    assert_eq!(report.bad_files.len(), 1);
    assert_eq!(report.bad_files[0].0, "b.gqodb.tick");
}

#[test]
fn missing_paths_are_path_errors() {
    let p = plan_of("show trades EURUSD in /definitely/not/here");
    let mut sink = Collect::default();
    assert!(matches!(
        FileExecutor.execute(&p, &Options::default(), &mut sink),
        Err(ExecError::Path(_))
    ));
}

#[test]
fn a_single_file_can_be_named_directly() {
    let (dir, _) = one_file();
    let (out, _) = run(&format!(
        "show trades EURUSD 1..3 on sequence in {}",
        dir.path("a.gqodb.tick").display()
    ));
    assert_eq!(out.rows.len(), 2);
}

#[test]
fn explain_says_what_would_be_read() {
    let (_dir, path) = one_file();
    let p = plan_of(&format!(
        "explain show trades EURUSD {}..{} on available in {path}",
        at(200),
        at(300)
    ));
    let lines = FileExecutor.explain(&p, &Options::default()).unwrap();
    assert!(
        lines[0].starts_with("plan: trades EURUSD on available"),
        "{lines:?}"
    );
    assert!(
        lines[1].contains("1 of 3 blocks to read") && lines[1].contains("index on available"),
        "{lines:?}"
    );
    let p = plan_of(&format!(
        "explain show trades EURUSD 5..9 on sequence in {path}"
    ));
    let lines = FileExecutor.explain(&p, &Options::default()).unwrap();
    assert!(
        lines[1].contains("3 of 3 blocks to read") && lines[1].contains("not indexed"),
        "{lines:?}"
    );
}

/// Compares many generated cases with a plain filter over the rows we wrote.
#[test]
fn many_generated_files_agree_with_a_plain_filter() {
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 >> 33
        }
    }
    let mut rng = Rng(7);
    for case in 0..60 {
        let dir = TempDir::new();
        let symbols = ["AAA", "BBB", "aaa"];
        let mut seq = 0;
        let mut blocks: Vec<Vec<Trade>> = Vec::new();
        for _ in 0..(1 + rng.next() % 4) {
            let mut rows = Vec::new();
            for _ in 0..(1 + rng.next() % 6) {
                seq += 1;
                let t = (rng.next() % 50) as i64;
                let mut r = trade(symbols[(rng.next() % 3) as usize], t, seq);
                r.source = if rng.next() % 4 == 0 {
                    None
                } else {
                    Some(BASE + (rng.next() % 50) as i64)
                };
                r.received = if rng.next() % 4 == 0 {
                    None
                } else {
                    Some(BASE + (rng.next() % 50) as i64)
                };
                rows.push(r);
            }
            blocks.push(rows);
        }
        write_trades(&dir.path("a.gqodb.tick"), &blocks, None, 3, 3);
        let all: Vec<&Trade> = blocks.iter().flatten().collect();
        let (clock, get): (&str, Getter) = match rng.next() % 4 {
            0 => ("available", Box::new(|t| Some(t.available))),
            1 => ("received", Box::new(|t| t.received)),
            2 => ("source", Box::new(|t| t.source)),
            _ => ("sequence", Box::new(|t| Some(t.sequence))),
        };
        let (lo, hi) = (rng.next() % 40, rng.next() % 40);
        let (lo, hi) = (lo.min(hi) as i64, lo.max(hi) as i64 + 1);
        let sym = symbols[(rng.next() % 3) as usize];
        // time clocks take UTC times, the sequence clock takes numbers; compare in absolute values
        let (lo_abs, hi_abs, range) = if clock == "sequence" {
            (lo, hi, format!("{lo}..{hi}"))
        } else {
            (BASE + lo, BASE + hi, format!("{}..{}", at(lo), at(hi)))
        };
        let (out, report) = run(&format!(
            "show trades {sym} {range} on {clock} in {}",
            dir.s()
        ));
        let expected: Vec<i64> = all
            .iter()
            .filter(|t| t.symbol == sym)
            .filter(|t| get(t).is_some_and(|v| v >= lo_abs && v < hi_abs))
            .map(|t| t.sequence)
            .collect();
        assert_eq!(
            out.ints("sequence"),
            expected,
            "case {case}: {sym} {lo}..{hi} on {clock}"
        );
        let null_expected = all
            .iter()
            .filter(|t| t.symbol == sym && get(t).is_none())
            .count() as u64;
        if clock != "available" {
            assert_eq!(report.null_clock_excluded, null_expected, "case {case}");
        }
        assert!(out.rows.iter().all(|r| !matches!(r[0], Cell::Null)));
    }
}
