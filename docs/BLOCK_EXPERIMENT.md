# B01 implementation and reproduction

This is an isolated Rust block experiment. It is not a persistent database, a live
recorder, a complete event schema, or an order-book reconstruction engine.

## Build and run

From the standalone gqodb checkout, using the committed Cargo.lock:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
taskset -c 2 target/release/gqodb-blocks --output results/b01-first --repetitions 5
```

The benchmark requires a clean committed source checkout and a new output
directory. CPU 2 is an example; select an available physical core and avoid an
occupied SMT sibling. The runner records affinity. It does not change CPU frequency,
stop services, flush OS caches or access trading data. A smoke test can specify
`--sizes 4096 --repetitions 1`; it is not a performance verdict.

The Rust parent launches five sequential child processes, rotating codec order.
Each child warms each case, then repeats encode/decode pairs for approximately
15ms based on calibration (1..128 iterations). Timings exclude generation and
equality checks. Encode allocation and Arrow construction are included; decode
allocation and materialization into the same owned integer columns are included.
Destruction of the returned result is outside its operation timer on both sides.
This contract can favor a native integer format over an application already using
Arrow: an Arrow-native comparison is necessary before broader conclusions.

Output contains raw per-process samples, medians/ranges, every candidate/baseline
comparison, a Pareto table, full dataset digests and machine/source/executable
provenance. A failed process aborts the run and leaves partial output visible.
Small blocks include disproportionate Parquet file metadata; the larger cases
show how far the effect survives amortization. Independent files per block are
not the same as a large multi-row-group Parquet dataset.

Parquet default features are disabled. Enabled compression: `lz4_flex`, `snap`
and `brotli`, all Rust. Zstd is excluded because its commonly used Rust binding
wraps the native library. `cargo tree` for the current Linux target must contain
no native compression `-sys` package or C compiler build dependency. The lockfile
can include unused dependencies for other targets; report the actual build graph.

## Experimental framing

Custom blocks contain little-endian integers and use no native struct layout:

| Offset | Field |
| --- | --- |
| 0..8 | Magic and version: ASCII `GQOBLK01` |
| 8 | Layout: 0 raw columns; 1 delta/zigzag/varint; 2 delta/zigzag/bitpack |
| 9 | Synthetic schema: 1 quote; 2 trade; 3 L2 record |
| 10..12 | u16 column count; exactly 8 for these schemas |
| 12..16 | u32 row count; maximum 1,048,576 |
| 16..80 | Eight directory entries: u32 expanded length, u32 compressed length |
| 80..end-4 | Independent raw LZ4 column payloads |
| end-4..end | CRC32 of every preceding byte |

Raw columns are i64 little-endian. Delta layouts start with one i64 first value
when nonempty, followed by wrapping adjacent differences, zigzag-mapped to u64.
Varints use base-128, at most ten bytes per value. Packed groups contain at most
128 deltas, a one-byte width 0..64 and values packed least-significant-bit first;
unused padding bits must be zero. No entropy-based lossy reduction is allowed.

The schema ID identifies the fixed fixture interpretation, not general instrument
metadata. Fixture identity is venue SYNTHETIC, instrument SYNTH-1, block session
B01, price scale 0.01 and quantity scale 0.001; epoch time is nanoseconds UTC.
Source-sequence resets in volatile fixtures intentionally do not imply a valid
exchange stream. Real adapters need richer epoch and reset metadata.

Eight i64 columns have the same physical widths for all contenders. Quote sizes,
trade sides and L2 actions could use narrower physical types in a future experiment;
this experiment does not claim to test every possible optimized schema.

Parquet files include an additional four-byte whole-file CRC for the same integrity
check. Strip that external CRC for standard Parquet tools. Parquet retains its
schema, statistics, indexes and footer, while custom blocks implement only a
schema ID and column directory. Results separately record non-column overhead;
the full Parquet column chunks include page headers, whereas custom payloads
do not. Full retained database storage is not measured.

## Interpretation

The research screen checks >=20% fewer block bytes and >=2x faster full decode.
Also inspect encode throughput, trial variation, all-Parquet failures and the
unmatched Brotli compression frontier. The README's range-query/book-replay gate
has not been tested by this kernel. Real data, partial queries, snapshots, WAL,
durability, ingestion bursts and simultaneous queries remain future work.

This is neither a Redis comparison nor proof that gqodb beats market products.
If a result loses, preserve it. Changes to datasets, codec knobs or measurement
contract require a named new protocol revision and a fresh result directory.
