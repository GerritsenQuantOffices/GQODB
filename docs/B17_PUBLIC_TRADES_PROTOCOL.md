# B17: public-data, file-level benchmark with matched baselines (pre-registered protocol)

Registered 2026-10-17, before any B17 runner code exists and before any timing or
size of any candidate on the real input has been computed. Amendments are only
allowed as dated entries in `docs/B17_PUBLIC_TRADES_ADDENDA.md`; this file is never
edited after its hash is pinned in `docs/B17_PUBLIC_TRADES_PROTOCOL.sha256`.

## 0. Why this exists

The B06 headline "GQODB wins every axis" is true only for in-memory codec passes on
inputs that are not redistributable and are no longer available (`results/b06` has
no receipts). No database baseline exists in this repository, and a reader cannot
rerun anything. B17 answers three narrower questions on **public data that anyone
can fetch and checksum**, at **file level** (durable write, full read, small
time-range read), against **matched, named baselines**. Every outcome, including
losses, is reported. The README headline is rescoped to what B17 supports.

## 1. Knowledge state at registration (what was already seen)

Honest list of what was looked at before this file was frozen:

* The two input archives were downloaded and their CSV layout was inspected
  (header absence, field count, timestamp magnitude), and one pass over all rows
  computed: row counts, whether `agg_id` increases by exactly one, whether
  `transact_time` ever decreases, decimal digit counts, price range, quantity
  maximum, and the set of (is_buyer_maker, is_best_match) pairs. These are
  properties of the input, not of any candidate.
* The candidate tools were installed and exercised on **synthetic 1,000-row
  fixtures** only, to confirm that the statements in section 5 are syntactically
  valid and the options exist. No candidate was run on the real input; no size or
  time of any candidate on real data exists.
* Licence texts were read (`docs/BASELINE_LICENCES.md`).

## 2. Input

Source: Binance Vision spot monthly `aggTrades`, instrument PAXGUSDT, fetched from
host `data.binance.vision`, object key
`data/spot/monthly/aggTrades/PAXGUSDT/PAXGUSDT-aggTrades-<YYYY-MM>.zip`, with the
sibling `.CHECKSUM` object. The `.CHECKSUM` value must equal the sha256 of the zip
and both must equal the pinned value below, otherwise the fetch tool refuses.

| Month | zip sha256 (pinned) | data rows | first / last `agg_id` |
| --- | --- | --- | --- |
| 2025-10 | `de68061c03e6c7c5487dd805bf2b6b24348dee02ec33db1a4f33d2ecb8f1ad52` | 3,132,841 | 16726937 / 19859777 |
| 2026-02 | `effc512bdc63dcd1c023d1e00c55101f9ad93bd3002698c9965796e542881bd5` | 3,653,318 | 25231411 / 28884728 |

Facts about the files (read before registration, section 1): no header line; eight
comma-separated fields per line; `agg_id` increases by exactly 1 on every row in
both months; `transact_time` never decreases in file order; every price and
quantity is printed with exactly 8 decimals; `is_best_match` is `True` on every
row; booleans are the literals `True`/`False`.

**Correction to the working assumption in the brief.** The brief called
`transact_time` milliseconds. For Binance **spot** files from 2025-01-01 onwards
the archive README states that timestamps are in **microseconds**, and the files
agree (16-digit values, for example 1759276803099156). The benchmark treats the
column as an opaque `Int64` and requires exact integer equality; the unit
(microseconds) is written into the schema metadata and the receipt. The
time-range window is expressed in the same unit.

Datasets (file order is the only order):

| Id | Content |
| --- | --- |
| D1 | first 1,048,576 data rows of 2025-10 (32 blocks of 32,768 rows) |
| D2 | first 1,048,576 data rows of 2026-02 (32 blocks) |
| S1 | full month 2025-10 (96 blocks), headline candidates only |
| S2 | full month 2026-02 (112 blocks), headline candidates only |

The table written to every candidate has nine columns (same order everywhere):
`agg_id` Int64, `price`, `qty`, `first_id` Int64, `last_id` Int64, `transact_time`
Int64, `is_buyer_maker` Boolean, `is_best_match` Boolean, `symbol` Utf8. The ninth
column is the constant `PAXGUSDT`. It is not in the CSV; it exists because the
GQODB segment format requires a string index column, and it is carried by **every**
candidate so that all of them store the same table.

* **V1 (source-oriented)**: `price`, `qty` are IEEE-754 binary64, parsed from the
  decimal text with correct rounding (nearest double). Exactness is bitwise.
