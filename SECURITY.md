# Security and data integrity

There is no published supported-version or response-time commitment. Report a
suspected vulnerability privately through GitHub's private vulnerability reporting
for this repository (Security → Report a vulnerability), not in a public issue. Do
not put secrets, account identifiers or proprietary payloads into issues.

Include the affected commit, crate, input shape, reproduction steps and observed
impact. Use synthetic data whenever possible.

The codecs and readers in this repository parse untrusted bytes. Frame checksums,
size bounds and validation reject malformed and truncated input and distinguish a
physically incomplete tail from a corrupt frame, but they do not establish
adversarial fuzzing or security-audit coverage. A checksum detects damage; it does
not authenticate who wrote a file.

Benchmark evidence under `results/` is hash-checked against its receipts. SHA-256
agreement proves the bytes match a recorded digest — not that the publisher is
trustworthy, and not that the measurement method was right.
