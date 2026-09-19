# B13 — Profile and improve the existing reader

Use the three existing B12 round-6 files, unchanged, and the same source-backed
full-row and five-query equality checks. Read-only mode does not write datasets;
write timings are zero and must not be interpreted as performance results.

First collect diagnostic frame+metadata, codec decode and decoded-index validation
times via a separate instrumented read path. Normal reads omit those clock calls
through a const-generic specialization. Capture the pre-optimization benchmark
binary and SHA before changing any read implementation.

Test the identified optimization against that frozen binary, alternating old/new
process order across three pairs. Each process uses B12's warmup plus six measured
rounds, with both Parquet controls unchanged. Preserve all samples and verify the
same exact returned rows/schema. No checks removed, data quantization or file changes.
Use CPU 2 and warm page-cache reads. No simultaneous bus benchmarks, host tuning
or claims about cold-disk IO. A profile run is development evidence, not confirmation.

Keep improvements only with correctness intact and report both full reads and real
queries, including regressions. Process-level summaries and variability matter;
individual within-process timings are not independent replication.

The initial three instrumented reads measured about 69.3 ms in decoded-index
validation, 92.6–93.4 ms in decode and 1.28 ms in frame/metadata reads. Selected
changes: replace duplicate-heavy bulk symbol-set collection with per-run insertion
(every row still inspected), and transfer owned Int64 buffers into Arrow on their
last metadata reference instead of cloning. Aliased value/validity references must
remain correct. Format, timestamps, checksums and all index comparisons stay intact.
