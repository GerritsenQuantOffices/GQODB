# B17: public-data, file-level benchmark (tooling)

Protocol (frozen, hash-pinned): [`docs/B17_PUBLIC_TRADES_PROTOCOL.md`](../../docs/B17_PUBLIC_TRADES_PROTOCOL.md).
Clarifications found while building it: [`docs/B17_PUBLIC_TRADES_ADDENDA.md`](../../docs/B17_PUBLIC_TRADES_ADDENDA.md).
Licence findings for every baseline and for the input: [`docs/BASELINE_LICENCES.md`](../../docs/BASELINE_LICENCES.md).

**Status: a timed run on real data exists**: `results/b17-public-trades-20261017/` (hash-checked
receipt, every sample, [assessment](../../results/b17-public-trades-20261017/ASSESSMENT.md)). The
tooling in this directory produced it. Nothing in this directory is itself a performance result, and
the fixtures under `tests/fixtures/b17/` are synthetic.

## Run it yourself

```sh
./benchmarks/run.sh public-trades build                       # builds the two Rust examples
python3 benchmarks/baselines/install_tools.py --dest ~/b17-tools   # DuckDB, pyarrow, ClickHouse; checksum-verified, outside the repo
export B17_PYTHON=~/b17-tools/venv/bin/python B17_CLICKHOUSE=~/b17-tools/bin/clickhouse
./benchmarks/run.sh public-trades plan                        # what a full run would execute (runs nothing)
./benchmarks/run.sh public-trades smoke --work /var/tmp/b17-smoke --fixtures FX2 --quick   # functional check on synthetic data
./benchmarks/run.sh public-trades prepare --work ~/b17-run --datasets D1,D2                # downloads and verifies the archives
B17_MEASURE_GO=1 ./benchmarks/run.sh public-trades measure --work ~/b17-run --out results/b17-public-trades-DATE
```

* `fetch.sh` downloads one monthly archive from `data.binance.vision`, compares its sha256
  with Binance's own `.CHECKSUM` and with the value pinned in the protocol, and refuses on
  any mismatch (exit 3). It also accepts a local copy (`--zip`, `--checksum`). No data file is
  ever written inside the repository.
* `trades_convert` (Rust example) turns the CSV into the typed Parquet source and prints the
  reference digests; `common.py` recomputes them independently from the CSV in Python and the
  run aborts if the two differ.
* `trades_candidate` (Rust example) is the adapter for GQODB and the Parquet configurations.
  The DuckDB and ClickHouse adapters live in `benchmarks/baselines/` and speak the same
  command line; each prints `{"skipped": ...}` and exits 0 when its tool is absent.
* `measure` needs `B17_MEASURE_GO=1`, a clean git tree, a protocol file that still matches its
  pinned hash and a disk filesystem. The shared machine rules (pinned core with idle
  hyperthread sibling, quiet-box wait, flagged and repeated rounds) are in the protocol.
* `python3 benchmarks/public_trades/verify_b17.py` (also called by
  `results/verify_public_evidence.py`) checks every `results/b17-*` directory: receipt, hashes,
  pinned protocol, clean tree, no leak patterns (generic: absolute home paths, carrier-grade-NAT
  addresses of the kind VPN overlays use, e-mail addresses, private-key blocks), round counts, and a full recomputation of
  `summary.json` from the raw samples. A result file that was reworded after sealing must be listed in
  `results/PUBLIC_REDACTION.json` with the receipted hash and the hash of the public copy.
* After a run, `ASSESSMENT.md` is generated with tables and two marked author sections. Fill
  them in, then run `harness.py seal --out DIR` to refresh the receipt hashes.

## Input data and attribution

**Input data: Binance Vision (https://data.binance.vision), aggregate trades of PAXGUSDT, spot, monthly files
2025-10 and 2026-02.** The data are provided by Binance under CC BY-NC-SA 4.0 and the "Binance
Vision Dataset Terms" (version 1.0, last updated 26 August 2026), which also apply to data derived
from them. This repository does not contain or redistribute any of that data: you must fetch it
yourself (`fetch.sh`) and comply with those terms. The published result files are aggregates only
(byte counts, timing medians, ratios and intervals, digests of whole tables). The frozen protocol and one
synthetic fixture do contain a handful of per-trade values (first and last trade id, one timestamp);
see `docs/BASELINE_LICENCES.md`.
Binance does not sponsor, endorse or partner with this project or its results.
