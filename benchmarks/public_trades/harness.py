#!/usr/bin/env python3
"""B17 harness: plan, prepare inputs, run the pre-registered matrix, write the evidence.

  harness.py plan     [--datasets D1,D2] [--json]            what a run would do (nothing is executed)
  harness.py prepare  --work DIR [--datasets D1,D2] [--zip-dir DIR]
  harness.py measure  --work DIR --out RESULTS_DIR [--datasets ...]    needs B17_MEASURE_GO=1
  harness.py smoke    --work DIR                              fixture-scale functional check, no evidence
  harness.py report   --samples FILE --out DIR                summary.json + ASSESSMENT.md from raw samples
  harness.py seal     --out DIR                               refresh the receipt hashes after editing ASSESSMENT.md

Everything is specified in docs/B17_PUBLIC_TRADES_PROTOCOL.md. The measure command refuses to run
unless B17_MEASURE_GO=1 is set (the timed matrix is stage 2 of the work package) and refuses a
dirty working tree and a tmpfs/overlay work directory.
"""
import argparse
import datetime
import itertools
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, HERE)
import common  # noqa: E402
import report  # noqa: E402

# (id, family, in headline set H)
CANDIDATES = [
    ("gqodb", "rust", True), ("parquet-delta-bss-lz4", "rust", True), ("parquet-dictionary-lz4", "rust", True),
    ("parquet-delta-bss-snappy", "rust", False), ("parquet-delta-bss-brotli5", "rust", False),
    ("duckdb-native", "duckdb", True), ("duckdb-parquet-default", "duckdb", True),
    ("duckdb-parquet-matched", "duckdb", False), ("duckdb-parquet-zstd", "duckdb", False),
    ("clickhouse-default", "clickhouse", True), ("clickhouse-tuned", "clickhouse", True),
]
DATASETS = {
    "D1": {"month": "2025-10", "rows": 1048576, "headline_only": False},
    "D2": {"month": "2026-02", "rows": 1048576, "headline_only": False},
    "S1": {"month": "2025-10", "rows": None, "headline_only": True},
    "S2": {"month": "2026-02", "rows": None, "headline_only": True},
}
SYMBOL = "PAXGUSDT"
PROFILE = {"official": {"warmups": 3, "measured": 15, "extra": 25},
           "smoke": {"warmups": 1, "measured": 2, "extra": 1},
           "quick": {"warmups": 0, "measured": 1, "extra": 0}}
REPEATED_PER_ROUND = {"source-read": ["gqodb", "duckdb-native", "clickhouse-default"]}
BAD_FILESYSTEMS = {"tmpfs", "overlay", "ramfs", "squashfs", "proc", "sysfs"}
USER_HZ = 100
SIBLING_MIN_JIFFIES = 5
SIBLING_FRACTION = 0.10
LOW_CPU_MIN_NS = 100_000_000


# ------------------------------------------------------------------ resolving tools

def target_dir():
    return os.environ.get("B17_TARGET") or os.environ.get("CARGO_TARGET_DIR") or os.path.join(ROOT, "target")


def example(name):
    return os.path.join(target_dir(), "release", "examples", name)


def adapter_prefix(family):
    python = os.environ.get("B17_PYTHON") or sys.executable
    if family == "rust":
        return [example("trades_candidate")]
    script = {"duckdb": "duckdb_adapter.py", "clickhouse": "clickhouse_adapter.py"}[family]
    return [python, os.path.join(ROOT, "benchmarks", "baselines", script)]


# ------------------------------------------------------------------ system probes

def read_proc_stat():
    cpus = {}
    with open("/proc/stat") as handle:
        for line in handle:
            m = re.match(r"cpu(\d+) (.*)", line)
            if m:
                v = [int(x) for x in m.group(2).split()]
                cpus[int(m.group(1))] = (v[0] + v[1] + v[2] + v[5] + v[6] + v[7], sum(v[:8]))
    return cpus


def sibling_of(cpu):
    with open(f"/sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list") as handle:
        text = handle.read().strip()
    members = []
    for part in text.split(","):
        a, _, b = part.partition("-")
        members.extend(range(int(a), int(b or a) + 1))
    others = [m for m in members if m != cpu]
    return others[0] if others else None


