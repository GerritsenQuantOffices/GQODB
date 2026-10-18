#!/usr/bin/env python3
"""ClickHouse adapter for the B17 benchmark (docs/B17_PUBLIC_TRADES_PROTOCOL.md).

  clickhouse_adapter.py PHASE --candidate clickhouse-default|clickhouse-tuned
                        --variant v1|v2 --source SOURCE.parquet --store DIR
                        [--symbol S --start T --end T]

Drives the official `clickhouse` binary (`clickhouse local --path DIR`, MergeTree on disk), one
process per statement. Binary lookup: $B17_CLICKHOUSE, else `clickhouse` on PATH. If the binary
or pyarrow is absent the adapter prints {"skipped": "..."} and exits 0.

op_ns is the wall time of the operation process minus the wall time of an immediately preceding
`SELECT 1` process (floored at zero); the raw numbers are stored too. cpu_ns and rss_hwm_kb are
the rusage figures of the ClickHouse process (wait4), not of this Python process. Rows reach
the client as an Arrow IPC stream through a pipe and are decoded after the clock stops.
"""
import argparse
import os
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "public_trades"))
import common  # noqa: E402

BINARY = os.environ.get("B17_CLICKHOUSE") or shutil.which("clickhouse")
try:
    import pyarrow as pa
except ImportError as error:
    common.emit({"skipped": f"clickhouse adapter needs pyarrow to decode results ({error})"})
    sys.exit(0)
if not BINARY or not os.access(BINARY, os.X_OK):
    common.emit({"skipped": "clickhouse binary not found (set B17_CLICKHOUSE or put it on PATH)"})
    sys.exit(0)

# Protocol section 4.1. Codec chains are pre-registered, not searched.
TUNED = {
    "v1": {"agg_id": "CODEC(DoubleDelta, ZSTD(1))", "price": "CODEC(Gorilla, ZSTD(1))",
           "qty": "CODEC(ZSTD(1))", "first_id": "CODEC(Delta, ZSTD(1))",
           "last_id": "CODEC(Delta, ZSTD(1))", "transact_time": "CODEC(DoubleDelta, ZSTD(1))",
           "is_buyer_maker": "CODEC(ZSTD(1))", "is_best_match": "CODEC(ZSTD(1))"},
}
TUNED["v2"] = dict(TUNED["v1"])
del TUNED["v2"]["price"], TUNED["v2"]["qty"]
TUNED["v2"]["price_e8"] = "CODEC(Delta, ZSTD(1))"
TUNED["v2"]["qty_e8"] = "CODEC(ZSTD(1))"
TYPES = {"agg_id": "Int64", "price": "Float64", "qty": "Float64", "price_e8": "Int64",
         "qty_e8": "Int64", "first_id": "Int64", "last_id": "Int64", "transact_time": "Int64",
         "is_buyer_maker": "Bool", "is_best_match": "Bool"}
INGEST_SETTINGS = ("max_insert_block_size = 67108864, min_insert_block_size_rows = 67108864, "
                   "min_insert_block_size_bytes = 17179869184, max_threads = 1, max_insert_threads = 1")


def ddl(candidate, variant):
    tuned = candidate == "clickhouse-tuned"
    names = [c[0] for c in common.COLUMNS[variant]]
    parts = []
    for name in names:
        if name == "symbol":
            parts.append("symbol LowCardinality(String)" if tuned else "symbol String")
        else:
            codec = (" " + TUNED[variant][name]) if tuned else ""
            parts.append(f"{name} {TYPES[name]}{codec}")
    return ("CREATE TABLE t (" + ", ".join(parts) + ") ENGINE = MergeTree "
            "ORDER BY (symbol, transact_time, agg_id) "
            "SETTINGS fsync_after_insert = 1, fsync_part_directory = 1")


