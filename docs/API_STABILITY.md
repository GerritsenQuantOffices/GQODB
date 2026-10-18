# API and format stability for 0.1.0 (preview)

**There is no stability promise in 0.1.0.** Any public item, command-line interface or
file layout may change in a 0.x release, without a deprecation period. This page says
which parts are meant to be used, which are experiments, and what is pinned by tests
today. It does not describe anything that is not in the repository.

Versioning: the crates follow Cargo's 0.x convention. A change that is not
backwards-compatible bumps the minor version (0.1 to 0.2). The four crates are released
together and share one version number. The declared minimum Rust version is 1.85; it is
built and tested on Linux with the committed `Cargo.lock` (see `docs/RELEASING.md`),
and it may be raised in a minor release.

## What is meant to be used

These are the entry points the README and the crate documentation describe. They have
tests, and their behaviour on bad input is documented and tested. They are still
subject to change.

| Crate | Entry points | What the tests pin |
| --- | --- | --- |
| `gqodb-types` | `BookEvent`, `Envelope`, `Level`, `DepthCoverage`; `timing::{ClockReading, ClockState, AssessedTime, EventTiming, SourceTime}` | Exact integer fields; clock-evidence assessment rules (7 tests); validated timing types have no `Deserialize` on purpose |
| `gqodb-blocks` | `real::{encode, decode, equal, Decoder, Mode, MAX_DECODED_STRING_BYTES}` (the lossless Arrow round trip), `batch::Options` | Exact round trip of schema, metadata, values, nulls and float bits; corrupt and truncated input fails; decoded string bytes above 128 MiB are refused by decode and by encode |
| `gqodb-store` | `Writer`, `Reader` (`open`, `read_all`, `query`, `recover`), the `ob_store` binary | Round trip, range queries including late rows, every truncation point of a fixture, corruption is not mistaken for a crash tail, no overwrite |
| `gqodb-codec` | `detect`, `encode`, `decode`, `inspect`, `validate_schema`, `write_block`, `write_segment`, the `gqodb-codec` binary | The header, not the file extension, selects the codec; unknown or mismatched families are refused; size bounds are checked before decoding |

## What is experimental

- **Benchmark and feasibility code in `gqodb-blocks`.** The `Codec` enum and the
  `adaptive`, `block`, `parquet`, `market` and `data` modules, the `gqodb-blocks` binary
  and the `inspect`, `market_benchmark`, `paired_benchmark` and `real_benchmark`
  examples exist to produce and check the published benchmark numbers. The crate
  documentation calls them experimental and says the crate is not a database. They may be
  reorganised or removed, and their output formats (JSON reports) are not stable.
- **The `storage_benchmark` example in `gqodb-store`** and the benchmark helpers in
  `results/` and `benchmarks/`.
- **The `prototype/` directories.** Each crate keeps its source and tests under
  `prototype/` for historical reasons. The directory name is not a statement about the
  maturity of one crate over another, and the layout may change.
- **Command-line interfaces.** `gqodb-codec` and `ob_store` subcommands, arguments and
  the JSON printed by `inspect` can change. Do not parse them in production scripts.
- **Demo helpers** such as `gqodb_codec::fixture` and the `demo` subcommand: synthetic
  fixtures, not an ingestion format.
- **Limits.** The size bounds in `gqodb-codec` (for example `MAX_BLOCK_BYTES`) and in
  the store may be adjusted.
- **Exact dependency pins.** `arrow-*` and `parquet` are pinned to `=59.3.0` in the
  crate manifests. A crate that depends on `gqodb-blocks`, `gqodb-store` or
  `gqodb-codec` inherits those pins and must use the same Arrow version.

## What is promised about files

Nothing beyond what the tests below pin for the present version. There is no
compatibility promise between releases, and no migration tool.

- **Header selects the codec.** A native block starts with `GQOREAL3` (tick profile) or
  `GQOREAL4` (order-book profile); an indexed segment starts with `GQOBHDR1` and
  declares its codec in that header. The file name is never used. Tested: a tick file
  renamed to `.ob` is still read as a tick file.
- **Exactness.** A round trip restores the Arrow schema, metadata, values, nulls and
  float bit patterns, or returns an error. Tested for both codecs on the fixtures and column
  types in the test suites; this is not a proof for every possible schema.
- **Damage is detected, not repaired.** Frames carry CRC checks. Corrupt or truncated
  blocks fail; a physically incomplete final frame of an unfinished segment is
  distinguished from a corrupt complete frame. Recovery writes a new file and never
  edits the old one. A CRC detects damage; it does not authenticate who wrote the file.
- **Bounds.** In `gqodb-codec` and `Reader::read_block_bounded`, declared lengths and
  dimensions are checked before decoding, so that oversized declared sizes and
  dictionary-expansion bombs are rejected first. The unchecked layer (`real::decode`,
  `Reader::read_block`, `read_all`, `visit_blocks`, `query`, `recover`, and the `ob_store verify|query|recover`
  commands that call them) refuses a block whose decoded strings exceed 128 MiB
  (`real::MAX_DECODED_STRING_BYTES`, the same 128 MiB as the codec facade and the store) before it
  allocates them; that closes the one class found by fuzzing (docs/FUZZING.md, finding 9) and is
  not a bound on everything: the numeric columns of a block are not capped there (a 16 KB block can
  decode to about 0.5 GiB), and the layer remains unaudited. Tested for the cases in the test suites
  and by 15 fuzz targets of limited duration; neither is a security audit.
- **The encoder refuses what the decoder would refuse.** `real::encode` (and so
  `Writer::append`, `gqodb_codec::encode` and `batch::Options::encode`) returns an error for a batch
  whose decoded string bytes (over all `LargeUtf8` columns, null rows counting as empty) would
  exceed 128 MiB, so that everything the encoder writes can be decoded again. This is the one
  behaviour change of the limit: encoder output for batches under the limit is byte-identical to
  before, and no file layout changed. The plain Parquet modes of `real` do not go through this
  check.

Not promised: that files written by 0.1.0 can be read by 0.2.0, that a newer reader
accepts older files, byte-for-byte reproducibility of the encoder output across
versions, durability of a segment after power loss (directory sync and physical
power-loss behaviour are not qualified), or that reading is safe against a hostile
writer beyond the bounds above.

## What is not here

The GQODB engine (journal, ledger, recovery of a live writer, quality checks,
features, causal queries, connectors, shared-memory bus) is not part of this
repository, and nothing in these crates is a stable interface to it.

## How this page changes

When a release changes a public item in the first table or a file layout, the release
notes and `CHANGELOG.md` say so under "Changed" or "Removed". If the owner later wants
a stability promise for some part of the API, that is a decision to be written here
explicitly, with the version from which it holds.