def wait_quiet(excluded, foreign=(), limit=4.0, window=10.0, poll=10.0, timeout=1800.0):
    """Protocol section 6: box-wide busy CPUs outside `excluded` must stay at or below `limit`.

    `excluded` holds the benchmark cores, their siblings, the harness core and the declared
    foreign cores (addendum A-003). Returns (counted busy CPUs, busy CPUs on the foreign cores)
    of the window that satisfied the rule."""
    waited = 0.0
    while True:
        a = read_proc_stat()
        time.sleep(window)
        b = read_proc_stat()
        share = {c: (b[c][0] - a[c][0]) / max(1, b[c][1] - a[c][1]) for c in b}
        busy = sum(v for c, v in share.items() if c not in excluded)
        if busy <= limit:
            return busy, sum(share.get(c, 0.0) for c in foreign)
        waited += window + poll
        if waited > timeout:
            raise SystemExit(f"box not quiet: {busy:.1f} busy CPUs outside the benchmark cores for {timeout:.0f}s")
        time.sleep(poll)


def filesystem_info(path):
    real = os.path.realpath(path)
    best, fstype = "", None
    with open("/proc/mounts") as handle:
        for line in handle:
            parts = line.split()
            mount = parts[1].replace("\\040", " ")
            if (real == mount or real.startswith(mount.rstrip("/") + "/")) and len(mount) > len(best):
                best, fstype = mount, parts[2]
    rotational = None
    st = os.stat(real)
    for candidate in (f"/sys/dev/block/{os.major(st.st_dev)}:{os.minor(st.st_dev)}/queue/rotational",):
        try:
            rotational = bool(int(open(candidate).read().strip()))
        except OSError:
            pass
    return {"type": fstype, "rotational": rotational}


def cpu_model_class():
    try:
        for line in open("/proc/cpuinfo"):
            if line.startswith("model name"):
                name = line.split(":", 1)[1].strip()
                return re.sub(r"\s+@.*$", "", re.sub(r"\((R|TM)\)", "", name)).strip() + " class"
    except OSError:
        pass
    return platform.processor() or "unknown"


# ------------------------------------------------------------------ running one adapter process

def run_adapter(prefix, phase, argv, cpu, env=None):
    command = ["taskset", "-c", str(cpu)] + prefix + [phase] + argv
    sib = sibling_of(cpu)
    before = read_proc_stat()
    load = open("/proc/loadavg").read().split()[0]
    with tempfile.TemporaryFile() as errors:
        started = time.perf_counter_ns()
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=errors, env=env)
        out = process.stdout.read()
        _pid, status, usage = os.wait4(process.pid, 0)
        process.stdout.close()
        process.returncode = os.waitstatus_to_exitcode(status)
        if phase == "write" and process.returncode == 0:
            common.fsync_tree(argv[argv.index("--store") + 1])
        e2e = time.perf_counter_ns() - started
        errors.seek(0)
        stderr = errors.read().decode(errors="replace")[-400:]
    after = read_proc_stat()
    payload = None
    for line in reversed(out.decode(errors="replace").splitlines()):
        if line.strip().startswith("{"):
            try:
                payload = json.loads(line)
            except ValueError:
                payload = None
            break
    sibling_busy = (after[sib][0] - before[sib][0]) if sib is not None else 0
    return {"rc": process.returncode, "e2e_ns": e2e, "payload": payload, "stderr": stderr,
            "proc_cpu_ns": int((usage.ru_utime + usage.ru_stime) * 1e9), "sibling_busy_jiffies": sibling_busy,
            "wall_jiffies": e2e / 1e9 * USER_HZ, "loadavg1": float(load)}


def make_sample(meta, result, expected=None):
    payload = result["payload"] or {}
    flags = []
    if result["rc"] != 0 or not payload:
        flags.append("process_failed")
    # Thresholds of addendum A-004 (the registered 2 jiffies / 2% sat below the box's normal
    # housekeeping noise on an idle sibling CPU).
    if result["sibling_busy_jiffies"] > max(SIBLING_MIN_JIFFIES, SIBLING_FRACTION * result["wall_jiffies"]):
        flags.append("sibling_busy")
    if meta["phase"] in ("read-full", "read-range") and meta.get("cache") == "warm" and result["e2e_ns"] >= LOW_CPU_MIN_NS:
        if result["proc_cpu_ns"] / result["e2e_ns"] < 0.85:
            flags.append("low_cpu_ratio")
    sample = dict(meta, op_ns=int(payload.get("op_ns", 0)), e2e_ns=result["e2e_ns"],
                  cpu_ns=payload.get("cpu_ns"), rss_kb=payload.get("rss_hwm_kb"),
                  proc_cpu_ns=result["proc_cpu_ns"], rows=payload.get("rows"),
                  sibling_busy_jiffies=result["sibling_busy_jiffies"], loadavg1=result["loadavg1"],
                  rc=result["rc"], flags=flags)
    if result["rc"] != 0:
        sample["stderr"] = result["stderr"]
    if expected is not None and meta["phase"] in ("read-full", "read-range", "source-read", "normalise"):
        sample["digest_ok"] = payload.get("digest") == expected
    for key in ("parts", "version", "effective", "blocks", "blocks_read", "raw_wall_ns", "startup_wall_ns", "resorted", "denormalise_ns"):
        if key in payload:
            sample[key] = payload[key]
    return sample


