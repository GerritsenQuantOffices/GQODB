# GQODB open layer 0.1.0 (preview)

Source release of the open layer of GQODB. **This is a preview**: it is not "1.0", it has
no format-stability promise and no signed artifact, and it is not published to crates.io.
The GQODB engine (raw journal, ledger and receipts, recovery, quality, features, causal
query, ops and connectors) is not part of this repository and is not licensed by it.

Licence: Apache-2.0. Date: to be filled in by the owner when the tag is created.

## What is in

- **Four crates** (one version number, released together):
  - `gqodb-types`: exact event and time types with explicit scales.
  - `gqodb-blocks`: lossless codec kernels, tick and book profiles, Parquet
    counter-tests and the benchmark tooling.
  - `gqodb-store`: indexed `.gqodb.ob` segments with range reads and prefix recovery.
  - `gqodb-codec`: the native format facade and file inspection.
- **Formats.** `.gqodb.tick` (adaptive tick profile) and `.gqodb.ob` (order-book profile)
  share `GQOBHDR1` framing; the header, not the file extension, selects the codec
  profile. Frames carry CRC checks; a physically incomplete tail is distinguished from
  a corrupt frame; recovery writes a new file. See `docs/B02_FORMAT.md` and
  `docs/BLOCK_PROTOCOL.md`. Nanosecond units do not imply nanosecond accuracy.
- **Evidence verifier.** `results/verify_public_evidence.py` checks the stored results
  against their build receipts (28 receipted result files) and recomputes the B11, B15
  and B17 summaries (B06 in part) from them; the B12 and B13 figures are not recomputed.
  `benchmarks/verify_manifest.py` checks that every declared benchmark file is tracked.
- **Source installer.** `./install.sh` builds, tests and installs `gqodb-codec` and
  `ob_store` from a clean checkout. It asks before installing system packages or Rust
  and is a source installer, not a signed binary release.
- **Release hygiene.** Crate metadata that passes a `cargo publish --dry-run`
  in a scratch copy, API docs that build without warnings, seven doc-tests, a
  1.85 minimum-Rust CI job, `cargo deny` (licences and advisories), pinned CI actions,
  `docs/API_STABILITY.md`.

## What is measured, and with which bounds

Copied from the README; the bounds are part of the claim. Benchmark figures are
observations from named runs on one host, mostly with warm caches (B17 also reports
reads after the files were evicted from the page cache). The raw inputs are not
redistributed: you can check the receipted result files against their receipts and
recompute the B11, B15 and B17 figures from the stored results, but re-running B11-B15
from scratch needs a Bybit capture of your own.

Tested toolchain: Rust 1.96.0, Linux x86_64 (integration run on 18 October 2026: 76 tests
passed, 0 failed, 0 ignored; the `zstd-input` build is green too; 28 receipted files
verify). The 7 doc-tests are not part of that count.

**Trades (B17).** `results/b17-public-trades-20261018` (run 2, the main result, measured on commit `c07fbde`,
the code that is released; run 1, `results/b17-public-trades-20261017`, used the same protocol on an earlier commit,
before the limit on decoded string bytes was added, and is kept for comparison): Binance aggregate trades of
PAXGUSDT (spot, 2025-10 and 2026-02), the first 1,048,576 rows of each month (D1, D2) and
both full months (3.1 and 3.7 million rows). Real files written durably, read fully and by
a 60-second time window; single thread, one pinned core, a fresh process per sample, up to 15
unflagged measured rounds after 3 warm-ups, medians with a block-bootstrap 95% interval.
Baselines: four Rust Parquet configurations, DuckDB 1.5.6 (native file and three Parquet
writers) and ClickHouse 26.3 (MergeTree, default and one pre-registered codec chain). The
protocol was pre-registered and hash-pinned; addendum A-004 is an amendment (interference
thresholds and repeat budget) made after a first start of run 1 was aborted; with the registered
rules every cell would have been UNDERPOWERED, and two cells of run 2 (D2/v1 with 11 valid rounds, D2/v2 with 12)
are UNDERPOWERED even with the amended rules. Input: Binance Vision, CC BY-NC-SA 4.0, not
redistributed; aggregates only.

