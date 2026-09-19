//! Reproducible process-isolated benchmark runner and report writer, entirely Rust.
#![forbid(unsafe_code)]
use anyhow::{Context, Result, ensure};
use bytes::Bytes;
use gqodb_blocks::{
    Codec,
    data::{Kind, Regime, generate},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Sample {
    case: String,
    kind: Kind,
    regime: Regime,
    seed: u64,
    rows: usize,
    dataset_sha256: String,
    codec: Codec,
    repetition: usize,
    iterations: usize,
    raw_bytes: usize,
    total_bytes: usize,
    framing_bytes: usize,
    encode_ns: f64,
    decode_ns: f64,
    roundtrip_ok: bool,
}

#[derive(Debug, Serialize)]
struct Summary {
    case: String,
    codec: Codec,
    rows: usize,
    repetitions: usize,
    total_bytes: usize,
    bytes_per_row: f64,
    encode_ns_median: f64,
    encode_ns_min: f64,
    encode_ns_max: f64,
    decode_ns_median: f64,
    decode_ns_min: f64,
    decode_ns_max: f64,
    encode_mrows_per_second: f64,
    decode_mrows_per_second: f64,
    encode_raw_mib_per_second: f64,
    decode_raw_mib_per_second: f64,
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

fn command(root: &Path, program: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()?;
    ensure!(
        out.status.success(),
        "{program} {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(String::from_utf8(out.stdout)?.trim().to_owned())
}

fn hash_file(path: impl AsRef<Path>) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn worker(output: &Path, sizes: &[usize], repetition: usize, seed_offset: u64) -> Result<()> {
    let mut samples = Vec::new();
    let mut case_index = 0;
    for kind in Kind::ALL {
        for regime in Regime::ALL {
            for &rows in sizes {
                let seed = 0x6710_db01
                    + kind as u64 * 100
                    + matches!(regime, Regime::Volatile) as u64
                    + seed_offset;
                let data = generate(kind, regime, rows, seed)?;
                let case = format!("{kind:?}/{regime:?}/{rows}");
                let digest = data.digest();
                let mut codecs = Codec::ALL;
                codecs.rotate_left((repetition + case_index) % Codec::ALL.len());
                for codec in codecs {
                    let start = Instant::now();
                    let warm = Bytes::from(codec.encode(black_box(&data))?);
                    let warm_decoded = codec.decode(warm.clone())?;
                    let warm_ns = start.elapsed().as_nanos().max(1);
                    ensure!(
                        warm_decoded == data,
                        "warmup roundtrip failed: {case} {codec:?}"
                    );
                    // Aim for ~15ms of work per result, capped to bound tiny cases.
                    let iterations = (15_000_000_u128 / warm_ns).clamp(1, 128) as usize;
                    let framing_bytes = codec.framing_bytes(warm.clone())?;
                    let mut encode_ns = 0_u128;
                    let mut decode_ns = 0_u128;
                    for _ in 0..iterations {
                        let start = Instant::now();
                        let encoded = codec.encode(black_box(&data))?;
                        encode_ns += start.elapsed().as_nanos();
                        ensure!(
                            encoded.len() == warm.len(),
                            "non-deterministic encoded size"
                        );
                        let encoded = Bytes::from(encoded);
                        let start = Instant::now();
                        let restored = codec.decode(black_box(encoded.clone()))?;
                        decode_ns += start.elapsed().as_nanos();
                        // Full materialized equality outside the timers; no checksum-only parity.
                        ensure!(restored == data, "roundtrip failed: {case} {codec:?}");
                        black_box(&restored);
                    }
                    samples.push(Sample {
                        case: case.clone(),
                        kind,
                        regime,
                        seed,
                        rows,
                        dataset_sha256: digest.clone(),
                        codec,
                        repetition,
                        iterations,
                        raw_bytes: data.raw_bytes(),
                        total_bytes: warm.len(),
                        framing_bytes,
                        encode_ns: encode_ns as f64 / iterations as f64,
                        decode_ns: decode_ns as f64 / iterations as f64,
                        roundtrip_ok: true,
                    });
                }
                case_index += 1;
                eprintln!("trial {repetition}: {case} complete");
            }
        }
    }
    write_json(&output.join(format!("trial-{repetition}.json")), &samples)
}

fn stats(mut v: Vec<f64>) -> (f64, f64, f64) {
    v.sort_by(f64::total_cmp);
    let mid = if v.len() % 2 == 0 {
        (v[v.len() / 2 - 1] + v[v.len() / 2]) / 2.0
    } else {
        v[v.len() / 2]
    };
    (mid, v[0], v[v.len() - 1])
}

fn summarize(samples: &[Sample]) -> Result<Vec<Summary>> {
    let mut groups: BTreeMap<String, Vec<&Sample>> = BTreeMap::new();
    for sample in samples {
        groups
            .entry(format!("{}:{:?}", sample.case, sample.codec))
            .or_default()
            .push(sample);
    }
    let mut result = Vec::new();
    for group in groups.values() {
        let first = group[0];
        ensure!(
            group.iter().all(|s| s.total_bytes == first.total_bytes
                && s.dataset_sha256 == first.dataset_sha256
                && s.roundtrip_ok),
            "repetition mismatch"
        );
        let (en, emin, emax) = stats(group.iter().map(|s| s.encode_ns).collect());
        let (dn, dmin, dmax) = stats(group.iter().map(|s| s.decode_ns).collect());
        result.push(Summary {
            case: first.case.clone(),
            codec: first.codec,
            rows: first.rows,
            repetitions: group.len(),
            total_bytes: first.total_bytes,
            bytes_per_row: first.total_bytes as f64 / first.rows as f64,
            encode_ns_median: en,
            encode_ns_min: emin,
            encode_ns_max: emax,
            decode_ns_median: dn,
            decode_ns_min: dmin,
            decode_ns_max: dmax,
            encode_mrows_per_second: first.rows as f64 * 1000.0 / en,
            decode_mrows_per_second: first.rows as f64 * 1000.0 / dn,
            encode_raw_mib_per_second: first.raw_bytes as f64 / 1_048_576.0 * 1e9 / en,
            decode_raw_mib_per_second: first.raw_bytes as f64 / 1_048_576.0 * 1e9 / dn,
        });
    }
    Ok(result)
}

fn report(output: &Path, samples: &[Sample], provenance: serde_json::Value) -> Result<()> {
    let summaries = summarize(samples)?;
    let mut comparisons = Vec::new();
    let mut all_gate_count = 0;
    let mut lz4_gate_count = 0;
    let mut all_three_axis_count = 0;
    let mut lz4_three_axis_count = 0;
    let mut all_practical_count = 0;
    let mut lz4_practical_count = 0;
    let mut candidate_count = 0;
    let mut md = String::from(
        "# B02 adaptive block results\n\nSynthetic warm in-memory blocks; owned integer columns in and out.\nFive process repetitions; actual counts and seed offset are in results.json.\nAll custom blocks and five Parquet variants include complete framing and CRC.\nNo Zstd, real data, disk IO, durability or database functionality tested.\n\n## New candidate screen\n\nAmbitious gate: >=20% smaller AND >=2x faster full decode AND no encode regression.\nThree-axis win: smaller AND faster encode AND faster decode.\nPractical gate: >=20% smaller AND >=20% faster encode AND >=20% faster decode.\nAll-Parquet applies each criterion against every tested Parquet variant.\nSmall differences are descriptive, not statistically established wins.\n\n| Case | Candidate | Bytes/row | Encode Mrow/s | Decode Mrow/s | Size / delta-LZ4 | Encode speedup / delta-LZ4 | Decode speedup / delta-LZ4 | All-Parquet three-axis win |\n| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n",
    );
    for candidate in summaries
        .iter()
        .filter(|s| matches!(s.codec, Codec::AdaptiveSimd | Codec::AdaptiveSimdLz4))
    {
        candidate_count += 1;
        let baselines: Vec<_> = summaries
            .iter()
            .filter(|s| s.case == candidate.case && s.codec.is_parquet())
            .collect();
        let matched = baselines
            .iter()
            .find(|s| matches!(s.codec, Codec::ParquetDeltaLz4))
            .context("missing matched baseline")?;
        let pass = |b: &Summary| {
            candidate.total_bytes as f64 <= b.total_bytes as f64 * 0.8
                && b.decode_ns_median / candidate.decode_ns_median >= 2.0
                && candidate.encode_ns_median <= b.encode_ns_median
        };
        let three_axis = |b: &Summary| {
            candidate.total_bytes < b.total_bytes
                && candidate.encode_ns_median < b.encode_ns_median
                && candidate.decode_ns_median < b.decode_ns_median
        };
        let practical = |b: &Summary| {
            candidate.total_bytes as f64 <= b.total_bytes as f64 * 0.8
                && b.encode_ns_median / candidate.encode_ns_median >= 1.2
                && b.decode_ns_median / candidate.decode_ns_median >= 1.2
        };
        let all_gate = baselines.iter().all(|b| pass(b));
        all_gate_count += all_gate as usize;
        lz4_gate_count += pass(matched) as usize;
        all_three_axis_count += baselines.iter().all(|b| three_axis(b)) as usize;
        lz4_three_axis_count += three_axis(matched) as usize;
        all_practical_count += baselines.iter().all(|b| practical(b)) as usize;
        lz4_practical_count += practical(matched) as usize;
        for baseline in &baselines {
            comparisons.push(json!({
                "case": candidate.case, "candidate": candidate.codec, "baseline": baseline.codec,
                "size_ratio": candidate.total_bytes as f64 / baseline.total_bytes as f64,
                "decode_speedup": baseline.decode_ns_median / candidate.decode_ns_median,
                "encode_speedup": baseline.encode_ns_median / candidate.encode_ns_median,
                "passes_block_screen": pass(baseline),
                "three_axis_win": three_axis(baseline),
                "practical_gate": practical(baseline),
                "candidate_dominated_by_baseline": baseline.total_bytes <= candidate.total_bytes && baseline.decode_ns_median <= candidate.decode_ns_median,
            }));
        }
        writeln!(
            md,
            "| {} | {:?} | {:.3} | {:.2} | {:.2} | {:.3} | {:.2}x | {:.2}x | {} |",
            candidate.case,
            candidate.codec,
            candidate.bytes_per_row,
            candidate.encode_mrows_per_second,
            candidate.decode_mrows_per_second,
            candidate.total_bytes as f64 / matched.total_bytes as f64,
            matched.encode_ns_median / candidate.encode_ns_median,
            matched.decode_ns_median / candidate.decode_ns_median,
            if baselines.iter().all(|b| three_axis(b)) {
                "YES"
            } else {
                "NO"
            }
        )?;
    }
    writeln!(
        md,
        "\nMatched delta-LZ4 ambitious gate: {lz4_gate_count}/{candidate_count}.\nAll-Parquet ambitious gate: {all_gate_count}/{candidate_count}.\nMatched delta-LZ4 three-axis wins: {lz4_three_axis_count}/{candidate_count}.\nAll-Parquet three-axis wins: {all_three_axis_count}/{candidate_count}.\nMatched delta-LZ4 practical gate: {lz4_practical_count}/{candidate_count}.\nAll-Parquet practical gate: {all_practical_count}/{candidate_count}.\n\n## All variants\n\nRates are medians across process repetitions. Min/max and individual timings are\nin results.json and trial files. Rates include format conversion and allocations.\n\n| Case | Codec | Bytes/row | Encode Mrow/s | Decode Mrow/s | Decode min–max ms |\n| --- | --- | ---: | ---: | ---: | ---: |"
    )?;
    for s in &summaries {
        writeln!(
            md,
            "| {} | {:?} | {:.3} | {:.2} | {:.2} | {:.3}–{:.3} |",
            s.case,
            s.codec,
            s.bytes_per_row,
            s.encode_mrows_per_second,
            s.decode_mrows_per_second,
            s.decode_ns_min / 1e6,
            s.decode_ns_max / 1e6
        )?;
    }
    md.push_str("\n## Parquet size/decode Pareto frontier\n\nFrontier membership ignores encode speed; inspect encode results separately.\n\n| Case | Nondominated Parquet variants |\n| --- | --- |\n");
    let mut cases: BTreeMap<&str, Vec<&Summary>> = BTreeMap::new();
    for s in summaries.iter().filter(|s| s.codec.is_parquet()) {
        cases.entry(&s.case).or_default().push(s);
    }
    for (case, variants) in cases {
        let frontier: Vec<_> = variants
            .iter()
            .filter(|v| {
                !variants.iter().any(|b| {
                    b.total_bytes <= v.total_bytes
                        && b.decode_ns_median <= v.decode_ns_median
                        && (b.total_bytes < v.total_bytes
                            || b.decode_ns_median < v.decode_ns_median)
                })
            })
            .map(|v| format!("{:?}", v.codec))
            .collect();
        writeln!(md, "| {case} | {} |", frontier.join(", "))?;
    }
    write_json(
        &output.join("results.json"),
        &json!({"experiment": "B02", "provenance": provenance, "samples": samples, "summaries": summaries, "comparisons": comparisons, "matched_lz4_gate_count": lz4_gate_count, "all_parquet_gate_count": all_gate_count, "matched_three_axis_count": lz4_three_axis_count, "all_three_axis_count": all_three_axis_count, "matched_practical_count": lz4_practical_count, "all_practical_count": all_practical_count, "candidate_case_count": candidate_count}),
    )?;
    fs::write(output.join("REPORT.md"), md)?;
    println!(
        "Matched delta-LZ4: {lz4_gate_count}/{candidate_count}; all tested Parquet: {all_gate_count}/{candidate_count}"
    );
    println!(
        "Three-axis wins vs matched/all: {lz4_three_axis_count}/{all_three_axis_count} out of {candidate_count}"
    );
    println!("Report: {}", output.join("REPORT.md").display());
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help") {
        println!(
            "gqodb-blocks --output NEW_DIRECTORY [--sizes 4096,65536,262144] [--repetitions 5] [--seed-offset 0]\nRun from a clean committed source tree. Release builds only for measurements."
        );
        return Ok(());
    }
    let output = PathBuf::from(arg(&args, "--output").context("--output required; see --help")?);
    let sizes_arg = arg(&args, "--sizes").unwrap_or_else(|| "4096,65536,262144".into());
    let sizes: Vec<usize> = sizes_arg
        .split(',')
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()?;
    ensure!(
        !sizes.is_empty()
            && sizes
                .iter()
                .all(|&s| s > 0 && s <= gqodb_blocks::data::MAX_ROWS),
        "invalid sizes"
    );
    let seed_offset: u64 = arg(&args, "--seed-offset")
        .unwrap_or_else(|| "0".into())
        .parse()?;
    ensure!(seed_offset <= 1_000_000_000, "seed offset out of bounds");
    if let Some(worker_id) = arg(&args, "--worker") {
        return worker(&output, &sizes, worker_id.parse()?, seed_offset);
    }
    ensure!(
        !cfg!(debug_assertions),
        "use a release build for benchmark runs"
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let status = command(root, "git", &["status", "--porcelain"])?;
    ensure!(
        status.is_empty(),
        "benchmark requires a clean committed source tree: {status}"
    );
    let repetitions: usize = arg(&args, "--repetitions")
        .unwrap_or_else(|| "5".into())
        .parse()?;
    ensure!((1..=30).contains(&repetitions), "repetitions out of bounds");
    ensure!(!output.exists(), "output directory already exists");
    fs::create_dir_all(&output)?;
    let output = output.canonicalize()?;
    let executable = std::env::current_exe()?;
    let proc_status = fs::read_to_string("/proc/self/status")?;
    let affinity = proc_status
        .lines()
        .find(|l| l.starts_with("Cpus_allowed_list:"))
        .unwrap_or("unknown");
    let provenance = json!({
        "source_sha": command(root, "git", &["rev-parse", "HEAD"])?,
        "source_clean_at_start": true,
        "cargo_lock_sha256": hash_file(root.join("Cargo.lock"))?,
        "protocol_sha256": hash_file(root.join("docs/B02_PROTOCOL.md"))?,
        "executable_sha256": hash_file(&executable)?,
        "rustc": command(root, "rustc", &["--version", "--verbose"])?,
        "cpu": command(root, "lscpu", &[])?,
        "kernel": command(root, "uname", &["-a"])?,
        "affinity": affinity, "meminfo": fs::read_to_string("/proc/meminfo")?,
        "cpu_governor": fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor").ok(),
        "loadavg": fs::read_to_string("/proc/loadavg")?,
        "cgroup_cpu_max": fs::read_to_string("/sys/fs/cgroup/cpu.max").ok(),
        "cgroup_memory_max": fs::read_to_string("/sys/fs/cgroup/memory.max").ok(),
        "cgroup_effective_cpus": fs::read_to_string("/sys/fs/cgroup/cpuset.cpus.effective").ok(),
        "invocation": args, "unix_start_seconds": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        "measurement": "warm in-memory owned column roundtrip; no disk IO in timings",
        "repetitions": repetitions, "sizes": sizes, "seed_offset": seed_offset,
        "pure_rust_compression": true, "zstd_tested": false,
    });
    let mut samples = Vec::new();
    for repetition in 0..repetitions {
        let status = Command::new(&executable)
            .arg("--worker")
            .arg(repetition.to_string())
            .arg("--output")
            .arg(&output)
            .arg("--sizes")
            .arg(&sizes_arg)
            .arg("--seed-offset")
            .arg(seed_offset.to_string())
            .status()?;
        ensure!(
            status.success(),
            "worker {repetition} failed; partial output preserved"
        );
        let trial: Vec<Sample> =
            serde_json::from_slice(&fs::read(output.join(format!("trial-{repetition}.json")))?)?;
        samples.extend(trial);
    }
    report(&output, &samples, provenance)
}
