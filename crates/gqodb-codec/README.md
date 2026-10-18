# gqodb-codec — checked native codec dispatch

> **0.1.0 preview.** Checked native format detection, inspection and validation for GQODB tick and order-book files. No stability promise: APIs, command-line tools and file
> formats may change in any 0.x release (see [API stability](https://github.com/GerritsenQuantOffices/GQODB/blob/main/docs/API_STABILITY.md)).
> Part of the [GQODB open layer](https://github.com/GerritsenQuantOffices/GQODB/blob/main/README.md); evidence and benchmark bounds are in
> the repository README.

Working safe-Rust facade over the existing specialized codecs and indexed store.
It selects decoders from native magic/header codec IDs, checks versions and
expansion bounds, and preserves exact Arrow schema, metadata, values and nulls.
No alternate JSON payload container is introduced.

* [Ticks/trades](src/tick/README.md): `GQOREAL3/GQOADP02`, adaptive predictors and LZ4.
* [Order books](src/orderbook/README.md): `GQOREAL4/GQOBOOK1`, runs and byte-plane LZ4.
* Existing indexed segment framing supports either explicit native codec; legacy
  order-book segments retain their original interpretation.

[Prototype guide](prototype/README.md) contains runnable commands, supported schema
contracts, exact limits and tests. Filename extensions never establish type or trust.

```sh
cargo test --offline -p gqodb-codec -p gqodb-store
cargo run --offline -p gqodb-codec -- --help
```
