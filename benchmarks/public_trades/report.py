"""Summaries, ASSESSMENT.md skeleton and receipt writer for B17 (protocol sections 8 to 10)."""
import hashlib
import json
import os

import stats

REFERENCE = "gqodb"
MEASURED_ROUNDS = 15
COLD_LIMIT = 0.10
HEADLINE_AXES = ["bytes", "write_op", "read_full_warm_op", "read_range_warm_op", "rss_write", "rss_full"]
HEADLINE_SET = ["gqodb", "parquet-delta-bss-lz4", "parquet-dictionary-lz4", "duckdb-native",
                "duckdb-parquet-default", "clickhouse-default", "clickhouse-tuned"]
METRIC_KIND = {"bytes": "bytes", "write_op": "time", "read_full_warm_op": "time", "read_full_cold_op": "time",
               "read_range_warm_op": "time", "read_range_cold_op": "time", "rss_write": "rss", "rss_full": "rss",
               "cpu_write": "cpu", "cpu_full": "cpu", "write_e2e": "time", "read_full_warm_e2e": "time",
               "read_range_warm_e2e": "time", "write_incl_norm": "time"}
COLD_METRICS = {"read_full_cold_op", "read_range_cold_op"}


def _find(samples, **match):
    return [s for s in samples if all(s.get(k) == v for k, v in match.items())]


def _one(samples, **match):
    found = _find(samples, **match)
    return found[0] if found else None


def valid_rounds(samples):
    """Measured rounds without any flagged sample, in chronological order."""
    measured = sorted({s["round"] for s in samples if s["role"] == "measured"})
    flagged = {s["round"] for s in samples if s.get("flags")}
    return [r for r in measured if r not in flagged]


def series(samples, candidate, metric, rounds, normalise_ns=None):
    """Per-round values of a metric for one candidate (None if a sample is missing)."""
    values = []
    for r in rounds:
        mine = [s for s in samples if s["candidate"] == candidate and s["round"] == r]
        noop = _one(mine, phase="noop")

        def pick(phase, cache=None):
            return _one(mine, phase=phase, cache=cache)

        value = None
        if metric == "bytes":
            w = pick("write")
            value = w["bytes"]["apparent"] if w else None
        elif metric in ("write_op", "write_e2e"):
            w = pick("write")
            value = (w["op_ns"] if metric == "write_op" else w["e2e_ns"]) if w else None
        elif metric == "write_incl_norm":
            w = pick("write")
            value = w["op_ns"] + normalise_ns[r] if w and normalise_ns and r in normalise_ns else None
        elif metric.startswith("read_"):
            _, which, cache, kind = metric.split("_")
            s = pick("read-full" if which == "full" else "read-range", cache)
            value = (s["op_ns"] if kind == "op" else s["e2e_ns"]) if s else None
        elif metric in ("rss_write", "rss_full", "cpu_write", "cpu_full"):
            s = pick("write") if metric.endswith("write") else pick("read-full", "warm")
            field = "rss_kb" if metric.startswith("rss") else "cpu_ns"
            if s and noop and s.get(field) is not None and noop.get(field) is not None:
                value = max(0, s[field] - noop[field])
        values.append(value)
    return values


