# Benchmark inventory

Every benchmark cited by the public README is indexed here. Runner source,
protocol, stored reports and receipt status are versioned in this repository.
Large market captures are deliberately not redistributed: the input column records
the required shape and provenance, and the protocol explains how to supply an
equivalent local input. Synthetic B01/B02 runs need no external dataset.

| Run | Runner in this tree | Protocol | Stored evidence | Input availability |
| --- | --- | --- | --- | --- |
| B01 | `gqodb-blocks` binary | `docs/BLOCK_EXPERIMENT.md` and `docs/BLOCK_PROTOCOL.md` | `results/b01-20260907/` | deterministic synthetic generator in Git |
| B02 | `gqodb-blocks` binary | `docs/B02_PROTOCOL.md` and `docs/B02_FORMAT.md` | `results/b02-20260907/` | deterministic synthetic generator in Git |
| B06 | `market_benchmark` example | `docs/MARKET_BENCHMARK.md` | `results/b06/` | user-supplied Binance trade Parquet or Bybit book capture; historical input not redistributed |
| B11 | `market_benchmark` example with `GQODB_BOOK_CODEC=1` | `docs/B11_BOOK_COMPRESSION.md` | `results/b11/` | user-supplied Bybit book capture; historical input not redistributed |
| B12 | `storage_benchmark` example | `docs/B12_STORAGE_PROTOCOL.md` | `results/b12/` | user-supplied Bybit book capture; historical input not redistributed |
| B13 | `storage_benchmark` read-only mode | `docs/B13_READ_PROTOCOL.md` | `results/b13/` | historical paired commits and unchanged B12 artifacts named in receipts |
| B15 | `storage_benchmark` read-only and full modes | `docs/B15_READ_PROTOCOL.md` | `results/b15/` | historical paired commits and unchanged B12 artifacts named in receipts |
| B17 | `benchmarks/public_trades/harness.py` with the `trades_convert` and `trades_candidate` examples | `docs/B17_PUBLIC_TRADES_PROTOCOL.md` (hash-pinned) | `results/b17-public-trades-20261018/` (main result, run 2 on the released commit) and `results/b17-public-trades-20261017/` (run 1, earlier commit); both with hash-checked receipts | public Binance Vision archives, fetched and checksum-verified by `fetch.sh`; nothing is redistributed |

`./benchmarks/run.sh verify` checks every published result covered by a receipt and
recomputes the B11, B15 and B17 summaries from the stored results (B06 in part). `./benchmarks/run.sh synthetic-smoke`
builds the current tree and performs a deliberately small B01/B02-family smoke run;
it is a correctness/reproducibility check, not a new performance result.

B17 has two result directories: run 2 (`results/b17-public-trades-20261018`, measured on the released commit) is the main
result and run 1 (`results/b17-public-trades-20261017`, same protocol, an earlier commit) is kept for comparison. The status line in
`benchmarks/public_trades/README.md` still names only run 1; that file was left unchanged on purpose (see addendum A-005).

B17 is the first benchmark in this tree that a reader can rerun end to end on public data against
named baselines (DuckDB, ClickHouse; licence notes in `docs/BASELINE_LICENCES.md`). Its result directories have hash-checked receipts and every sample;
the input data (Binance Vision, CC BY-NC-SA 4.0) are not redistributed and must be fetched by the user.

The B13/B15 old-versus-new timings remain historical evidence: their receipts name
private-development commit IDs because the public repository was extracted after
those runs. The current public runner contains the final implementation. A new
public performance claim should use a fresh protocol and public commit rather than
pretending those old IDs exist in this repository.