* **V2 (exact-normalised)**: columns are named `price_e8`, `qty_e8` and are Int64
  holding the decimal value times 10^8, parsed **directly from the decimal text**
  (no floating point involved). A V2 value that has more than 8 decimals, is
  negative, or overflows is a hard error.

Normalisation cost (V1 to V2). A separate phase `normalise` starts from the V1
table in memory and produces V2 with the proof that every double converts back to
the identical bits (`round(x * 1e8)` as Int64 and back by division). Its median
time and the denormalisation time are reported per dataset. Two derivations of V2
must agree digest-for-digest (text to int, and float to int): if they do not, V2 is
declared not available for that dataset. A second V2 write measurement, "write incl.
normalisation", is the median V2 write time plus the median normalisation time, the
same constant added to every candidate (it is derived, not a separate run).

## 3. Exactness rule

A candidate result is valid only if the canonical digest of what it returns equals
the reference digest. Definition of the digest (SHA-256, streaming):

for each column in table order, feed the bytes
`"col|" + name + "|" + type + "|" + nrows + "\n"` followed by the column data, where
type is `i64`, `f64`, `bool` or `utf8`; `i64`: 8 bytes little endian per value;
`f64`: the 8 little-endian bytes of the IEEE bit pattern (so `-0.0` differs from
`0.0` and NaN payloads count); `bool`: one byte 0 or 1 per value; `utf8`: the u64
little-endian total byte length, then one u32 little-endian length per value, then
all value bytes concatenated. Rows are in ascending `agg_id` order, which equals
file order (verified: strictly increasing). A tool that returns another physical
order sorts by `agg_id` before hashing, outside the timed region. Nulls are not
allowed anywhere; a null is a failure. Schema metadata is outside the digest, but
the logical types are inside it (an Int64 returned as Int32 fails).

The reference digest is derived **two independent ways** and the run aborts if they
differ: (R1) the Rust converter parses the CSV; (R2) a separate Python
implementation parses the same CSV with `int()` and `float()` and hashes with its own
digest code. The time-range expectation (row set and digest) is derived by both too.

A candidate with any digest mismatch on a dataset is reported as `EXACTNESS FAIL`
for that dataset and is excluded from every ratio on it; it is never counted as a
win.

## 4. Candidates, layouts, tuning bound

Common to all: single thread, the same table, 32,768-row blocks where the format
has a block notion, durable output (section 6).

| Id | System | Layout and settings |
| --- | --- | --- |
| `gqodb` | this repository, `gqodb-store` `Writer` with the tick codec (`Mode::Gqodb`) | one `.gqodb.ob` file, 32,768-row blocks, index columns `transact_time` and `symbol` |
| `parquet-delta-bss-lz4` | arrow-rs parquet 59.3.0 (the pinned workspace version) | one file, one row group per 32,768 rows, Parquet V2, page statistics, DELTA_BINARY_PACKED for ints and bools-as-PLAIN, BYTE_STREAM_SPLIT for floats, DELTA_BYTE_ARRAY for the symbol, dictionary off, LZ4_RAW |
| `parquet-dictionary-lz4` | same | as above but dictionary on (the listed encodings are the fallback), LZ4_RAW |
| `parquet-delta-bss-snappy` | same | as the first, SNAPPY |
| `parquet-delta-bss-brotli5` | same | as the first, BROTLI level 5 |
| `duckdb-native` | DuckDB 1.5.6 Python | `CREATE TABLE t AS SELECT * FROM read_parquet(source)` into a database file, defaults (storage compression automatic, its own row-group size), `CHECKPOINT`, close |
| `duckdb-parquet-default` | DuckDB 1.5.6 | `COPY ... TO file (FORMAT PARQUET)` with all defaults of 1.5.6 (effective codec and row-group size are read back with `parquet_metadata()` and recorded) |
| `duckdb-parquet-matched` | DuckDB 1.5.6 | `(FORMAT PARQUET, COMPRESSION lz4_raw, ROW_GROUP_SIZE 32768, PARQUET_VERSION V2)` |
| `duckdb-parquet-zstd` | DuckDB 1.5.6 | `(FORMAT PARQUET, COMPRESSION zstd, ROW_GROUP_SIZE 32768, PARQUET_VERSION V2)` |
| `clickhouse-default` | ClickHouse `clickhouse local` 26.3.54.6 (LTS) | MergeTree on disk, no codec clause (server default LZ4), `symbol String`, `ORDER BY (symbol, transact_time, agg_id)` |
| `clickhouse-tuned` | same | same table, `symbol LowCardinality(String)`, codec chains in section 4.1 |