def run_process(sql, path, capture, cpu_note=None):
    """One `clickhouse local` process. Returns (wall_ns, cpu_ns, maxrss_kb, stdout_bytes)."""
    command = [BINARY, "local", "--path", path, "--max_threads", "1", "--multiquery", "--query", sql]
    with tempfile.TemporaryFile() as errors:
        started = common.now_ns()
        process = subprocess.Popen(command, stdout=subprocess.PIPE if capture else subprocess.DEVNULL,
                                   stderr=errors)
        data = process.stdout.read() if capture else b""
        _pid, status, usage = os.wait4(process.pid, 0)
        wall = common.now_ns() - started
        process.returncode = os.waitstatus_to_exitcode(status)
        if capture:
            process.stdout.close()
        if process.returncode != 0:
            errors.seek(0)
            raise SystemExit(f"clickhouse failed ({process.returncode}): {errors.read().decode(errors='replace')[-400:]}")
    cpu = int((usage.ru_utime + usage.ru_stime) * 1e9)
    return wall, cpu, usage.ru_maxrss, data


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("phase")
    parser.add_argument("--candidate", default="clickhouse-default")
    parser.add_argument("--variant", default="v1")
    parser.add_argument("--source")
    parser.add_argument("--store")
    parser.add_argument("--symbol", default=common.SYMBOL)
    parser.add_argument("--start", type=int)
    parser.add_argument("--end", type=int)
    args = parser.parse_args()
    version = subprocess.run([BINARY, "local", "--version"], capture_output=True, text=True).stdout.strip()
    base = {"phase": args.phase, "candidate": args.candidate, "variant": args.variant, "version": version}

    def startup(path):
        return run_process("SELECT 1 FORMAT Null", path, False)

    def report(wall, cpu, rss, wall_startup, extra=None):
        payload = dict(base, op_ns=str(max(0, wall - wall_startup)), raw_wall_ns=str(wall),
                       startup_wall_ns=str(wall_startup), cpu_ns=cpu, rss_hwm_kb=rss)
        payload.update(extra or {})
        return payload

    if args.phase == "noop":
        scratch = tempfile.mkdtemp(prefix="b17-ch-noop-")
        try:
            wall, cpu, rss, _ = startup(os.path.join(scratch, "db"))
        finally:
            shutil.rmtree(scratch, ignore_errors=True)
        common.emit(dict(base, op_ns="0", raw_wall_ns=str(wall), cpu_ns=cpu, rss_hwm_kb=rss))
        return
    if args.candidate not in ("clickhouse-default", "clickhouse-tuned"):
        raise SystemExit(f"unknown ClickHouse candidate {args.candidate}")
    store = os.path.abspath(args.store) if args.store else None
    source = common.safe_text(os.path.abspath(args.source)) if args.source else None

    if args.phase == "write":
        if os.path.exists(store):
            raise SystemExit("store directory exists")
        os.makedirs(os.path.dirname(store), exist_ok=True)
        scratch = tempfile.mkdtemp(prefix="b17-ch-startup-")
        try:
            wall_startup = startup(os.path.join(scratch, "db"))[0]
        finally:
            shutil.rmtree(scratch, ignore_errors=True)
        sql = (ddl(args.candidate, args.variant) + "; INSERT INTO t SELECT * FROM file('" + source +
               "', Parquet) SETTINGS " + INGEST_SETTINGS)
        started = common.now_ns()
        wall, cpu, rss, _ = run_process(sql, store, False)
        common.fsync_tree(store)
        wall = common.now_ns() - started  # spawn to end of the fsync pass
        parts = run_process("SELECT count() FROM system.parts WHERE table = 't' AND active FORMAT TSV",
                            store, True)[3].decode().strip()
        common.emit(report(wall, cpu, rss, wall_startup, {"parts": int(parts)}))
        return

    if args.phase != "source-read":
        wall_startup = startup(store)[0]
    if args.phase == "read-full":
        sql = "SELECT * FROM t SETTINGS max_threads = 1 FORMAT ArrowStream"
    elif args.phase == "read-range":
        symbol = common.safe_text(args.symbol)
        sql = (f"SELECT * FROM t WHERE symbol = '{symbol}' AND transact_time BETWEEN {int(args.start)} "
               f"AND {int(args.end)} SETTINGS max_threads = 1 FORMAT ArrowStream")
    elif args.phase == "source-read":
        scratch = tempfile.mkdtemp(prefix="b17-ch-source-")
        try:
            wall_startup = startup(os.path.join(scratch, "db"))[0]
            wall, cpu, rss, data = run_process(
                f"SELECT * FROM file('{source}', Parquet) SETTINGS max_threads = 1 FORMAT ArrowStream",
                os.path.join(scratch, "db"), True)
        finally:
            shutil.rmtree(scratch, ignore_errors=True)
        table = pa.ipc.open_stream(pa.BufferReader(data)).read_all()
        digest, resorted = common.canonical_digest(table)
        common.emit(report(wall, cpu, rss, wall_startup, {"digest": digest, "rows": table.num_rows, "resorted": resorted}))
        return
    else:
        raise SystemExit(f"unknown phase {args.phase}")

    wall, cpu, rss, data = run_process(sql, store, True)  # the clock covers spawn to complete pipe read
    table = pa.ipc.open_stream(pa.BufferReader(data)).read_all()
    if args.phase == "read-full":
        digest, resorted = common.canonical_digest(table)
        common.emit(report(wall, cpu, rss, wall_startup, {"digest": digest, "rows": table.num_rows, "resorted": resorted}))
    else:
        digest = common.canonical_digest(table)[0] if table.num_rows else None
        common.emit(report(wall, cpu, rss, wall_startup, {"digest": digest, "rows": table.num_rows}))


if __name__ == "__main__":
    main()