# ------------------------------------------------------------------ plan

def selected_candidates(dataset):
    spec = DATASETS.get(dataset, {"headline_only": False})
    return [c for c in CANDIDATES if c[2] or not spec["headline_only"]]


def plan(datasets, profile="official"):
    p = PROFILE[profile]
    attempts = p["warmups"] + p["measured"] + p["extra"]
    rows = []
    total_min = total_max = 0
    for dataset in datasets:
        cands = selected_candidates(dataset)
        for variant in ("v1", "v2"):
            per_round = len(cands) * 6 + len(REPEATED_PER_ROUND["source-read"]) + (1 if variant == "v2" else 0)
            low = (p["warmups"] + p["measured"]) * per_round
            high = attempts * per_round
            total_min += low
            total_max += high
            rows.append({"dataset": dataset, "variant": variant, "candidates": [c[0] for c in cands],
                         "phases_per_candidate_round": ["noop", "write", "read-range cold", "read-full cold",
                                                        "read-range warm", "read-full warm"],
                         "processes_per_round": per_round, "rounds": f"{p['warmups']} warm-up + {p['measured']} measured "
                         f"(+ up to {p['extra']} repeats)", "adapter_processes_min": low, "adapter_processes_max": high})
    return {"profile": profile, "cells": rows, "adapter_processes_min": total_min, "adapter_processes_max": total_max}


# ------------------------------------------------------------------ preparing inputs

def run_json(command, **kwargs):
    done = subprocess.run(command, capture_output=True, text=True, **kwargs)
    if done.returncode != 0:
        raise SystemExit(f"command failed: {' '.join(command[:3])}...: {done.stderr[-400:]}")
    return json.loads(done.stdout.strip().splitlines()[-1])


def reference_matches(r1, r2):
    keys = ("rows", "digest")
    ok = all(r1[k] == r2[k] for k in keys) and all(r1["window"][k] == r2["window"][k] for k in ("start", "end", "rows", "digest"))
    return ok


def prepare_from_csv_lines(name, csv_stream_command, lines_factory, work, rows):
    """Convert one CSV stream into V1 and V2 sources and compare the Rust reference (R1) with
    the independent Python reference (R2). Returns the inputs record."""
    base = os.path.join(work, "inputs", name)
    os.makedirs(base, exist_ok=True)
    record = {"dataset": name, "variants": {}}
    for variant in ("v1", "v2"):
        out = os.path.join(base, f"source-{variant}.parquet")
        if os.path.exists(out):
            os.remove(out)
        command = [example("trades_convert"), "csv", "--csv", "-", "--variant", variant, "--rows",
                   "all" if rows is None else str(rows), "--out", out, "--report", out + ".json"]
        feeder = subprocess.Popen(csv_stream_command, stdout=subprocess.PIPE)
        done = subprocess.run(command, stdin=feeder.stdout, capture_output=True, text=True)
        feeder.stdout.close()
        feeder.wait()
        if done.returncode != 0:
            raise SystemExit(f"converter failed for {name}/{variant}: {done.stderr[-400:]}")
        r1 = json.loads(done.stdout.strip().splitlines()[-1])
        r2 = common.csv_reference(lines_factory(), variant)
        if not reference_matches(r1, r2):
            raise SystemExit(f"{name}/{variant}: the Rust and the Python reference digests differ; aborting")
        record["variants"][variant] = {"source": os.path.relpath(out, work), "report": r1, "python_reference": r2,
                                       "references_agree": True}
    record["variants"]["v2"]["normalise_check"] = check_normalise(work, record)
    return record


def check_normalise(work, record):
    v1 = os.path.join(work, record["variants"]["v1"]["source"])
    done = subprocess.run([example("trades_candidate"), "normalise", "--candidate", "gqodb", "--variant", "v1",
                           "--source", v1, "--store", os.path.join(work, "unused")], capture_output=True, text=True)
    if done.returncode != 0:
        return {"float_path": "refused", "reason": done.stderr.strip()[-200:]}
    digest = json.loads(done.stdout.strip().splitlines()[-1])["digest"]
    agree = digest == record["variants"]["v2"]["report"]["digest"]
    if not agree:
        raise SystemExit("V2 by float path differs from V2 by text path: V2 not available")
    return {"float_path": "agrees_with_text_path"}


