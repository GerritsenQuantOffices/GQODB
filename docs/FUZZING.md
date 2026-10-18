# Fuzzing the readers and decoders

Status: **partly done, with one open finding.** 15 libFuzzer targets cover the block decoders, the
store reader and recovery, the codec facade and `gqodb-types`, plus bit-exact round-trip targets.
Every target ran at least 35 minutes on one machine (the Parquet decoder 2 x 60 minutes). The
campaign found and fixed eight defects, each with a regression test that fails without the fix, and
one finding is still open because the fix would change what the decoder accepts (see
[Open finding](#open-finding-owner-decision-pending)). This is a bounded campaign, not a proof:
coverage was still growing slowly on several targets when the runs ended.

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

Seeds (`fuzz/corpus/`, about 1 MB) are generated from the repository's own synthetic fixtures
(`gqodb_blocks::data::generate`, `gqodb_codec::fixture`, a fixed-seed generator). They contain no
market data. `fuzz/dict/gqodb.dict` holds the magic strings and metadata keys.

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
| `real_tick_decode` | `real::decode` (`GQOREAL3`), `Decoder` with a reused scratch buffer, `batch::Options` | reused scratch gives the same outcome as a fresh decoder; sequential and 2-worker decoding agree; `decode(encode(decode(x))) == decode(x)` |
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

## Findings

Eight defects were fixed, all in `gqodb-blocks`: two in this repository's own code (1 and 7) and six reached through the `parquet`/`arrow-*` dependencies (2-6 and 8). The input is refused with an error after each fix; valid
inputs, the encoders' output and every on-disk format are unchanged (`every_parquet_codec_still_round_trips_after_the_guards`
and the existing suite pass). Each regression test is in `crates/gqodb-blocks/prototype/tests/malformed.rs`. Fixes 2 to 6 and 8 have a
minimized reproducer stored under `tests/regressions/`; fixes 1 and 7 have no stored file, their tests
build the input inline. All of them fail on the pre-fix code.

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

Fixes 2-5 turn upstream panics into decode errors, which relies on unwinding panics (the default;
with `panic = "abort"` the process would still abort). Fix 6 is a guard for the cases the fuzzer
found, not a proof that the `parquet` crate has no other unbounded loop. The Parquet codecs in this
repository are benchmark controls, not the GQODB format; for untrusted bytes, prefer the native
codecs and the checked entry points (`gqodb-codec`, `Reader::read_block_bounded`). The upstream
defects (2-6) have not been reported yet.

The fuzz panic hook lets panics located in the `parquet` and `arrow-*` dependency sources through
only because the library contains them; a panic that escapes the library, or one in `gqodb-*`
code, still aborts the target. `GQODB_FUZZ_TRACE=1` prints the contained panics.

## Open finding (owner decision pending)

`real::decode`, `Reader::read_block`, `read_all`, `visit_blocks`, `query`, `profile_read_all` and
`Reader::recover` allocate `rows x (longest dictionary string)` bytes for a `LargeUtf8` column before
checking any bound. The `ob_store verify`, `query` and `recover` commands call these functions and are
therefore affected; `gqodb-codec inspect` and `validate` run a preflight first and are not. A
60 KB block whose dictionary string is 60,000 bytes long and whose column holds 1,048,576 rows
requests 62,914,560,000 bytes (`memory allocation of 62914560000 bytes failed`). The checked layers
refuse the same bytes: `gqodb-codec` (`worst-case decoded expansion exceeds 128 MiB`) and
`Reader::read_block_bounded`.

The fuzzer did not find this by itself (it needs a consistent edit of the JSON metadata and its
length field); it was found by reading the code and confirmed by hand, after which the mutator and
the `store_structured` builder were taught to produce it. Reproducer: `cargo run --release --example
string_bomb -- out.bin` writes the block (it does not decode it), and the ignored test
`unchecked_decode_must_not_allocate_rows_times_string_length` decodes it (run it only under
`ulimit -v`).

Why it is not fixed: a cap on decoded string bytes in the unchecked layer would refuse blocks that
decode today if their strings exceed the cap, i.e. it changes what the decoder accepts. Proposed fix
for the owner to accept or reject: in `real.rs` `restore`, sum the decoded string bytes (the
`try_fold` that already exists for the no-validity branch, and the same sum for the validity branch)
and `ensure!` it is at most 128 MiB, the constant the codec facade and the store already use, with
the same check in `real::encode` so that anything the encoder writes can be decoded. Until then the
targets that call the unchecked API (`real_*`, `store_*`) skip inputs whose estimate exceeds 128 MiB
(`harness::known_string_bomb`, same formula as the codec preflight) so that this one known finding
does not mask others; they therefore do not test for it.

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
- The Parquet implementation itself (`parquet`, `arrow-*`): only the way this repository calls it.
  New upstream versions need a new run.
- The three round-trip targets that include Parquet execute under 20 inputs per second, so their
  coverage of generated data is shallow; the generators are biased towards edge values rather than
  uniform.
- Results depend on one machine class and one seed per run.
