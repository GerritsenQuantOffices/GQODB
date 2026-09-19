# gqodb-store — werkend prototype

Status: zelfstandige Rust-proef, geen publieke productierelease. Broncode,
tests en eventuele voorbeelden staan in deze map. Het Cargo-manifest blijft
[één niveau hoger](../Cargo.toml), zodat pakketnamen en workspacecommando's gelijk blijven.

## Doel

Experimentele .gqodb.ob-segmenten schrijven, selecteren en herstellen.

## Invoer en uitvoer

- Invoer: Genormaliseerde Arrow-levelrijen met behoud van schema en exacte waarden.
- Uitvoer: Segmenten met blokken/index, geselecteerde batches of de complete herstelbare prefix.

## Wat nu werkt

- Writer met append, checkpoint en finish.
- Geïndexeerde ranges met expliciete decode-/schemacontroles.
- Herstel van complete blokken bij fysieke truncatie; complete corruptie wordt afgewezen.

## Bestanden en toegangspunten

- [src/lib.rs](src/lib.rs): lib `gqodb_store`.
- [src/bin/ob_store.rs](src/bin/ob_store.rs): bin `ob_store`.
- [examples/storage_benchmark.rs](examples/storage_benchmark.rs): example `storage_benchmark`.
- [tests/segments.rs](tests/segments.rs): test `segments`.

## Bouwen en functioneel testen

Voer dit uit vanuit de repositoryroot, of vanuit deze prototype-map; Cargo
vindt in beide gevallen de workspace en hetzelfde package:

```sh
cargo build -p gqodb-store --locked --offline
cargo test -p gqodb-store --locked --offline
cargo clippy -p gqodb-store --all-targets --locked --offline -- -D warnings
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

- Dit historische levelrijformaat bevat nog niet alle nieuwe causale tijd-/snapshotmetadata.
- Geen volledige boekreconstructie uitsluitend uit deze oude bestanden.
- Benchmarks zijn dataset-/cacheafhankelijk; timingbenchmarks worden nu niet uitgevoerd.

## Vervolg en acceptatie

Geversioneerde uitbreiding met capturemetadata en duurzame klokstatustabel.

Behoud bestaande roundtrip-, volgorde- en fouttests bij verdere ontwikkeling.
Nieuwe integratie moet eigen contracttests krijgen; een werkend los prototype
bewijst nog geen correcte complete suite. Performancemetingen van de nieuwe
tijdregistratie staan op de todo en zijn uitgesteld.

## Verdere documentatie

- [Componentoverzicht en bestaande gebruiksdetails](../README.md).
- Alle prototypes en hun status.
- [docs/B12_STORAGE_PROTOCOL.md](../../../docs/B12_STORAGE_PROTOCOL.md).
- [results/b15/ASSESSMENT.md](../../../results/b15/ASSESSMENT.md).