def prepare_dataset(name, work, zip_dir=None):
    """Fetch/verify, convert and cross-check one dataset; a finished record is reused on resume."""
    cache = os.path.join(work, "inputs", name, "record.json")
    if os.path.exists(cache):
        record = json.load(open(cache))
        if all(os.path.exists(os.path.join(work, v["source"])) for v in record["variants"].values()):
            return record
    record = _prepare_dataset(name, work, zip_dir)
    with open(cache + ".tmp", "w") as handle:
        json.dump(record, handle, sort_keys=True)
    os.replace(cache + ".tmp", cache)
    return record


def _prepare_dataset(name, work, zip_dir=None):
    spec = DATASETS[name]
    fetch = os.path.join(HERE, "fetch.sh")
    command = [fetch, SYMBOL, spec["month"], "--out", os.path.join(work, "dl")]
    zipname = f"{SYMBOL}-aggTrades-{spec['month']}.zip"
    if zip_dir and os.path.exists(os.path.join(zip_dir, zipname)):
        command += ["--zip", os.path.join(zip_dir, zipname), "--checksum", os.path.join(zip_dir, zipname + ".CHECKSUM")]
    done = subprocess.run(command, capture_output=True, text=True)
    if done.returncode != 0:
        raise SystemExit(f"fetch refused or failed ({done.returncode}): {done.stderr.strip()[-300:]}")
    zip_path = done.stdout.strip().splitlines()[-1]
    receipt = json.load(open(os.path.join(work, "dl", zipname + ".fetch.json")))

    import zipfile

    def lines():
        archive = zipfile.ZipFile(zip_path)
        member = archive.namelist()[0]
        with archive.open(member) as handle:
            for number, line in enumerate(handle):
                if spec["rows"] is not None and number >= spec["rows"]:
                    return
                yield line.rstrip(b"\n")

    record = prepare_from_csv_lines(name, ["unzip", "-p", zip_path], lines, work, spec["rows"])
    record["input"] = {"host": receipt["host"], "key": receipt["key"], "zip_sha256": receipt["zip_sha256"], "zip_bytes": receipt["zip_bytes"],
                       "pinned_sha256": receipt["pinned_sha256"]}
    return record


def prepare_fixtures(work):
    """Fixture datasets: FX1 edge CSV, FX3 normalisable CSV, FX2 synthetic edge table."""
    fixtures = os.path.join(ROOT, "tests", "fixtures", "b17")
    records = {}
    for name, filename in (("FX1", "edge_aggtrades.csv"), ("FX3", "normalisable_aggtrades.csv")):
        path = os.path.join(fixtures, filename)

        def lines(path=path):
            return [line.rstrip(b"\n") for line in open(path, "rb")]

        records[name] = prepare_from_csv_lines(name, ["cat", path], lines, work, None)
        records[name]["input"] = {"fixture": filename}
    base = os.path.join(work, "inputs", "FX2")
    os.makedirs(base, exist_ok=True)
    record = {"dataset": "FX2", "variants": {}, "input": {"fixture": "synthetic-edge"}}
    for variant in ("v1", "v2"):
        out = os.path.join(base, f"source-{variant}.parquet")
        if os.path.exists(out):
            os.remove(out)
        r1 = run_json([example("trades_convert"), "synthetic-edge", "--variant", variant, "--out", out, "--report", out + ".json"])
        import pyarrow.parquet as pq
        table = pq.read_table(out)  # second path: pyarrow reads the Rust-written file and hashes it itself
        digest2 = common.digest_table(table)
        if digest2 != r1["digest"]:
            raise SystemExit(f"FX2/{variant}: pyarrow digest differs from the Rust digest")
        record["variants"][variant] = {"source": os.path.relpath(out, work), "report": r1, "references_agree": True}
    records["FX2"] = record
    return records


# ------------------------------------------------------------------ measuring

def rotate(candidates, r):
    k = r % len(candidates)
    order = candidates[k:] + candidates[:k]
    return order[::-1] if r % 2 else order


def warm_file(path):
    with open(path, "rb") as handle:
        while handle.read(1 << 24):
            pass


