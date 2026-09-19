# B02: simultaneous write/read/size gains on many synthetic block cases

Date: 2026-09-07. Source: `2a9013d809e391700711e16b8c3198d5230ee688`.

**Decision: GO for further bounded validation of the adaptive representation.**
B02 demonstrates that B01's negative combined outcome was not a hard limit: many
tested cases now improve size, encoding throughput and full decoding throughput
simultaneously against the tested Parquet alternatives. This does not authorize a
production engine or establish superiority over the entire database market.

## What changed

- Replaced per-byte scalar unpacking with the safe API of the Rust BitPacker4x
  implementation, which selects Rust SIMD or scalar kernels as appropriate.
- Chose a predictor per column: absolute values, adjacent differences, a linear
  step, or a residual against an already encoded column.
- Stored a full-column residual minimum and exact GCD scale, then packed the
  reduced unsigned integers. No precision loss or event/column deletion.
- Added optional LZ4 only when it reduces payload size. A packed-only control
  isolates the effects of that extra compression layer.

Selection inspects at most 1,024 rows, but the complete input determines stored
residuals and parameters. A broken sampled pattern changes compression efficiency,
not information preservation. Explicit u64 fallback handles full-range values.
All original columns, including redundant ones, are reconstructed exactly.

Source, framing, tests and methodology: [B02 protocol](../../docs/B02_PROTOCOL.md),
[format and reproduction](../../docs/B02_FORMAT.md). B01 files remain unchanged.

## Confirmation design

Two separate runs use the same 18 schema/regime/size cases and all eight B01 codecs
plus two new candidates. Each run has five new sequential worker processes pinned
to CPU 2. There are **900 measurement groups per run, 1,800 total**. Every timed
decode is checked for exact equality outside its timer; all groups passed.

The seen set retains B01 seeds. The fresh set adds 1,000,003 to those seeds.
Both were specified before B02 timings and no codec tuning occurred between runs.
Fresh seeds are validation within the same synthetic generator family, not
real-exchange or broad distributional validation.

Sizes include complete block framing and CRC. Encode includes predictor selection,
parameter discovery, allocation and format construction. Decode includes checksum,
decompression, predictor reconstruction and materialization into owned i64 columns.
Both runs use the same frozen source and release executable. Full source/lock/
protocol/executable/data hashes, host metadata and per-process observations are
in [seen/results.json](seen/results.json) and [fresh/results.json](fresh/results.json).

## Three-axis results

Counts are candidate/case pairs, with two new candidates across 18 cases. A strict
win means fewer bytes, lower median encode time and lower median decode time.
These are descriptive median comparisons, not confidence-qualified claims.

| Criterion | Seen seeds | Fresh seeds |
| --- | ---: | ---: |
| Three-axis win against Parquet delta/LZ4 | 32/36 | 32/36 |
| Three-axis win against every tested Parquet variant | 27/36 | 27/36 |
| >=20% smaller, >=1.2x encode and >=1.2x decode vs delta/LZ4 | 18/36 | 17/36 |
| Same practical gate against every tested Parquet variant | 14/36 | 13/36 |
| >=20% smaller, >=2x decode, no encode regression vs delta/LZ4 | 11/36 | 11/36 |
| Same ambitious gate against every tested Parquet variant | 3/36 | 7/36 |

Equal aggregate counts do not mean identical cases win in both runs. The all-
Parquet set changes for some quiet L2 and quote cases. Threshold crossings around
2x are sensitive to baseline timing variation; the 3-to-7 count change should
not be read as an improvement made between runs. No implementation changed.

## Concrete fresh-seed example: quiet trades, 262,144 rows

| Format | Bytes/record | Encode million records/s | Decode million records/s |
| --- | ---: | ---: | ---: |
| B02 adaptive SIMD + LZ4 | **0.776** | **12.65** | **72.79** |
| B02 adaptive SIMD, packed only | 0.939 | 12.69 | 72.82 |
| B01 delta/varint + LZ4 | 2.074 | 23.85 | 31.02 |
| Parquet delta/LZ4 | 1.918 | 8.75 | 34.26 |
| Parquet delta/Snappy | 1.978 | 8.77 | 36.69 |
| Parquet delta/Brotli level 5 | 1.192 | 3.64 | 12.81 |

