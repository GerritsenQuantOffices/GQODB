# B13 — Reader improvement with unchanged files and validation

**Keep the optimization.** Across three alternating process pairs, full-file
read medians improve from **115.078–165.926 ms to 47.497–47.685 ms**.
The paired observed speedup is approximately **2.4–3.5x**. The three short
time-window queries improve from 7.49–8.86 ms to 2.96–3.21 ms.
These are warm/page-cache results on the single B12 million-row book fixture,
not cold IO or a universal speed guarantee.

The file remains **3,703,770 bytes**, with exactly the same SHA-256.
Every schema, row, float/integer bit, timestamp and index/CRC validation remains.
There is no new encoding and no data migration.

## Change and profiling evidence

The original index reconstruction bulk-collected all symbols into a BTreeSet.
Instead we inspect every row but only insert at a symbol-run transition.
All distinct symbols are still collected and sorted. Decoding now transfers an
owned nonnull Int64 vector into Arrow on its last metadata use, instead of cloning.
Reference counts preserve layouts that alias value or validity columns; a new
regression test exercises that case. This applies to both gqodb real codecs,
but encoding is unchanged.

Initial diagnostics: about 69.3 ms decoded-index checking, 92.6–93.4 ms decoding
and 1.28 ms frame/metadata reading. Candidate diagnostics in the first process:
about 8.64 ms index checking, 87.3–87.4 ms decoding and 1.28 ms frame reading.
The diagnostic calls are separate from normal timed reads, retain all output,
and occur before the benchmark warmup.

Do not add/subtract those component timings as if they explained the entire
steady warm-read improvement. Full-read times change across process phases;
the unchanged Parquet control also varies (roughly 42–93 ms in old processes).
Memory/allocation state is a possible explanation, not established by this
coarse profile. We measured the two optimizations together, not isolated factors.

## Current Parquet comparison

Ranges of per-process medians in the three candidate processes:

| Reader | Full-file read ms | File bytes |
| --- | ---: | ---: |
| New GQODB | 47.497–47.685 | 3,703,770 |
| Dictionary-LZ4 Parquet | 41.524–41.778 | 5,950,242 |
| Delta-LZ4 Parquet | 56.265–56.658 | 9,242,314 |

GQODB full-file reads still take about 14–15% more time than dictionary-Parquet.
GQODB is smaller and faster than delta-Parquet
in these reads. No claim that GQODB wins every metric.

## Queries

Each timing includes fresh open, footer/index read, candidate decode and exact
full-row filtering. Every result is checked against original source rows.
Both formats prune the same number of blocks/groups here.

| Query | Rows | Blocks/groups | Old GQODB ms | New GQODB ms | Dictionary-LZ4 ms | Delta-LZ4 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| window-25-10ms | 1268 | 2 | 8.386–8.858 | 3.186–3.205 | 3.228–3.232 | 4.156–5.355 |
| window-50-10ms | 266 | 2 | 7.489–7.506 | 2.988–2.994 | 2.909–2.924 | 3.932–3.945 |
| window-75-10ms | 57 | 2 | 7.494–7.515 | 2.959–2.998 | 2.895–2.898 | 3.932–3.963 |
| full-symbol | 333852 | 32 | 140.981–141.542 | 65.256–65.611 | 114.130–115.700 | 128.669–134.133 |
| absent-symbol | 0 | 0 | 0.110–0.111 | 0.109–0.110 | 0.368–0.373 | 0.348–0.354 |

Short queries are now close to dictionary-Parquet: one slightly faster, two
slightly slower; small differences are descriptive. The whole-symbol query is
faster with the candidate. The absent-symbol test does not substitute for useful
queries. Page-level pruning and column projection are not benchmarked.

## Method and correctness

Frozen baseline binary at 8f5c6ea; candidate implementation at 8c3bedc.
Process sequence: old0/new0, new1/old1, old2/new2. Each process has three separate
diagnostic reads and then B12's one warmup plus six measured rounds over three
codecs. Normal reads compile without diagnostic clock calls. Total: 126 timed
full reads and 630 query comparisons including warmups, plus 18 diagnostic reads.
All succeeded. Initial profile-before.json is separate development evidence.

CPU 2, same existing B12 round-6 files, same 1,048,576 rows and source hashes.
No concurrent GQODB builds or bus benchmarks. Files were opened read-only;
post-run hashes match B12. Write timings are deliberately zero because this
experiment performs no dataset writes. Warmup observations are retained but
excluded from summaries. Ranges report independent-process medians, not a
statistical confidence interval.

The full workspace/all-target tests and Clippy with warnings denied passed after
both changes, including exact codec roundtrips, shared value/validity restoration,
byte-by-byte truncation recovery and corrupt-file rejection.

See [protocol](../../docs/B13_READ_PROTOCOL.md), [per-process summaries](summary.json)
and [build/data receipts](BUILD_RECEIPTS.json). Raw runs:
[old0](old-0.json), [new0](new-0.json), [old1](old-1.json),
[new1](new-1.json), [old2](old-2.json), [new2](new-2.json).
