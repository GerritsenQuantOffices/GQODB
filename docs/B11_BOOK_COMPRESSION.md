# B11 — Orderboekcompressie zonder gegevensverlies

Hypothese: de B06-levelprojectie bevat lange runs van identieke eventmetadata.
Runreductie vermijdt het herhaald coderen hiervan. Voor de overblijvende integers
normaliseren we exact met minimum en GCD, splitsen bytes in planes en gebruiken
safe Rust LZ4. Prijzen, hoeveelheden, volgorde en timestamps blijven exact gelijk.

Nieuw opt-in `real::Mode::GqodbBook`, apart formaat `GQOREAL4` / `GQOBOOK1`.
De bestaande `Gqodb` en de zes bestaande benchmarkmodes blijven ongewijzigd.
Een kolom gebruikt runs wanneer het aantal runs hoogstens een kwart van het
aantal rijen is; anders worden alle waarden rechtstreeks gecodeerd. Geen sorting,
kwantisatie of selectie van orderboekniveaus door de codec. Runlengtes zijn u32,
waarden reconstrueren alle i64-bitpatronen. CRC, framing en schema tellen mee.

## Vooraf gekozen vergelijking

Gebruik dezelfde bron en hetzelfde miljoen-row prefix als B06: Bybit BTC/ETH
full_orderbook_delta, 2026-09-03 06 UTC. De parser stopt op een eventgrens.
32.768 rijen per blok, één worker op CPU 2, één warmup en negen afwisselende
metingen tegen de oude gqodb-codec en alle vijf B06-Parquetconfiguraties.
Een korte screen van drie herhalingen mag vooraf fouten en regressies opsporen;
die wordt apart bewaard en is geen onafhankelijk bevestigingsresultaat.

```sh
cargo build -p gqodb-blocks --release --locked --offline --example market_benchmark
GQODB_BOOK_CODEC=1 taskset -c 2 target/release/examples/market_benchmark book SOURCE OUTPUT.json 1048576 32768 9 0 0
```

Iedere decode moet bit-exact dezelfde Arrow-batch inclusief schema, strings en
validity opleveren. Vergelijk complete encode- en decodepasses, bytes en RSS.
Geen IO-, durability-, volledige boekreplay- of L10-snapshotclaim. Originele
bestanden blijven onaangeroerd. Een eigen codec wint pas op alle drie assen als
grootte, encoding en decoding daadwerkelijk beter zijn dan de sterkste relevante
controle; een kleinere output alleen is geen algemene prestatiewinst.

Na de vaste vergelijking: één afzonderlijk uur als generalisatiecontrole indien
lokaal beschikbaar, zonder selectie op gunstige codecresultaten. Geen automatische
promotie naar standaardcodec op basis van een enkel positief sample.
De lokale volgende uurfile (2026-09-03 07 UTC, dezelfde collector-ID) is aanwezig
en wordt vóór de eerste meetuitkomst gekozen als deze generalisatiecontrole.

De eerste screen (`screen.json`) gaf 3.692.593 bytes, maar 64,746 ms decoding
tegen 34,982 ms voor dictionary-Parquet. Daarom wordt vóór de vaste vergelijking
de decoder aangepast: aaneengesloten byteplanes reconstrueren in vectoriseerbare
lussen. Framing en bytes blijven hetzelfde; de screen blijft als ontwikkelmeting
behouden. Er wordt geen decodecontrole verwijderd.