| Slice | Axis | GQODB | Strongest baseline | Ratio | Verdict |
| --- | --- | ---: | --- | ---: | --- |
| D1 | Stored bytes | 6.51 MB | `duckdb-parquet-zstd`, 6.70 MB | 0.972 | better |
| D1 | Durable write | 225.2 ms | `parquet-delta-bss-snappy`, 253.6 ms | 0.888 | better |
| D1 | Full read, warm | 65.5 ms | `parquet-delta-bss-lz4`, 81.2 ms | 0.806 | better |
| D1 | 60 s range read, warm | 3.2 ms | `parquet-delta-bss-lz4`, 4.1 ms | 0.772 | better |
| D1 | Peak memory, write | 72.6 MiB | `duckdb-parquet-matched`, 17.1 MiB | 4.244 | worse |
| D1 | Peak memory, full read | 68.1 MiB | `parquet-delta-bss-snappy`, 68.8 MiB | 0.990 | equivalent |
| D2 | Stored bytes | 6.21 MB | `duckdb-parquet-zstd`, 7.63 MB | 0.814 | better |
| D2 | Durable write | 228.5 ms | `parquet-delta-bss-snappy`, 250.0 ms | 0.914 | better |
| D2 | Full read, warm | 66.5 ms | `parquet-delta-bss-lz4`, 77.8 ms | 0.855 | better |
| D2 | 60 s range read, warm | 3.2 ms | `parquet-delta-bss-snappy`, 4.1 ms | 0.786 | better |
| D2 | Peak memory, write | 72.5 MiB | `duckdb-parquet-matched`, 17.4 MiB | 4.175 | worse |
| D2 | Peak memory, full read | 68.1 MiB | `parquet-delta-bss-snappy`, 68.8 MiB | 0.991 | equivalent |

GQODB does not win every axis and the earlier headline "wins every axis" is withdrawn.
Losses: the peak memory of the write is 2.2 to 4.3 times that of the DuckDB Parquet writers
on the slices and up to 5.0 times that of DuckDB's default Parquet writer on the full months
(that is the adapter path, which reads the whole table into memory first for every Rust
candidate, not a property of the format); full-read memory on the full months is 2.5 to 3.0
times that of the ClickHouse engine process (not like for like); the warm range read against
ClickHouse is inconclusive in 7 of 16 cells (ClickHouse range operation times are often 0 and
these ratios are noise); in V2 the write-time advantage over the Rust Parquet writers
disappears (1.06 against `parquet-delta-bss-snappy` on D1, worse) and the CPU time of the write against
`parquet-delta-bss-snappy` is worse on D1 (1.07). Run 1 and run 2 agree: 26 of 864 verdicts differ, 15 of them
range reads against ClickHouse, and none of the stored-bytes verdicts; the run-2 medians are 0.4% to 7% lower than
run 1's for GQODB and for the baselines alike (a different machine state), so the extra pass over string columns
added by the limit fix is not visible, but an effect of a percent or two cannot be excluded. The run-2 receipt names
`c07fbde`, which is in this history; run 1's receipt names a commit that is not (addendum A-005).

**Trades (B06, historical, superseded by B17).** `results/b06`: 1,048,576 aggregate trades per
sample (32 blocks), in-memory codec passes, one warm-up and nine measured passes per mode,
medians. GQODB is compared per axis against the strongest of five Parquet configurations on
that axis. B06 has no build receipts: these figures are historically measured, not
hash-checked, and the input files no longer exist.

| Sample | Axis | GQODB | Strongest Parquet | Difference |
| --- | --- | ---: | --- | ---: |
| ETH | size | 5.089 MB | normalized-LZ4, 5.464 MB | -6.9% |
| ETH | encode | 78.363 ms | delta/BSS-Snappy, 146.439 ms | -46.5% |
| ETH | decode | 23.666 ms | normalized-LZ4, 38.719 ms | -38.9% |
| UNI | size | 4.708 MB | normalized-LZ4, 6.189 MB | -23.9% |
| UNI | encode | 65.478 ms | delta/BSS-LZ4, 142.945 ms | -54.2% |
| UNI | decode | 21.277 ms | delta/BSS-LZ4, 31.280 ms | -32.0% |

**Order books: a loss, and what changed.** The same B06 run lost on order books: against
dictionary-LZ4 Parquet the adaptive codec was 22.6% larger and decoded 22.5% slower,
winning only on encode. The next four runs are the record of fixing that, one boundary
at a time.

| Run | What changed | Size vs dictionary-LZ4 | Write | Read | Evidence |
| --- | --- | --- | --- | --- | --- |
| B06 | adaptive codec, in memory | 22.6% larger | encode 7.0% faster than best | decode 22.5% slower | historically measured |
| B11 | dedicated book profile, in memory | 38.4-42.9% smaller | encode time -40% | decode 14.3-14.5% slower | hash checked |
| B12 | real `.gqodb.ob` files | 37.8% smaller | write + sync 28.5% less | full reads and window queries slower | historically measured |
| B13 | reader rewrite, same file bytes | unchanged | unchanged | full reads 2.4-3.5x faster than B12 | hash checked |
| B15 | six measured rounds on files | 37.75% smaller | 36.67% less | 3.96% less | hash checked |

