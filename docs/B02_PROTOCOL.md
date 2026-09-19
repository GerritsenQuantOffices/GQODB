# B02: adaptive predictors and SIMD integer blocks

Frozen before B02 measurements. B01 files and codec implementations remain intact.
Motivation: B01 scalar unpacking is slow and uniform delta encoding wastes bits on
small absolute quantities/sides and cross-column redundancy. This is a new block
experiment, not a promise to dominate databases or a full system implementation.

Two new candidates use the same adaptive representation: packed blocks alone, or
packed blocks with LZ4 only when it saves bytes. Pure-Rust bitpacking 0.9.3
BitPacker4x provides SIMD packing/unpacking through its safe public API; Rust-only
scalar fallback is available in that library. No C/C++ compression backend.

Per column choose among absolute, adjacent-delta, linear-step and already encoded
column-reference predictors using at most the first 1,024 rows. Score the residual
range after lossless integer GCD scaling; penalize delta's serial reconstruction
by one estimated bit. The selected predictor is applied to **every actual value**;
it is never assumed correct because a sample fits. Full-column minimum and GCD
are stored. Signed differences and linear/reference reconstruction use wrapping
arithmetic so every i64 bit pattern remains representable.

Pack groups of 128 u32 residual quotients with BitPacker4x. Quotients exceeding
u32 use an explicit full-u64 group. Zero-padding, lengths and references are
validated. No fitted constants, hardcoded prices, discarded columns, timestamp
deduplication, precision loss or synthetic-seed identifiers enter the codec.
Physical column order is receive-time then exchange-time, then the remaining
schema order; this allows exact clock residuals and is fixed before timing.

Use all eight B01 codecs as controls and both new candidates. Run the same 18
schema/regime/size cases, five fresh processes and the same owned-i64 input/output
and CRC contract. Keep seed-offset 0 as a comparison set and 1,000,003 as a fresh
seed confirmation set. No tuning after observing either set in this round.
Fresh seeds test the same generator family; they are not real-world validation.

Report three axes explicitly for every baseline: encoded size, full encode time,
full decode time. Count strict three-axis improvement (smaller, faster writing,
faster reading), a practical screen (>=20% smaller, >=20% faster encode and decode),
and the original ambitious screen (>=20% smaller, >=2x decode, with no encode
regression). Report paired baselines and all-Parquet counts separately. An isolated
codec match does not mean domination of every measured baseline or all products.

The B01 limitations remain: synthetic full blocks in RAM; no disk, durability,
market feeds, book correctness, partial queries, narrow integer physical schemas,
Arrow-native workload or Zstd comparison. All timings include predictor selection,
GCD, frame construction, checksum, conversion and materialization. CRC validation
is included in every decode. Retain failed cases and previous results unchanged.
