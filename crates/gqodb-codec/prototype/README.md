# Native codec facade

## What is implemented

`gqodb-codec` dispatches and validates existing native formats. Tick/adaptive blocks
use `GQOREAL3` containing `GQOADP02`; book blocks use `GQOREAL4` containing
`GQOBOOK1`. Indexed segments keep the existing `GQOBHDR1` framing and declare their
native codec in the header. No new compressed representation or renamed JSON format
is introduced. Compression kernels remain in `gqodb-blocks`; segment framing,
indexes and crash-prefix recovery remain in `gqodb-store`.

The physical codec profile and normalized logical schema are separate. `inspect`
fully decodes and checks native structure, checksums, lengths and schemas, then
reports whether the optional normalized contract also validates. Existing native
files without that metadata remain inspectable; they are not silently certified as
normalized tick/book data. `validate FILE FAMILY` additionally requires that strict
logical contract and matching physical codec family.

## CLI examples

From the repository root:

```sh
codec_demo=$(mktemp -d /tmp/gqodb-codec.XXXXXXXX)
cargo run --offline -p gqodb-codec -- demo tick "$codec_demo/trades.gqodb.tick"
cargo run --offline -p gqodb-codec -- validate "$codec_demo/trades.gqodb.tick" tick
cargo run --offline -p gqodb-codec -- demo orderbook "$codec_demo/book-block.gqodb.ob"
cargo run --offline -p gqodb-codec -- validate "$codec_demo/book-block.gqodb.ob" orderbook
cargo run --offline -p gqodb-codec -- demo tick-segment "$codec_demo/ticks-segment.gqodb.tick"
cargo run --offline -p gqodb-codec -- demo book-segment "$codec_demo/book-segment.gqodb.ob"
cargo run --offline -p gqodb-codec -- inspect "$codec_demo/book-segment.gqodb.ob"
```

Installed binaries take the same arguments. Outputs must be new. The small demos
are synthetic exact roundtrip fixtures, not a benchmark or complete venue history.
Both a standalone native block and an indexed multi-block segment may use the
`.gqodb.tick`/`.gqodb.ob` extension; the magic determines which container is present.
Renaming a tick file to `.ob` cannot change decoder selection or pass book validation.

Reports include physical format, actual codec profile, file SHA256/length, block/
row counts, exact Arrow schema with metadata, whether normalized-contract checks
passed and the reason if they did not. Successful inspection means all blocks were
decoded, not merely that the footer or filename looked plausible.

## Library API

* `encode(&RecordBatch, Family) -> Vec<u8>` validates a normalized schema and calls
  the specialized existing codec.
* `decode(Bytes, Option<Family>) -> RecordBatch` preflights and decodes one native
  block. `Some(family)` adds strict normalized-contract validation.
* `write_block` creates and syncs a standalone native block.
* `write_segment` creates a book segment. `write_segment_with_family` explicitly
  selects either family; neither changes the supplied Arrow schema.
* `inspect(Path, Option<Family>)` checks blocks or sealed indexed segments. It
  performs no recovery, mutation, reordering, projection or data repair.

For direct indexed storage, `gqodb_store::Writer::create_with_codec(path, schema,
time_column, symbol_column, Mode::Gqodb | Mode::GqodbBook)` selects the stored
native codec. Existing `Writer::create` remains book-compatible. `Reader::codec`
reports the validated header choice; normal reads and prefix recovery dispatch
using that same choice. Unsupported Parquet modes are rejected by this native
segment writer.

Facade segment writing preflights the supplied blocks before opening the output;
it currently recompresses them through the store writer. That extra validation
work is deliberate and must not be confused with kernel throughput benchmarks.
Writes use new paths and sync completed content, but this facade is not an atomic
catalog publisher: a disk failure can leave an incomplete new output. Runtime
publication and recovery should use their corresponding components.

## Normalized schema contract v1

Metadata keys are mandatory for strict validation:

| Key | Meaning |
|---|---|
| `gqodb.family` | `tick` or `orderbook` |
| `gqodb.schema_version` | `1` |
| `gqodb.source_sha256` | Nonzero 64-character lowercase SHA256 of source provenance |
| `gqodb.price_decimals` | Integer scale, 0–12 |
| `gqodb.quantity_decimals` | Integer scale, 0–12 |
| `gqodb.subtype` | Tick only: `trade` or `quote` |
| `gqodb.depth` | Book only: source depth per side, 1–100,000 |

Common fields: `source_utc_ns`, `received_utc_ns`, `clock_error_ns` are Int64 and may
be null; `available_utc_ns`, `sequence` are nonnullable Int64; `symbol` is
nonnullable LargeUtf8. Missing source timestamps and clock uncertainty remain null.
Trade fields are Int64 `price`, `quantity` and Int8 `side`. Quote fields are Int64
`bid`, `ask`, `bid_quantity`, `ask_quantity`. Book event fields are Int64 `price`,
`quantity`, `level` and Int8 `side`, `action`. Required payload fields are nonnullable.

Extra supported Arrow columns and all metadata are preserved. The codec does not
interpret venue-specific side/action codes, reconstruct a book, certify clock
accuracy, infer units or reject unusual market values as “bad compression”. Those
semantic controls belong in reference, quality and book components.

The runtime's separate `gqodb.runtime.layout=normalized-columns-v1` schema carries
per-record metadata templates and has its own decoder/contract. `inspect` preserves
and checks its native physical bytes; it does not falsely label that runtime schema
as the facade's normalized schema v1. Use the runtime decoder for runtime semantics.

## Preflight limits and integrity

* Native blocks at most 128 MiB, layout metadata at most 1 MiB, at most 64 normalized
  numeric columns and 1,048,576 rows (matching the underlying codecs).
* Before native decode, the facade checks a conservative expansion estimate for
  normalized vectors plus restored Arrow columns, including the maximum dictionary
  string width multiplied by rows. Estimates over 128 MiB are refused. This guards
  a tiny repeated-string dictionary expanding into huge restored arrays. The
  estimate is an allocation guard, not an exact process peak-RSS guarantee.
* Segment files at most 1 GiB, 256 indexed blocks and 16,777,216 total rows. Segment
  frames are preflighted before the store reader decodes values; block checksums,
  magic, lengths, header codec and decoded/index metadata must agree.
* Unknown versions, codec IDs, invalid CRC, truncation, extreme lengths, extra
  payload bytes, wrong logical family and unsupported schema types fail explicitly.

CRC detects corruption, not publisher authenticity. File SHA256 receipts identify
exact inspected bytes. Native decoding requires ordinary regular files; production
workspaces must prevent concurrent modification during inspection. This is an
integrity facade, not a hostile-filesystem sandbox.

## Tests

```sh
cargo test --offline -p gqodb-codec -p gqodb-store
cargo clippy --offline -p gqodb-codec -p gqodb-store --all-targets -- -D warnings
```

Tests cover both native families, real tick segment selection/read/recovery, exact
schema/metadata/null preservation, extra Float64 NaN payloads/infinity/signed zero,
misleading extensions, strict versus legacy validation, every truncated byte-prefix,
checksums, unknown versions/schema families, extreme dimensions and dictionary
expansion rejection before the native decoder allocates restored arrays. Existing
store tests continue to cover indexed ranges and crash-prefix recovery.
