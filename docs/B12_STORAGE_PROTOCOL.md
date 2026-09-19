# B12 — Real order-book files, indexed reads and prefix recovery

Independent gqodb-store Rust prototype. The .gqodb.ob extension now denotes an
experimental versioned container around B11 blocks, not a finalized public format.

Before measuring, freeze the B11 hour06 source prefix: 1,048,576 level rows,
32,768 rows per block/Parquet row group, original order and full shared schema.
Use receipt_ns and symbol as index fields. No repartitioning or sorting to favor
one format. Compare the book codec with dictionary-LZ4 and delta-LZ4 Parquet.

One warmup round plus six alternating measured rounds, each using a new file
for each mode. Single process pinned to CPU 2. Time creation, encoding, index/footer
and file sync_all together. Time full file open/read/decode and query open/read/
filter separately. Full Arrow equality checks run outside timers. Every codec
includes schema; GQODB validates frame/block CRCs and decoded statistics. Parquet
uses its native reader validation; no identical corruption-guarantee claim.

Queries are preselected at input row positions 25%, 50% and 75%, each for that
row's symbol and an inclusive 10ms receipt-time interval. Also test one entire
symbol and an absent symbol. Expected rows come from a separate full filter
of the original input. Parquet prunes row groups by time and symbol min/max;
GQODB prunes blocks by time min/max and exact symbol sets. Both return full rows.
Page-level filtering, bloom filters and projection pushdown are not tested.

Reads are explicitly warm/page-cache tests. No cache flushing, kernel tuning or
live services. File sync is included for both writers; directory fsync, atomic
rename/catalog publication and proof of physical-device persistence are excluded.
Do not label this as a cold-disk throughput benchmark or universal Parquet result.

Correctness tests truncate a small sealed fixture at every byte offset and check
the exact complete-block prefix. A complete corrupt frame fails recovery instead
of being silently skipped. Missing/partial footer prevents ordinary open; read-only
recovery scans and validates complete blocks. Recovery can write a separate new
sealed file. The original is never truncated. Unsynced data lost by the OS cannot
be reconstructed. Process kill/power-cut and ENOSPC fault injection remain future tests.

```sh
cargo build -p gqodb-store --release --locked --offline --bins --examples
taskset -c 2 target/release/examples/storage_benchmark SOURCE NEW_ARTIFACT_DIR NEW_REPORT.json
```

Source file SHA and every consumed event payload SHA must match. Input remains
the B06/B11 level projection, not raw collector JSON or a full validated exchange
book. No live bus/replay integration is part of B12.

Development attempt at commit ef69b2b stopped during warmup: the Parquet control
lost custom Arrow schema metadata. No measured rounds or result report were
accepted. The control now explicitly writes and restores native file key/value
metadata (as in B06) and has its own regression test. Failed warmup files remain
under artifacts/b12-storage; the corrected run uses a new directory.
