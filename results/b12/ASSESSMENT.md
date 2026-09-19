# B12 — Working .gqodb.ob files; read performance needs further work

**Implemented:** separate gqodb-store crate with a versioned file container,
schema, CRC-protected blocks, a symbol/time index, append/checkpoint/seal,
exact range queries, streaming verification and recovery to a new file.
This is an independent prototype, not integration with the bus or causal replay.

**Measured verdict:** the storage saving survives file framing. GQODB is 37.8%
smaller than dictionary-LZ4 Parquet and takes 28.5% less time to write and sync.
However, full reads and the three small time-window queries are slower. Delta-LZ4
Parquet also writes faster than this GQODB implementation. No universal win.

## Complete file comparison

Same 1,048,576 Bybit BTC/ETH level rows as B11 hour06, 7,265 events,
32 blocks/row groups of 32,768 rows. Six measured rounds after one separate
warmup; medians are the average of the two middle observations.

| Format | File MB | Create/write/seal/sync ms | Open/read/decode ms |
| --- | ---: | ---: | ---: |
| Gqodb | 3.704 | 259.26 | 170.00 |
| DictionaryLz4 | 5.950 | 362.78 | 93.17 |
| DeltaLz4 | 9.242 | 245.58 | 107.39 |

The GQODB file is 3,703,770 bytes, including 11,177 bytes beyond the B11 block
payload total (about 0.30% overhead). All seven final files of each format have
identical respective hashes within this process. This does not guarantee
cross-process canonical metadata serialization.

These are **warm/page-cache reads**, after writing; no caches were evicted.
Both writers call file sync_all. Directory fsync and atomic catalog publication
are excluded. Filesystem: local ext4; CPU: Xeon E5-2690 v4, pinned CPU 2.
This is not a cold-device IO or power-failure durability result.

## Queries

Times include a new file open and index/footer load, candidate decode and exact
full-row filtering. All returned rows and schemas match filtering the original
input. Boundaries are inclusive. All three formats read the same block/group
counts in this fixture; all groups contain both BTC and ETH.

| Query | Result rows | Blocks/groups read | GQODB ms | Dictionary-LZ4 ms | Delta-LZ4 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| window-25-10ms | 1268 | 2 | 9.185 | 6.052 | 7.056 |
| window-50-10ms | 266 | 2 | 7.858 | 2.929 | 3.980 |
| window-75-10ms | 57 | 2 | 7.869 | 2.911 | 3.992 |
| full-symbol | 333852 | 32 | 146.920 | 121.774 | 139.858 |
| absent-symbol | 0 | 0 | 0.109 | 0.369 | 0.353 |

Parquet uses row-group min/max pruning for time and symbol. GQODB uses time min/max
and exact symbol sets. Neither uses page-level filtering, bloom filters or
projection pushdown here. Do not use the fast absent-symbol result to hide the
slower useful queries. Observed full-read durations vary substantially across
rounds (e.g. dictionary-Parquet about 42–93 ms); raw samples remain intact.

The benchmark retains full read/query output in memory. It includes GQODB's frame,
codec and decoded-index validation; Parquet uses its native reader validation.
Whole-process peak RSS was 503,576 KiB (about 492 MiB), including the original
input, concatenated reference and returned results; this is not a per-codec RSS.
Those integrity contracts are not identical. The cause of the read overhead has
not been profiled; B11's per-block timings cannot be substituted for these results.

## Recovery and correctness

Deterministic tests cover every byte truncation of a small two-block file, exact
complete-prefix recovery, full corruption rejection, late/out-of-order rows,
schema rejection, no-overwrite creation and CLI recovery without changing source.

Additionally, a copy of the real 32-block file was cut at byte 3,641,256,
halfway through the last block. Recovery wrote a separate sealed file containing
**31 verified blocks / 1,015,808 rows**. The interrupted source hash was unchanged.
The original sealed real file independently verified all 32 blocks / 1,048,576 rows.

A physically partial last frame can be excluded; a fully present corrupt frame
fails closed. Checksums cannot reconstruct absent or lost unsynced bytes.
Tests simulate truncation, not actual power cuts, killed writers or disk-full.
Ordinary open requires a complete footer. Recovery does not modify the original
or resume writing to it. Atomic rename/catalog handling remains future work.

## Reproduction and remaining scope

See [protocol](../../docs/B12_STORAGE_PROTOCOL.md), [crate/CLI documentation](../../crates/gqodb-store/README.md),
[raw results](storage.json), [summary](summary.json) and [receipts](BUILD_RECEIPTS.json).
Artifacts remain under artifacts/b12-storage-final and artifacts/b12-recovery;
market data files are not committed.

The first attempt at ef69b2b failed in warmup because the Parquet control did not
restore schema metadata. It contributes no final samples. Commit 8408e11 fixes
this and adds a regression test. The final run has 21 successful codec passes
(3 warmup + 18 measured) and 105 exact query comparisons.

There is no automatic live ingest, .tick writer, catalog, compaction, concurrency
contract, complete exchange-book reconstruction or causal query enforcement here.
Next performance work should profile file read/validation/Arrow allocation and
candidate filtering on this preserved baseline, before making broader speed claims.
