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

`./benchmarks/run.sh verify` checks every published result covered by a receipt and
recomputes the figures printed in the README. `./benchmarks/run.sh synthetic-smoke`
builds the current tree and performs a deliberately small B01/B02-family smoke run;
it is a correctness/reproducibility check, not a new performance result.

The B13/B15 old-versus-new timings remain historical evidence: their receipts name
private-development commit IDs because the public repository was extracted after
those runs. The current public runner contains the final implementation. A new
public performance claim should use a fresh protocol and public commit rather than
pretending those old IDs exist in this repository.
