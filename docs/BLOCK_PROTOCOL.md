# B01: Rust-only block feasibility

Status: protocol frozen before implementation measurements, 2026-09-07.

This experiment tests isolated storage blocks, not a database or live bus. All
generators, encoders, decoders, timing, integrity checks and report generation are
Rust. No Python runner and no C/C++ compression backend. OS utilities are used for
building, CPU affinity and source provenance. Public data acquisition is unnecessary.

## Frozen experiment

- Three synthetic fixed-schema families: quotes, public trades, L2 update records.
- Two regimes: regular/quiet and irregular/volatile. Instrument, venue, session and
  decimal scales are constant per block and identified in the schema metadata.
- Columns retain exchange and receive nanoseconds, ingest/source sequence, price,
  quantities and event-specific side/action/ID fields. Same integers and order in
  every contender; collisions and late timestamps are retained.
- Row counts: 4,096; 65,536; 262,144. Five independent measured repetitions, one
  untimed warmup for each case. Rotate contender order between repetitions. Each
  repetition runs in a fresh process, with calibrated 1..128 encode/decode pairs
  per result targeting approximately 15ms. Retain the iteration count.
- Use per-case deterministic seeds, record full raw integer SHA-256 and verify
  exact equality after every timed decode, outside the timed region.
- Candidates: wrapping delta + zigzag + varint + LZ4, and wrapping delta + zigzag
  + 128-value fixed-bit-width groups + LZ4. Wrapping arithmetic preserves all i64
  bit patterns, including extrema. No lossy quantization or timestamp deduplication.
- Controls: raw little-endian columns + LZ4; Parquet plain + LZ4; Parquet delta +
  LZ4; Parquet dictionary with delta fallback + LZ4; Parquet delta + Snappy;
  Parquet delta + Brotli level 5. Use current pinned Arrow/Parquet 59.3.0.
- Parquet uses one row group per tested block, data pages up to 64K rows / 1 MiB,
  statistics and the same integer columns. Complete file size includes the footer.
- Custom formats include schema ID, dimensions, a per-column directory, lengths,
  and a whole-block CRC. Parquet receives the same four-byte external whole-file
  CRC, stripped before standard parsing. Report non-column framing overhead
  separately from column chunks (which include Parquet page headers).
  The test does not represent full catalogs, WAL retention or order-book checkpoints.
- Input/output contract is owned integer columns for all implementations. Include
  Arrow conversion in end-to-end encode time and column materialization in decode
  time. Allocation is included. Payload validation and dataset generation are not.
- Record encode/decode nanoseconds, rows/s, raw MiB/s, total bytes and bytes/row;
  retain every repetition, medians and min/max. No p99 claim from five repetitions.
- In-memory warm blocks only. No filesystem throughput, durability, SQL, predicate
  pushdown, partial reads, causal book replay or end-to-end engine claims.

## Decision

Report results against each baseline and identify the Parquet Pareto frontier.
Highlight whether either candidate is both >=20% smaller and >=2x faster at full
decode than each Parquet contender; include encode regressions. This is a block
research screen, not the README's database range-query/book-reconstruction gate.
LZ4-matched comparisons isolate format effects. Snappy/Brotli broaden codec choice.
Zstd is excluded because the usual Rust wrapper calls C; this missing important
baseline prevents any claim to outperform the entire Parquet market.

Synthetic L2 records are compression fixtures, not a complete valid exchange book
or evidence about production book reconstruction. No access to closed OOS datasets.
Useful signal justifies the next bounded experiment; a negative result is retained.

## Validation and provenance

Tests cover all formats, empty blocks, repeated timestamps, extrema, malformed
headers, truncation, corruption, invalid group widths, oversized allocations and
deterministic generator output. Run formatting, clippy and workspace tests before
release measurements. Commit source/config/tests together; build release with
`--locked` from that clean local commit. Record source SHA, lock SHA, executable
SHA, toolchain, CPU/OS, affinity and invocation with each result. No remote push or
publication is required for this local research run.