The in-memory decode loss of B11 is still there: Parquet decodes that projection faster
than both GQODB codecs. What B13 fixed is the file reader around it.

**Order-book files (B15).** `results/b15`, `storage.json`: 1,048,576 Bybit level rows
from 7,265 events, 32 blocks or row groups. One warm-up, six measured rounds, medians.
Warm page cache, one host. File-sync counted; directory sync and physical power-loss
qualification not.

| Format | Bytes | Write + file-sync | Full read |
| --- | ---: | ---: | ---: |
| GQODB | 3,703,770 | 198.361740 ms | 39.972222 ms |
| Parquet dictionary-LZ4 | 5,950,242 | 313.210893 ms | 41.620335 ms |
| Parquet delta-LZ4 | 9,242,314 | 248.710886 ms | 56.811581 ms |

Against dictionary-LZ4: -37.7543% bytes, -36.6683% write, -3.9599% read, using
`100 x (1 - GQODB / reference)`. This is an order-book level projection that excludes
the collector envelope, so it is not evidence that the same saving holds for every
source's original bytes, nor a claim about cold disk or network access.

## What is not done

From the README, "Still not measured":

| Measurement | Status |
| --- | --- |
| A cold device, datasets larger than about 3.7 million rows | not measured: B17 evicts the page cache only, the other runs are warm-cache |
| Instruments and venues beyond Bybit levels, Binance ETH/UNI trades (B06) and Binance PAXGUSDT trades (B17) | not measured |
| Multi-threaded, concurrent or networked use; other hosts | not measured |
| kdb+ / q | no comparison published |
| Rust versions other than 1.85.0 and 1.96.0, Windows and macOS | only 1.85.0 and 1.96.0 were built and tested, on Linux; no CI run exists yet; Windows and macOS not run |
| Adversarial fuzzing of the readers | partly done: 15 libFuzzer targets, 9 defects fixed, none open; the unchecked decoding layer remains unaudited (the 128 MiB limit on decoded string bytes closes one class, not all); see `docs/FUZZING.md` |

Also not done: crates.io publication, a signed tag or artifact, a format-stability
promise, directory-sync and power-loss qualification of segment files.

## Install and verify

Build from source (Linux, Rust 1.96.0 tested; 1.85 is the declared minimum, built and
tested locally on Linux; the workflow has a job for it that has not run on GitHub yet):

```sh
git clone https://github.com/GerritsenQuantOffices/GQODB.git
cd GQODB
git checkout v0.1.0            # once the owner has created the tag
cargo build --workspace --bins --release --locked
cargo test  --workspace --all-targets --locked
cargo test  --workspace --doc --locked
python3 results/verify_public_evidence.py
python3 benchmarks/verify_manifest.py
```

Or use `./install.sh --prefix "$HOME/.local" --ref v0.1.0`, which asks before installing
anything it needs. For a source archive, compare it with the checksum file attached to
the GitHub release (`sha256sum -c`). A checksum shows the bytes match what the owner
published; it does not say who the publisher is. There is no signature on the archive.

## Known limitations

- No stability promise for APIs, command-line interfaces or file formats: see
  `docs/API_STABILITY.md`. Breaking changes are possible in any 0.x release.
- Much of `gqodb-blocks` is benchmark and feasibility code, not a database.
- Exact `arrow`/`parquet` version pins (`=59.3.0`) are inherited by dependent crates.
- The readers parse untrusted bytes. The unchecked decoding layer (`real::decode`,
  `Reader::read_block`, `read_all`, `visit_blocks`, `query`, `recover`, and therefore the
  `ob_store verify`, `query` and `recover` commands) is not audited. The one fuzzing finding in it
  (a crafted 60 KB block requesting 62.9 GB for a string column) is fixed by a 128 MiB limit on
  decoded string bytes, which also makes `real::encode` refuse a batch above that limit. The limit
  closes that class, not all: the numeric columns of a block are not capped there (a 16 KB block can
  decode to about 0.5 GiB). `gqodb-codec` (including `inspect` and `validate`) and
  `Reader::read_block_bounded` check a total expansion estimate of 128 MiB first. Frame checksums,
  declared-size checks and validation rejected the malformed and truncated inputs used in the
  test suites and by 15 fuzz targets of limited duration (`docs/FUZZING.md`); this is not a
  security-audit result.
- macOS and Windows have not been run by the author, and no CI run exists yet; the macOS and
  Windows CI entries are marked `continue-on-error` until a first CI run exists.
- Some crate READMEs under `crates/` are partly in Dutch.
- B06 has no build receipts; B12 is historically measured only. The B13, B15 and B17
  receipts name private-development commit IDs that do not exist in this repository; for
  B17 the measured code is mapped to this history by tree hashes (addendum A-005).
