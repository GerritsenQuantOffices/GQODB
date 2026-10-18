# Security and data integrity

There is no published supported-version or response-time commitment. Report a
suspected vulnerability privately through GitHub's private vulnerability reporting
for this repository (Security → Report a vulnerability), not in a public issue. Do
not put secrets, account identifiers or proprietary payloads into issues.

Include the affected version or commit, crate, input shape, reproduction steps and observed
impact. Use synthetic data whenever possible.

The codecs and readers in this repository parse untrusted bytes, and one finding is
open. The unchecked decoding layer (`real::decode`, `Reader::read_block`, `read_all`,
`visit_blocks`, `query`, `profile_read_all`, `recover`, and therefore the `ob_store
verify`, `query` and `recover` commands) can be made to request a very large allocation
by a crafted block, while `gqodb-codec` (including its `inspect` and `validate` commands)
and `Reader::read_block_bounded` refuse the same bytes. Treat files from untrusted
sources with the checked entry points only.

Frame checksums, declared-size checks and validation rejected the malformed and truncated
inputs used in the test suites and by fifteen libFuzzer targets of limited duration (see
[docs/FUZZING.md](docs/FUZZING.md) for what ran, what was found and what was not reached),
and a physically incomplete tail is distinguished from a corrupt frame. That is partial
coverage, not a security audit. A checksum detects damage; it does not authenticate who
wrote a file.

Benchmark evidence under `results/` is hash-checked against its receipts. SHA-256
agreement proves the bytes match a recorded digest — not that the publisher is
trustworthy, and not that the measurement method was right.