def summarise(samples, config=None, include_flagged=False):
    """Full summary: per dataset x variant cells, ratios against gqodb and verdicts.

    include_flagged=True is the sensitivity analysis over all measured rounds (protocol 6)."""
    config = config or {}
    out = {"cells": {}, "claims": {}, "protocol_constants": {"measured_rounds": MEASURED_ROUNDS, "cold_limit": COLD_LIMIT}}
    keys = sorted({(s["dataset"], s["variant"]) for s in samples if s["phase"] != "normalise"})
    for dataset, variant in keys:
        mine = [s for s in samples if s["dataset"] == dataset and s["variant"] == variant]
        attempted = sorted({s["round"] for s in mine if s["role"] == "measured"})
        rounds = attempted if include_flagged else valid_rounds(mine)
        flagged = [r for r in attempted if r not in valid_rounds(mine)]
        candidates = sorted({s["candidate"] for s in mine if s["candidate"]})
        norm = {s["round"]: s["op_ns"] for s in samples
                if s["phase"] == "normalise" and s["dataset"] == dataset and s["role"] == "measured"}
        cell = {"rounds_valid": rounds, "rounds_attempted": len(attempted), "rounds_flagged": flagged,
                "underpowered": len(rounds) < MEASURED_ROUNDS, "candidates": {}, "comparisons": {}}
        if norm:
            vals = [norm[r] for r in rounds if r in norm]
            cell["normalise_ns"] = stats.describe(vals) if vals else None
        for candidate in candidates:
            cs = [s for s in mine if s["candidate"] == candidate]
            exact = [s["digest_ok"] for s in cs if s.get("digest_ok") is not None]
            cold = [s["resident"] for s in cs if s.get("cache") == "cold" and s.get("resident") is not None]
            entry = {"exact": all(exact) if exact else None,
                     "cold_comparable": (stats.median(cold) <= COLD_LIMIT) if cold else None,
                     "metrics": {}}
            for metric in METRIC_KIND:
                vals = series(mine, candidate, metric, rounds, norm if variant == "v2" else None)
                if metric == "write_incl_norm" and variant != "v2":
                    continue
                if vals and all(v is not None for v in vals):
                    entry["metrics"][metric] = stats.describe(vals)
                    entry["metrics"][metric]["values"] = vals
            cell["candidates"][candidate] = entry
        reference = cell["candidates"].get(REFERENCE)
        if reference:
            for candidate, entry in cell["candidates"].items():
                if candidate == REFERENCE:
                    continue
                result = {}
                for metric, kind in METRIC_KIND.items():
                    if metric not in entry["metrics"] or metric not in reference["metrics"]:
                        continue
                    if reference["exact"] is False or entry["exact"] is False:
                        result[metric] = {"verdict": "EXACTNESS FAIL"}
                    elif metric in COLD_METRICS and (entry["cold_comparable"] is False or reference["cold_comparable"] is False):
                        result[metric] = {"verdict": "NOT COMPARABLE", "reason": "cold state not reached"}
                    elif len(rounds) < 2:
                        result[metric] = {"verdict": "UNDERPOWERED", "n": len(rounds)}
                    else:
                        r = stats.compare(reference["metrics"][metric]["values"], entry["metrics"][metric]["values"],
                                          METRIC_KIND[metric], deterministic=(metric == "bytes"))
                        if cell["underpowered"]:
                            r["note"] = "UNDERPOWERED"
                        result[metric] = r
                cell["comparisons"][candidate] = result
        out["cells"][f"{dataset}/{variant}"] = cell
    out["claims"] = headline_claims(out["cells"])
    if not include_flagged:
        out["sensitivity_all_rounds"] = summarise(samples, config, include_flagged=True)["cells"]
    return out


def headline_claims(cells):
    """Section 10: a win needs BETTER on D1 and D2 (V1). Lists every cell, wins or not."""
    claims = {}
    d1, d2 = cells.get("D1/v1"), cells.get("D2/v1")
    if not d1 or not d2:
        return {"status": "D1/v1 and D2/v1 required for headline claims"}
    for baseline in HEADLINE_SET:
        if baseline == REFERENCE:
            continue
        row = {}
        for axis in HEADLINE_AXES:
            verdicts = []
            for cell in (d1, d2):
                v = cell["comparisons"].get(baseline, {}).get(axis)
                verdicts.append(v["verdict"] if v else "MISSING")
            row[axis] = {"D1": verdicts[0], "D2": verdicts[1], "win": verdicts == ["BETTER", "BETTER"]}
        claims[baseline] = row
    claims["wins_every_headline_axis"] = bool(claims) and all(
        axis["win"] for key, row in claims.items() if isinstance(row, dict) and key != "wins_every_headline_axis"
        for axis in row.values())
    return claims


def _fmt(value, unit):
    if value is None:
        return "n/a"
    if unit == "ns":
        return f"{value / 1e6:,.1f} ms"
    if unit == "kb":
        return f"{value / 1024:,.1f} MiB"
    if unit == "bytes":
        return f"{value / 1e6:,.3f} MB"
    return f"{value:,.0f}"


