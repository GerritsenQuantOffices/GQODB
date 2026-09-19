# Tick profile — working prototype

Implementation and tests are shared by the parent package, not duplicated here:
[facade source](../../../prototype/src/lib.rs),
[roundtrip and rejection tests](../../../prototype/tests/codec.rs),
[full contract](../../../prototype/README.md).

From the repository root:

```sh
tick_demo=$(mktemp -d /tmp/gqodb-tick-codec.XXXXXXXX)
cargo run --offline -p gqodb-codec -- demo tick "$tick_demo/block.gqodb.tick"
cargo run --offline -p gqodb-codec -- demo tick-segment "$tick_demo/segment.gqodb.tick"
cargo run --offline -p gqodb-codec -- validate "$tick_demo/segment.gqodb.tick" tick
cargo test --offline -p gqodb-codec
```

The physical tick profile uses native adaptive columns. The strict schema accepts
explicit `trade` or `quote` subtypes and preserves their separate payload columns.
A changed extension cannot make an orderbook block pass tick validation.