Alias for continuity with B06: `parquet-normalized-lz4` is exactly
`parquet-delta-bss-lz4` on the V2 table (there are no floats left, so BYTE_STREAM_SPLIT
is moot).

Applies to V1 and V2 alike; V2 swaps `price`, `qty` for `price_e8`, `qty_e8`.
The headline set H, used for S1 and S2 and for the headline claims, is
{`gqodb`, `parquet-delta-bss-lz4`, `parquet-dictionary-lz4`, `duckdb-native`,
`duckdb-parquet-default`, `clickhouse-default`, `clickhouse-tuned`}.

### 4.1 Tuning bound (fixed here, not searched)

Tuning is **one pre-registered alternative per tool, chosen from the vendors'
documented guidance, never by running on the real input**. Nothing is added after
results are seen. The published list is exactly the table above; an invited
reviewer who knows a better configuration is asked to send it as a pull request,
and it would be reported as an addendum with its date, not merged into the
original table.

ClickHouse `clickhouse-tuned` codec chains (V1): `agg_id Int64 CODEC(DoubleDelta,
ZSTD(1))`, `price Float64 CODEC(Gorilla, ZSTD(1))`, `qty Float64 CODEC(ZSTD(1))`,
`first_id Int64 CODEC(Delta, ZSTD(1))`, `last_id Int64 CODEC(Delta, ZSTD(1))`,
`transact_time Int64 CODEC(DoubleDelta, ZSTD(1))`, `is_buyer_maker Bool
CODEC(ZSTD(1))`, `is_best_match Bool CODEC(ZSTD(1))`, `symbol
LowCardinality(String)`. V2: `price_e8 Int64 CODEC(Delta, ZSTD(1))`, `qty_e8 Int64
CODEC(ZSTD(1))`, all other columns as in V1.

Both ClickHouse candidates add the same durability and layout settings, which are
not tuning: MergeTree settings `fsync_after_insert = 1`, `fsync_part_directory = 1`;
the insert runs with `max_insert_block_size` and `min_insert_block_size_rows` raised
to 2^26, `max_threads = 1`, `max_insert_threads = 1`, so that one input becomes one
part; the number of parts is recorded. The engine's default granule size (8192) is
kept.

DuckDB native storage has no user-visible block size or codec setting that is
documented as the normal path; its defaults are used.

## 5. Phases and the adapter contract

Every candidate is driven by a program (an adapter) that accepts

`adapter PHASE --candidate ID --variant v1|v2 --source SOURCE.parquet --store DIR
[--symbol S --start T --end T]`

and prints a JSON object as its last stdout line. Phases:

* `noop`: start the tool (import library, open runtime), do nothing. Measures
  startup, startup RSS, startup CPU.
* `write`: ingest `SOURCE.parquet` (the typed source file, see 7) and write the
  candidate's files into a fresh `DIR`.
* `read-full`: read every row and every column into the client's memory.
* `read-range`: all columns of rows with `symbol = S` and `T0 <= transact_time <=
  T1` (inclusive), materialised. One window per dataset: `T0` is the `transact_time`
  of the row at index `floor(n / 2)` and `T1 = T0 + 60,000,000` (60 seconds in
  microseconds). The window must contain at least one row.
* `source-read` (control): read the source file and materialise it without writing
  anything. It measures the work all candidates share in `write`.
* `normalise` (Rust adapter only): section 2.

Each adapter measures its own region with a monotonic clock:

* `write`: the clock starts after process start and library load, immediately before
  the first read of the source; it stops after **all output files and their parent
  directories have been fsynced** by the common procedure `fsync_tree` (every
  regular file, then every directory bottom-up, then the parent of the output
  root). The identical procedure is implemented in the Rust adapter and in the
  Python adapters; the tools' own sync settings are on in addition (they are
  not credited separately).
* `read-full`, `read-range`: the clock starts before the first call into the
  storage layer (open included) and stops when the result is complete in client
  memory as typed columns. The digest and the comparison happen after the clock
  stops and are not timed. The adapter records `rss_hwm_kb` (VmHWM) and CPU time
  (sum of `schedstat` / process CPU clock) at the moment the clock stops, so
  hashing does not pollute either.
* ClickHouse is driven through `clickhouse local --path DIR`, a separate process
  per statement. Its adapter runs a `SELECT 1` start immediately before the
  operation and defines `op_ns` as wall time of the operation process minus that
  startup wall time (floored at zero). Raw numbers are also stored. Rows are
  delivered to the client with `FORMAT ArrowStream` through a pipe and decoded by
  pyarrow outside timing. Resource figures are those of the ClickHouse process
  (`wait4` rusage).

