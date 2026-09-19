# gqodb-types — werkend prototype

Status: zelfstandige Rust-proef, geen publieke productierelease. Broncode,
tests en eventuele voorbeelden staan in deze map. Het Cargo-manifest blijft
[één niveau hoger](../Cargo.toml), zodat pakketnamen en workspacecommando's gelijk blijven.

## Doel

Gedeelde, protocolonafhankelijke Rust-contracten voor exacte data en tijdkwaliteit.

## Invoer en uitvoer

- Invoer: L2-events, schaalinformatie, bronmetadata en expliciet aangeleverde klokmetingen.
- Uitvoer: Getypeerde events, dekking en beoordeelde UTC-intervallen of expliciet onbekende kwaliteit.

## Wat nu werkt

- L2-envelope, snapshots, deltas, gaps en resets.
- Timingmodule met afzonderlijke ontvangst-/gereedtijd en causale klokstatus.
- Integerberekeningen, leeftijdsmarge, epoch-/sessiecontrole en overflowafhandeling.

## Bestanden en toegangspunten

- [src/lib.rs](src/lib.rs): lib `gqodb_types`.
- [tests/timing.rs](tests/timing.rs): test `timing`.

## Bouwen en functioneel testen

Voer dit uit vanuit de repositoryroot, of vanuit deze prototype-map; Cargo
vindt in beide gevallen de workspace en hetzelfde package:

```sh
cargo build -p gqodb-types --locked --offline
cargo test -p gqodb-types --locked --offline
cargo clippy -p gqodb-types --all-targets --locked --offline -- -D warnings
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

- Geen host-I/O of automatische onderbouwing van het verouderingsmodel.
- Timing is companionmetadata en nog niet ingebouwd in alle containers.
- Een geschat interval bewijst geen fysieke ns-nauwkeurigheid.

## Vervolg en acceptatie

De tijdtypes verbinden met provider, journal en geversioneerde readers.

Behoud bestaande roundtrip-, volgorde- en fouttests bij verdere ontwikkeling.
Nieuwe integratie moet eigen contracttests krijgen; een werkend los prototype
bewijst nog geen correcte complete suite. Performancemetingen van de nieuwe
tijdregistratie staan op de todo en zijn uitgesteld.

## Verdere documentatie

- [Componentoverzicht en bestaande gebruiksdetails](../README.md).
- Alle prototypes en hun status.
- docs/CLOCK_SYNC_CONTRACT.md.
