"""Check the public evidence against its receipts and recompute the published figures.

No download, no benchmark run and no build: this only reads files under results/.
Exit status is 1 when any receipt or redaction check fails.
"""
from pathlib import Path
import hashlib
import json
import statistics
import sys

ROOT = Path(__file__).resolve().parents[1]


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(relative):
    return json.loads((ROOT / relative).read_text())


redaction = load("results/PUBLIC_REDACTION.json")
redacted = {entry["path"]: entry for entry in redaction["files"]}

checks = []
for receipt in sorted(ROOT.glob("results/*/BUILD_RECEIPTS.json")):
    for name, recorded in json.loads(receipt.read_text()).get("result_sha256", {}).items():
        relative = str((receipt.parent / name).relative_to(ROOT))
        actual = sha256(ROOT / relative)
        if actual == recorded:
            status = "original bytes"
        elif relative in redacted and redacted[relative]["original_sha256"] == recorded \
                and redacted[relative]["public_sha256"] == actual:
            status = "redacted copy of the receipted original"
        else:
            status = "MISMATCH"
        checks.append({"path": relative, "status": status})

stale = [entry["path"] for entry in redaction["files"]
         if sha256(ROOT / entry["path"]) != entry["public_sha256"]]

# B15 — order-book files: six measured rounds after one warm-up, medians.
b15 = load("results/b15/storage.json")
table = []
for mode in ("Gqodb", "DictionaryLz4", "DeltaLz4"):
    rows = [x for x in b15["samples"] if x["mode"] == mode and x["round"] != b15["warmup_round"]]
    assert len(rows) == 6, f"B15 {mode}: expected six measured rounds, found {len(rows)}"
    table.append({"mode": mode,
                  "bytes": statistics.median(x["bytes"] for x in rows),
                  "write_sync_ms": statistics.median(x["write_sync_ns"] for x in rows) / 1e6,
                  "read_ms": statistics.median(x["read_ns"] for x in rows) / 1e6})
b15_savings = {key: 100 * (1 - table[0][key] / table[1][key])
               for key in ("bytes", "write_sync_ms", "read_ms")}

# B11 — order-book codec in memory: per-codec medians as recorded in each sample file.
b11 = {}
for sample in ("hour06", "hour07"):
    summary = {row["mode"]: row for row in load(f"results/b11/{sample}.json")["summary"]}
    book = summary["gqodb-book-runs-planes-lz4"]
    parquet = summary["parquet-dictionary-lz4"]
    previous = summary["gqodb-adaptive-lz4"]
    b11[sample] = {
        "book_mb": book["bytes"] / 1e6,
        "book_encode_ms": book["encode_median_ns"] / 1e6,
        "book_decode_ms": book["decode_median_ns"] / 1e6,
        "parquet_mb": parquet["bytes"] / 1e6,
        "parquet_decode_ms": parquet["decode_median_ns"] / 1e6,
        "smaller_than_parquet_percent": 100 * (1 - book["bytes"] / parquet["bytes"]),
        "smaller_than_previous_percent": 100 * (1 - book["bytes"] / previous["bytes"]),
        "decode_longer_than_parquet_percent": 100 * (book["decode_median_ns"] / parquet["decode_median_ns"] - 1),
    }

# B06 — tick codec on aggregate trades, plus the order-book projection it lost.
# No build receipts exist for B06: these figures are historically measured, not hash-checked.
b06 = {}
for sample in ("eth-final", "uni-final", "bybit-levels"):
    rows = load(f"results/b06/{sample}.json")["summary"]
    gqodb = next(r for r in rows if r["mode"] == "gqodb-adaptive-lz4")
    parquet = [r for r in rows if r["mode"].startswith("parquet-")]
    b06[sample] = {
        "gqodb": {"mb": gqodb["bytes"] / 1e6, "encode_ms": gqodb["encode_median_ns"] / 1e6,
                  "decode_ms": gqodb["decode_median_ns"] / 1e6},
        "parquet_configurations": len(parquet),
        "gqodb_best_on": [axis for axis in ("bytes", "encode_median_ns", "decode_median_ns")
                          if all(gqodb[axis] < r[axis] for r in parquet)],
    }

failed = [c for c in checks if c["status"] == "MISMATCH"]
print(json.dumps({
    "receipted_files_checked": len(checks),
    "original_bytes": sum(c["status"] == "original bytes" for c in checks),
    "redacted_copies": sum(c["status"].startswith("redacted") for c in checks),
    "mismatches": failed,
    "redaction_records_out_of_date": stale,
    "b15_medians": table,
    "b15_savings_vs_dictionary_lz4_percent": b15_savings,
    "b11": b11,
    "b06_historically_measured": b06,
}, indent=2))
s = b15_savings
smaller = [v["smaller_than_parquet_percent"] for v in b11.values()]
slower = [v["decode_longer_than_parquet_percent"] for v in b11.values()]
print(f"receipts: {len(checks)} checked, {len(failed)} mismatches, "
      f"{len(stale)} stale redaction records\n"
      f"B15 vs dictionary-LZ4: bytes -{s['bytes']:.4f}%, write -{s['write_sync_ms']:.4f}%, "
      f"read -{s['read_ms']:.4f}%\n"
      f"B11 book profile vs dictionary-LZ4: {min(smaller):.1f}-{max(smaller):.1f}% smaller, "
      f"decode {min(slower):.1f}-{max(slower):.1f}% longer", file=sys.stderr)
sys.exit(1 if failed or stale else 0)
