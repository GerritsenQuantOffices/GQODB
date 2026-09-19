# B01 assessment: write gains, no combined storage/read win

Date: 2026-09-07. Source commit: `4ee163c71d8f4ce3f568ebee7c22d98edc411ffa`.
Decision: **do not build a complete custom database on the strength of B01.**

## What was actually tested

The entire experiment is Rust, including generation, compression, decoding,
correctness checks, worker orchestration, statistics and report generation. Rust
Parquet/Arrow 59.3.0 is compared with two small custom delta layouts and a raw
column/LZ4 control. Compression backends are Rust LZ4, Snappy and Brotli.

There are 18 cases: quotes, trades and L2 update records; quiet and volatile
regimes; 4,096, 65,536 and 262,144 rows. Eight formats and five fresh worker
processes yield **720 case/format/process measurements**. Each measurement uses
1..128 calibrated iterations and verifies exact equality outside the timers.
All measured round trips succeeded. Timings include allocations and conversion
to/from the common owned integer-column representation.

The release executable was built with `cargo build --release --locked` from the
clean source commit above. Measurements were pinned to CPU 2 on an Intel Xeon
E5-2690 v4. Exact CPU, kernel, cgroup limits, affinity, invocation, compiler, source,
lockfile, executable and dataset hashes are recorded in [results.json](results.json).
Cgroup limit files were unavailable in this execution environment and are recorded
as null; host-visible core/RAM counts are not proof of an effective resource quota.
This was a shared research host, not a controlled dedicated performance laboratory.

## Findings

1. **Writing is faster.** For the 262,144-row blocks, the custom bitpack and varint
   variants encode approximately 2.2–2.9x faster than Parquet delta/LZ4 across the
   six dataset/regime cases. This is the end-to-end owned-column-to-block contract,
   including Parquet's Arrow construction, metadata and statistics. It is not an
   isolated entropy-codec speedup or durable-ingestion result.
2. **Reading is usually slower.** At that block size, bitpack achieves only about
   0.42–0.65x Parquet delta/LZ4 decode throughput. Varint achieves about 0.69–1.16x,
   depending on the case. The scalar packed decoder is not competitive here.
3. **The best custom sizes mostly match Parquet rather than beat it.** Large quiet
   bitpack blocks are only about 1.2–1.4% smaller than Parquet delta/LZ4; volatile
   blocks are about 0.8–1.1% larger. Varint large blocks are about 5–22% larger.
4. **Small-block framing can mislead.** At 4,096 quiet trade rows, custom bitpack
   is about 23% smaller than delta/LZ4 Parquet, but decodes at only 0.65x its rate.
   By 262,144 rows the size saving is about 1.4%. The small-file footer advantage
   does not establish a superior compression algorithm.
5. **A stronger compression profile matters.** For large quiet trades, Parquet
   delta/Brotli occupies 1.191 bytes/row versus custom bitpack's 1.892. Brotli's
   encode/decode rates are lower, so this is a tradeoff rather than a universal
   winner. The full Parquet size/decode Pareto table is retained in the report.

### Concrete example: quiet trade records, 262,144 rows

Rates below are the medians of the five process measurements. One record contains
eight exact i64 fields (64 raw bytes). Sizes include complete block framing and CRC.

| Format | Bytes/record | Encode million records/s | Decode million records/s |
| --- | ---: | ---: | ---: |
| Custom delta/bitpack/LZ4 | 1.892 | 22.36 | 18.38 |
| Custom delta/varint/LZ4 | 2.075 | 24.28 | 31.06 |
| Parquet delta/LZ4 | 1.919 | 8.89 | 39.77 |
| Parquet delta/Snappy | 2.010 | 8.94 | 39.71 |
| Parquet delta/Brotli level 5 | 1.191 | 3.67 | 12.97 |

The raw-column/LZ4 and Parquet plain/dictionary controls are also included in
[REPORT.md](REPORT.md). They must not replace the stronger delta controls when
claiming a speed/compression advantage.

## Gate outcome

The proposed block screen requires **>=20% fewer bytes and >=2x faster full
decode simultaneously**. This is an experimental proxy, not the database
range-query/order-book-reconstruction gate in the architecture.

- Against Parquet delta/LZ4: **0/36** candidate/case pairs pass.
- Against every tested Parquet variant: **0/36** pass.
- Exact measured round trips: **720/720 measurement groups pass**, including each
  group's internal timed decode iterations.
- Easier operation, real feed capture, recovery and database usability: **not tested**.

The compression outcome alone rules out the combined gate in almost every case;
measurement noise cannot reasonably turn ~1% large-block savings into 20% savings.
Some timing ranges are broad, particularly large quote blocks and some Parquet
reads. Use raw trials and min/max ranges before interpreting small speed differences.
Five repetitions do not support tail-latency or strong statistical significance claims.

## What this does and does not justify

**Retain the faster write-path observation as a research lead.** If a later live
recorder proves CPU-bound in block encoding, a specialized transient format could
be valuable while using standard analytical storage afterward. B01 has not tested
that system, and conversion/retention overhead could erase the benefit.

**Do not promote either layout to the default historical engine.** Its combination
of size and read speed is not compelling against tested Parquet alternatives.
This is not a failure of all possible gqodb designs; it is a negative result for
these initial two layouts and this full-block contract.

The most specific next experiment, if pursued, is a separately preregistered
decoder/block-access comparison: established Rust packing kernels, an Arrow-native
output contract, and selective column/range reads with the same block boundaries.
That would test the observed scalar decode and materialization costs, rather than
starting an entire database or adding features to hide the result. Only profile-
supported changes should trigger a new tuning round.

Important limits:

- Synthetic data only; L2 records are not a valid reconstructed exchange book.
- In-memory warm blocks, not disk, journals, sync acknowledgements or streaming.
- Whole blocks and all eight i64 columns; no narrow types, partial reads or SQL.
- Small independent Parquet files, not a catalog of multi-row-group files.
- No Zstd baseline: the ordinary Rust wrapper uses C. No claim to beat all Parquet
  implementations or all market products follows from this Rust-only baseline set.
- No Redis/iceoryx2 transport benchmark and no real connector was run.
- All custom block headers and CRCs are counted, but a database's catalog, WAL,
  snapshots, retained source records and compaction space are not present.

## Validation

Passed before release measurements:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Seven test functions cover schema/regime/group boundaries, empty and constant
blocks, repeated timestamps, full-width random integers, extrema, ragged columns,
every byte truncation/corruption of small blocks, invalid dimensions/lengths and
malformed transforms. The current Linux build graph has no native compression
`-sys` library or `cc` build dependency. This is correctness screening, not a
completed security/fuzz audit or production release validation.
