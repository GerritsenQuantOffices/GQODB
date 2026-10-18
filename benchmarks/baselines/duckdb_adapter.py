#!/usr/bin/env python3
"""DuckDB adapter for the B17 benchmark (docs/B17_PUBLIC_TRADES_PROTOCOL.md).

  duckdb_adapter.py PHASE --candidate ID --variant v1|v2 --source SOURCE.parquet --store DIR
                    [--symbol S --start T --end T]

Candidates: duckdb-native, duckdb-parquet-default, duckdb-parquet-matched, duckdb-parquet-zstd.
Needs the `duckdb` and `pyarrow` Python packages (install with install_tools.py into a
virtual environment outside the repository). If either is missing the adapter prints
{"skipped": "..."} and exits 0. The last stdout line is one JSON object.
"""
import argparse
import os
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "public_trades"))
import common  # noqa: E402

try:
    import duckdb
    import pyarrow  # noqa: F401
except ImportError as error:  # tool absent: skip cleanly
    common.emit({"skipped": f"duckdb adapter needs duckdb and pyarrow ({error})"})
    sys.exit(0)

COPY_OPTIONS = {
    "duckdb-parquet-default": "FORMAT PARQUET",
    "duckdb-parquet-matched": "FORMAT PARQUET, COMPRESSION lz4_raw, ROW_GROUP_SIZE 32768, PARQUET_VERSION V2",
    "duckdb-parquet-zstd": "FORMAT PARQUET, COMPRESSION zstd, ROW_GROUP_SIZE 32768, PARQUET_VERSION V2",
}
FILE_NAME = {"duckdb-native": "trades.duckdb"}


def file_of(store, candidate):
    return os.path.join(store, FILE_NAME.get(candidate, "trades.parquet"))


def to_arrow(result):
    for name in ("fetch_arrow_table", "to_arrow_table", "arrow"):
        if hasattr(result, name):
            table = getattr(result, name)()
            return table.read_all() if hasattr(table, "read_all") else table
    raise RuntimeError("no Arrow export on this DuckDB version")


def finish(base, started, extra=None):
    """Counters are read at the moment the clock stops, before any hashing."""
    op_ns = common.now_ns() - started
    payload = dict(base, op_ns=str(op_ns), cpu_ns=time.process_time_ns(), rss_hwm_kb=common.vm_hwm_kb())
    payload.update(extra or {})
    return payload


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("phase")
    parser.add_argument("--candidate", default="duckdb-native")
    parser.add_argument("--variant", default="v1")
    parser.add_argument("--source")
    parser.add_argument("--store")
    parser.add_argument("--symbol", default=common.SYMBOL)
    parser.add_argument("--start", type=int)
    parser.add_argument("--end", type=int)
    args = parser.parse_args()

    if args.phase == "noop":
        common.emit(finish({"phase": "noop", "version": duckdb.__version__}, common.now_ns()))
        return
    if args.candidate not in FILE_NAME and args.candidate not in COPY_OPTIONS:
        raise SystemExit(f"unknown DuckDB candidate {args.candidate}")
    base = {"phase": args.phase, "candidate": args.candidate, "variant": args.variant,
            "version": duckdb.__version__}
    source = common.safe_text(os.path.abspath(args.source)) if args.source else None
    store = os.path.abspath(args.store) if args.store else None
    target = file_of(store, args.candidate) if store else None
    native = args.candidate == "duckdb-native"

    def connect(read_only):
        connection = duckdb.connect(target if native else ":memory:", read_only=read_only and native)
        connection.execute("SET threads = 1")
        return connection

    def scan():
        return "t" if native else f"read_parquet('{common.safe_text(target)}')"

    if args.phase == "write":
        if os.path.exists(store):
            raise SystemExit("store directory exists")
        os.makedirs(os.path.dirname(store), exist_ok=True)
        started = common.now_ns()
        os.mkdir(store)
        connection = connect(False)
        if native:
            connection.execute(f"CREATE TABLE t AS SELECT * FROM read_parquet('{source}')")
            connection.execute("CHECKPOINT")
        else:
            connection.execute(
                f"COPY (SELECT * FROM read_parquet('{source}')) TO '{common.safe_text(target)}' "
                f"({COPY_OPTIONS[args.candidate]})")
        connection.close()
        common.fsync_tree(store)
        payload = finish(base, started)
        meta = {}
        if not native:  # effective writer settings, recorded outside the clock
            check = duckdb.connect(":memory:")
            codecs = [r[0] for r in check.execute(
                f"SELECT DISTINCT compression FROM parquet_metadata('{target}')").fetchall()]
            meta = {"parquet_compression": sorted(codecs),
                    "row_groups": check.execute(
                        f"SELECT count(DISTINCT row_group_id) FROM parquet_metadata('{target}')").fetchone()[0]}
        payload["effective"] = meta
        common.emit(payload)
        return

    if args.phase == "source-read":
        started = common.now_ns()
        connection = duckdb.connect(":memory:")
        connection.execute("SET threads = 1")
        table = to_arrow(connection.execute(f"SELECT * FROM read_parquet('{source}')"))
        payload = finish(base, started)
        digest, resorted = common.canonical_digest(table)
        common.emit(dict(payload, digest=digest, rows=table.num_rows, resorted=resorted))
        return

    if args.phase == "read-full":
        started = common.now_ns()
        connection = connect(True)
        table = to_arrow(connection.execute(f"SELECT * FROM {scan()}"))
        payload = finish(base, started)
        digest, resorted = common.canonical_digest(table)
        common.emit(dict(payload, digest=digest, rows=table.num_rows, resorted=resorted))
        return

    if args.phase == "read-range":
        started = common.now_ns()
        connection = connect(True)
        table = to_arrow(connection.execute(
            f"SELECT * FROM {scan()} WHERE symbol = ? AND transact_time BETWEEN ? AND ?",
            [args.symbol, args.start, args.end]))
        payload = finish(base, started)
        digest = common.canonical_digest(table)[0] if table.num_rows else None
        common.emit(dict(payload, digest=digest, rows=table.num_rows))
        return

    raise SystemExit(f"unknown phase {args.phase}")


if __name__ == "__main__":
    main()