def run_round(ctx, dataset, variant, r, role, candidates, inputs):
    ref = inputs["variants"][variant]["report"]
    source = os.path.join(ctx["work"], inputs["variants"][variant]["source"])
    samples = []
    env = dict(os.environ)
    window = ref["window"]
    for cid, family, _ in rotate(candidates, r):
        prefix = adapter_prefix(family)
        store_parent = os.path.join(ctx["work"], "stores", f"{dataset}-{variant}-{cid}-r{r}")
        shutil.rmtree(store_parent, ignore_errors=True)
        os.makedirs(store_parent)
        store = os.path.join(store_parent, "data")
        common_args = ["--candidate", cid, "--variant", variant, "--source", source, "--store", store]
        meta = {"dataset": dataset, "variant": variant, "candidate": cid, "round": r, "role": role}
        samples.append(make_sample(dict(meta, phase="noop", cache=None), run_adapter(prefix, "noop", common_args, ctx["cpu"], env)))
        warm_file(source)
        write = make_sample(dict(meta, phase="write", cache=None), run_adapter(prefix, "write", common_args, ctx["cpu"], env))
        if write["rc"] == 0:
            write["bytes"] = common.tree_bytes(store)
        samples.append(write)
        if write["rc"] != 0:
            shutil.rmtree(store_parent, ignore_errors=True)
            continue
        range_args = common_args + ["--symbol", SYMBOL, "--start", str(window["start"]), "--end", str(window["end"])]
        for cache, phase, args, expected in (
                ("cold", "read-range", range_args, window["digest"]), ("cold", "read-full", common_args, ref["digest"]),
                ("warm", "read-range", range_args, window["digest"]), ("warm", "read-full", common_args, ref["digest"])):
            resident = None
            if cache == "cold":
                common.evict(store)
            resident = common.resident_fraction(store)
            sample = make_sample(dict(meta, phase=phase, cache=cache), run_adapter(prefix, phase, args, ctx["cpu"], env), expected)
            sample["resident"] = resident
            samples.append(sample)
        shutil.rmtree(store_parent, ignore_errors=True)
    for cid in REPEATED_PER_ROUND["source-read"]:
        match = [c for c in candidates if c[0] == cid]
        if not match:
            continue
        prefix = adapter_prefix(match[0][1])
        store = os.path.join(ctx["work"], "stores", f"unused-{cid}")
        meta = {"dataset": dataset, "variant": variant, "candidate": cid, "round": r, "role": role}
        samples.append(make_sample(dict(meta, phase="source-read", cache="warm"),
                                   run_adapter(prefix, "source-read", ["--candidate", cid, "--variant", variant, "--source", source, "--store", store], ctx["cpu"], env),
                                   ref["digest"]))
    if variant == "v2" and inputs["variants"]["v2"].get("normalise_check", {}).get("float_path") == "agrees_with_text_path":
        v1_source = os.path.join(ctx["work"], inputs["variants"]["v1"]["source"])
        samples.append(make_sample(dict(dataset=dataset, variant=variant, candidate=None, round=r, role=role, phase="normalise", cache=None),
                                   run_adapter(adapter_prefix("rust"), "normalise", ["--candidate", "gqodb", "--variant", "v1", "--source", v1_source, "--store", os.path.join(ctx["work"], "unused")], ctx["cpu"], env),
                                   ref["digest"]))
    return samples


def append_checkpoint(path, record):
    if os.path.exists(path) and os.path.getsize(path):
        with open(path, "rb") as handle:  # a crash can leave a torn last line: start a fresh line
            handle.seek(-1, os.SEEK_END)
            torn = handle.read(1) != b"\n"
        if torn:
            with open(path, "a") as handle:
                handle.write("\n")
    with open(path, "a") as handle:
        handle.write(json.dumps(record, sort_keys=True) + "\n")
        handle.flush()
        os.fsync(handle.fileno())


def load_checkpoint(path):
    header, rounds = None, {}
    if not os.path.exists(path):
        return header, rounds
    with open(path) as handle:
        for line in handle:
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except ValueError:  # a torn last line from a crash: ignore it, the round is rerun
                continue
            if record.get("type") == "header":
                header = record
            elif record.get("type") == "round":
                rounds.setdefault((record["dataset"], record["variant"]), []).append(record)
    return header, rounds


