# B06: trades win; order-book dominance is not established

**Verdict:** gqodb beats all five tested Parquet configurations on compression
time, decompression time and storage for both aggregate-trade samples. On the
order-book level projection, dictionary-LZ4 Parquet is smaller and decodes faster.
The requirement of winning on every axis for every quant workload is not met.

## Final medians

Each dataset contains 1,048,576 rows (32 blocks of 32,768 rows). Measurements are
single-worker in-memory codec passes, one warmup plus nine alternating measured
passes per mode. Times below cover all rows; MB means decimal millions of bytes.

| Dataset | Codec | MB | Encode ms | Decode ms |
| --- | --- | ---: | ---: | ---: |
| ETH aggregate trades | gqodb | 5.089 | 78.363 | 23.666 |
| ETH aggregate trades | Parquet delta/BSS-LZ4 | 13.036 | 156.456 | 41.047 |
| ETH aggregate trades | Parquet dictionary-LZ4 | 26.574 | 432.667 | 75.222 |
| ETH aggregate trades | Parquet delta/BSS-Snappy | 13.382 | 146.439 | 43.513 |
| ETH aggregate trades | Parquet delta/BSS-Brotli5 | 8.848 | 1145.794 | 273.124 |
| ETH aggregate trades | Parquet normalized-LZ4 | 5.464 | 154.260 | 38.719 |
| UNI aggregate trades | gqodb | 4.708 | 65.478 | 21.277 |
| UNI aggregate trades | Parquet delta/BSS-LZ4 | 11.349 | 142.945 | 31.280 |
| UNI aggregate trades | Parquet dictionary-LZ4 | 33.102 | 491.874 | 82.472 |
| UNI aggregate trades | Parquet delta/BSS-Snappy | 11.140 | 144.303 | 35.951 |
| UNI aggregate trades | Parquet delta/BSS-Brotli5 | 8.671 | 923.189 | 251.562 |
| UNI aggregate trades | Parquet normalized-LZ4 | 6.189 | 143.191 | 36.763 |
| Bybit level updates | gqodb | 7.350 | 206.429 | 42.493 |
| Bybit level updates | Parquet delta/BSS-LZ4 | 9.288 | 233.747 | 52.123 |
| Bybit level updates | Parquet dictionary-LZ4 | 5.995 | 300.469 | 34.699 |
| Bybit level updates | Parquet delta/BSS-Snappy | 9.407 | 222.083 | 51.856 |
| Bybit level updates | Parquet delta/BSS-Brotli5 | 7.412 | 1191.393 | 363.403 |
| Bybit level updates | Parquet normalized-LZ4 | 9.329 | 263.520 | 61.481 |

Against normalized-LZ4, ETH encoding is 1.97x and decoding 1.64x as fast, with
6.9% fewer bytes. UNI is 2.19x and 1.73x as fast, with 23.9% fewer bytes.
These ratios use that particular baseline; the complete table above also retains
faster individual Parquet metrics (e.g. UNI delta/BSS-LZ4 decoding).

For book updates, against dictionary-LZ4 gqodb encoding is 1.46x as fast, but
decode time is 22.5% longer and storage is 22.6% larger. Selecting only normalized
Parquet would hide this counterexample. gqodb still wins all three metrics
against the other four tested book configurations.

## Data and correctness

- ETHUSDT May 2021: row group 46 from a 96,451,646-row Binance aggregate-trade
  source. One selected million-row sample, not the entire source benchmarked.
- UNIUSDT May 2022: row group 1, same sample size. Original fields and schema
  retained for both trade datasets.
- Bybit BTC/ETH, closed hour 2026-09-03 06 UTC: 7,265 source events projected into
  1,048,576 order-book level updates. Zero empty events in the consumed prefix.
  Projection and excluded collector information are documented in the protocol.
- Every codec roundtrip matched its Arrow input, including schema metadata,
  nulls, integer values and exact floating-point bits. Source/payload checksums
  passed for the book data. No original market files were rewritten.

## Resources and reproducibility

Local research host, single pinned logical CPU 2, Rust-only implementation and
codecs. The datahoarder was checked read-only but not used for computation.
Observed process peak RSS was roughly 85–140 MiB; book scratch capacity was
454,528 bytes. Scratch is separate from resident benchmark input and codec outputs.

Final code commit: `87b7b7b`. All-target Rust tests, Clippy with warnings denied,
formatting, and release example builds passed before final measurements.

Final JSON with all samples, parameters, hashes, medians, observed p95 and memory:
[ETH](eth-final.json), [UNI](uni-final.json), [Bybit](bybit-levels.json).
Pre-fix trade results remain in [ETH preliminary](eth-group46.json) and
[UNI preliminary](uni-group1.json). The failed book attempts are described in the
[protocol](../../docs/MARKET_BENCHMARK.md); no failed run contributes timings.

## Decision boundary

This is useful evidence for the specialized trade codec. It is not evidence of
universal database superiority. It excludes disk IO, durability, queries, complete
order-book replay and live ingestion. Only three fixed samples on one shared host
were tested. Parquet Zstd encoding was not benchmarked; page/index features also
differ between the formats.

For further order-book work, dictionary-based numeric encoding is a motivated
candidate to test because dictionary-LZ4 is the counterexample here. Its causal
benefit inside gqodb is not yet measured. Preserve this dataset and the winning
Parquet control as a regression benchmark instead of weakening the comparison.
