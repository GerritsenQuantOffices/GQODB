#!/usr/bin/env python3
"""Verify every published B17 result directory (results/b17-*) against its receipt.

Checks, per directory: required files; receipt fields; hashes of every result file; protocol
hash equal to the pinned one; clean-tree flag; purpose "published run"; no leak patterns;
a result file whose text was corrected after sealing must be listed in results/PUBLIC_REDACTION.json
with the receipted original hash and the hash of the public copy; completeness of the rounds; and a full recomputation of summary.json from
the raw samples (medians, ratios, bootstrap intervals, verdicts). No download, no build,
no benchmark run. Exit status 1 on any failure.

  verify_b17.py [--root REPO_ROOT]
"""
import argparse
import glob
import hashlib
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import report  # noqa: E402

REQUIRED = ["BUILD_RECEIPTS.json", "ASSESSMENT.md", "samples.json", "summary.json", "inputs.json", "plan.json"]
# Generic patterns that must never appear in published files: absolute home-directory paths,
# carrier-grade-NAT addresses (100.64.0.0/10, used by VPN overlays), e-mail addresses, private-key
# blocks, API-key prefixes and absolute mount paths. They name no person, host or project.
LEAK = re.compile("|".join([
    r"/home/[A-Za-z0-9_.-]+/",
    r"/Users/[A-Za-z0-9_. -]+/",
    r"(?<![0-9.])100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.[0-9]{1,3}\.[0-9]{1,3}(?![0-9])",
    r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+",
    r"BEGIN [A-Z ]*PRIVATE KEY",
    r"sk-ant-",
    r"/mnt/",
    r"/data/",
]), re.I)


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def verify_dir(directory, root):
    """Return a list of failure strings for one result directory (empty means verified)."""
    failures = []
    name = os.path.basename(directory.rstrip("/"))
    fail = lambda message: failures.append(f"{name}: {message}")  # noqa: E731
    for required in REQUIRED:
        if not os.path.isfile(os.path.join(directory, required)):
            fail(f"missing {required}")
    if failures:
        return failures
    receipt = json.load(open(os.path.join(directory, "BUILD_RECEIPTS.json")))
    if receipt.get("experiment") != "B17":
        fail("receipt experiment is not B17")
    if receipt.get("purpose") != "published run":
        fail(f"receipt purpose is {receipt.get('purpose')!r}, not 'published run' (fixture or smoke output is not evidence)")
    env = receipt.get("environment", {})
    if env.get("commit_dirty") is not False:
        fail("receipt does not state a clean working tree")
    if not re.fullmatch(r"[0-9a-f]{40}", str(env.get("commit", ""))):
        fail("receipt has no commit id")
    for key in ("rustc", "cargo_lock_sha256", "kernel", "cpu_model_class", "tools", "filesystem"):
        if not env.get(key):
            fail(f"receipt environment lacks {key}")
    if (env.get("filesystem") or {}).get("type") in {"tmpfs", "overlay", "ramfs"}:
        fail("benchmark ran on a filesystem where cache eviction is meaningless")
    protocol = os.path.join(root, "docs", "B17_PUBLIC_TRADES_PROTOCOL.md")
    pinned = open(os.path.join(root, "docs", "B17_PUBLIC_TRADES_PROTOCOL.sha256")).read().split()[0]
    if sha256(protocol) != pinned:
        fail("the protocol file no longer matches its pinned sha256")
    if receipt.get("protocol_sha256") != pinned:
        fail("receipt names a different protocol hash than the pinned one")
    recorded = receipt.get("result_sha256", {})
    redactions = {}
    redaction_file = os.path.join(root, "results", "PUBLIC_REDACTION.json")
    if os.path.isfile(redaction_file):
        redactions = {e["path"]: e for e in json.load(open(redaction_file)).get("files", [])}
    for filename in REQUIRED:
        if filename == "BUILD_RECEIPTS.json":
            continue
        if filename not in recorded:
            fail(f"{filename} is not covered by result_sha256")
        else:
            actual = sha256(os.path.join(directory, filename))
            if recorded[filename] == actual:
                continue
            entry = redactions.get(os.path.relpath(os.path.join(directory, filename), root).replace(os.sep, "/"))
            if entry is None:
                fail(f"{filename} does not match its receipt hash")
            elif entry.get("original_sha256") != recorded[filename] or entry.get("public_sha256") != actual:
                fail(f"{filename} does not match its receipt hash or its redaction record")
    for filename in sorted(os.listdir(directory)):
        path = os.path.join(directory, filename)
        if os.path.isfile(path) and filename != "BUILD_RECEIPTS.json" and filename not in recorded:
            fail(f"{filename} is present but not covered by the receipt")
        if os.path.isfile(path):
            text = open(path, errors="replace").read()
            if LEAK.search(text):
                fail(f"{filename} contains a pattern that must not be published: {LEAK.search(text).group(0)!r}")
    if failures:
        return failures
    samples = json.load(open(os.path.join(directory, "samples.json")))
    summary = json.load(open(os.path.join(directory, "summary.json")))
    assessment = open(os.path.join(directory, "ASSESSMENT.md")).read()
    settings = receipt.get("settings", {})
    for key in ("measured", "warmups"):
        if key not in settings:
            fail(f"receipt settings lack {key}")
    for cell_name, cell in summary["cells"].items():
        valid = len(cell["rounds_valid"])
        if valid < settings.get("measured", 15) and not cell["underpowered"]:
            fail(f"{cell_name}: only {valid} valid rounds but not marked underpowered")
        if cell["underpowered"] and "UNDERPOWERED" not in assessment:
            fail(f"{cell_name}: underpowered cell is not labelled in ASSESSMENT.md")
        warmups = {s["round"] for s in samples if s["dataset"] + "/" + s["variant"] == cell_name and s["role"] == "warmup"}
        if len(warmups) != settings.get("warmups", 3):
            fail(f"{cell_name}: {len(warmups)} warm-up rounds recorded, protocol says {settings.get('warmups', 3)}")
    if "AUTHOR TO WRITE" in assessment:
        fail("ASSESSMENT.md still contains unfilled author sections")
    recomputed = json.loads(json.dumps(report.summarise(samples)))
    if recomputed != summary:
        fail("summary.json differs from a recomputation of the raw samples")
    # every headline cell must be stated, wins and losses alike
    claims = summary.get("claims", {})
    if "wins_every_headline_axis" in claims and str(claims["wins_every_headline_axis"]) not in assessment:
        fail("ASSESSMENT.md does not state the headline verdict")
    return failures


def verify_all(root):
    results = sorted(glob.glob(os.path.join(root, "results", "b17-*")))
    failures = []
    for directory in results:
        if os.path.isdir(directory):
            failures.extend(verify_dir(directory, root))
    return results, failures


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=os.path.abspath(os.path.join(HERE, "..", "..")))
    args = parser.parse_args()
    results, failures = verify_all(args.root)
    print(f"B17 evidence: {len(results)} result directories, {len(failures)} failures")
    for failure in failures:
        print("  FAIL", failure)
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
