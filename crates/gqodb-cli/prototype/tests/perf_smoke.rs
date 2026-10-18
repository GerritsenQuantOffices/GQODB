mod support;

use support::*;

/// `cargo test -p gqodb-cli --test perf_smoke --release -- --ignored --nocapture`
#[test]
#[ignore = "a measurement, not a gate"]
fn two_hundred_thousand_rows_in_fifty_blocks() {
    let dir = TempDir::new();
    let blocks: Vec<Vec<Trade>> = (0..50)
        .map(|b| {
            (0..4_000)
                .map(|i| trade("EURUSD", b * 4_000 + i, b * 4_000 + i + 1))
                .collect()
        })
        .collect();
    write_trades(&dir.path("a.gqodb.tick"), &blocks, None, 5, 2);
    let t = std::time::Instant::now();
    let (out, report) = run_with(
        &format!(
            "show trades EURUSD {}..{} on available in {}",
            at(100_000),
            at(104_000),
            dir.s()
        ),
        &gqodb_cli::exec::Options {
            max_rows: 100_000,
            ..Default::default()
        },
    );
    println!(
        "index clock: {} rows in {:?}; blocks {} of {}",
        out.rows.len(),
        t.elapsed(),
        report.blocks_read,
        report.blocks_total
    );
    assert!(
        report.blocks_read <= 2,
        "the index must skip nearly every block"
    );
    let t = std::time::Instant::now();
    let (out, report) = run_with(
        &format!(
            "show trades EURUSD {}..{} on received in {}",
            at(100_000),
            at(104_000),
            dir.s()
        ),
        &gqodb_cli::exec::Options {
            max_rows: 100_000,
            max_scan_rows: 1_000_000,
            ..Default::default()
        },
    );
    println!(
        "received clock: {} rows in {:?}; blocks {} of {}",
        out.rows.len(),
        t.elapsed(),
        report.blocks_read,
        report.blocks_total
    );
    assert_eq!(
        report.blocks_read, 50,
        "a clock that is not indexed reads every block with the symbol"
    );
}
