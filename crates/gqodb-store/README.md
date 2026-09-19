# gqodb-store

De werkende broncode en tests staan onder [`prototype/`](prototype/README.md).
Daar staan bouwcommando's, invoer/uitvoer, beperkingen en vervolgstappen.
Het Cargo-manifest blijft hier; bestaande packagecommando's blijven geldig.

Status: zelfstandig werkend Rust-prototype voor geïndexeerde `.gqodb.ob`-bestanden.
Het formaat is experimenteel; geen definitieve publieke compatibiliteitsbelofte.

**B15:** verdere decoderoptimalisatie geeft circa 17% kortere warme leestijd
dan B13. In de volledige bestandsproef: 198 ms schrijven+sync en 40,0 ms lezen,
tegenover 313 ms en 41,6 ms voor dictionary-LZ4 Parquet; 3,70 tegenover 5,95 MB.
Dit is één orderboeksample, geen algemene of cold-IO-garantie. Zie
[laatste resultaten](../../results/b15/ASSESSMENT.md).

**B13 (eerdere meting):** indexvalidatie en bufferhergebruik zijn verbeterd zonder formaatwijziging.
In drie warme meetparen leest hetzelfde bestand in 47,5–47,7 ms in plaats van
115–166 ms; korte queries kosten circa 3 ms. Dictionary-Parquet was toen sneller
bij volledig lezen. Zie [resultaten](../../results/b13/ASSESSMENT.md).
`profile_read_all` biedt afzonderlijke diagnostische fasetimings; gewone reads
voeren die extra klokmetingen niet uit.

## Werkt nu

- `Writer::create` maakt uitsluitend een nieuw bestand. `append` schrijft een
  complete Arrow-batch met de B11-orderboekcodec en behoudt schema/metadata.
- `Writer::create_with_codec(..., Mode::Gqodb | Mode::GqodbBook)` kiest expliciet
  de bestaande adaptieve tickcodec of de orderboekcodec. `Reader::codec()` toont
  die keuze; lezen en prefixherstel volgen het opgeslagen codec-ID. Bestaande
  orderboekbestanden en `Writer::create` behouden hun oorspronkelijke gedrag.
  De [codec-facade](../gqodb-codec/README.md) biedt native dispatch en validatie
  voor zowel `.gqodb.tick` als `.gqodb.ob`; de extensie bepaalt de decoder niet.
- `checkpoint` synchroniseert geschreven bytes; `finish` schrijft index/footer
  en synchroniseert het bestand. Append alleen is geen duurzame bevestiging.
- `Reader::open` leest header/index, zonder alle datablokken te decomprimeren.
  `query(symbol, start_ns, end_ns)` gebruikt inclusieve grenzen en slaat blokken
  over via tijd-min/max en symbolensets. Kandidaten worden exact gefilterd.
  Ongesorteerde en late rijen behouden hun oorspronkelijke volgorde.
- `read_all` verzamelt alle batches; `visit_blocks` houdt één batch tegelijk
  vast voor verificatie/export. Queryresultaten worden voorlopig in RAM verzameld.
- `Reader::recover` leest zonder de bron te wijzigen het volledige geldige
  blokprefix uit een onafgesloten bestand. Volledige corrupte frames falen;
  een fysiek onvolledige laatste frame wordt niet als volledig blok aangemerkt.

Indexvelden zijn expliciet: een Int64-tijd in genormaliseerde nanoseconden en
een LargeUtf8-symbool, beide zonder nulls. De overige velden volgen de ondersteunde
Arrow-types van de bestaande codec. De opslaglaag reconstrueert geen boek en
begrensst geen historische kennis; causale toegang hoort bij `gqodb-replay`.

## Gebruik

```sh
cargo build -p gqodb-store --release --locked --offline --bins --examples
target/release/ob_store inspect bestand.gqodb.ob
target/release/ob_store verify bestand.gqodb.ob
target/release/ob_store query bestand.gqodb.ob BTCUSDT 1788415200000000000 1788415200010000000
target/release/ob_store recover onderbroken.gqodb.ob hersteld.gqodb.ob
```

De query-CLI toont aantallen en gelezen blokbytes; de Rust-API retourneert batches.
Recovery schrijft naar een nieuw bestand en weigert bestaande doelen. De importer
voor de benchmark gebruikt exact dezelfde bronprojectie als B06/B11.

## Container v1

Alles little-endian. Frameheader: magic (8 bytes), metadatalengte (u32),
payloadlengte (u32), CRC32 van metadata+payload (u32), CRC32 van de eerste
20 headerbytes (u32). Daarna metadata en payload.

1. `GQOBHDR1`: JSON-schema, indexkolomnamen en codec-ID; geen payload.
2. N maal `GQOBBLK1`: JSON-blokstatistieken en B11-payload (`GQOREAL4/GQOBOOK1`).
   Metadata bevat offset, rowcount, min/max, symbolen en lengte nul; de index
   bevat de werkelijke framelengte om zelfreferentiële lengtes te vermijden.
3. `GQOBIDX1`: JSON-blokdirectory in payload; geen metadata.
4. Trailer: `GQOBEND1` (8 bytes), indexoffset (u64), CRC32 over die 16 bytes (u32).

Maximaal 100.000 blokken; metadata/index maximaal 16 MiB, payload per frame
maximaal 128 MiB, rijen per blok volgens de codecgrens. CRC detecteert accidentele
beschadiging en is geen cryptografisch authenticiteitsbewijs. Normaal openen
controleert indexstructuur; datablokken worden gecontroleerd bij lezen. Gebruik
`verify` voor controle van het hele segment, ook ongequeryde blokken.

## Getest en nog niet bewezen

Tests dekken exact roundtrip, gerichte selectie, late rijen, iedere mogelijke
afkapbyte van een fixture, corrupte blokken, onafgesloten checkpoints, schemafouten,
weigeren van overschrijven en herstel via CLI naar een nieuw bestand.
Zie [B12-meetprotocol](../../docs/B12_STORAGE_PROTOCOL.md) en
[metingen](../../results/b12/ASSESSMENT.md).

Geen directory-fsync, atomische publicatie/catalogus, concurrente writer/reader,
append-hervatting op een beschadigd bestand, `.gqodb.tick`-writer of compaction.
Filesystem/power-lossgaranties en ENOSPC/kill-injectie zijn nog niet bewezen.
Een ontbrekend bestand of verloren niet-gesynchroniseerde bytes zijn niet
herstelbaar door een checksum. Live ingestie en busintegratie volgen later.

## Verdere ontwerpdoelen

Immutable `.gqodb.tick`-/`.gqodb.ob`-segmenten, catalogus, blokdirectory,
indexen, atomische publicatie en begrensde compaction. Geen maandbestand
herschrijven bij iedere nieuwe batch.

Bewaar de relatie tussen segment, schema, bronwatermark, checksums en duurzame
receipt. Dictionary's, metadata, indexen en checkpoints tellen mee in opslag.
Readers pinnen een generatie; retentie verwijdert geen nog benodigde data.

Eerste toets: complete publicatie versus crash-halfbestand, range-selectie,
disk-full en late correcties. De bestaande RAM-codecbenchmark is geen bewijs
dat deze schijf- en herstelketen al werkt.