Materialisation by candidate (what "in client memory" means): Rust candidates give
Arrow `RecordBatch`es; DuckDB gives an Arrow table (`fetch_arrow_table`); ClickHouse
gives bytes of an Arrow IPC stream received by the client. These are not identical
work and the assessment says so.

## 6. Procedure

* Filesystem: the benchmark directory must be on a disk filesystem (not tmpfs,
  not overlay). The filesystem type and rotational flag are recorded.
* One physical core: candidate processes are launched with `taskset -c C` where C
  is fixed per run (default CPU 10, hyperthread sibling CPU 24 left idle; the
  harness itself runs on another physical core). Any other core pair may be used if
  declared before the run; CPUs 6 and 20 are never used here because another
  benchmark may hold them. All engines are configured to one thread (Rust code is
  single-threaded; DuckDB `threads = 1`; ClickHouse `max_threads = 1`).
* A fresh process for every phase of every sample.
* Per round r (0 to 17; rounds 0, 1, 2 are warm-ups and are not analysed; rounds 3
  to 17 are the 15 measured rounds) and per candidate, the fixed sequence is:
  `noop`; `write` into a new directory; evict; `read-range` (cold-ish); evict;
  `read-full` (cold-ish); `read-range` (warm, cache primed by the cold full read);
  `read-full` (warm); delete the directory. `source-read` runs once per round for one
  representative candidate of each tool family (`gqodb`, `duckdb-native`,
  `clickhouse-default`).
* Eviction is `posix_fadvise(POSIX_FADV_DONTNEED)` on the files of that candidate
  only, after they were fsynced (so the pages are clean). A global cache drop is
  never used. Residency is measured with `mincore` before every read phase and
  stored; a cold-ish sample counts as cold-ish only when at most 10% of the
  candidate's pages are resident, else the cold result for that candidate is
  `NOT COMPARABLE`. The tool binaries, libraries and the source file stay cached.
  Before a `write` the source file is read once (untimed) so it is resident.
* Candidates are alternated round by round: in round r the candidate list
  (in the order of the table above) is rotated by r positions and reversed when r is
  odd. Every phase of candidate A in round r happens before candidate B starts.
* Stored per sample: all raw numbers (`op_ns`, end-to-end wall, CPU, VmHWM, bytes
  apparent and allocated, number of files, parts, digest, resident fractions,
  per-core busy deltas, load average), never only the summary.
* Interference rule (the machine is shared). Before every round the harness waits
  until the box-wide busy time of all CPUs other than the candidate core, its
  sibling, the harness core and CPUs 6 and 20 has been at most 4 CPU-equivalents over
  the previous 10 seconds (polling every 10 s, at most 30 minutes, otherwise it stops
  and reports "box not quiet"). A **sample is flagged** if the sibling CPU shows more
  than max(2 jiffies, 2% of the sample's wall time) of busy time while the sample ran,
  or if, in a warm read phase, CPU time / wall time of the engine process is below
  0.85, or if the process fails. A round with any flagged sample is repeated; at most
  5 extra rounds are added (20 attempts in total). Statistics use only complete,
  unflagged rounds; if fewer than 15 remain the result is reported with its n and
  the label `UNDERPOWERED`. Flagged samples stay in the raw data and a sensitivity
  table uses all rounds. **No sample is dropped for its timing value.** The number of
  flagged rounds is part of the report.

## 7. Source and receipts

* `fetch.sh` downloads and verifies the zip, unpacks the single CSV member (no CSV
  is stored in the repository).
* `trades_convert` writes a typed Parquet **source**: the table of section 2 (V1 or
  V2), PLAIN encoding, no compression, no dictionary, row groups of 32,768 rows,
  in file order, schema metadata `unit=us`, `symbol=PAXGUSDT`. It refuses any
  deviation of the CSV from the checked facts (field count, decimal digits, boolean
  literals, strictly increasing `agg_id`, non-decreasing time).
* The receipt (`BUILD_RECEIPTS.json`) records the protocol file name and sha256, the
  input zip sha256 and the source Parquet sha256, git commit and a clean-tree flag,
  `rustc` version, `Cargo.lock` sha256, tool versions (DuckDB, pyarrow, ClickHouse,
  Python), CPU model class, kernel, filesystem type, pinned core, all settings in this
  document in effect, and `result_sha256` for every result file in the format already
  verified by `results/verify_public_evidence.py`. A dirty working tree is refused for
  published runs. Paths in receipts are relative; host names and user names are not
  recorded.

## 8. Metrics

Per dataset x variant x candidate:

1. Stored bytes: sum of apparent file sizes of **all** files the candidate created
   (including database metadata, catalog and access files, WAL, index files); also
   allocated bytes (`st_blocks * 512`) and the file count.
2. Durable write time `op_ns` (section 5), plus the derived "write incl.
   normalisation" for V2.
3. Full-read time, warm and cold-ish.
4. Small time-range read time, warm and cold-ish.
5. Peak RSS above the tool's own `noop` RSS (median of this round's noop for that tool),
   for the write and for the full-read phase.
