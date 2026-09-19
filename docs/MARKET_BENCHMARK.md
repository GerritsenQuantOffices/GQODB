# B06: larger real market-data codec comparison

## Reproduce

Build `cargo build --release --locked --example market_benchmark`.

For Zstd-compressed Parquet inputs, add `-p gqodb-blocks --features zstd-input`.

```text
market_benchmark trades SOURCE.parquet OUTPUT.json ROWS BATCH_ROWS REPS ROW_GROUP START_BATCH
market_benchmark book SOURCE.jsonl.gz OUTPUT.json ROWS BATCH_ROWS REPS 0 0
```

Recorded runs use 1,048,576 requested rows, 32,768-row blocks, nine measured
repetitions and one warmup pass for each mode. Trade sources select one explicit
Parquet row group and keep every original column. Book sources stop at a complete
event boundary and may therefore exceed the requested level count slightly.
The process refuses to overwrite an existing output.

Mode order rotates/reverses between repetitions. Every individual pass is saved;
summaries use the sorted middle sample (upper median for even counts). Reported
p95 is nearest rank; with nine samples it is simply the maximum, not a reliable
tail-latency estimate. All exact-roundtrip checks must pass before output is saved.

## Workloads and information preservation

Trade data are Binance **aggregate trades**, not every constituent execution or
quote tick. All source fields, nullable Int8, timestamp timezone, schema metadata
and bitwise floating-point values survive the codec roundtrip.

The book workload is a projection of Bybit BTC/ETH full-depth messages into level
rows: source event index, receipt time, exchange time, cts, update ID, sequence,
side, position within the event, price, quantity, snapshot/delta flag and symbol.
Prices and quantities use exact integer units of 1e-8; unsupported precision or
overflow fails instead of rounding. Gzip file SHA-256 is checked against the
collector sidecar; raw payload hashes are also checked for every consumed event.

The collector envelope, JSON formatting and lexical decimal representation are
excluded from this projection. Empty events produce no level rows and are counted
explicitly. Full message storage, gaps, book reconstruction and live ingest
correctness are outside this experiment. All codecs receive the same projection.

## What timing and memory mean

Single worker, CPU 2 on the local research node. Each block is encoded and decoded
in RAM. Timings include codec allocation, checksums, normalization and Arrow
restoration. Source reading/hashing, JSON projection and roundtrip checking are
outside timing. The gqodb decoder reuses scratch. All decoded outputs have the
same schema and values; no columns are skipped during decoding.

Resident Arrow input is reported and must be at most 256 MiB before timing starts.
The input is retained for repeated comparison; outputs are processed one block at
a time. `/proc/self/status` reports whole-process peak RSS separately from scratch
capacity. This is not a disk, durability, cache-hit or query benchmark.

## Parquet controls and limitations

Controls are original-schema delta/BYTE_STREAM_SPLIT-LZ4, dictionary-LZ4,
delta/BSS-Snappy, delta/BSS-Brotli5, and normalized-integer LZ4. Each uses the
same independent block boundaries and exact source values. The encoder is
Parquet 59.3.0, Parquet V2, page statistics enabled. gqodb has no comparable
query statistics/index; these are block formats with different query features.

The historical runs read original Zstd Parquet files through a Rust-only adapter
outside timing. The public runner instead uses the optional `zstd-input` feature,
which links the native Zstd library to decode those inputs. The default build
does not include it. **Zstd encoding is not a controlled baseline here**.
Results cannot be generalized to every Parquet codec/configuration.
Gzip ingestion uses flate2's Rust backend (miniz_oxide), not a C library.

## Host and source version

Research host: Xeon E5-2690 v4, 14 cores / 28 logical CPUs, 60 GiB RAM;
Linux 7.0.0-30-generic x86_64; rustc 1.96.0, LLVM 22.1.2. Tests and builds completed
before the final benchmark sequence; candidates did not run concurrently.
The host is shared, not CPU-isolated. Small dispersion within a run is not proof
of generalization across hosts or market regimes.

Final source commit: `87b7b7b`. An initial run used `a5aa806`. The first order-book
attempt failed schema equality because our Parquet adapter omitted custom scale
metadata, although every value matched. The adapter was fixed and regression
tested, then all final workloads were rerun. Failed runs produced no successful
result JSON and no timing from them is used. Preliminary trade JSON is retained
alongside the final runs for auditability.

The datahoarder was checked read-only: ample free RAM but no Cargo/rustc in PATH.
Its collectors were left running and no toolchain or benchmark was installed
there. Local copies were sufficient. No data purchases or source-file mutations.
