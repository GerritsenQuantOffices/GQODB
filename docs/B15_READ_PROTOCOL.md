# B15 — Continue decoder optimization without format changes

Baseline: the frozen B13 candidate binary (implementation 8c3bedc). Reuse the
same B13 read-only harness, B12 files, all full-row/schema checks, warmup,
queries and Parquet controls. Raw harness reports retain their B13 schema tag;
this protocol and result directory identify the B15 experiment.

Candidates: preallocate nonnull dictionary-string output and append identical-ID
runs; specialize byte-plane reconstruction for its fixed 0–8 byte widths.
Encoding, stored bytes, row order, precision and integrity checks stay unchanged.
Nullable strings retain their checked general path. Compare the combined candidate
in one development screen; if promising, freeze it and alternate three old/new
process pairs with six measured rounds each. Retain failed screens and regressions.

Single CPU 2, warm page-cache, no concurrent bus/build work or host changes.
Keep a change only if exactness remains and measured reads/queries justify it.
No universal or cold-IO claim; compare against dictionary-Parquet even if it wins.

Development screen at 0faaa23: about 39.3 ms warm full reads versus about 41.6 ms
for dictionary-Parquet in that process. This justifies the predefined three pairs,
not a win claim yet. If those pairs confirm improvement, rerun the complete B12
file-write/read/query protocol once with the final candidate to check the current
integrated behavior. Keep its sync/write timings distinct from read-only results.
