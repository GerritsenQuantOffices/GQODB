#!/usr/bin/env bash
# Run one fuzz target for a fixed wall time with the resource limits that turn hangs and
# over-allocation into findings. Usage: fuzz/run_target.sh TARGET SECONDS [extra libFuzzer flags]
# Environment: FUZZ_BIN_DIR (built targets), FUZZ_WORK (working corpus, artifacts, logs),
# FUZZ_CPUS (taskset list, default all CPUs). Seeds are read from fuzz/corpus/TARGET (never written).
set -euo pipefail
target=${1:?target}
seconds=${2:?seconds}
shift 2
here=$(cd "$(dirname "$0")" && pwd)
bin_dir=${FUZZ_BIN_DIR:?set FUZZ_BIN_DIR to <target-dir>/x86_64-unknown-linux-gnu/release}
work=${FUZZ_WORK:?set FUZZ_WORK to a scratch directory}
cpus=${FUZZ_CPUS:-0-$(( $(nproc) - 1 ))}
mkdir -p "$work/corpus/$target" "$work/artifacts/$target" "$work/logs"
case $target in
  roundtrip_*|store_structured|types_parse|codec_schema|lang_parse) max_len=4096 ;;
  *) max_len=65536 ;;
esac
dict="$here/dict/gqodb.dict"
[ "$target" = lang_parse ] && dict="$here/dict/gqodb-lang.dict"
exec nice -n 19 taskset -c "$cpus" "$bin_dir/$target" \
  "$work/corpus/$target" "$here/corpus/$target" \
  -dict="$dict" \
  -max_len=$max_len -timeout=10 -rss_limit_mb=4096 -malloc_limit_mb=4096 \
  -use_value_profile=1 -detect_leaks=0 -print_final_stats=1 -max_total_time="$seconds" \
  -artifact_prefix="$work/artifacts/$target/" "$@"
