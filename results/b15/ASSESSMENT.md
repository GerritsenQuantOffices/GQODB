# B15 — Further read gains, with a small lead over dictionary-Parquet

**Keep both decoder changes.** In three alternating process pairs, the previous
reader's full-file median is **47.398–47.490 ms**;
the candidate is **39.362–39.432 ms**.
That is about **17% less read time**. Short queries improve from about 3 ms to
2.4–2.7 ms. All exact row/schema comparisons and integrity checks pass.

In those same candidate processes, dictionary-Parquet full reads take
41.678–41.790 ms. The measured lead is small (about
5–6% less time), not a universal performance claim. This is one million-row
Bybit level projection, on one host, with warm/page-cache reads.

## Implementation

- Nonnull dictionary strings validate all IDs, compute exact total output
  capacity with overflow checks, then append identical-ID runs into a preallocated
  LargeStringBuilder. Nullable strings retain the existing general path.
- Byte-plane restoration specializes widths 0 through 8 and fuses reconstruction
  and integer restoration, avoiding repeated passes over the output vector.
- Stored format, encoder, row order, integer/float precision and checksums are
  unchanged. Existing .gqodb.ob files need no conversion. No validation is skipped.

Tests cover empty strings, Unicode, long runs and transitions; all byte widths,
extreme integers, random patterns, null/float/schema preservation, aliased
value/validity columns and corrupted/truncated files remain covered.
No claim about speed on arbitrary high-cardinality text or untested schemas.

## Paired read-only evidence

Old/new process order: old0/new0, new1/old1, old2/new2. Each process retains
three separate diagnostic reads, one warmup round, then six measured rounds over
GQODB and both Parquet controls. Full reads and five queries are checked against
the original source rows. In total, 126 timed full reads and 630 query checks,
including warmups, succeed in the pairs.

Per-process medians and diagnostics: [summary](summary.json).
Raw reports: [old0](old-0.json), [new0](new-0.json), [old1](old-1.json),
[new1](new-1.json), [old2](old-2.json), [new2](new-2.json).
The [development screen](screen.json) is excluded from confirmation summaries.

Whole-symbol results vary more than short queries: new process medians range
54.4–64.0 ms, versus approximately 65.1 ms for the old version.
The absent-symbol test is essentially unchanged. Do not present a selected
whole-symbol observation as a stable speedup.

Diagnostic timings occur before normal warmup and are not interchangeable with
steady read timings. The screen records about 75 ms decode, 8.2 ms index checking
and 1.26 ms frame/metadata; normal measured reads are about 39 ms. Phase and
allocation/cache state matter; do not subtract profile components to manufacture
an end-to-end result. The two changes are tested together, not isolated factors.

## Complete current file benchmark

After the pairs passed, run the full B12 create/write/seal/sync/read/query protocol
with the final candidate: one warmup plus six alternating measured rounds, on
1,048,576 rows, 7,265 events, 32 blocks/row groups. Values below are medians;
MB is decimal. All 21 codec passes and 105 query checks succeed.

| Format | File MB | Write + file sync ms | Full read ms |
| --- | ---: | ---: | ---: |
| Gqodb | 3.704 | 198.362 | 39.972 |
| DictionaryLz4 | 5.950 | 313.211 | 41.620 |
| DeltaLz4 | 9.242 | 248.711 | 56.812 |

Against dictionary-LZ4 Parquet: 37.8% smaller, approximately 36.7% less write+sync
time and 4.0% less full-read time in this run. Against delta-LZ4: 59.9% smaller,
20.2% less write+sync time and 29.6% less read time. These three-axis medians are
positive for this fixture and these two controls only. The dictionary-read
difference is small and needs wider workloads before any general claim.

| Query | Rows | Blocks/groups | GQODB ms | Dictionary-LZ4 ms | Delta-LZ4 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| window-25-10ms | 1268 | 2 | 2.602 | 3.554 | 4.419 |
| window-50-10ms | 266 | 2 | 2.546 | 2.951 | 3.994 |
| window-75-10ms | 57 | 2 | 2.519 | 2.936 | 3.993 |
| full-symbol | 333852 | 32 | 65.446 | 124.579 | 140.023 |
| absent-symbol | 0 | 0 | 0.106 | 0.369 | 0.354 |

[All write/read/query samples](storage.json). This rerun includes earlier B13
reader/index improvements as well as B15; its write-time change versus historical
B12 must not be attributed solely to the new decoder changes.

## Unchanged data and remaining limitations

The three existing B12 read-only files retain their original SHA-256 hashes.
New writes have identical sizes and logical contents, but hashes differ across
processes: inspected headers show price_scale/quantity_scale metadata map keys
in a different order. Canonical cross-process metadata serialization is not yet
provided. Neither original source nor old files were overwritten.

Same Xeon E5-2690 v4, CPU 2, local ext4, no competing GQODB benchmarks or builds
during timing, no host tuning or cache eviction. All writes include file sync_all;
directory publication and physical power-failure guarantees remain excluded.
No cold-disk benchmark, live ingestion, bus integration, complete order-book
reconstruction or broad market validation. Parquet page pruning, column projection,
Zstd and other engine/configuration alternatives are not tested here.

See [protocol](../../docs/B15_READ_PROTOCOL.md) and [receipts](BUILD_RECEIPTS.json).
The unchanged B13 harness tags raw JSON as B13 (or B12 for the integrated run);
this B15 directory/protocol identifies this experiment. Frozen binaries establish
the compared code versions.