def measure_cell(ctx, dataset, variant, inputs, available, profile, done_rounds=()):
    p = PROFILE[profile]
    candidates = [c for c in selected_candidates(dataset) if c[0] in available]
    samples, valid = [], 0
    for record in done_rounds:  # resume: rounds already in the checkpoint
        samples.extend(record["samples"])
        valid += record["role"] == "measured" and not record["flagged"]
    attempt = len(done_rounds)
    limit = p["warmups"] + p["measured"] + p["extra"]
    while attempt < limit and valid < p["measured"]:
        role = "warmup" if attempt < p["warmups"] else "measured"
        busy, foreign = wait_quiet(ctx["excluded"], ctx.get("foreign", ())) if ctx.get("quiet") else (None, None)
        round_samples = run_round(ctx, dataset, variant, attempt, role, candidates, inputs)
        for s in round_samples:
            s["box_busy"] = busy
            s["foreign_busy"] = foreign
        failed = [s for s in round_samples if s["rc"] != 0]
        if attempt == 0 and failed and ctx.get("stop_on_first_failure"):
            raise SystemExit(f"first round of {dataset}/{variant} had process failures; fix the cause first: "
                             + "; ".join(f"{s['candidate']}/{s['phase']}: {s.get('stderr', '')[-160:]}" for s in failed[:3]))
        flagged = any(s["flags"] for s in round_samples)
        samples.extend(round_samples)
        if ctx.get("checkpoint"):
            append_checkpoint(ctx["checkpoint"], {"type": "round", "dataset": dataset, "variant": variant, "attempt": attempt,
                                                  "role": role, "flagged": flagged, "box_busy": busy, "foreign_busy": foreign,
                                                  "samples": round_samples})
        if role == "measured" and not flagged:
            valid += 1
        print(f"  {dataset}/{variant} round {attempt} ({role}) flagged={sum(bool(s['flags']) for s in round_samples)} "
              f"valid={valid}/{p['measured']} box_busy={busy if busy is None else round(busy, 2)} "
              f"foreign_busy={foreign if foreign is None else round(foreign, 2)} "
              f"{datetime.datetime.now().strftime('%H:%M:%S')}", flush=True)
        attempt += 1
    return samples


def detect_available(ctx):
    available, skipped = set(), {}
    for cid, family, _ in CANDIDATES:
        result = run_adapter(adapter_prefix(family), "noop", ["--candidate", cid], ctx["cpu"])
        payload = result["payload"] or {}
        if result["rc"] == 0 and "skipped" not in payload and payload:
            available.add(cid)
        else:
            skipped[cid] = payload.get("skipped") or result["stderr"][-200:] or "adapter failed"
    return available, skipped


def environment(ctx, tools):
    git = lambda *a: subprocess.run(["git", *a], cwd=ROOT, capture_output=True, text=True).stdout.strip()  # noqa: E731
    lock = report.sha256_file(os.path.join(ROOT, "Cargo.lock"))
    return {"commit": git("rev-parse", "HEAD"), "commit_dirty": bool(git("status", "--porcelain")),
            "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
            "cargo_lock_sha256": lock, "tools": tools, "python": platform.python_version(),
            "cpu_model_class": cpu_model_class(), "kernel": platform.release(),
            "filesystem": filesystem_info(ctx["work"]), "pinned_cpu": ctx["cpu"], "sibling_cpu": sibling_of(ctx["cpu"]),
            "harness_cpu": ctx.get("harness_cpu"), "foreign_cpus": ctx.get("foreign", [])}


def tool_versions(samples):
    versions = {}
    for s in samples:
        if s.get("version") and s["phase"] == "noop":
            versions[s["candidate"].split("-")[0]] = s["version"]
    return versions


