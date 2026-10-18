# Security and data integrity

There is no published supported-version or response-time commitment. Report a
suspected vulnerability privately through GitHub's private vulnerability reporting
for this repository (Security → Report a vulnerability), not in a public issue. Do
not put secrets, account identifiers or proprietary payloads into issues.

Include the affected version or commit, crate, input shape, reproduction steps and observed
impact. Use synthetic data whenever possible.

The codecs and readers in this repository parse untrusted bytes. The unchecked decoding
layer (`real::decode`, `Reader::read_block`, `read_all`, `visit_blocks`, `query`,
`profile_read_all`, `recover`, and therefore the `ob_store verify`, `query` and `recover`
commands) is not audited. One class of problem in it, found by reading the code and not by a fuzzer, is fixed: a
crafted block could request `rows x longest string` bytes for a string column (62.9 GB from a
60 KB block). That layer now refuses a block whose decoded strings exceed 128 MiB, before it
allocates them (`real::MAX_DECODED_STRING_BYTES`; `docs/FUZZING.md`, finding 9). The limit
closes that class, not all of them: it covers string bytes only, so a 16 KB block with
1,048,576 rows of 64 integer columns still decodes there to roughly 0.5 GiB (measured peak
529 MB). `gqodb-codec` (including its `inspect` and `validate` commands) and
`Reader::read_block_bounded` check a total expansion estimate of 128 MiB first and refuse
both blocks. Treat files from untrusted sources with the checked entry points only.

Frame checksums, declared-size checks and validation rejected the malformed and truncated
inputs used in the test suites and by fifteen libFuzzer targets of limited duration (see
[docs/FUZZING.md](docs/FUZZING.md) for what ran, what was found and what was not reached),
and a physically incomplete tail is distinguished from a corrupt frame. That is partial
coverage, not a security audit. A checksum detects damage; it does not authenticate who
wrote a file.

The `gqodb` command (`gqodb-cli`) reads data blocks only through `Reader::read_block_bounded`, with the
size bound of `gqodb-codec` on what a block may expand to, and never through the unchecked layer
above. A block that does not decode, or a file whose footer or index is invalid, is an error that
names the file and the reason (exit code 1); it is never repaired. With `--skip-bad` such a file is skipped
instead, named on standard error, and the exit code is still 1. Every read is bounded by budgets for
output rows, scanned rows and blocks (defaults 1,000, 1,048,576 and 4,096; hard maxima 1,000,000,
25,600,000 and 100,000), and by 10,000 files per directory and 2,000,000 rows for `order`; a command
text longer than 1 MiB is refused, but only after it has been read completely (see the known flaws below).
A budget that stops a read is reported, and under `--strict` it is an error. The command
creates no file except the one named by `--out`, which it never overwrites, and it does not follow a
symbolic link that points outside the directory it lists. The language parser has a libFuzzer target;
the executor has none (`docs/FUZZING.md` lists what was not run), and the command language has no
stability promise. Known flaws are listed in [docs/LANGUAGE.md](docs/LANGUAGE.md#13-what-is-not-in-phase-1-and-what-is-known-to-be-wrong).

Benchmark evidence under `results/` is hash-checked against its receipts. SHA-256
agreement proves the bytes match a recorded digest — not that the publisher is
trustworthy, and not that the measurement method was right.