For this example, adaptive/LZ4 is approximately **59.5% smaller, 1.45x faster to
encode and 2.12x faster to decode than delta/LZ4 Parquet**. It is also smaller,
faster to encode and faster to decode than all five Parquet variants in this
particular case. The fastest tested Parquet decode here is Snappy, against which
the adaptive decoder is about 1.98x faster rather than 2.12x.

The fresh volatile-quote case at the same size is another useful example:
adaptive/LZ4 uses 10.675 versus 13.878 bytes/record and reaches 13.12 versus 7.75
million encoded records/s, and 67.72 versus 23.86 million decoded records/s,
relative to delta/LZ4 Parquet. That is about **23.1% smaller, 1.69x encode and
2.84x decode** under this measurement contract.

These are examples, not global average speedups. Every case and control, including
losing cases, is in the [seen report](seen/REPORT.md) and [fresh report](fresh/REPORT.md).

## Tradeoffs and remaining failures

1. **The old custom writer can still be faster.** In the quiet-trade example B01
   varint encodes 23.85 million records/s versus B02's 12.65. B02 trades some of
   B01's write-only advantage for far better size/read performance while staying
   faster than Parquet on all three axes in that example. Do not claim that every
   metric improved relative to the fastest previous custom variant.
2. **Small quiet trade and L2 writes can lose.** At 4,096 rows the adaptive predictor
   search overhead is not fully amortized. These cases improve size/read but do
   not beat matched Parquet write throughput. This is not a universal three-axis win.
3. **Volatile L2/trade compression gains are modest.** Some large blocks save only
   around 4% relative to delta/LZ4 Parquet, despite stronger read/write throughput.
   They do not pass the 20% size-reduction screen.
4. **Packed-only and LZ4 are not universally interchangeable.** Large quiet quotes
   show marked decode variation, and the LZ4 version does not beat every Parquet
   read path in the fresh run. An adaptive codec policy selected after examining
   results would require a new preregistered confirmation experiment.
5. **Host and memory effects remain unresolved.** Some decode timings are bimodal
   or broad even at fixed affinity. In particular, first-set large quiet quote
   reads vary by roughly a factor of three. Allocator/cache/page behaviour and
   shared-host scheduling need profiling before declaring stable latency guarantees.

## Why this is not yet a market-wide result

The engine benefits from exactly reversible relationships in these synthetic
columns. Real streams can have different precision, irregular IDs, missing fields,
session changes, corrections and weaker cross-column relationships. Fresh seeds
retain those generator assumptions. They do not prove real-data compression.

The baseline is five configurations of the Rust Parquet 59.3.0 implementation.
Zstd, Pcodec, ClickHouse, QuestDB, kdb+, Arrow-native query paths, narrow physical
integer schemas and equivalent predictor preprocessing ahead of Parquet were not
benchmarked. Gains may come substantially from representation/preprocessing rather
than superiority of a new generic compression algorithm.

The same owned-column contract is used on both sides, including Arrow conversion
for Parquet. No disk, durable ingestion, indexes/catalogs, snapshots, selective
queries, order-book replay, IPC or ease-of-use benefit is established by B02.

## Next evidence required

The justified next work is bounded: profile the unstable decode cases and sample
selection cost; compare an Arrow-native output contract; test the same reversible
predictors with Parquet as a control; then use permitted representative market
data and selective reads. No need to build a complete database to answer those
questions. Stronger reference codecs should enter before a market-superiority claim.

## Validation and implementation status

All ten correctness test functions passed, including every prior B01 codec plus
new predictor/reference/dimension checks, malformed SIMD widths and a deliberate
pattern failure after the sampling prefix. Formatting and strict Clippy passed.
The Linux dependency graph has no native compression `-sys` library or `cc` build
dependency. gqodb source forbids unsafe code; the packing dependency internally
uses Rust SIMD where available. All benchmark orchestration/report generation is
Rust. No production service, account, source collector or proprietary data changed.
