# Orderbook codec profile

Implemented by `gqodb-codec` through existing
`gqodb-blocks::real::Mode::GqodbBook` and `gqodb-store`. The physical representation
uses run reduction, exact numeric normalization and byte-plane LZ4. Both
`GQOREAL4/GQOBOOK1` blocks and indexed segments retain every original row.
[Runnable prototype guide](prototype/README.md).

Depth metadata describes levels per side; there is no hardcoded maximum of ten
levels in the event-column profile. Compression never silently selects top-N or
changes snapshots/deltas into another semantic representation. Reconstruction
still belongs to `gqodb-book`, and venue action interpretation belongs to its adapter.

See [B11 protocol](../../../../docs/B11_BOOK_COMPRESSION.md) and
[measurements](../../../../results/b11/ASSESSMENT.md). The facade itself makes no
new throughput or compression-ratio claim. Tick and book segment headers select
their own native codec rather than relying on `.gqodb.ob` naming.
