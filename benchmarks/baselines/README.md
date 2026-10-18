# Baseline adapters for B17

| Adapter | Tool | Notes |
| --- | --- | --- |
| `duckdb_adapter.py` | DuckDB 1.5.6 (Python package) | native file and three Parquet writer configurations |
| `clickhouse_adapter.py` | ClickHouse 26.3.54.6 LTS (`clickhouse local`) | MergeTree on disk, default and pre-registered codec chain |
| `install_tools.py` | n/a | downloads the pinned tool versions into a directory outside the repository and verifies sha256 (and the vendor's sha512 for ClickHouse); never runs a downloaded script |

All adapters share one command line and print a JSON object as the last stdout line:
`adapter PHASE --candidate ID --variant v1|v2 --source SOURCE.parquet --store DIR [--symbol S --start T --end T]`
with PHASE one of `noop`, `write`, `read-full`, `read-range`, `source-read`. They skip
cleanly (exit 0, `{"skipped": ...}`) when their tool is missing. Their timing, durability
and materialisation rules are in section 5 of the protocol.
