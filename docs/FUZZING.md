# Fuzzing the readers and decoders

Status: **partly done; nine defects found and fixed, none open.** 15 libFuzzer targets cover the
block decoders, the store reader and recovery, the codec facade and `gqodb-types`, plus bit-exact
round-trip targets. Every target ran at least 35 minutes on one machine (the Parquet decoder
2 x 60 minutes). The campaign found and fixed eight defects, each with a regression test that
fails without the fix. A ninth finding, found by reading the code and confirmed by hand, is fixed
by a 128 MiB limit on decoded string bytes in the unchecked layer
([finding 9](#finding-9-string-expansion-in-the-unchecked-layer-fixed)); after it, the ten targets that reach
the changed code were run again for about 11 minutes each and found nothing
([Re-run after fix 9](#re-run-after-fix-9)). This is a bounded campaign, not a proof: coverage was
still growing slowly on several targets when the runs ended, and the unchecked layer is not audited.

Nothing here replaces a security review. A checksum detects damage; it does not authenticate who
wrote a file (see [SECURITY.md](../SECURITY.md)).

## How to run

The harness is the crate `fuzz/` (its own Cargo workspace; the stable CI gate never builds it).

```console
$ rustup toolchain install nightly --profile minimal
$ cargo +nightly install cargo-fuzz --version 0.13.2 --locked
$ cd fuzz
$ cargo +nightly fuzz build                       # sanitizer build of all targets
$ cargo +nightly fuzz list
$ cargo +nightly fuzz run store_reader -- -dict=dict/gqodb.dict -max_len=65536 -timeout=10 -rss_limit_mb=4096
$ FUZZ_BIN_DIR=<target-dir>/x86_64-unknown-linux-gnu/release FUZZ_WORK=<scratch-dir> \
    ./run_target.sh store_reader 1800             # the exact flags used for the numbers below
$ cargo +nightly fuzz run store_reader <crash-file>   # replay a finding
$ cargo run --release --example seedgen -- corpus     # regenerate the seed corpora (stable)
```

`cargo fuzz run` writes new inputs into `fuzz/corpus/<target>`, which holds the committed seeds; use
`run_target.sh` (or `-- <your-corpus-dir> fuzz/corpus/<target>`) to keep the seeds clean.
`run_target.sh` reads seeds from `fuzz/corpus/<target>` (never written) and keeps its working corpus,
artifacts and logs under `FUZZ_WORK`. Resource limits are part of the harness so that a hang or an
over-allocation is a finding rather than noise: input at most 64 KiB (4 KiB for the generated-data
targets), 10 s per input, 4096 MB RSS and 4096 MB per allocation. `.github/workflows/fuzz.yml` runs
60 s per target on pull requests with the pinned `cargo-fuzz` 0.13.2 (a weekly 600 s run is
scheduled); that workflow has not been run on GitHub yet.

Seeds (`fuzz/corpus/`, about 1.4 MB) are generated from the repository's own synthetic fixtures
(`gqodb_blocks::data::generate`, `gqodb_codec::fixture`, a fixed-seed generator). They contain no
market data. Since fix 9 the `real_*` and `store_*` corpora also hold the 60 KB reproducer of that
finding (as a block, and as a one-block segment) so that the refusal path is reached from the first
execution. `fuzz/dict/gqodb.dict` holds the magic strings and metadata keys.

## What the harness does

Libraries are built with AddressSanitizer, debug assertions and overflow checks (the `cargo fuzz`
default), so an arithmetic overflow that wraps silently in a release build is reported as a panic.

For the byte-oriented targets a custom mutator mutates normally and then recomputes every CRC layer
of whichever native format the (mutated) magic names, so mutations get past the checksum gates (one
input in eight keeps a stale checksum to exercise the rejection path). One input in six also grows
the JSON metadata of a `GQOREAL3/4` block while keeping its length field consistent.

| Target | Entry points | Beyond "no panic, hang or runaway allocation" |
| --- | --- | --- |
| `block_decode` | `block::decode` (`GQOBLK01`, three layouts) | accepted data re-encodes and decodes to the same columns in all layouts |
| `adaptive_decode` | `adaptive::decode` (`GQOADP02`), `Codec::AdaptiveSimd*` | same; facade and module agree on accept/reject |
| `real_tick_decode` | `real::decode` (`GQOREAL3`), `Decoder` with a reused scratch buffer, `batch::Options` | reused scratch gives the same outcome as a fresh decoder; sequential and 2-worker decoding agree; `decode(encode(decode(x))) == decode(x)`; accepted data holds at most 128 MiB of decoded string bytes |
| `real_book_decode` | same for `GQOREAL4` / `GQOBOOK1` | same |
| `parquet_decode` | the five Parquet `Codec`s, `parquet::framing_bytes`, the five Parquet `real` modes | accepted data re-encodes and decodes to the same value |
| `store_reader` | `Reader::open`, `read_block`, `read_block_bounded`, `query`, `profile_read_all` on arbitrary bytes and on checksum-repaired mutations of valid segments | an indexed range query equals a hand-written scan of every block |
| `store_recover` | `Reader::recover` | every recovered block can be read again; recovering the verified prefix is idempotent; `open` and `recover` agree when both succeed |
| `store_structured` | builds a segment from fuzz bytes with correct checksums at every layer, then injects inconsistencies (index entries that lie about offset, length, rows, time range and symbols; wrong frame magic; flipped payload bytes; wrong codec; missing footers; oversized dictionaries) | a segment with nothing injected MUST be accepted and read back bit-exactly, its query must match a scan, and `recover` must agree |
| `codec_facade` | `detect`, `decode` (no family, tick, orderbook), `inspect` on blocks and segments | decode/encode/decode is stable |
| `codec_schema` | `validate_schema` and `Family::parse` on arbitrary Arrow schema JSON | |
| `types_parse` | JSON parsing of the public `gqodb-types` types; `AssessedTime::assess` and `EventTiming::new` on arithmetic edge values | parse/serialise/parse is stable; an `Estimated` interval is symmetric around the UTC reading and contains it |
| `roundtrip_columns` | all ten `Codec`s on generated 8-column fixtures (0-300 rows, constants, runs, ramps, common divisors, values above 32 bits, i64 extremes) | bit-exact equality |
| `roundtrip_real` | all seven `real` modes on generated Arrow batches (Int8, Timestamp with and without time zone, Int64, Float64 including NaN payloads and signed zero, Boolean, LargeUtf8, nulls, schema metadata) | bit-exact equality (`real::equal`), also through the 2-worker path |
| `roundtrip_codec` | `gqodb_codec::encode/decode`, `write_segment_with_family`, `inspect` on generated normalized batches | bit-exact equality |
| `roundtrip_store` | `Writer`, `Reader::open/read_all/query/recover` on generated batches, both codecs | bit-exact blocks; query equals brute force; cutting the file anywhere after the header recovers exactly the complete blocks |

## Results

Machine class: Linux x86_64 workstation, shared with an unrelated timing benchmark; the fuzzers
were pinned to 4 logical CPUs at `nice 19`, one process per CPU, 60 GB RAM. Toolchain:
`rustc 1.101.0-nightly (5d8fec066 2026-10-16)`, `cargo-fuzz 0.13.2`, `libfuzzer-sys 0.4.13`;
the stable gate stays on Rust 1.96.0. Runs 17-18 October 2026. In the final runs libFuzzer ran in
`-fork=1 -ignore_crashes=1 -ignore_timeouts=1 -ignore_ooms=1` mode so that a finding did not end the
run. "Edges" are libFuzzer's inline coverage counters reached (they include the dependencies); the
corpus is the size of libFuzzer's working corpus at the end.

| Target | Minutes | Executions | Exec/s | Edges | Corpus | OOM/timeout/crash |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `block_decode` | 35.2 | 13,114,818 | 6,216 | 627 | 1,560 | 0/0/0 |
| `adaptive_decode` | 35.2 | 5,790,140 | 2,742 | 1,017 | 2,411 | 0/0/0 |
| `real_tick_decode` | 35.6 | 1,212,115 | 568 | 4,251 | 3,369 | 0/0/0 |
| `real_book_decode` | 35.5 | 1,457,520 | 684 | 3,893 | 3,189 | 0/0/0 |
| `parquet_decode` run A | 61.2 | 803,665 | 219 | 17,917 | 3,218 | 0/0/0 |
| `parquet_decode` run B | 60.5 | 695,969 | 192 | 17,622 | 3,103 | 0/0/0 |
| `store_reader` | 36.0 | 3,426,742 | 1,589 | 5,973 | 4,518 | 0/0/0 |
| `store_recover` | 35.6 | 4,415,826 | 2,064 | 5,740 | 5,715 | 0/0/0 |
| `store_structured` | 35.9 | 121,917 | 57 | 8,029 | 4,513 | 0/0/0 |
| `codec_facade` | 35.5 | 3,772,980 | 1,774 | 6,327 | 4,057 | 0/0/0 |
| `codec_schema` | 35.3 | 22,406,586 | 10,569 | 2,218 | 5,774 | 0/0/0 |
| `types_parse` | 35.4 | 24,821,614 | 11,686 | 3,916 | 8,519 | 0/0/0 |
| `roundtrip_columns` | 35.8 | 16,493 | 8 | 15,883 | 2,042 | 0/0/0 |
| `roundtrip_real` | 36.0 | 15,321 | 7 | 24,150 | 2,670 | 0/0/0 |
| `roundtrip_codec` | 35.6 | 43,073 | 20 | 5,488 | 2,175 | 0/0/0 |
| `roundtrip_store` | 35.7 | 80,817 | 38 | 6,251 | 3,303 | 0/0/0 |

The table lists the last run of each target, on the code described next; it totals about
620 CPU-minutes and 82 million executions. Earlier runs, which found the defects below, added about
180 minutes (a 12-minute discovery pass over all 15 targets) plus the interrupted reruns of
`parquet_decode` after each Parquet fix.

Which code each run saw. The Parquet-touching targets (`parquet_decode`, `roundtrip_columns`,
`roundtrip_real`) ran on the final commit of the fixes below. The other twelve ran on a tree that
contained only fix 1; the later fixes change `parquet.rs`, the Parquet branches of `real.rs` and the
Parquet encoder, which those twelve do not execute. The run directory of those twelve did not record a
revision; this statement is the campaign operator's and could not be verified afterwards. Coverage on the other targets was still
increasing at the end (for example `store_recover` 5,090 edges after 10 minutes, 5,491 after 20,
5,740 after 35), so more time would likely reach more.

Measured line coverage of the library sources after replaying the final corpora (`cargo fuzz
coverage`, llvm-cov; union of all 15 targets):

| File | Lines covered |
| --- | ---: |
| `gqodb-blocks` `block.rs` | 100.0% |
| `gqodb-blocks` `adaptive.rs` | 99.7% |
| `gqodb-blocks` `parquet.rs` | 98.4% |
| `gqodb-blocks` `book.rs` | 97.4% |
| `gqodb-store` `lib.rs` | 93.8% |
| `gqodb-blocks` `real.rs` | 93.5% |
| `gqodb-types` `timing.rs` | 85.1% |
| `gqodb-blocks` `batch.rs` | 81.8% |
| `gqodb-codec` `lib.rs` | 79.9% |
| `gqodb-blocks` `data.rs` (fixture generator) | 30.4% |
| `gqodb-blocks` `market.rs` (benchmark importer, not a format reader) | 0% |

The unreached lines are mostly writers (`write_block`, `write_segment*`, `Writer::checkpoint`),
accessors, `Display` impls and I/O error branches; `market.rs` is the benchmark's source importer.

## Re-run after fix 9

Run on 18 October 2026, 07:47 to 08:21 UTC, on the commit that contains fix 9 and the harness change
that removes the skip for the string-expansion class (`harness::known_string_bomb` is gone, so the
`real_*` and `store_*` targets now test it). Same machine class and toolchain as above
(`rustc 1.101.0-nightly (5d8fec066 2026-10-16)`, `cargo-fuzz 0.13.2`); one process per CPU,
pinned to CPUs 8, 9, 22 and 23, `nice 19`, in three waves (4 + 4 + 2 targets). Flags as in
`run_target.sh` plus `-fork=1 -ignore_crashes=1 -ignore_timeouts=1 -ignore_ooms=1`, 660 seconds
requested per target (fork-mode jobs finish after the deadline, so the last progress line is a little
later); seeds are the committed corpus, including the new reproducer seeds. "Minutes", "Executions",
"Edges" and "Corpus" are the last libFuzzer progress line; exec/s is executions divided by time.

| Target | Minutes | Executions | Exec/s | Edges | Corpus | OOM/timeout/crash |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `real_tick_decode` | 11.0 | 418,856 | 635 | 3,658 | 2,023 | 0/0/0 |
| `real_book_decode` | 11.4 | 506,708 | 738 | 3,205 | 1,818 | 0/0/0 |
| `parquet_decode` | 11.4 | 180,825 | 263 | 16,783 | 1,872 | 0/0/0 |
| `store_reader` | 11.2 | 1,344,289 | 1,997 | 5,599 | 3,832 | 0/0/0 |
| `store_recover` | 11.6 | 1,325,615 | 1,907 | 4,698 | 3,257 | 0/0/0 |
| `store_structured` | 11.4 | 32,021 | 47 | 7,381 | 2,727 | 0/0/0 |
| `codec_facade` | 11.5 | 1,191,590 | 1,729 | 6,140 | 2,587 | 0/0/0 |
| `roundtrip_real` | 11.1 | 7,226 | 11 | 22,783 | 1,435 | 0/0/0 |
| `roundtrip_codec` | 11.3 | 11,887 | 18 | 5,448 | 1,440 | 0/0/0 |
| `roundtrip_store` | 11.2 | 73,243 | 109 | 6,238 | 3,201 | 0/0/0 |

Together 113 CPU-minutes and 5.09 million executions; no crash, timeout, out-of-memory or assertion
failure, and no artifact file was written. These are the ten targets that execute `real::decode`,
`real::encode`, `Reader` reads or `Reader::recover` (`parquet_decode` through the
`ParquetNormalizedLz4` mode, which uses the same restore step). The other five (`block_decode`,
`adaptive_decode`, `codec_schema`, `types_parse`, `roundtrip_columns`) do not execute the changed code
and were not re-run; their results above stand. Eleven minutes each is short: the round-trip
targets executed between 7,000 and 73,000 generated inputs each (7 to 109 per second).

## Findings

Nine defects were fixed, all in `gqodb-blocks`: three in this repository's own code (1, 7 and 9) and six reached through the `parquet`/`arrow-*` dependencies (2-6 and 8). The input is refused with an error after each fix; valid
inputs, the encoders' output for such inputs and every on-disk format are unchanged (fix 9 also makes the encoder refuse a batch above the new limit, see below) (`every_parquet_codec_still_round_trips_after_the_guards`
and the existing suite pass). Each regression test is in `crates/gqodb-blocks/prototype/tests/malformed.rs` (fix 9 also in `crates/gqodb-store/prototype/tests/segments.rs`). Fixes 2 to 6 and 8 have a
minimized reproducer stored under `tests/regressions/`; fixes 1 and 7 have no stored file, their tests
build the input inline; fix 9 has both. All of them fail on the pre-fix code.

| # | Target | Root cause (one line) | Fix |
| --- | --- | --- | --- |
| 1 | `real_tick_decode` | `real::decode`: a 12-15 byte block with a valid CRC makes `end - 12` underflow before the schema-length check (panic with overflow checks; a wrapped bound and an out-of-range slice without) | refuse when `end < 12` |
| 2 | `parquet_decode` | `arrow-ipc` 59.3.0 `convert.rs` calls `unimplemented!`/`unwrap`/`expect` on unknown or missing type tags of the embedded `ARROW:schema` (four sites: lines 198, 332, 354, 514) | contain the panic (`catch_unwind`) around the Parquet reader |
| 3 | `parquet_decode` | `parquet` 59.3.0 `encodings/decoding.rs:750`: a forged DELTA_BINARY_PACKED header makes `values_left -= 1` underflow (a panic only with overflow checks) | same containment |
| 4 | `parquet_decode` | `parquet` 59.3.0 `util/bit_util.rs:868`: shift past the integer width on a forged bit width (a panic only with overflow checks) | same containment |
| 5 | `parquet_decode` | `arrow-buffer` 59.3.1 `bit_chunk_iterator.rs:224` asserts "offset + len out of bounds" on forged definition levels (also in release builds) | same containment |
| 6 | `parquet_decode` | `parquet` 59.3.0 skips unknown thrift fields of a page header and of the file footer with no size bound: a list or map of N elements that cost no input bytes (booleans, or doubles/binaries read past the end of input, where `io::copy` does not fail) costs N iterations, so a 0.8-2 KB file kept a core busy for minutes. The header parser also reads past the column chunk into the next one | a minimal Thrift walker visits the footer and every page header before the reader does, with a budget of 4,096 free elements, and refuses the file when it is exceeded; a walk that cannot parse something is left to the reader |
| 7 | `parquet_decode` | `real::parquet_encode` keys the per-column encoding by column name; a Parquet file with two same-named columns of different types decodes, and re-encoding it applied DELTA_BINARY_PACKED to a Float64 column, which panics in `parquet` 59.3.0 | the encoder returns an error for duplicate names that need different encodings (same-name, same-encoding columns are written as before) |
| 8 | `parquet_decode` | (follow-up to 6) the first page-header walk was clipped to the column chunk and missed a bomb reached through an over-long binary field | the walk runs to the end of the file, like the reader |
| 9 | `real_*`, `store_*` (found by reading the code, not by a fuzzer) | `real::decode` (`restore`) allocates `rows x (longest dictionary string)` bytes for a `LargeUtf8` column before any bound: a 60 KB block requests 62,914,560,000 bytes | decoded string bytes above 128 MiB are refused before allocation, in decode and in encode (below) |

Fixes 2-5 turn upstream panics into decode errors, which relies on unwinding panics (the default;
with `panic = "abort"` the process would still abort). Fix 6 is a guard for the cases the fuzzer
found, not a proof that the `parquet` crate has no other unbounded loop. The Parquet codecs in this
repository are benchmark controls, not the GQODB format; for untrusted bytes, prefer the native
codecs and the checked entry points (`gqodb-codec`, `Reader::read_block_bounded`). The upstream
defects (2-6) have not been reported yet.

The fuzz panic hook lets panics located in the `parquet` and `arrow-*` dependency sources through
only because the library contains them; a panic that escapes the library, or one in `gqodb-*`
code, still aborts the target. `GQODB_FUZZ_TRACE=1` prints the contained panics.

## Finding 9: string expansion in the unchecked layer (fixed)

`real::decode`, `Reader::read_block`, `read_all`, `visit_blocks`, `query`, `profile_read_all` and
`Reader::recover` (and so the `ob_store verify`, `query` and `recover` commands) allocated
`rows x (longest dictionary string)` bytes for a `LargeUtf8` column before checking any bound. A
60 KB block whose dictionary string is 60,000 bytes long and whose column holds 1,048,576 rows
claims 62,914,560,000 decoded string bytes (the allocator was asked for 62,917,705,728:
`memory allocation of 62917705728 bytes failed`, an abort). The checked layers (`gqodb-codec`,
`Reader::read_block_bounded`) refused the same bytes already. The fuzzer did not find this by itself
(it needs a consistent edit of the JSON metadata and its length field); it was found by reading the
code and confirmed by hand, after which the mutator and the `store_structured` builder were taught
to produce it. Reproducer: `cargo run --release --example string_bomb -- out.bin [60000
out.gqodb.ob]` in `fuzz/` writes the block (and, with a third argument, a one-block segment); it
does not decode it.

**Fix (owner decision of 18 October 2026: a 128 MiB limit, no format change).** In
`crates/gqodb-blocks/prototype/src/real.rs`, `restore` sums the decoded string bytes of each
string column before it allocates anything: the length of the dictionary entry of every valid row
(every row when there is no validity vector), added with saturating `u64` arithmetic, and all string
columns of the block together must stay within `real::MAX_DECODED_STRING_BYTES` = 128 MiB (134,217,728
bytes), the same value as `gqodb_codec::MAX_DECODED_BYTES` and the store's frame limit (a test asserts
the equality with the codec constant and the store's `read_block_bounded` range check). The error is
`decoded string bytes exceed the limit of 134217728 bytes (128 MiB)`. The same sum runs in
`real::encode` (`normalize`, before a string is copied into the dictionary), so that anything the
encoder writes can be decoded; consequently `Writer::append` and `gqodb_codec::encode` return an
error for a batch above the limit. That is the one behaviour change: a batch with more than 128 MiB
of decoded string data used to encode and now returns an error (its block would be refused by the
decoder). The limit counts decoded string bytes only (see below for what it does not bound). The
`Parquet` modes other than `ParquetNormalizedLz4` do not go through `restore` and have no such limit.

Evidence:

- **No format change.** The encoder output of 75 batches in all seven `real` modes (525 encodings: 72
  generated batches of 1 to 5,000 rows, with and without nulls, with 0, 1 or 3 string columns and 0 or
  1 metadata keys; the two codec fixtures; a maximum-row batch), plus two `gqodb_codec::encode`
  outputs, was hashed with SHA-256 before and after the change. 517 of the 527 lines are reproducible
  between two runs of the same binary (the metadata JSON of the native blocks was compared in
  canonical key order), and all 517 are identical before and after. The other 10 lines (the five
  Parquet modes of the two codec fixtures, which carry six and seven metadata keys) differ between
  two runs of the same binary, before the change as well, because a schema's metadata `HashMap` is
  serialised in random key order; so byte-for-byte reproducibility of encoder output is not
  something this repository promises. The existing round-trip and malformed-input tests pass.
- **Pure-function boundary tests** (no 128 MiB allocated): `real.rs` `restore_tests` check the sum at
  limit - 1, limit and limit + 1 for valid rows, invalid rows, shared limits over several string
  columns and sums above 32 bits, and check that `normalize` (encode side) and `restore_with_limit`
  (decode side) accept and refuse exactly the same inputs at those limits.
- **Negative control.** With the check removed from the sum (limit constant and tests unchanged) and
  tests run under `ulimit -v 8000000`: 5 of the 6 `restore_tests` fail, 3 of 13 `malformed` tests
  fail (the reproducer, the stored 60 KB block, the 129-byte-per-row case; the allocation panics in
  `LargeStringBuilder::with_capacity`), 2 of 8 store `segments` tests fail (the stored segment and the
  writer refusal). With the fix they all pass without `ulimit`. With the check removed, the four
  seeded fuzz targets (`real_tick_decode`, `real_book_decode`, `store_reader`, `store_recover`) abort
  on the new reproducer seeds.
- **Every entry point, on the stored 60 KB block and the stored one-block segment, with an 8 GB
  address-space limit.** Before the fix each of the first eight rows aborted with
  `memory allocation of 62917705728 bytes failed` (peak RSS about 21 MB at the abort); after it each
  returns the error above:

| Path | Before | After (peak RSS) |
| --- | --- | --- |
| `real::decode` | abort | error, 20.8 MB |
| `Decoder::decode` (reused scratch) | abort | error, 21.0 MB |
| `batch::Options { workers: 2 }.decode` | abort | error, 33.9 MB |
| `Reader::read_block` | abort | error, 21.4 MB |
| `Reader::read_all` | abort | error, 21.1 MB |
| `Reader::visit_blocks` | abort | error, 21.1 MB |
| `Reader::query` | abort | error, 21.1 MB |
| `Reader::profile_read_all` | abort | error, 21.3 MB |
| `Reader::recover` | abort | error, 21.3 MB |
| `ob_store verify` / `query` / `recover` | abort | exit 1 with the error, 21.4 / 21.2 / 21.4 MB (no output file) |
| `gqodb_codec::decode`, `Reader::read_block_bounded` | error (`worst-case decoded expansion exceeds 128 MiB`, `bounded decoded expansion limit`) | unchanged, 8.4 and 8.6 MB |

The 21 MB are the 1,048,576-row index columns the decoder builds before it reaches the string
column; they are not string memory. Peak RSS is `ru_maxrss` of a child process.

**What the limit does not do.** It closes this class, not the unchecked layer's whole exposure.
It bounds string bytes only: a 16 KB block of 1,048,576 rows and 64 constant integer columns still
decodes in the unchecked layer with a peak RSS of 529 MB (measured; `gqodb-codec` refuses it with
`worst-case decoded expansion exceeds 128 MiB`). Time is not bounded either. Treat the unchecked layer as
unaudited and use `gqodb-codec` or `Reader::read_block_bounded` for files you did not write.

## What this does not cover

- Sanitizers other than AddressSanitizer; no MemorySanitizer, no Miri, no Valgrind. Release builds
  without overflow checks were not fuzzed (the fuzz build enables them).
- Concurrency (beyond comparing 1- and 2-worker batch decoding), crash and power-loss behaviour,
  `ENOSPC`, filesystem races, symlinks and a file that changes while it is read.
- Whole-file semantic attacks beyond the mutations above: for example a valid-looking segment with
  plausible but false prices. Checksums do not authenticate content.
- The writers (`Writer`, `write_segment*`) except as the source of round-trip data, and the CLI
  binaries.
- `market.rs` (the benchmark's gzip/JSON importer).
- Inputs above 64 KiB (4 KiB for generated-data targets), so no block with a large real row count
  except the single maximum-row seeds; `MAX_ROWS` x 64-column expansions were exercised only through
  those seeds.
- The unchecked decoding layer as a whole: fix 9 closes one class (string bytes); the numeric expansion
  (rows x columns x 8 bytes, 529 MB measured for 16 KB of input) and decoding time are not capped there, and
  the re-run was 11 minutes per target.
- The Parquet implementation itself (`parquet`, `arrow-*`): only the way this repository calls it.
  New upstream versions need a new run.
- The three round-trip targets that include Parquet execute under 20 inputs per second, so their
  coverage of generated data is shallow; the generators are biased towards edge values rather than
  uniform.
- Results depend on one machine class and one seed per run.
