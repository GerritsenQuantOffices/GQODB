#!/bin/sh
# Public benchmark entry point. It never downloads market data.
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
    help|-h|--help)
        printf '%s\n' \
            'Usage: ./benchmarks/run.sh verify' \
            '       ./benchmarks/run.sh synthetic-smoke' \
            '' \
            'Market benchmarks require an explicitly supplied local input; see benchmarks/README.md.'
        ;;
    *)
        printf 'unknown benchmark command: %s\n' "$command_name" >&2
        exit 2
        ;;
esac
