# Tick and trade codec profile

Implemented by the parent `gqodb-codec` facade using the existing
`gqodb-blocks::real::Mode::Gqodb` adaptive predictor/bitpacking/LZ4 codec.
Both standalone `GQOREAL3/GQOADP02` blocks and indexed tick segments work.
[Runnable prototype guide](prototype/README.md).

Strict normalized metadata identifies trade versus quote without guessing from
a filename or numeric values. Original row order, Arrow schema, integer precision,
null timing fields and source metadata survive exact roundtrip. No price rounding,
aggregation or event filtering is presented as compression gain.

Earlier [B06 results](../../../../results/b06/ASSESSMENT.md) remain scoped to their
sampled trades and Parquet settings. The facade adds validation; it does not create
a new benchmark win or claim all tick workloads outperform every competitor.
