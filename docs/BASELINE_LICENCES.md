# Licences of the baselines and of the benchmark input

Status: read on 2026-10-17. This is a record of what the published texts say, not
legal advice. Nothing here was negotiated with any vendor. The texts can change;
the retrieval digests below identify exactly what was read.

## Summary

| Item | Licence | Restriction on publishing results found in the text? | Consequence for this repository |
| --- | --- | --- | --- |
| DuckDB | MIT | None found. | DuckDB results are published. |
| ClickHouse | Apache-2.0 | None found. | ClickHouse results are published. |
| Binance Vision (benchmark input) | CC BY-NC-SA 4.0 plus "Vision Dataset Terms" v1.0 (updated 26 Aug 2026) | Non-commercial terms; see below. | No input data is committed or redistributed; results are aggregates only; users fetch the data themselves. |

## DuckDB: MIT

Source: `https://raw.githubusercontent.com/duckdb/duckdb/main/LICENSE` (sha256
`075c33400ffcb0c586dd106a029d3e733e9da3693f8fcb9cebfc651c8a2f14e3`), "Copyright
2018-2026 Stichting DuckDB Foundation". It is the standard MIT licence: permission
"to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies"
subject to keeping the notice and the "AS IS" disclaimer. The text contains no
restriction on publishing performance results. The Python wheel bundles third-party
components that carry their own (permissive) licences; those were not audited
here.

## ClickHouse: Apache-2.0

Source: `https://raw.githubusercontent.com/ClickHouse/ClickHouse/master/LICENSE`
(sha256 `af5d33e7f3dbbd25e889d8bdc61c83562c257d20c22386bb04106cd22937341a`),
"Copyright 2016-2026 ClickHouse, Inc.", Apache License Version 2.0. The text has no
restriction on publishing performance results. Apache-2.0 section 6 does not grant
trademark rights, so the name "ClickHouse" is used here only to identify the
product that was measured (nominative use) and no endorsement is implied. The
official static binary bundles third-party code under other licences; not audited.

## Benchmark input: Binance Vision

The input is public Binance aggregate trades from `https://data.binance.vision/`.
The data archive README (`binance/binance-public-data`) states "Licence: MIT" for
the repository (scripts and documentation) and refers to
`TERMS_AND_CONDITIONS.md` for the data (sha256 of the retrieved file
`99c4921dbe023cd68c1a447775707d621453cc78aec6ac0bfe2e6c06a5114ee0`, "BINANCE
VISION DATASET TERMS", version 1.0, last updated 26 August 2026). Those terms
state, among others:

> **3.1.** Datasets are provided to you under the Creative Commons
> Attribution-NonCommercial-ShareAlike 4.0 International Public License
> ("CC BY-NC-SA 4.0") ...

> **4.1.** Permitted non-commercial uses are limited to academic research,
> non-monetized open educational projects, algorithmic historical backtesting for
> purely personal non-production research, and non-commercial open-source data
> science evaluations.

> **4.4.** You shall not integrate the Datasets into commercial financial products,
> hedge fund management tools, paid subscription newsletters, or commercial
> trading bot platforms.

> **4.5.** You shall ensure that any redistributed derivative work based on the
> Datasets provides appropriate attribution to Binance Vision and remains licensed
> under the identical CC BY-NC-SA 4.0 terms.

> **2.4.** Use of any data derived from, calculated with, or extracted from the
> Datasets (including charts, models, and derivative analytical indicators) is
> strictly governed by these Vision Dataset Terms.

and clause 8.2/8.3 forbid implying Binance endorsement or using the Binance name to
market commercial applications.

Handling in this repository:

* The benchmark input is **Binance Vision (https://data.binance.vision)**, PAXGUSDT spot
  aggregate trades, and is attributed as such in the benchmark README and in every
  `ASSESSMENT.md`. Binance does not sponsor, endorse or partner with this project (clauses
  8.2 and 8.3); no Binance branding is used.
* No data file and no converted file is committed. The fetch tool downloads from the official
  host onto the user's own disk and verifies Binance's own `.CHECKSUM`; users must fetch the data
  themselves and comply with the terms above. The published result files are aggregates only:
  byte counts, timing medians, ratios and intervals, row counts and digests of whole tables;
  `inputs.json` omits the first and last trade id and all timestamps. Two places do contain
  per-trade values and are named here so that nobody has to look for them: the frozen protocol
  (`docs/B17_PUBLIC_TRADES_PROTOCOL.md`, section 2) lists the first and last aggregate trade id of
  each of the two months and quotes the first transaction time of 2025-10 as an example of the unit
  (microseconds), and the synthetic fixture `tests/fixtures/b17/edge_aggtrades.csv` starts at the
  real first aggregate trade id of 2025-10 (its prices, quantities and the remaining ids are
  synthetic). These are a handful of values; they belong to the open point below.
* Whether such aggregates count as "data derived from the Datasets" (clause 2.4) and whether a
  benchmark published by a company that sells an open-core database counts as non-commercial
  (clause 4.1) is not settled by this repository and this text is not legal advice. **On
  2026-10-17 the owner decided to use these data for the public benchmark at his own risk,
  accepting the legal uncertainty** (addendum A-003 in `docs/B17_PUBLIC_TRADES_ADDENDA.md`).
* The harness accepts any file in the same CSV layout, so a different input can replace this
  one without changing the protocol.
* The fixture files under `tests/fixtures/b17/` are synthetic and written for this repository;
  they contain no Binance data.

## Open point for the owner: clause 4.5 (share-alike) and the result files

Clause 4.5 says that a redistributed derivative work based on the Datasets must give
attribution and remain licensed under the identical CC BY-NC-SA 4.0 terms. What this
repository does: the files under `results/b17-*` (byte counts, timing medians, ratios,
intervals, row counts, digests of whole tables) are published here with the rest of the
repository under Apache-2.0; every `ASSESSMENT.md` and the benchmark README attribute the
input to Binance Vision; no data file or converted file is distributed; and the owner's
decision of 2026-10-17 (addendum A-003) accepts the legal uncertainty. What is **not**
decided: whether those aggregates are a "derivative work" in the sense of clause 4.5, and,
if they are, under which licence the data-derived result files are granted. This
repository does not grant the data-derived part of `results/b17-*` under any licence other
than the one stated for the repository, and does not claim that this is compatible with
clause 4.5. It is an open point for the owner and not a legal conclusion; it is not
settled by this text.
