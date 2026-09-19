# B02 adaptive block format and reproducibility

Experimental little-endian format; not a production file specification. Version
B01 codecs are retained unchanged. The new format is identified by `GQOADP02`.

## Layout

Header (16 bytes): magic[8], allow-LZ4 flag[1], synthetic schema ID[1], column
count u16 (8), row count u32 (at most 1,048,576). Then eight 44-byte entries, then
concatenated column payloads, then four-byte CRC32 of all previous bytes.

Each directory entry contains:

| Relative offset | Field |
| --- | --- |
| 0 | Original column index |
| 1 | Predictor: absolute=0, delta=1, linear=2, reference=3 |
| 2 | Reference column index, or 255 for no reference |
| 3 | Payload compressed with LZ4: 0/1 |
| 4 | i64 origin |
| 12 | i64 linear step |
| 20 | i64 residual minimum |
| 28 | u64 residual GCD scale, never zero |
| 36 | u32 expanded packed-payload length |
| 40 | u32 stored-payload length |

Total fixed framing is 372 bytes. Physical encoding order is [1,0,2,3,4,5,6,7].
References must point to a column already decoded; duplicate columns and forward
references are errors. Every original column is restored to its original position.

Transform original values using the predictor, then subtract the full residual
minimum, divide by the exact full-column GCD and pack unsigned quotients. Sample
selection changes compression efficiency only: the complete original values
always determine the actual residuals and stored parameters. A pattern breaking
after the first 1,024 rows cannot remove events or precision.

Groups hold up to 128 quotients. Tag 0..32 is a BitPacker4x bit width followed by
`width * 16` bytes, with zero-filled tail integers. Tag 255 is an explicit u64
fallback followed by eight bytes per actual quotient. Other tags fail. Decoder
validates lengths and padding before invoking safe packing APIs. Width zero needs
no packed bytes. The optional LZ4 layer is kept only if it reduces payload length.

Predictors use wrapping i64 operations, and residual offsets use their exact u64
representation. This supports full-width i64 extrema without rounding. There is
no new null/missing-value model; the same eight non-null integer fixture columns
as B01 are retained.

The core remains `#![forbid(unsafe_code)]`. The dependency may internally select
Rust SIMD intrinsics or Rust scalar fallback; it does not link a C codec. Validate
cross-architecture formats before declaring support beyond the tested platform.
Source: [bitpacking crate](https://docs.rs/bitpacking/0.9.3/bitpacking/).

## Reproduce

Use the B02 source commit recorded in each result, from a clean checkout:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
taskset -c 2 target/release/gqodb-blocks --output artifacts/b02-seen --seed-offset 0
taskset -c 2 target/release/gqodb-blocks --output artifacts/b02-fresh --seed-offset 1000003
```

The ignored artifacts directory permits both runs from the same clean source
commit. Preserve both result directories afterward. Fresh seeds are a confirmation
within the same generator family, not validation on real market data.

B02 adds simultaneous size/write/read comparisons. Practical and ambitious gate
counts are separate; no weaker threshold silently replaces the original ambition.
Old B01 output remains reproducible by checking out its recorded source commit.