def cmd_measure(args, profile="official"):
    smoke = profile in ("smoke", "quick")
    if not smoke:
        if os.environ.get("B17_MEASURE_GO") != "1":
            raise SystemExit("REFUSED: the timed matrix needs B17_MEASURE_GO=1 (stage 2 of the work package)")
        pinned = open(os.path.join(ROOT, "docs", "B17_PUBLIC_TRADES_PROTOCOL.sha256")).read().split()[0]
        if report.sha256_file(os.path.join(ROOT, "docs", "B17_PUBLIC_TRADES_PROTOCOL.md")) != pinned:
            raise SystemExit("REFUSED: protocol file differs from its pinned sha256; use an addendum instead")
        if subprocess.run(["git", "status", "--porcelain"], cwd=ROOT, capture_output=True, text=True).stdout.strip():
            raise SystemExit("REFUSED: working tree is dirty; commit first so the receipt names the exact code")
        if getattr(args, "out", None) and os.path.exists(os.path.abspath(args.out)):
            raise SystemExit(f"REFUSED: output directory exists: {args.out}")
    work = os.path.abspath(args.work)
    info = filesystem_info(work if os.path.exists(work) else os.path.dirname(work))
    if not smoke and info["type"] in BAD_FILESYSTEMS:
        raise SystemExit(f"REFUSED: work directory is on {info['type']}; cache eviction would be meaningless")
    cpu = int(args.cpu)
    harness_cpu = int(args.harness_cpu)
    os.makedirs(work, exist_ok=True)
    os.sched_setaffinity(0, {harness_cpu})
    sib = sibling_of(cpu)
    foreign = {int(c) for c in str(args.foreign_cpus).split(",") if c != ""}
    ctx = {"work": work, "cpu": cpu, "harness_cpu": harness_cpu, "quiet": not smoke, "foreign": sorted(foreign),
           "excluded": ({cpu, sib, harness_cpu, sibling_of(harness_cpu)} | foreign) - {None},
           "stop_on_first_failure": not smoke}
    available, skipped = detect_available(ctx)
    if "gqodb" not in available:
        raise SystemExit("the gqodb adapter is required (build the examples first)")
    done = {}
    if smoke:
        inputs = prepare_fixtures(work)
        names = [n for n in inputs if n in args.fixtures.split(",")]
        env_record = environment(ctx, {})
    else:
        names = args.datasets.split(",")
        inputs = {n: prepare_dataset(n, work, args.zip_dir) for n in names}
        ctx["checkpoint"] = os.path.join(work, "checkpoint.jsonl")
        header, done = load_checkpoint(ctx["checkpoint"])
        env_record = environment(ctx, {})
        identity = {"commit": env_record["commit"], "datasets": names,
                    "protocol_sha256": report.sha256_file(os.path.join(ROOT, "docs", "B17_PUBLIC_TRADES_PROTOCOL.md"))}
        if header:
            if any(header.get(k) != v for k, v in identity.items()):
                raise SystemExit("REFUSED: the checkpoint belongs to another commit, protocol or dataset list; "
                                 "move the work directory's checkpoint.jsonl away to start over")
            env_record = header["environment"]
            print(f"resuming: {sum(len(v) for v in done.values())} rounds already recorded", flush=True)
        else:
            append_checkpoint(ctx["checkpoint"], dict(identity, type="header", environment=env_record,
                                                      started_utc=datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")))
    samples = []
    for name in names:
        for variant in ("v1", "v2"):
            print(f"== {name}/{variant}", flush=True)
            samples.extend(measure_cell(ctx, name, variant, inputs[name], available, profile, done.get((name, variant), ())))
    summary = report.summarise(samples)
    env_record = dict(env_record, tools=tool_versions(samples))
    return {"samples": samples, "summary": summary, "inputs": inputs, "skipped_candidates": skipped,
            "environment": env_record, "profile": profile}


def sanitise_inputs(inputs):
    """Published inputs carry no per-trade values: drop first/last ids and times and window bounds."""
    clean = json.loads(json.dumps(inputs))
    for record in clean.values():
        for variant in record.get("variants", {}).values():
            for report_key in ("report", "python_reference"):
                r = variant.get(report_key)
                if not r:
                    continue
                for key in ("first_agg_id", "last_agg_id", "first_time", "last_time"):
                    r.pop(key, None)
                for key in ("start", "end"):
                    r.get("window", {}).pop(key, None)
    return clean


def write_evidence(outdir, result, protocol_sha):
    os.makedirs(outdir, exist_ok=False)
    files = {"samples.json": result["samples"], "summary.json": result["summary"],
             "inputs.json": sanitise_inputs(result["inputs"]), "plan.json": plan(list(result["inputs"]))}
    for name, payload in files.items():
        with open(os.path.join(outdir, name), "w") as handle:
            json.dump(payload, handle, indent=1, sort_keys=True)
            handle.write("\n")
    with open(os.path.join(outdir, "ASSESSMENT.md"), "w") as handle:
        handle.write(report.render_assessment(result["summary"]))
    receipt = {"experiment": "B17", "purpose": "published run", "protocol": "docs/B17_PUBLIC_TRADES_PROTOCOL.md",
               "protocol_sha256": protocol_sha, "environment": result["environment"], "profile": result["profile"],
               "skipped_candidates": sorted(result["skipped_candidates"]),
               "inputs": {n: v.get("input") for n, v in result["inputs"].items()},
               "settings": {"warmups": PROFILE["official"]["warmups"], "measured": PROFILE["official"]["measured"],
                            "extra_rounds_allowed": PROFILE["official"]["extra"], "block_bootstrap": 3,
                            "resamples": 10000, "seed": 20261017},
               "generated_utc": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")}
    report.write_receipt(outdir, receipt, list(files) + ["ASSESSMENT.md"])


# ------------------------------------------------------------------ command line

def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("plan")
    p.add_argument("--datasets", default="D1,D2,S1,S2")
    p.add_argument("--json", action="store_true")
    for name in ("prepare", "measure", "smoke"):
        p = sub.add_parser(name)
        p.add_argument("--work", required=True)
        p.add_argument("--datasets", default="D1,D2,S1,S2")
        p.add_argument("--zip-dir")
        p.add_argument("--out")
        p.add_argument("--fixtures", default="FX1,FX2,FX3")
        p.add_argument("--quick", action="store_true", help="smoke only: one measured round, no warm-up")
        p.add_argument("--cpu", default=os.environ.get("B17_CPU", "10"))
        p.add_argument("--harness-cpu", default=os.environ.get("B17_HARNESS_CPU", "11"))
        p.add_argument("--foreign-cpus", default=os.environ.get("B17_FOREIGN_CPUS", "6,20"),
                       help="cores held by declared foreign jobs; excluded from the quiet rule and reported (addendum A-003)")
    p = sub.add_parser("report")
    p.add_argument("--samples", required=True)
    p.add_argument("--out", required=True)
    p = sub.add_parser("seal", help="recompute result_sha256 after ASSESSMENT.md was completed by hand")
    p.add_argument("--out", required=True)
    args = parser.parse_args()

    if args.command == "plan":
        result = plan(args.datasets.split(","))
        if args.json:
            print(json.dumps(result, indent=1))
        else:
            for cell in result["cells"]:
                print(f"{cell['dataset']}/{cell['variant']}: {len(cell['candidates'])} candidates, {cell['rounds']}, "
                      f"{cell['processes_per_round']} adapter processes per round")
            print(f"adapter processes: {result['adapter_processes_min']} to {result['adapter_processes_max']}")
            print("nothing was executed")
        return
    if args.command == "prepare":
        work = os.path.abspath(args.work)
        os.makedirs(work, exist_ok=True)
        records = {n: prepare_dataset(n, work, args.zip_dir) for n in args.datasets.split(",")}
        with open(os.path.join(work, "inputs.json"), "w") as handle:
            json.dump(records, handle, indent=1, sort_keys=True)
        print(f"prepared {', '.join(records)}; Rust and Python references agree")
        return
    if args.command == "seal":
        path = os.path.join(args.out, "BUILD_RECEIPTS.json")
        receipt = json.load(open(path))
        names = list(receipt["result_sha256"])
        report.write_receipt(args.out, {k: v for k, v in receipt.items() if k != "result_sha256"}, names)
        print(f"sealed {len(names)} files")
        return
    if args.command == "report":
        data = json.load(open(args.samples))
        samples = data["samples"] if isinstance(data, dict) else data
        os.makedirs(args.out, exist_ok=True)
        summary = report.summarise(samples)
        json.dump(summary, open(os.path.join(args.out, "summary.json"), "w"), indent=1, sort_keys=True)
        open(os.path.join(args.out, "ASSESSMENT.md"), "w").write(report.render_assessment(summary))
        return
    if args.command == "smoke":
        result = cmd_measure(args, "quick" if args.quick else "smoke")
        bad = [(s["dataset"], s["variant"], s["candidate"], s["phase"], s["cache"]) for s in result["samples"]
               if s.get("digest_ok") is False or s["rc"] != 0]
        for cid, why in sorted(result["skipped_candidates"].items()):
            print(f"skipped {cid}: {why}")
        print(f"smoke: {len(result['samples'])} adapter runs; exactness/process failures: {len(bad)}")
        for item in bad:
            print("  FAIL", item)
        if args.out:
            os.makedirs(args.out, exist_ok=True)
            json.dump(result, open(os.path.join(args.out, "smoke.json"), "w"), indent=1)
        sys.exit(1 if bad else 0)
    if args.command == "measure":
        result = cmd_measure(args, "official")
        if os.path.exists(os.path.abspath(args.out)):
            raise SystemExit(f"output directory exists: {args.out}")
        protocol_sha = report.sha256_file(os.path.join(ROOT, "docs", "B17_PUBLIC_TRADES_PROTOCOL.md"))
        write_evidence(os.path.abspath(args.out), result, protocol_sha)
        print(f"evidence written to {args.out}")


if __name__ == "__main__":
    main()
