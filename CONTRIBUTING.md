# Contributing to GQODB

This repository is the open layer of GQODB: the exact event and time types, the
lossless codec kernels, indexed order-book segments, the native format facade, and
the benchmark protocols and evidence behind them. Each crate keeps its code and tests
in its own `prototype/` directory, with a Cargo manifest at the crate root. Document
inputs, outputs, failure behaviour and format compatibility in the crate README.

Use safe Rust. Preserve exact integers and explicit scales, and never let a codec
change a value silently: a round trip either restores the input exactly or fails.
Format detection belongs in the header, not the file extension.

## Local checks

The tested compiler is Rust 1.96.0 on Linux. The workspace declares a 1.85
minimum, but an MSRV build has not been qualified; do not claim that minimum is
tested.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo build --workspace --bins --release --locked
python3 results/verify_public_evidence.py
```

The default build is pure Rust. `--features zstd-input` on `gqodb-blocks` adds the
native zstd library from crates.io, only to read zstd-compressed Parquet inputs such
as the original B06 samples; it is never a measured codec. The CI workflow runs these
checks on Linux; its existence does not mean a given remote run has passed.

## Changes and measurements

Add deterministic tests for round trips, ordering, corrupt or truncated input and
arithmetic changes. A benchmark must identify its source commit, input and config
hashes, hardware, release flags, cache state, repetitions and what it counts as
written. Show regressions as well as wins; the tables in this repository keep the
runs where Parquet wins.

Do not commit tokens, credentials, proprietary datasets, generated binaries, Cargo
target directories, raw machine captures or absolute local paths. Keep compact
provenance and result receipts under `results/`. New fixtures should be synthetic or
have clear redistribution rights. Never publish proprietary market samples just to
make a benchmark downloadable.

## Licence

This repository is licensed under the Apache License, Version 2.0 (see `LICENSE`).
Contributions intentionally submitted for inclusion are licensed under the same
terms, as section 5 of that licence describes. The GQODB engine is not part of this
repository, and nothing here grants rights to it.
