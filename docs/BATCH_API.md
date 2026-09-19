# Optional concurrency and reusable decoding

These APIs are experimental block processing, not a live database service.

## Worker choice

`gqodb_blocks::batch::Options::default()` uses one worker and creates no threads.
Set `Options { workers: 2 }` to process independent blocks concurrently. Valid
values are 1 through 64. Work is capped by the number of input blocks. Results
preserve input order and the encoded bytes are identical to sequential encoding.

```rust
use gqodb_blocks::{batch::Options, real::Mode};
use bytes::Bytes;
# fn example(batches: &[arrow_array::RecordBatch]) -> anyhow::Result<()> {
let options = Options { workers: 2 }; // Optional; default is 1.
let blocks = options.encode(batches, Mode::Gqodb)?;
let blocks: Vec<Bytes> = blocks.into_iter().map(Bytes::from).collect();
let restored = options.decode(&blocks, Mode::Gqodb)?;
# Ok(())
# }
```

Threads are scoped to each call, joined before return, and errors propagate.
There is no persistent pool. Small calls may be slower due to thread startup.
Each decode worker reuses one LZ4 scratch buffer across its assigned blocks.
Additional workers consume additional workspace and thread stacks. The API
retains the supplied inputs and complete outputs: it is not a bounded streaming
queue. Applications should pass bounded groups of blocks, rather than an entire
historical dataset.

## Reuse without concurrency

For a sequential stream, keep `real::Decoder::default()` alive and call
`decoder.decode(block, Mode::Gqodb)` repeatedly. `retained_bytes()` reports scratch
capacity; `release()` drops it. Returned Arrow batches own their buffers and stay
valid across later calls. Scratch contains packed LZ4 output, not a cache of
decoded historical rows. The stateless `real::decode` function remains available.

The decoder fuses predictor reconstruction into unpacking. Arrow conversion is
still a separate pass because columns can reference earlier integer columns.
Checksums, dimension checks and lossless value restoration remain enforced.

## Integer prices remain optional

Existing `Float64` input continues to work with full bitwise roundtrip checks.
Feeds that already supply exact scaled integer prices may instead supply an
Arrow `Int64` column. Store the scale explicitly in Arrow field metadata, for
example `price_scale=100` for `12345` representing `123.45`. Metadata is preserved
by the format; the codec does not interpret this application-defined key or
automatically convert decoded integers back to floats.

Choose integer input at the feed/schema boundary. Do not obtain it by blindly
rounding a float: the caller must preserve the venue's exact price and scale.
Integer input and worker count are independent choices. Neither requires
changing compression levels or dropping data.

## Benchmark

`cargo run --release --locked --example paired_benchmark -- SOURCE OUTPUT.json 4096 32 2`

This compares frozen B04 sources from commit `44785d7` to current sequential
reuse, optional workers, and normalized Parquet. It uses three warmup rounds,
rotated/reversed candidate order and saves each encode/decode sample. Bytes are
checked against B04 and every restored batch against original Arrow values.
Source loading is outside timing. Worker startup is inside timing. Inputs are
resident in RAM; this is not an IO or durability benchmark.
