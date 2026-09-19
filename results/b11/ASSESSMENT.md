# B11 — Smaller order-book blocks; Parquet still decodes faster

The optional book codec reduces storage **48.9–49.8% versus the old gqodb
codec** on two consecutive Bybit samples. Encoding takes 13.2–13.4% less time;
decoding takes 2.9–7.7% less time. The small decode difference on hour07 is
descriptive, not an established statistical speedup.

Against dictionary-LZ4 Parquet, storage is **38.4–42.9% smaller** and encoding
takes **40.1–40.6% less time**, but decoding takes **14.3–14.5% longer**.
Compression improved on these samples; winning on every axis is not achieved.

## Final comparison

Single worker pinned to CPU 2, in-memory Arrow-to-bytes and bytes-to-Arrow,
one warmup and nine alternating measured passes per mode. MB is decimal.
Schema, framing, checksums, normalization, restoration and output allocation
are included. Source parsing and full exact equality assertions are outside
timers. Equality is checked after every decode; every measured pass succeeded.

| Sample | Codec | MB | Encode median ms | Decode median ms |
| --- | --- | ---: | ---: | ---: |
| hour06 | gqodb-adaptive-lz4 | 7.350 | 206.842 | 43.348 |
| hour06 | parquet-delta-bss-lz4 | 9.288 | 231.203 | 51.475 |
| hour06 | parquet-dictionary-lz4 | 5.995 | 298.941 | 34.941 |
| hour06 | parquet-delta-bss-snappy | 9.407 | 220.560 | 51.469 |
| hour06 | parquet-delta-bss-brotli5 | 7.412 | 1187.160 | 361.879 |
| hour06 | parquet-normalized-lz4 | 9.329 | 263.755 | 61.499 |
| hour06 | gqodb-book-runs-planes-lz4 | 3.693 | 179.077 | 40.016 |
| hour07 | gqodb-adaptive-lz4 | 6.901 | 204.962 | 40.480 |
| hour07 | parquet-delta-bss-lz4 | 8.558 | 226.696 | 48.778 |
| hour07 | parquet-dictionary-lz4 | 6.183 | 299.867 | 34.372 |
| hour07 | parquet-delta-bss-snappy | 8.680 | 216.490 | 49.026 |
| hour07 | parquet-delta-bss-brotli5 | 6.885 | 1107.458 | 349.255 |
| hour07 | parquet-normalized-lz4 | 8.634 | 259.776 | 59.880 |
| hour07 | gqodb-book-runs-planes-lz4 | 3.529 | 178.008 | 39.289 |

All samples, observed p95s and resource counters:
[hour06](hour06.json), [hour07](hour07.json).
Order rotates and reverses; nine repetitions do not completely balance all
seven mode positions. This is a local comparison, not a universal guarantee.

## Change and development screen

Adjacent repeated values become value/run-length pairs when runs number at most
a quarter of rows. Values use an exact minimum/GCD transform and byte planes
before LZ4. Compressed or raw planes are selected by actual length. No sorting,
quantization or removal. Decoding restores original rows, timestamps and IDs.

The first [screen](screen.json) already produced 3.693 MB but needed 64.746 ms
to decode. Contiguous plane loops replaced per-row variable-width loops before
final measurements; output size stayed identical. The screen is not independent
final validation. Its pre-optimization source was not separately committed;
its binary hash is recorded in the receipts.

## Data and limitations

- Hour06: exact B06 source prefix, 1,048,576 level rows, 7,265 events, 32 blocks.
- Hour07: chosen before results, 1,048,603 rows, 4,493 events, 33 blocks.
  The last 27 rows preserve the event boundary; normal block size is 32,768.
- Both: Bybit BTC/ETH level projections from 2026-09-03. Zero empty events
  excluded in either prefix. All projected fields, symbols, precision and
  Arrow schema metadata survive exactly.
- Source SHA sidecars and per-event payload hashes verified. Original files
  read only. Collector envelopes and JSON lexical form are outside the shared
  projection for all codecs, as in B06.
- No claim about L10 snapshot arrays, full book reconstruction, file IO, live
  compression, crash recovery or queries. The .ob segment format is not built.
  Parquet Zstd encoding is not tested.
- Whole-process peak RSS about 139 MiB; all codecs share this process.
  New decoder scratch retains 128 KiB versus 444 KiB for the old adaptive codec.
  Scratch is not total codec memory, and RSS is not a per-mode measurement.

## Usage and decision

Select `real::Mode::GqodbBook` explicitly; `Mode::Gqodb` preserves the old
codec. `Mode::ALL` retains the six historical controls.
`GQODB_BOOK_CODEC=1` adds the candidate in market_benchmark.
New magic `GQOREAL4` / `GQOBOOK1` separates it from the old format.

Keep it as an option for storage-focused book workloads, with dictionary-Parquet
as the faster decode control. No automatic default promotion or database claim.
See [protocol](../../docs/B11_BOOK_COMPRESSION.md) and [receipts](BUILD_RECEIPTS.json).

