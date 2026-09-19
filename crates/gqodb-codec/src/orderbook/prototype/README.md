# Orderbook profile — working prototype

The parent package contains the actual [facade](../../../prototype/src/lib.rs),
[tests](../../../prototype/tests/codec.rs) and [full contract](../../../prototype/README.md).
This directory documents the dedicated book path without duplicating the codec.

```sh
book_demo=$(mktemp -d /tmp/gqodb-book-codec.XXXXXXXX)
cargo run --offline -p gqodb-codec -- demo orderbook "$book_demo/block.gqodb.ob"
cargo run --offline -p gqodb-codec -- demo book-segment "$book_demo/segment.gqodb.ob"
cargo run --offline -p gqodb-codec -- validate "$book_demo/segment.gqodb.ob" orderbook
cargo test --offline -p gqodb-codec -p gqodb-store
```

Validation decodes actual native payloads and checks sealed segment metadata and
checksums. An unknown clock-error field remains null; source timestamps are never
synthesized. Bad depth/version/family metadata fails strict validation. Compression
validity does not imply exchange-valid book state or gap-free market history.