def render_assessment(summary, receipt_name="BUILD_RECEIPTS.json", purpose="published run"):
    """ASSESSMENT.md skeleton: generated tables, fixed sections, and marked author blanks."""
    lines = [f"# B17: public-data file-level benchmark ({purpose})", "",
             "Generated from `summary.json`; protocol `docs/B17_PUBLIC_TRADES_PROTOCOL.md`. "
             "Every cell below is reported, wins and losses alike. Verdict codes: BETTER, WORSE, "
             "EQUIVALENT, INCONCLUSIVE (ratio gqodb / baseline, 95% moving-block bootstrap interval), "
             "plus EXACTNESS FAIL, NOT COMPARABLE, UNDERPOWERED.", "",
             f"Receipt: `{receipt_name}`.", "",
             "**Input data: Binance Vision (https://data.binance.vision), PAXGUSDT spot aggregate trades.** "
             "Provided by Binance under CC BY-NC-SA 4.0 and the Binance Vision Dataset Terms (version 1.0, "
             "last updated 26 August 2026). The data are not redistributed here; fetch them yourself and comply "
             "with those terms. This assessment contains aggregates only. Binance does not sponsor, endorse or "
             "partner with this project or these results. See `docs/BASELINE_LICENCES.md`.", ""]
    for name, cell in summary["cells"].items():
        lines += [f"## {name}", "",
                  f"Valid rounds {len(cell['rounds_valid'])} of {cell['rounds_attempted']} attempted; "
                  f"flagged rounds: {len(cell['rounds_flagged'])}"
                  + ("; **UNDERPOWERED**" if cell["underpowered"] else "") + ".", ""]
        unit = {"bytes": "bytes", "rss_write": "kb", "rss_full": "kb", "cpu_write": "ns", "cpu_full": "ns"}
        metrics = ["bytes", "write_op", "read_full_warm_op", "read_full_cold_op", "read_range_warm_op",
                   "read_range_cold_op", "rss_write", "rss_full", "cpu_write", "cpu_full"]
        lines.append("| candidate | exact | " + " | ".join(metrics) + " |")
        lines.append("|" + " --- |" * (len(metrics) + 2))
        for candidate, entry in cell["candidates"].items():
            row = [candidate, {True: "yes", False: "**FAIL**", None: "n/a"}[entry["exact"]]]
            for metric in metrics:
                m = entry["metrics"].get(metric)
                row.append(_fmt(m["median"], unit.get(metric, "ns")) + (f" (MAD {_fmt(m['mad'], unit.get(metric, 'ns'))})" if m and metric != "bytes" else "") if m else "n/a")
            lines.append("| " + " | ".join(row) + " |")
        lines += ["", "Ratios gqodb / baseline (median ratio [95% interval] verdict):", ""]
        lines.append("| baseline | " + " | ".join(metrics) + " |")
        lines.append("|" + " --- |" * (len(metrics) + 1))
        for candidate, result in cell["comparisons"].items():
            row = [candidate]
            for metric in metrics:
                r = result.get(metric)
                if not r:
                    row.append("n/a")
                elif "ratio" in r:
                    row.append(f"{r['ratio']:.3f} [{r['lo']:.3f}, {r['hi']:.3f}] {r['verdict']}")
                else:
                    row.append(r["verdict"])
            lines.append("| " + " | ".join(row) + " |")
        lines.append("")
    lines += ["## Headline claims (protocol section 10)", ""]
    claims = summary["claims"]
    if "status" in claims:
        lines.append(claims["status"])
    else:
        lines.append("| baseline | " + " | ".join(HEADLINE_AXES) + " |")
        lines.append("|" + " --- |" * (len(HEADLINE_AXES) + 1))
        for baseline, row in claims.items():
            if baseline == "wins_every_headline_axis":
                continue
            lines.append("| " + baseline + " | " + " | ".join(
                f"{row[a]['D1']} / {row[a]['D2']}" + (" **win**" if row[a]["win"] else "") for a in HEADLINE_AXES) + " |")
        lines += ["", f"GQODB wins every headline axis against every headline candidate: **{claims['wins_every_headline_axis']}**."]
    lines += ["", "## What this does not show", "",
              "Aggregate trades only, one instrument, two months, one host class, one thread, no concurrency, "
              "no network; a young prototype against mature products; defaults plus one pre-registered "
              "alternative per tool; embedded libraries against a process per statement for ClickHouse. "
              "Full list: protocol section 11.", "",
              "## Interpretation", "", "AUTHOR TO WRITE AFTER THE RUN: restate every headline cell that is not BETTER "
              "in plain words, list NOT COMPARABLE and EXACTNESS FAIL cells with their reason, and give the "
              "number of flagged rounds. Do not remove this heading before it is filled in.", "",
              "## Deviations from the protocol", "", "AUTHOR TO WRITE: copy the dated entries of "
              "`docs/B17_PUBLIC_TRADES_ADDENDA.md` that apply, or state that there are none.", ""]
    return "\n".join(lines)


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def write_receipt(outdir, receipt, result_files):
    """BUILD_RECEIPTS.json in the format verify_public_evidence.py already checks."""
    receipt = dict(receipt)
    receipt["result_sha256"] = {name: sha256_file(os.path.join(outdir, name)) for name in sorted(result_files)}
    with open(os.path.join(outdir, "BUILD_RECEIPTS.json"), "w") as handle:
        json.dump(receipt, handle, indent=2, sort_keys=True)
        handle.write("\n")
    return receipt
