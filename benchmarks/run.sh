#!/bin/sh
# Public benchmark entry point. Only `public-trades fetch|prepare|measure` download data (the
# registered Binance Vision archives, checksum-verified); every other command is offline.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
command_name=${1:-help}

case "$command_name" in
    verify)
        cd "$root"
        python3 results/verify_public_evidence.py
        ;;
    synthetic-smoke)
        cd "$root"
        output=$(mktemp -d "${TMPDIR:-/tmp}/gqodb-benchmark-smoke.XXXXXX")
        trap 'rm -rf "$output"' EXIT HUP INT TERM
        cargo build -p gqodb-blocks --release --locked
        target/release/gqodb-blocks --output "$output/run" --sizes 4096 --repetitions 1
        printf 'Synthetic smoke output verified in %s (removed on exit).\n' "$output/run"
        ;;
    public-trades)
        # B17: docs/B17_PUBLIC_TRADES_PROTOCOL.md. Sub-commands: build, plan, fetch, prepare,
        # measure, smoke, report. `plan` and `--dry-run` execute nothing.
        cd "$root"
        sub=${2:-help}
        py=${B17_PYTHON:-python3}
        case "$sub" in
            build)
                nice -n 19 cargo build --release --locked -p gqodb-store --examples
                ;;
            plan|--dry-run)
                shift 2
                "$py" benchmarks/public_trades/harness.py plan "$@"
                ;;
            fetch)
                shift 2
                exec benchmarks/public_trades/fetch.sh "$@"
                ;;
            prepare|measure|smoke|report)
                shift 1
                "$py" benchmarks/public_trades/harness.py "$@"
                ;;
            *)
                printf '%s\n' \
                    'Usage: ./benchmarks/run.sh public-trades build' \
                    '       ./benchmarks/run.sh public-trades plan [--datasets D1,D2,S1,S2] [--json]   (executes nothing)' \
                    '       ./benchmarks/run.sh public-trades fetch PAXGUSDT 2025-10 [--out DIR] [--zip F --checksum F]' \
                    '       ./benchmarks/run.sh public-trades prepare --work DIR [--datasets D1,D2] [--zip-dir DIR]' \
                    '       ./benchmarks/run.sh public-trades smoke --work DIR [--fixtures FX2] [--quick]' \
                    '       B17_MEASURE_GO=1 ./benchmarks/run.sh public-trades measure --work DIR --out results/b17-public-trades-DATE' \
                    '' \
                    'Needs: built examples (build), B17_PYTHON (virtual environment with duckdb and pyarrow) and' \
                    'B17_CLICKHOUSE from benchmarks/baselines/install_tools.py. See docs/B17_PUBLIC_TRADES_PROTOCOL.md.'
                [ "$sub" = help ] || exit 2
                ;;
        esac
        ;;
    help|-h|--help)
        printf '%s\n' \
            'Usage: ./benchmarks/run.sh verify' \
            '       ./benchmarks/run.sh synthetic-smoke' \
            '       ./benchmarks/run.sh public-trades [build|plan|fetch|prepare|smoke|measure|report]' \
            '' \
            'Market benchmarks require an explicitly supplied local input; see benchmarks/README.md.'
        ;;
    *)
        printf 'unknown benchmark command: %s\n' "$command_name" >&2
        exit 2
        ;;
esac
