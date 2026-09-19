# gqodb-blocks — werkend prototype

Status: zelfstandige Rust-proef, geen publieke productierelease. Broncode,
tests en eventuele voorbeelden staan in deze map. Het Cargo-manifest blijft
[één niveau hoger](../Cargo.toml), zodat pakketnamen en workspacecommando's gelijk blijven.

## Doel

Verliesloze kolomcompressie en reproduceerbare vergelijking met Parquet.

## Invoer en uitvoer

- Invoer: Exacte Arrow-batches, synthetische blokken of expliciet aangeleverde marktdata.
- Uitvoer: Gecomprimeerde blokbytes, exact herstelde batches en expliciete benchmarkresultaten.

## Wat nu werkt

- Tick-/trade- en orderboekprofielen met integers, validity en schemaherstel.
- Batchworkers en meerdere referentie-/experimentele codecs.
- Randgevallen, corruptie en schema-/floatbitbehoud worden getest.

## Bestanden en toegangspunten

- [src/lib.rs](src/lib.rs): lib `gqodb_blocks`.
- [src/main.rs](src/main.rs): bin `gqodb-blocks`.
- [examples/inspect.rs](examples/inspect.rs): example `inspect`.
- [examples/paired_benchmark.rs](examples/paired_benchmark.rs): example `paired_benchmark`.
- [examples/real_benchmark.rs](examples/real_benchmark.rs): example `real_benchmark`.
- [tests/batch.rs](tests/batch.rs): test `batch`.
- [tests/real_roundtrip.rs](tests/real_roundtrip.rs): test `real_roundtrip`.
- [tests/roundtrip.rs](tests/roundtrip.rs): test `roundtrip`.

## Bouwen en functioneel testen

Voer dit uit vanuit de repositoryroot, of vanuit deze prototype-map; Cargo
vindt in beide gevallen de workspace en hetzelfde package:

```sh
cargo build -p gqodb-blocks --locked --offline
cargo test -p gqodb-blocks --locked --offline
cargo clippy -p gqodb-blocks --all-targets --locked --offline -- -D warnings
```

Offline werkt wanneer de dependencies uit Cargo.lock al lokaal aanwezig zijn.
Deze commando's starten geen marktcollector of performancebenchmark. Library-
prototypes worden via hun API en tests gebruikt; niet ieder onderdeel heeft
een zelfstandig serverproces. De testfixtures zijn lokaal en deterministisch.

Voor de hele workspace gelden daarnaast `cargo fmt --all --check` en
`cargo test --workspace --locked --offline`. Tests onder `prototype/tests/`
en voorbeelden onder `prototype/examples/` zijn expliciet geregistreerd in
het Cargo-manifest en mogen bij toekomstige verplaatsingen niet verdwijnen.

## Beperkingen en foutgedrag

- Dit is blokverwerking, geen complete database of live recorder.
- Een voorbeeldprogramma met benchmark in de naam is een meettool, geen standaard startcommando.
- Historische winst geldt voor de opgeslagen datasets/configuraties; er is geen universele snelheidsclaim.

## Vervolg en acceptatie

Tijdmetadata verliesloos meenemen en codecs pas na correctness opnieuw vergelijken.

Behoud bestaande roundtrip-, volgorde- en fouttests bij verdere ontwikkeling.
Nieuwe integratie moet eigen contracttests krijgen; een werkend los prototype
bewijst nog geen correcte complete suite. Performancemetingen van de nieuwe
tijdregistratie staan op de todo en zijn uitgesteld.

## Verdere documentatie

- [Componentoverzicht en bestaande gebruiksdetails](../README.md).
- Alle prototypes en hun status.
- [docs/MARKET_BENCHMARK.md](../../../docs/MARKET_BENCHMARK.md).
- [results/b15/ASSESSMENT.md](../../../results/b15/ASSESSMENT.md).

