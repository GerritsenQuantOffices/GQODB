#!/usr/bin/env python3
"""Fail if a declared benchmark source, protocol or result is outside Git."""

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "benchmarks" / "manifest.json"


def main() -> None:
    manifest = json.loads(MANIFEST.read_text())
    output = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT)
    tracked = {item.decode() for item in output.split(b"\0") if item}
    required = {
        "benchmarks/README.md",
        "benchmarks/manifest.json",
        "benchmarks/run.sh",
        "benchmarks/verify_manifest.py",
        "results/verify_public_evidence.py",
        *manifest["benchmark_sources"],
    }
    for run in manifest["runs"]:
        required.add(run["runner"])
        required.update(run["protocols"])
        prefix = run["evidence_directory"].rstrip("/") + "/"
        if run.get("evidence_status") == "not_yet_measured":
            # Registered but not yet run (B17 stage 1): runner and protocol must be tracked, the
            # evidence directory does not exist yet. Remove the marker when results are committed.
            if any(path.startswith(prefix) for path in tracked):
                raise SystemExit(f"{run['id']}: evidence exists under {prefix}; drop evidence_status")
        elif not any(path.startswith(prefix) for path in tracked):
            raise SystemExit(f"{run['id']}: no tracked evidence under {prefix}")
        # A run may keep earlier result directories (B17: run 1 next to the main run 2).
        for extra in run.get("additional_evidence_directories", []):
            extra_prefix = extra.rstrip("/") + "/"
            if not any(path.startswith(extra_prefix) for path in tracked):
                raise SystemExit(f"{run['id']}: no tracked evidence under {extra_prefix}")

    missing = sorted(required - tracked)
    if missing:
        raise SystemExit("required benchmark files are not tracked:\n" + "\n".join(missing))

    untracked_results = sorted(
        str(path.relative_to(ROOT))
        for path in (ROOT / "results").rglob("*")
        if path.is_file() and str(path.relative_to(ROOT)) not in tracked
    )
    if untracked_results:
        raise SystemExit("result files exist outside Git:\n" + "\n".join(untracked_results))

    print(
        f"benchmark manifest: {len(manifest['runs'])} runs, "
        f"{len(manifest['benchmark_sources'])} source files, all declared files tracked"
    )


if __name__ == "__main__":
    main()
