# gqodb-types

De werkende broncode en tests staan onder [`prototype/`](prototype/README.md).
Daar staan bouwcommando's, invoer/uitvoer, beperkingen en vervolgstappen.
Het Cargo-manifest blijft hier; bestaande packagecommando's blijven geldig.

Status: werkende Rust-crate voor exacte L2-events, envelopes en dekking.
Alleen serde als dependency; geen FIX- of brokerdependency. Beschikbaarheid
en ontvangsttijd gebruiken een genormaliseerde lokale/monotone ns-klok.
De CLI-replaywrapper bewaart daarnaast UTC-times en een clock epoch.

## Gedeelde tijdmodule

`gqodb_types::timing` bevat een zelfstandig werkende Rust-module die als
metadata naast ticks, book-events en nieuws kan worden gebruikt:

- `SourceTime`: optionele genormaliseerde UTC-brontijd met resolutie en betekenis.
- `ClockReading`: originele UTC plus monotone meting, uitleesbreedte en klokdomein.
- `ClockState`: onveranderlijke meetstatus met collectie-/publicatietijd,
  geldigheidsduur, foutschatting en expliciete verouderingsmarge in ppb.
- `AssessedTime::assess`: berekent een geschat interval of geeft expliciet aan
  waarom kwaliteit ontbreekt. De UTC-waarde blijft ongewijzigd.
- `EventTiming::new`: koppelt bron, ontvangst en beschikbaarheid aan een
  capture-sequence. Controleert lokale volgorde, sessie en klokepoch.

Een status moet vóór het begin van de UTC-uitleesbracket beschikbaar zijn.
Verlopen, toekomstige, ongesynchroniseerde of anderszins onbruikbare evidence
levert geen interval op; dit verhindert het bewaren van de ruwe tijd niet.
Foutmarges groeien vanaf het begin van de meting, ronden naar boven af en
controleren overflow. Ontvangst en gereedtijd worden onafhankelijk beoordeeld.
`available_by_estimated_utc` gebruikt het boveneinde van het beschikbaarheids-
interval; de replaydriver moet daarnaast streamvolgorde en afhankelijkheden bewaken.

De module bevat geen I/O, achtergrondthreads of globale klokwijzigingen en
voert geen hostkalibratie uit. De provider moet de gebruikte fout-/groeimarges
onderbouwen en een unieke sessie-ID aan host/boot binden. Dit is geen fysieke
nauwkeurigheidsclaim of authenticatie van aangeleverde klokstatus.

De types zijn companionmetadata: nog niet gekoppeld aan het bestaande
`BookEvent`-Envelope, de CLI of de tick-/orderboekcontainers. Oude schemas blijven
ongewijzigd. `Serialize` is beschikbaar voor Rust-diagnostiek, met integervelden;
de toekomstige publieke JSON-presentatie moet absolute ns als strings afbeelden.
Beoordeelde types hebben private velden en geen `Deserialize`: een importer moet
de oorspronkelijke inputs opnieuw beoordelen in plaats van een opgeslagen
kwaliteitslabel klakkeloos te vertrouwen.

Validatie: `cargo test -p gqodb-types --locked --offline` (zeven tests).
Snelheidsmetingen staan op de todo.

## Breder datamodel

Gedeelde contracten voor marktdata, schema's, instrumenten, schaalversies en
eventidentiteit. Tick- en orderboekprofielen mogen deze betekenis niet wijzigen.

- Envelope: venue, instrument, stream, epoch, bronsequence, ontvangstvolgorde,
  brontijd, beschikbaarheidstijd, schemaversie en kwaliteitsflags.
- Exacte prijs-/hoeveelheidsrepresentatie en expliciete null-/overflowregels.
- Afzonderlijke eventtypen voor quotes, trades, snapshots, deltas, gaps en resets.
- Geen brokerlogica, modeltraining of database-IO in deze module.
- Transportsequence, eventuele applicatiesequence en ontvangstvolgorde blijven
  apart; commitgroepen behouden bronbericht-ID en groepsindex. Geen verplichte
  FIX-types. Zie adaptercontract.

Eerste toets: deterministische serialisatie en exact roundtrip van randgevallen.
Zie het gedeelde ontwerp.