6. CPU time (user + system) above the tool's own `noop` CPU, same phases.
7. End-to-end wall time of the whole process including startup (secondary, stored
   for every phase): it is the figure a command-line user sees. Startup costs of
   interpreters and engines are therefore visible, not hidden.

## 9. Statistics

Per cell: n, median, MAD (median of absolute deviations, unscaled), minimum, maximum.
For the ratio `gqodb / baseline` of a metric the point estimate is the ratio of medians.
The 95% interval is a **moving-block bootstrap over rounds** (rounds are paired because
all candidates are measured in the same round; block length 3; 10,000 resamples;
seed 20261017; percentile interval). The ordinary paired bootstrap over rounds is
reported as a sensitivity result. No tail percentiles are reported (n = 15).
Because candidate rounds are consecutive in time, drift is part of what the block
bootstrap captures; the study does not claim independence beyond that.

## 10. Decision rules (fixed in advance)

Practical-equivalence band per metric: bytes 2%, time 5%, RSS 10%, CPU 5%.
Let `rho` be the ratio `gqodb / baseline` (smaller is better for every metric),
`[lo, hi]` the interval, `b` the band as a fraction.

* **BETTER**: `hi < 1` and `rho <= 1 - b`.
* **WORSE**: `lo > 1` and `rho >= 1 + b`.
* **EQUIVALENT**: `lo >= 1 - b` and `hi <= 1 + b`.
* **INCONCLUSIVE**: everything else.

Stored bytes are deterministic given the data; they are compared on the point ratio
alone (the interval degenerates), with the same thresholds.

A statement of the form "GQODB wins axis A against baseline B" requires BETTER on A
on **both** D1 and D2 (replication across months), for V1. "GQODB wins every axis"
may be written only if that holds for every headline axis against every candidate
of H, where the headline axes are: stored bytes, write time, warm full read, warm
range read, peak RSS (write), peak RSS (full read). Cold-ish reads, CPU time, V2,
S1 and S2 are reported with the same rules but never used to promote a claim that
the headline axes do not support. If any headline cell is WORSE, EQUIVALENT or
INCONCLUSIVE, the README states that cell. `EXACTNESS FAIL`, `NOT COMPARABLE`
(for example a cold state that was not reached, a tool that cannot do a phase) and
`UNDERPOWERED` are first-class outcomes.

Count of cells (all reported, none selected): for the slices D1 and D2, 2 variants,
10 baseline candidates (every candidate other than `gqodb`), 6 headline axes plus the
secondary ones, against `gqodb`. The number of distinct protocols run is one; there
is no search over thresholds, windows or configurations, so no multiplicity
correction is applied. Cells are not tested against zero; they are interval
statements against the band.

## 11. What B17 does not show

* Aggregate trades only: not quotes, order books or any query other than full scan
  and one time window. The index-and-query machinery of GQODB is exercised by one
  window per dataset.
* One instrument (PAXGUSDT), two months, one host class (a 14-core Xeon E5-2690 v4,
  Linux, one filesystem), one thread, no concurrency, no network, no multi-node.
* A young prototype storage layer against mature products. Maturity-related
  properties (transactions, recovery, concurrency, SQL, ecosystem) are outside the
  measurement. GQODB verifies a CRC32 on every frame it reads; not every Parquet
  reader path does by default; that difference is part of the compared work.
* Defaults and one pre-registered alternative per tool, not an exhaustive tuning
  study. A better configuration of any baseline may exist.
* Process model differences: embedded libraries (Rust, DuckDB) against a separate
  process per statement (ClickHouse) are compared on operation time with startup
  subtracted, and on end-to-end time with startup included; neither is "the" fair
  number alone.
* No claim about live ingestion, durability against power loss (only that the
  common fsync procedure was applied), nor about other data or other hosts.

## 12. Deviation policy

Anything that differs from this document is recorded in
`docs/B17_PUBLIC_TRADES_ADDENDA.md` with a UTC timestamp and a statement of what had
already been seen at that moment. A change made after seeing a result makes the
affected comparison exploratory and it is labelled so.
