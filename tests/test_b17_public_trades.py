#!/usr/bin/env python3
"""Offline, deterministic tests of the B17 tooling (docs/B17_PUBLIC_TRADES_PROTOCOL.md).

Run: python3 tests/test_b17_public_trades.py

Uses only synthetic fixtures under tests/fixtures/b17 and generated data. No network, no
timed benchmark run: the fixture-scale smoke run discards its timings. Tests that need a
built example, DuckDB/pyarrow or ClickHouse skip with a message when the tool is absent;
the adapters themselves must then skip cleanly, which is tested too.
"""
import copy
import hashlib
import json
import os
import random
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, os.path.join(ROOT, "benchmarks", "public_trades"))
import common  # noqa: E402
import harness  # noqa: E402
import report  # noqa: E402
import stats  # noqa: E402
import verify_b17  # noqa: E402

FIXTURES = os.path.join(ROOT, "tests", "fixtures", "b17")

def examples_built():
    """Locate (building if necessary) the two Rust examples; None when that is not possible."""
    if not os.path.exists(harness.example("trades_candidate")) and not os.environ.get("B17_NO_BUILD"):
        subprocess.run(["cargo", "build", "--release", "--locked", "-p", "gqodb-store", "--examples"],
                       cwd=ROOT, check=False, capture_output=True)
    ok = all(os.path.exists(harness.example(n)) for n in ("trades_candidate", "trades_convert"))
    return ok


def have_python_tools():
    py = os.environ.get("B17_PYTHON", sys.executable)
    done = subprocess.run([py, "-c", "import duckdb, pyarrow"], capture_output=True)
    return done.returncode == 0


class Stats(unittest.TestCase):
    def test_median_and_mad(self):
        self.assertEqual(stats.median([3, 1, 2]), 2)
        self.assertEqual(stats.median([4, 1, 3, 2]), 2.5)
        self.assertEqual(stats.mad([1, 2, 3, 4, 100]), 1)

    def test_decision_rules_and_boundaries(self):
        self.assertEqual(stats.decide(0.80, 0.70, 0.90, 0.05), "BETTER")
        self.assertEqual(stats.decide(1.30, 1.10, 1.50, 0.05), "WORSE")
        self.assertEqual(stats.decide(1.00, 0.97, 1.03, 0.05), "EQUIVALENT")
        self.assertEqual(stats.decide(0.90, 0.70, 1.20, 0.05), "INCONCLUSIVE")
        # interval below one but the effect is inside the band: not BETTER, and not equivalent
        # because the interval leaves the band
        self.assertEqual(stats.decide(0.97, 0.80, 0.99, 0.05), "INCONCLUSIVE")
        # exact edge of the band is BETTER (tie order documented in the addenda)
        self.assertEqual(stats.decide(0.95, 0.95, 0.95, 0.05), "BETTER")
        # deterministic bytes: ratio only
        self.assertEqual(stats.compare([90.0] * 5, [100.0] * 5, "bytes", deterministic=True)["verdict"], "BETTER")
        self.assertEqual(stats.compare([101.0] * 5, [100.0] * 5, "bytes", deterministic=True)["verdict"], "EQUIVALENT")

    def test_bootstrap_is_reproducible_and_ordered(self):
        rng = random.Random(3)
        a = [100 + rng.random() * 5 for _ in range(15)]
        b = [150 + rng.random() * 5 for _ in range(15)]
        first = stats.bootstrap_ratio(a, b)
        self.assertEqual(first, stats.bootstrap_ratio(a, b))
        self.assertLess(first[0], first[1])
        self.assertLess(first[1], 0.75)
        iid = stats.bootstrap_ratio(a, b, iid=True)
        self.assertLess(iid[0], iid[1])
        self.assertEqual(stats.compare(a, b, "time")["verdict"], "BETTER")
        self.assertEqual(stats.compare(b, a, "time")["verdict"], "WORSE")
        self.assertEqual(stats.compare(a, [x + 0.1 for x in a], "time")["verdict"], "EQUIVALENT")


@unittest.skipUnless(examples_built(), "Rust examples are not built")
class RustReference(unittest.TestCase):
    """The Rust converter (R1) and the independent Python reference (R2) must agree."""

    def convert(self, csv, variant, rows="all", out=None):
        out = out or os.path.join(tempfile.mkdtemp(prefix="b17-test-"), "source.parquet")
        done = subprocess.run([harness.example("trades_convert"), "csv", "--csv", csv, "--variant", variant,
                               "--rows", rows, "--out", out], capture_output=True, text=True)
        return done, out

    def test_references_agree_on_fixtures(self):
        for name in ("edge_aggtrades.csv", "normalisable_aggtrades.csv"):
            path = os.path.join(FIXTURES, name)
            lines = [line.rstrip(b"\n") for line in open(path, "rb")]
            for variant in ("v1", "v2"):
                done, _ = self.convert(path, variant)
                self.assertEqual(done.returncode, 0, done.stderr)
                r1 = json.loads(done.stdout.strip().splitlines()[-1])
                r2 = common.csv_reference(lines, variant)
                self.assertTrue(harness.reference_matches(r1, r2), f"{name}/{variant}")
                self.assertGreaterEqual(r1["window"]["rows"], 1)

    def test_row_limit_takes_the_first_rows_in_file_order(self):
        path = os.path.join(FIXTURES, "edge_aggtrades.csv")
        done, _ = self.convert(path, "v1", rows="50")
        r1 = json.loads(done.stdout.strip().splitlines()[-1])
        lines = [line.rstrip(b"\n") for line in open(path, "rb")][:50]
        self.assertEqual(r1["rows"], 50)
        self.assertEqual(r1["digest"], common.csv_reference(lines, "v1")["digest"])
        done, _ = self.convert(path, "v1", rows="5000")
        self.assertNotEqual(done.returncode, 0)

    def test_converter_refuses_bad_input(self):
        good = open(os.path.join(FIXTURES, "normalisable_aggtrades.csv")).read().splitlines()
        cases = {
            "non-increasing agg_id": [good[1], good[0]],
            "nine decimals": ["1,1.000000001,1.00000000,1,1,5,True,True"],
            "short decimals": ["1,1.0,1.00000000,1,1,5,True,True"],
            "negative": ["1,-1.00000000,1.00000000,1,1,5,True,True"],
            "exponent": ["1,1e3,1.00000000,1,1,5,True,True"],
            "seven fields": ["1,1.00000000,1.00000000,1,1,5,True"],
            "lowercase bool": ["1,1.00000000,1.00000000,1,1,5,true,True"],
            "time goes back": ["1,1.00000000,1.00000000,1,1,9,True,True", "2,1.00000000,1.00000000,1,1,8,True,True"],
            "int64 overflow": ["1,1.00000000,1.00000000,1,1,9223372036854775808,True,True"],
            "v2 overflow": ["1,92233720369.00000000,1.00000000,1,1,5,True,True"],
        }
        for label, lines in cases.items():
            directory = tempfile.mkdtemp(prefix="b17-test-")
            path = os.path.join(directory, "bad.csv")
            with open(path, "w") as handle:
                handle.write("\n".join(lines) + "\n")
            for variant in ("v1", "v2"):
                if label == "v2 overflow" and variant == "v1":
                    continue
                done, _ = self.convert(path, variant, out=os.path.join(directory, f"{variant}.parquet"))
                self.assertNotEqual(done.returncode, 0, f"{label}/{variant} was accepted")
                with self.assertRaises(ValueError, msg=label):
                    common.csv_reference([l.encode() for l in lines], variant)

    def test_converter_refuses_to_overwrite_and_missing_newline(self):
        path = os.path.join(FIXTURES, "normalisable_aggtrades.csv")
        done, out = self.convert(path, "v1")
        self.assertEqual(done.returncode, 0)
        again, _ = self.convert(path, "v1", out=out)
        self.assertNotEqual(again.returncode, 0)
        directory = tempfile.mkdtemp(prefix="b17-test-")
        nonl = os.path.join(directory, "nonl.csv")
        open(nonl, "w").write("1,1.00000000,1.00000000,1,1,5,True,True")
        done, _ = self.convert(nonl, "v1", out=os.path.join(directory, "o.parquet"))
        self.assertNotEqual(done.returncode, 0)

    def test_v2_float_path_agrees_or_refuses(self):
        work = tempfile.mkdtemp(prefix="b17-test-")
        results = {}
        for name in ("normalisable_aggtrades.csv", "edge_aggtrades.csv"):
            sources = {}
            for variant in ("v1", "v2"):
                done, out = self.convert(os.path.join(FIXTURES, name), variant)
                self.assertEqual(done.returncode, 0, done.stderr)
                sources[variant] = (out, json.loads(done.stdout.strip().splitlines()[-1]))
            norm = subprocess.run([harness.example("trades_candidate"), "normalise", "--candidate", "gqodb", "--variant", "v1",
                                   "--source", sources["v1"][0], "--store", os.path.join(work, "x")], capture_output=True, text=True)
            results[name] = (norm, sources)
        norm, sources = results["normalisable_aggtrades.csv"]
        self.assertEqual(norm.returncode, 0, norm.stderr)
        self.assertEqual(json.loads(norm.stdout)["digest"], sources["v2"][1]["digest"])
        norm, _ = results["edge_aggtrades.csv"]  # contains a value that is not exactly reversible
        self.assertNotEqual(norm.returncode, 0)
        self.assertIn("exact range", norm.stderr)


@unittest.skipUnless(examples_built(), "Rust examples are not built")
class RustAdapters(unittest.TestCase):
    """Every Rust candidate is bit-exact on the edge table (negative zero, NaN payloads, int64 limits)."""

    def test_edge_table_roundtrip_per_candidate(self):
        work = tempfile.mkdtemp(prefix="b17-test-")
        for variant in ("v1", "v2"):
            source = os.path.join(work, f"edge-{variant}.parquet")
            done = subprocess.run([harness.example("trades_convert"), "synthetic-edge", "--variant", variant, "--out", source],
                                  capture_output=True, text=True)
            self.assertEqual(done.returncode, 0, done.stderr)
            ref = json.loads(done.stdout.strip().splitlines()[-1])
            for cid in ("gqodb", "parquet-delta-bss-lz4", "parquet-dictionary-lz4", "parquet-delta-bss-snappy",
                        "parquet-delta-bss-brotli5"):
                store = os.path.join(work, f"{variant}-{cid}", "data")
                os.makedirs(os.path.dirname(store))
                args = ["--candidate", cid, "--variant", variant, "--source", source, "--store", store]
                window = ["--symbol", "PAXGUSDT", "--start", str(ref["window"]["start"]), "--end", str(ref["window"]["end"])]
                prefix = harness.adapter_prefix("rust")
                for phase, extra, expected in (("write", [], None), ("read-full", [], ref["digest"]),
                                               ("read-range", window, ref["window"]["digest"])):
                    done = subprocess.run(prefix + [phase] + args + extra, capture_output=True, text=True)
                    self.assertEqual(done.returncode, 0, f"{cid} {phase}: {done.stderr}")
                    payload = json.loads(done.stdout.strip().splitlines()[-1])
                    if expected:
                        self.assertEqual(payload["digest"], expected, f"{variant}/{cid}/{phase}")
                self.assertTrue(os.listdir(store))
                again = subprocess.run(prefix + ["write"] + args, capture_output=True, text=True)
                self.assertNotEqual(again.returncode, 0, "write must refuse an existing store")

    def test_a_wrong_answer_and_a_corrupt_file_are_detected(self):
        """The exactness check must fail for a wrong answer, and a damaged GQODB frame must not read."""
        work = tempfile.mkdtemp(prefix="b17-test-")
        source = os.path.join(work, "edge.parquet")
        subprocess.run([harness.example("trades_convert"), "synthetic-edge", "--variant", "v1", "--out", source],
                       capture_output=True, check=True)
        store = os.path.join(work, "s", "data")
        os.makedirs(os.path.dirname(store))
        args = ["--candidate", "parquet-delta-bss-lz4", "--variant", "v1", "--source", source, "--store", store]
        prefix = harness.adapter_prefix("rust")
        subprocess.run(prefix + ["write"] + args, capture_output=True, check=True)
        wrong = subprocess.run(prefix + ["read-full"] + args, capture_output=True, text=True)
        digest = json.loads(wrong.stdout.strip().splitlines()[-1])["digest"]
        sample = harness.make_sample({"phase": "read-full", "cache": "warm"},
                                     {"rc": 0, "e2e_ns": 1, "payload": {"digest": digest, "op_ns": "1"}, "stderr": "",
                                      "proc_cpu_ns": 1, "sibling_busy_jiffies": 0, "wall_jiffies": 0, "loadavg1": 0.0},
                                     expected="0" * 64)
        self.assertFalse(sample["digest_ok"])
        gstore = os.path.join(work, "g", "data")
        os.makedirs(os.path.dirname(gstore))
        gargs = ["--candidate", "gqodb", "--variant", "v1", "--source", source, "--store", gstore]
        subprocess.run(prefix + ["write"] + gargs, capture_output=True, check=True)
        path = os.path.join(gstore, "trades.gqodb.ob")
        data = bytearray(open(path, "rb").read())
        data[len(data) // 2] ^= 0x01
        with open(path, "wb") as handle:
            handle.write(bytes(data))
        broken = subprocess.run(prefix + ["read-full"] + gargs, capture_output=True, text=True)
        self.assertNotEqual(broken.returncode, 0, "a corrupted frame must not decode silently")


class Digest(unittest.TestCase):
    def test_arrow_digest_matches_column_digest(self):
        try:
            import pyarrow as pa
        except ImportError:
            self.skipTest("pyarrow not installed")
        values = [0, 1, -1, 2**63 - 1, -2**63, 123456789]
        floats = [0.0, -0.0, 1.5, float("inf"), 5e-324, 1e308]
        flags = [True, False, True, True, False, False]
        names = ["", "PAXGUSDT", "Åß", "x", "yy", "zzz"]
        import array
        columns = [("agg_id", "i64", array.array("q", values)), ("price", "f64", array.array("d", floats)),
                   ("first_id", "i64", array.array("q", values[::-1])), ("is_buyer_maker", "bool", bytes(int(f) for f in flags)),
                   ("symbol", "utf8", [n.encode() for n in names])]
        expected = common.digest_columns(columns, 6)
        table = pa.table({"agg_id": pa.array(values, pa.int64()), "price": pa.array(floats, pa.float64()),
                          "first_id": pa.array(values[::-1], pa.int64()), "is_buyer_maker": pa.array(flags),
                          "symbol": pa.array(names, pa.large_string())})
        self.assertEqual(common.digest_table(table), expected)
        # chunking, slicing and string width must not matter
        chunked = pa.concat_tables([table.slice(0, 2), table.slice(2, 1), table.slice(3)])
        self.assertEqual(common.digest_table(chunked), expected)
        narrow = table.set_column(4, "symbol", table.column("symbol").cast(pa.string()))
        self.assertEqual(common.digest_table(narrow), expected)
        # a changed sign of zero changes the digest
        flipped = table.set_column(1, "price", pa.array([-0.0] + floats[1:], pa.float64()))
        self.assertNotEqual(common.digest_table(flipped), expected)
        # order canonicalisation
        shuffled = table.take(pa.array([3, 1, 0, 2, 5, 4]))
        digest, resorted = common.canonical_digest(table.sort_by("agg_id").take(pa.array([5, 4, 3, 2, 1, 0])))
        self.assertTrue(resorted)
        self.assertEqual(digest, common.digest_table(table.sort_by("agg_id")))
        del shuffled

    def test_null_is_refused(self):
        try:
            import pyarrow as pa
        except ImportError:
            self.skipTest("pyarrow not installed")
        with self.assertRaises(ValueError):
            common.digest_table(pa.table({"agg_id": pa.array([1, None], pa.int64())}))


class Fetch(unittest.TestCase):
    def fetch(self, *args, env=None):
        merged = dict(os.environ, **(env or {}))
        return subprocess.run([os.path.join(ROOT, "benchmarks", "public_trades", "fetch.sh"), *args],
                              capture_output=True, text=True, env=merged)

    def make_archive(self, directory, name, tamper=False):
        data = os.urandom(2048)
        with open(os.path.join(directory, name), "wb") as handle:
            handle.write(data)
        digest = hashlib.sha256(data if not tamper else data + b"x").hexdigest()
        with open(os.path.join(directory, name + ".CHECKSUM"), "w") as handle:
            handle.write(f"{digest}  {name}")  # Binance writes no trailing newline
        return digest

    def test_local_copy_verified_and_receipt_written(self):
        directory = tempfile.mkdtemp(prefix="b17-test-")
        digest = self.make_archive(directory, "TESTUSDT-aggTrades-2030-01.zip")
        out = os.path.join(directory, "out")
        done = self.fetch("TESTUSDT", "2030-01", "--out", out, "--zip", os.path.join(directory, "TESTUSDT-aggTrades-2030-01.zip"),
                          "--checksum", os.path.join(directory, "TESTUSDT-aggTrades-2030-01.zip.CHECKSUM"))
        self.assertEqual(done.returncode, 0, done.stderr)
        receipt = json.load(open(os.path.join(out, "TESTUSDT-aggTrades-2030-01.zip.fetch.json")))
        self.assertEqual(receipt["zip_sha256"], digest)
        self.assertEqual(receipt["source"], "local")
        self.assertEqual(receipt["host"], "data.binance.vision")

    def test_checksum_mismatch_is_refused(self):
        directory = tempfile.mkdtemp(prefix="b17-test-")
        self.make_archive(directory, "TESTUSDT-aggTrades-2030-01.zip", tamper=True)
        done = self.fetch("TESTUSDT", "2030-01", "--out", os.path.join(directory, "out"),
                          "--zip", os.path.join(directory, "TESTUSDT-aggTrades-2030-01.zip"),
                          "--checksum", os.path.join(directory, "TESTUSDT-aggTrades-2030-01.zip.CHECKSUM"))
        self.assertEqual(done.returncode, 3)
        self.assertIn("REFUSED", done.stderr)
        self.assertFalse(os.path.exists(os.path.join(directory, "out", "TESTUSDT-aggTrades-2030-01.zip.fetch.json")))

    def test_pinned_value_is_enforced_even_when_the_checksum_file_agrees(self):
        directory = tempfile.mkdtemp(prefix="b17-test-")
        self.make_archive(directory, "PAXGUSDT-aggTrades-2025-10.zip")  # self-consistent but not the registered file
        args = ["PAXGUSDT", "2025-10", "--out", os.path.join(directory, "out"), "--zip",
                os.path.join(directory, "PAXGUSDT-aggTrades-2025-10.zip"), "--checksum",
                os.path.join(directory, "PAXGUSDT-aggTrades-2025-10.zip.CHECKSUM")]
        done = self.fetch(*args)
        self.assertEqual(done.returncode, 3)
        self.assertIn("pinned", done.stderr)
        self.assertEqual(self.fetch(*args, "--no-pin").returncode, 0)

    def test_download_path_through_a_file_url(self):
        directory = tempfile.mkdtemp(prefix="b17-test-")
        remote = os.path.join(directory, "remote", "data", "spot", "monthly", "aggTrades", "TESTUSDT")
        os.makedirs(remote)
        digest = self.make_archive(remote, "TESTUSDT-aggTrades-2030-02.zip")
        out = os.path.join(directory, "out")
        done = self.fetch("TESTUSDT", "2030-02", "--out", out, env={"B17_FETCH_BASE": "file://" + os.path.join(directory, "remote")})
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(json.load(open(os.path.join(out, "TESTUSDT-aggTrades-2030-02.zip.fetch.json")))["zip_sha256"], digest)
        # a corrupted download is refused and a second run does not silently reuse the file
        with open(os.path.join(remote, "TESTUSDT-aggTrades-2030-02.zip"), "ab") as handle:
            handle.write(b"corrupt")
        out2 = os.path.join(directory, "out2")
        refused = self.fetch("TESTUSDT", "2030-02", "--out", out2, env={"B17_FETCH_BASE": "file://" + os.path.join(directory, "remote")})
        self.assertEqual(refused.returncode, 3)
        again = self.fetch("TESTUSDT", "2030-02", "--out", out, env={"B17_FETCH_BASE": "file://" + os.path.join(directory, "remote")})
        self.assertEqual(again.returncode, 2)

    def test_bad_arguments(self):
        self.assertEqual(self.fetch("paxg", "2025-10").returncode, 2)
        self.assertEqual(self.fetch("PAXGUSDT", "2025-1").returncode, 2)
        self.assertEqual(self.fetch("PAXGUSDT").returncode, 2)


class DryRun(unittest.TestCase):
    def test_plan_executes_nothing_and_counts_the_matrix(self):
        done = subprocess.run([sys.executable, os.path.join(ROOT, "benchmarks", "public_trades", "harness.py"), "plan", "--json"],
                              capture_output=True, text=True)
        self.assertEqual(done.returncode, 0, done.stderr)
        plan = json.loads(done.stdout)
        cells = {(c["dataset"], c["variant"]): c for c in plan["cells"]}
        self.assertEqual(len(cells), 8)
        self.assertEqual(len(cells[("D1", "v1")]["candidates"]), 11)
        self.assertEqual(len(cells[("S1", "v1")]["candidates"]), 7)
        self.assertIsNone(verify_b17.LEAK.search(json.dumps(plan)))
        self.assertEqual(cells[("D1", "v1")]["processes_per_round"], 11 * 6 + 3)
        self.assertEqual(cells[("D1", "v2")]["processes_per_round"], 11 * 6 + 3 + 1)
        self.assertIn("3 warm-up + 15 measured", cells[("D2", "v1")]["rounds"])
        wrapper = subprocess.run([os.path.join(ROOT, "benchmarks", "run.sh"), "public-trades", "plan", "--datasets", "D1"],
                                 capture_output=True, text=True)
        self.assertEqual(wrapper.returncode, 0, wrapper.stderr)
        self.assertIn("nothing was executed", wrapper.stdout)

    def test_rotation_alternates_and_covers_everyone(self):
        names = [c[0] for c in harness.CANDIDATES]
        firsts = [harness.rotate(harness.CANDIDATES, r)[0][0] for r in range(6)]
        self.assertGreaterEqual(len(set(firsts)), 3)  # rotation by r, reversed on odd r
        for r in range(4):
            self.assertEqual(sorted(c[0] for c in harness.rotate(harness.CANDIDATES, r)), sorted(names))
        self.assertNotEqual(harness.rotate(harness.CANDIDATES, 0), harness.rotate(harness.CANDIDATES, 1))

    def test_measure_is_refused_without_the_go_flag(self):
        env = {k: v for k, v in os.environ.items() if k != "B17_MEASURE_GO"}
        done = subprocess.run([sys.executable, os.path.join(ROOT, "benchmarks", "public_trades", "harness.py"), "measure",
                               "--work", tempfile.mkdtemp(prefix="b17-test-"), "--out", "unused"], capture_output=True, text=True, env=env)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("B17_MEASURE_GO", done.stderr + done.stdout)


class Adapters(unittest.TestCase):
    def test_absent_tools_skip_cleanly(self):
        """With no DuckDB, no pyarrow and no ClickHouse, every adapter exits 0 with a message."""
        env = {k: v for k, v in os.environ.items() if not k.startswith("B17_")}
        env["PATH"] = "/usr/bin:/bin"
        probe = subprocess.run(["/usr/bin/python3", "-c", "import duckdb"], capture_output=True, env=env)
        python = "/usr/bin/python3"
        for script, extra in (("clickhouse_adapter.py", []),):
            done = subprocess.run([python, os.path.join(ROOT, "benchmarks", "baselines", script), "noop", *extra],
                                  capture_output=True, text=True, env=env)
            self.assertEqual(done.returncode, 0, done.stderr)
            self.assertIn("skipped", done.stdout)
        if probe.returncode != 0:
            done = subprocess.run([python, os.path.join(ROOT, "benchmarks", "baselines", "duckdb_adapter.py"), "noop"],
                                  capture_output=True, text=True, env=env)
            self.assertEqual(done.returncode, 0, done.stderr)
            self.assertIn("skipped", done.stdout)

    def test_installer_refuses_a_destination_inside_the_repository(self):
        script = os.path.join(ROOT, "benchmarks", "baselines", "install_tools.py")
        inside = subprocess.run([sys.executable, script, "--dest", os.path.join(ROOT, "artifacts", "tools"), "--dry-run"],
                                capture_output=True, text=True)
        self.assertNotEqual(inside.returncode, 0)
        outside = subprocess.run([sys.executable, script, "--dest", tempfile.mkdtemp(prefix="b17-test-"), "--dry-run"],
                                 capture_output=True, text=True)
        self.assertEqual(outside.returncode, 0, outside.stderr)
        self.assertNotIn("curl", outside.stdout)


@unittest.skipUnless(examples_built() and have_python_tools(), "built examples plus duckdb and pyarrow are needed")
class EndToEndFixture(unittest.TestCase):
    def test_smoke_over_the_edge_fixtures_is_exact_for_every_available_candidate(self):
        work = tempfile.mkdtemp(prefix="b17-smoke-")
        py = os.environ.get("B17_PYTHON", sys.executable)
        done = subprocess.run([py, os.path.join(ROOT, "benchmarks", "public_trades", "harness.py"), "smoke", "--work", work,
                               "--fixtures", "FX1,FX2", "--quick", "--out", os.path.join(work, "out")],
                              capture_output=True, text=True)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("exactness/process failures: 0", done.stdout)
        data = json.load(open(os.path.join(work, "out", "smoke.json")))
        self.assertEqual(data["profile"], "quick")
        candidates = {s["candidate"] for s in data["samples"] if s["candidate"]}
        self.assertTrue({"gqodb", "parquet-delta-bss-lz4"} <= candidates)
        checked = [s for s in data["samples"] if s.get("digest_ok") is not None]
        self.assertTrue(checked)
        self.assertTrue(all(s["digest_ok"] for s in checked))
        # both references agreed for every fixture (the run would have aborted otherwise)
        for record in data["inputs"].values():
            for variant in record["variants"].values():
                self.assertTrue(variant["references_agree"])
        # smoke output is not evidence
        self.assertIsNone(verify_b17.LEAK.search(json.dumps(data)))


def fake_samples(measured=15, warmups=3, flagged=(), datasets=("D1", "D2"), exact=True):
    rng = random.Random(11)
    samples = []
    for dataset in datasets:
        for r in range(warmups + measured):
            role = "warmup" if r < warmups else "measured"
            for cand, scale in (("gqodb", 1.0), ("parquet-delta-bss-lz4", 1.5), ("duckdb-native", 2.2)):
                meta = {"dataset": dataset, "variant": "v1", "candidate": cand, "round": r, "role": role}

                def add(phase, cache, ns, **more):
                    flags = ["sibling_busy"] if (dataset, r) in flagged and phase == "write" and cand == "gqodb" else []
                    base = dict(meta, phase=phase, cache=cache, op_ns=int(ns * (1 + rng.random() * 0.03)),
                                e2e_ns=int(ns * 1.4), cpu_ns=int(ns), rss_kb=50_000 if phase != "noop" else 4_000,
                                proc_cpu_ns=int(ns), rows=1, rc=0, flags=flags)
                    base.update(more)
                    samples.append(base)

                add("noop", None, 1e6)
                add("write", None, 2e8 * scale, bytes={"apparent": int(5e6 * scale), "allocated": int(5e6 * scale), "files": 1})
                for cache in ("cold", "warm"):
                    extra = {"digest_ok": exact or cand != "duckdb-native"}
                    add("read-range", cache, 1e7 * scale, resident=0.0 if cache == "cold" else 1.0, **extra)
                    add("read-full", cache, 8e7 * scale, resident=0.0 if cache == "cold" else 1.0, **extra)
    return samples


def build_good_evidence(root, samples=None, measured=15):
    samples = samples if samples is not None else fake_samples(measured=measured)
    docs = os.path.join(root, "docs")
    os.makedirs(docs, exist_ok=True)
    for name in ("B17_PUBLIC_TRADES_PROTOCOL.md", "B17_PUBLIC_TRADES_PROTOCOL.sha256"):
        shutil.copy(os.path.join(ROOT, "docs", name), docs)
    pinned = open(os.path.join(docs, "B17_PUBLIC_TRADES_PROTOCOL.sha256")).read().split()[0]
    result = {"samples": samples, "summary": report.summarise(samples),
              "inputs": {"D1": {"input": {"host": "example.invalid"}}, "D2": {"input": {"host": "example.invalid"}}},
              "skipped_candidates": {}, "profile": "official",
              "environment": {"commit": "a" * 40, "commit_dirty": False, "rustc": "rustc 1.96.0", "cargo_lock_sha256": "b" * 64,
                              "kernel": "6.0", "cpu_model_class": "test class", "tools": {"duckdb": "1.5.6"},
                              "filesystem": {"type": "ext4", "rotational": False}}}
    out = os.path.join(root, "results", "b17-public-trades-test")
    harness.write_evidence(out, result, pinned)
    text = open(os.path.join(out, "ASSESSMENT.md")).read()
    text = text.replace("AUTHOR TO WRITE AFTER THE RUN", "Written by the test").replace("AUTHOR TO WRITE", "Written by the test")
    open(os.path.join(out, "ASSESSMENT.md"), "w").write(text)
    harness_seal(out)
    return out


def harness_seal(out):
    receipt = json.load(open(os.path.join(out, "BUILD_RECEIPTS.json")))
    names = list(receipt["result_sha256"])
    report.write_receipt(out, {k: v for k, v in receipt.items() if k != "result_sha256"}, names)


class Evidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = tempfile.mkdtemp(prefix="b17-evidence-")
        cls.good = build_good_evidence(cls.root)

    def tampered(self, mutate):
        root = tempfile.mkdtemp(prefix="b17-evidence-")
        shutil.copytree(os.path.join(self.root, "docs"), os.path.join(root, "docs"))
        target = os.path.join(root, "results", "b17-public-trades-test")
        shutil.copytree(self.good, target)
        mutate(target, root)
        return verify_b17.verify_dir(target, root)

    def edit_json(self, target, filename, change, reseal=True):
        path = os.path.join(target, filename)
        data = json.load(open(path))
        change(data)
        json.dump(data, open(path, "w"), indent=1, sort_keys=True)
        if reseal:
            harness_seal(target)

    def test_good_directory_verifies(self):
        self.assertEqual(verify_b17.verify_dir(self.good, self.root), [])
        summary = json.load(open(os.path.join(self.good, "summary.json")))
        cell = summary["cells"]["D1/v1"]
        self.assertEqual(len(cell["rounds_valid"]), 15)
        self.assertEqual(cell["comparisons"]["duckdb-native"]["write_op"]["verdict"], "BETTER")
        self.assertEqual(cell["comparisons"]["duckdb-native"]["bytes"]["verdict"], "BETTER")
        self.assertTrue(summary["claims"]["parquet-delta-bss-lz4"]["write_op"]["win"])
        # receipts are in the format the existing verifier already hash-checks
        receipt = json.load(open(os.path.join(self.good, "BUILD_RECEIPTS.json")))
        self.assertIn("ASSESSMENT.md", receipt["result_sha256"])

    def test_tampered_result_file_is_detected(self):
        def mutate(target, root):
            path = os.path.join(target, "samples.json")
            data = json.load(open(path))
            data[5]["op_ns"] += 1
            json.dump(data, open(path, "w"))  # no reseal: the receipt hash must catch it
        failures = self.tampered(mutate)
        self.assertTrue(any("samples.json does not match its receipt hash" in f for f in failures), failures)

    def test_resealed_but_inconsistent_summary_is_detected_by_recomputation(self):
        def mutate(target, root):
            def change(summary):
                summary["cells"]["D1/v1"]["comparisons"]["duckdb-native"]["write_op"]["verdict"] = "WORSE"
            self.edit_json(target, "summary.json", change)
        failures = self.tampered(mutate)
        self.assertTrue(any("differs from a recomputation" in f for f in failures), failures)

    def test_resealed_sample_edit_changes_the_recomputation(self):
        def mutate(target, root):
            def change(samples):
                for s in samples:
                    if s["candidate"] == "gqodb" and s["phase"] == "write" and s["role"] == "measured":
                        s["op_ns"] *= 5
            self.edit_json(target, "samples.json", change)
        failures = self.tampered(mutate)
        self.assertTrue(any("differs from a recomputation" in f for f in failures), failures)

    def test_fixture_output_is_not_evidence(self):
        failures = self.tampered(lambda t, r: self.edit_json(t, "BUILD_RECEIPTS.json", lambda d: d.update(purpose="fixture-smoke"), reseal=False))
        self.assertTrue(any("not 'published run'" in f for f in failures), failures)

    def test_dirty_tree_and_missing_commit(self):
        failures = self.tampered(lambda t, r: self.edit_json(
            t, "BUILD_RECEIPTS.json", lambda d: d["environment"].update(commit_dirty=True, commit="zz"), reseal=False))
        self.assertTrue(any("clean working tree" in f for f in failures), failures)
        self.assertTrue(any("no commit id" in f for f in failures), failures)

    def test_wrong_protocol_hash(self):
        failures = self.tampered(lambda t, r: self.edit_json(
            t, "BUILD_RECEIPTS.json", lambda d: d.update(protocol_sha256="0" * 64), reseal=False))
        self.assertTrue(any("different protocol hash" in f for f in failures), failures)
        def edit_protocol(target, root):
            with open(os.path.join(root, "docs", "B17_PUBLIC_TRADES_PROTOCOL.md"), "a") as handle:
                handle.write("\nedited after registration\n")
        failures = self.tampered(edit_protocol)
        self.assertTrue(any("no longer matches its pinned sha256" in f for f in failures), failures)

    def test_generic_leak_patterns_are_rejected(self):
        def add_text(text):
            def mutate(target, root):
                path = os.path.join(target, "ASSESSMENT.md")
                open(path, "a").write("\n" + text + "\n")
                harness_seal(target)
            return mutate
        def rejected(text):
            return any("must not be published" in f for f in self.tampered(add_text(text)))
        for leak in ("/home/alice/work/data.csv", "/Users/alice/Documents", "host 100.64.1.2", "host 100.127.255.254",
                     "contact alice@example.org", "-----BEGIN OPENSSH PRIVATE KEY-----", "-----BEGIN PRIVATE KEY-----",
                     "key sk-ant-abc", "/mnt/disk/x"):
            self.assertTrue(rejected(leak), leak)
        # text that merely looks similar is not a leak: public addresses, version numbers, relative paths
        for fine in ("host 100.63.1.2", "host 100.128.0.1", "version 1.00.64.1", "see docs/B17_PUBLIC_TRADES_PROTOCOL.md",
                     "home/ is a word", "a@b is not an address"):
            self.assertFalse(rejected(fine), fine)

    def test_redacted_result_file_needs_a_matching_redaction_record(self):
        def corrected(with_record, public_hash_ok=True):
            def mutate(target, root):
                path = os.path.join(target, "ASSESSMENT.md")
                original = hashlib.sha256(open(path, "rb").read()).hexdigest()
                open(path, "a").write("\nA corrected sentence.\n")
                public = hashlib.sha256(open(path, "rb").read()).hexdigest()
                receipt_path = os.path.join(target, "BUILD_RECEIPTS.json")
                receipt = json.load(open(receipt_path))
                receipt["result_sha256"]["ASSESSMENT.md"] = original
                json.dump(receipt, open(receipt_path, "w"), indent=1, sort_keys=True)
                if with_record:
                    record = {"files": [{"path": "results/b17-public-trades-test/ASSESSMENT.md", "original_sha256": original,
                                         "public_sha256": public if public_hash_ok else "0" * 64}]}
                    json.dump(record, open(os.path.join(root, "results", "PUBLIC_REDACTION.json"), "w"))
            return mutate
        self.assertEqual(self.tampered(corrected(True)), [])
        self.assertTrue(any("ASSESSMENT.md does not match its receipt hash" in f for f in self.tampered(corrected(False))))
        self.assertTrue(any("redaction record" in f for f in self.tampered(corrected(True, public_hash_ok=False))))

    def test_unfilled_author_sections_are_rejected(self):
        def mutate(target, root):
            path = os.path.join(target, "ASSESSMENT.md")
            open(path, "a").write("\nAUTHOR TO WRITE\n")
            harness_seal(target)
        self.assertTrue(any("unfilled author sections" in f for f in self.tampered(mutate)))

    def test_missing_file_and_uncovered_file(self):
        self.assertTrue(any("missing plan.json" in f for f in self.tampered(lambda t, r: os.remove(os.path.join(t, "plan.json")))))
        def extra(target, root):
            open(os.path.join(target, "extra.txt"), "w").write("x")
        self.assertTrue(any("not covered by the receipt" in f for f in self.tampered(extra)))

    def test_filesystem_without_eviction_is_rejected(self):
        failures = self.tampered(lambda t, r: self.edit_json(
            t, "BUILD_RECEIPTS.json", lambda d: d["environment"]["filesystem"].update(type="tmpfs")))
        self.assertTrue(any("cache eviction is meaningless" in f for f in failures), failures)

    def test_incomplete_rounds_must_be_labelled_underpowered(self):
        root = tempfile.mkdtemp(prefix="b17-evidence-")
        out = build_good_evidence(root, fake_samples(measured=6), measured=6)
        failures = verify_b17.verify_dir(out, root)
        self.assertEqual(failures, [])
        summary = json.load(open(os.path.join(out, "summary.json")))
        self.assertTrue(summary["cells"]["D1/v1"]["underpowered"])
        self.assertIn("UNDERPOWERED", open(os.path.join(out, "ASSESSMENT.md")).read())

    def test_flagged_rounds_are_excluded_but_kept(self):
        samples = fake_samples(flagged={("D1", 5), ("D1", 9)}, measured=17)
        summary = report.summarise(samples)
        cell = summary["cells"]["D1/v1"]
        self.assertEqual(cell["rounds_flagged"], [5, 9])
        self.assertEqual(len(cell["rounds_valid"]), 15)
        self.assertNotIn(5, cell["rounds_valid"])
        self.assertEqual(len(summary["sensitivity_all_rounds"]["D1/v1"]["rounds_valid"]), 17)
        self.assertEqual(sum(1 for s in samples if s["flags"]), 2)

    def test_exactness_failure_and_cold_state_are_first_class(self):
        summary = report.summarise(fake_samples(exact=False))
        verdict = summary["cells"]["D1/v1"]["comparisons"]["duckdb-native"]["read_full_warm_op"]["verdict"]
        self.assertEqual(verdict, "EXACTNESS FAIL")
        warm = fake_samples()
        for s in warm:
            if s["candidate"] == "parquet-delta-bss-lz4" and s.get("cache") == "cold":
                s["resident"] = 0.9
        v = report.summarise(warm)["cells"]["D1/v1"]["comparisons"]["parquet-delta-bss-lz4"]["read_full_cold_op"]
        self.assertEqual(v["verdict"], "NOT COMPARABLE")
        self.assertEqual(report.summarise(warm)["cells"]["D1/v1"]["comparisons"]["parquet-delta-bss-lz4"]["write_op"]["verdict"], "BETTER")


class Resume(unittest.TestCase):
    """The long run must be resumable and must not publish per-trade values."""

    def make_ctx(self, directory):
        return {"work": directory, "cpu": 10, "quiet": False, "excluded": set(), "stop_on_first_failure": True,
                "checkpoint": os.path.join(directory, "checkpoint.jsonl")}

    def fake_run_round(self, calls, flag=()):
        def run_round(ctx, dataset, variant, r, role, candidates, inputs):
            calls.append(r)
            sample = {"dataset": dataset, "variant": variant, "candidate": "gqodb", "round": r, "role": role,
                      "phase": "write", "cache": None, "op_ns": 1, "e2e_ns": 1, "rc": 0,
                      "flags": ["sibling_busy"] if r in flag else []}
            return [sample]
        return run_round

    def test_resume_continues_at_the_next_round_and_keeps_the_flag_rule(self):
        directory = tempfile.mkdtemp(prefix="b17-test-")
        ctx = self.make_ctx(directory)
        original = harness.run_round
        try:
            calls = []
            harness.run_round = self.fake_run_round(calls, flag={4})
            # first run is "killed" after five rounds: emulate by limiting through a failing round 5
            def dying(ctx_, dataset, variant, r, role, candidates, inputs, inner=harness.run_round):
                if r == 5:
                    raise KeyboardInterrupt
                return inner(ctx_, dataset, variant, r, role, candidates, inputs)
            harness.run_round = dying
            with self.assertRaises(KeyboardInterrupt):
                harness.measure_cell(ctx, "D1", "v1", {}, {"gqodb"}, "official")
            self.assertEqual(calls, [0, 1, 2, 3, 4])
            header, done = harness.load_checkpoint(ctx["checkpoint"])
            self.assertEqual(len(done[("D1", "v1")]), 5)
            self.assertTrue(done[("D1", "v1")][4]["flagged"])
            # a torn last line (crash during write) is ignored
            with open(ctx["checkpoint"], "a") as handle:
                handle.write('{"type": "round", "dataset": "D1", "var')
            header, done = harness.load_checkpoint(ctx["checkpoint"])
            self.assertEqual(len(done[("D1", "v1")]), 5)
            calls2 = []
            harness.run_round = self.fake_run_round(calls2, flag={4})
            samples = harness.measure_cell(ctx, "D1", "v1", {}, {"gqodb"}, "official", done[("D1", "v1")])
            self.assertEqual(calls2[0], 5, "must not repeat finished rounds")
            measured_valid = {s["round"] for s in samples if s["role"] == "measured" and not s["flags"]}
            self.assertEqual(len(measured_valid), 15)
            self.assertEqual(len({s["round"] for s in samples}), 19)  # 3 warm-ups, 15 valid, 1 flagged
            # a complete cell is not run again
            calls3 = []
            harness.run_round = self.fake_run_round(calls3)
            _, done = harness.load_checkpoint(ctx["checkpoint"])
            harness.measure_cell(ctx, "D1", "v1", {}, {"gqodb"}, "official", done[("D1", "v1")])
            self.assertEqual(calls3, [])
        finally:
            harness.run_round = original

    def test_repeat_budget_is_bounded(self):
        directory = tempfile.mkdtemp(prefix="b17-test-")
        ctx = self.make_ctx(directory)
        original = harness.run_round
        try:
            calls = []
            harness.run_round = self.fake_run_round(calls, flag=set(range(3, 100)))
            samples = harness.measure_cell(ctx, "D1", "v1", {}, {"gqodb"}, "official")
            self.assertEqual(len(calls), 3 + 15 + harness.PROFILE["official"]["extra"])
            self.assertEqual(report.valid_rounds(samples), [])
        finally:
            harness.run_round = original

    def test_interference_flag_thresholds_of_addendum_a004(self):
        def sample(sibling, wall_ns, cpu_ns, phase="read-full", cache="warm"):
            return harness.make_sample({"phase": phase, "cache": cache},
                                       {"rc": 0, "e2e_ns": wall_ns, "payload": {"op_ns": "1"}, "stderr": "", "proc_cpu_ns": cpu_ns,
                                        "sibling_busy_jiffies": sibling, "wall_jiffies": wall_ns / 1e9 * 100, "loadavg1": 0.0})["flags"]
        self.assertEqual(sample(5, 10**9, 10**9), [])           # housekeeping noise on an idle sibling
        self.assertEqual(sample(6, 10**9, 10**9), [])           # 6% of the sample
        self.assertEqual(sample(12, 10**9, 10**9), ["sibling_busy"])
        self.assertEqual(sample(6, 3 * 10**8, 3 * 10**8), ["sibling_busy"])  # 20% of a 0.3 s sample
        self.assertEqual(sample(0, 10**7, 10**6), [])           # a 10 ms process: CPU ratio is not meaningful
        self.assertEqual(sample(0, 5 * 10**8, 10**8), ["low_cpu_ratio"])
        self.assertEqual(sample(0, 5 * 10**8, 10**8, cache="cold"), [])

    def test_systematic_failure_in_the_first_round_stops_the_run(self):
        directory = tempfile.mkdtemp(prefix="b17-test-")
        ctx = self.make_ctx(directory)
        original = harness.run_round
        try:
            harness.run_round = lambda *a, **k: [{"dataset": "D1", "variant": "v1", "candidate": "gqodb", "round": 0,
                                                   "role": "warmup", "phase": "write", "rc": 1, "flags": ["process_failed"],
                                                   "stderr": "boom"}]
            with self.assertRaises(SystemExit):
                harness.measure_cell(ctx, "D1", "v1", {}, {"gqodb"}, "official")
            self.assertFalse(os.path.exists(ctx["checkpoint"]), "the failed round must not be checkpointed")
        finally:
            harness.run_round = original

    def test_published_inputs_have_no_per_trade_values_and_the_assessment_attributes_binance_vision(self):
        inputs = {"D1": {"variants": {"v1": {"report": {"rows": 5, "first_agg_id": 1, "last_agg_id": 5, "first_time": 7, "last_time": 9,
                                                          "digest": "d", "window": {"start": 7, "end": 8, "rows": 2, "digest": "w"}},
                                             "python_reference": {"rows": 5, "window": {"start": 7, "end": 8, "rows": 2}}}}}}
        clean = harness.sanitise_inputs(inputs)
        text = json.dumps(clean)
        for key in ("first_agg_id", "last_agg_id", "first_time", "last_time", '"start"', '"end"'):
            self.assertNotIn(key, text)
        self.assertEqual(clean["D1"]["variants"]["v1"]["report"]["window"]["rows"], 2)
        self.assertIn("first_agg_id", json.dumps(inputs), "the original must not be modified")
        assessment = report.render_assessment(report.summarise(fake_samples(measured=15)))
        for needle in ("Binance Vision", "CC BY-NC-SA 4.0", "Dataset Terms (version 1.0", "does not sponsor, endorse"):
            self.assertIn(needle, assessment)


class RepositoryState(unittest.TestCase):
    def test_both_result_directories_verify(self):
        """Run 1 (earlier commit) and run 2 (main result, released commit) are both checked."""
        results, failures = verify_b17.verify_all(ROOT)
        self.assertEqual(failures, [])
        names = sorted(os.path.basename(d) for d in results)
        self.assertEqual(names, ["b17-public-trades-20261017", "b17-public-trades-20261018"])
        for directory in results:
            self.assertTrue(os.path.isfile(os.path.join(directory, "BUILD_RECEIPTS.json")))

    def test_manifest_declares_run_2_as_main_and_run_1_as_additional(self):
        manifest = json.load(open(os.path.join(ROOT, "benchmarks", "manifest.json")))
        run = next(r for r in manifest["runs"] if r["id"] == "B17")
        self.assertEqual(run["evidence_directory"], "results/b17-public-trades-20261018")
        self.assertEqual(run["additional_evidence_directories"], ["results/b17-public-trades-20261017"])
        for directory in [run["evidence_directory"]] + run["additional_evidence_directories"]:
            self.assertTrue(os.path.isdir(os.path.join(ROOT, directory)))

    def test_run_2_receipt_names_a_commit_of_this_history_and_a_clean_tree(self):
        receipt = json.load(open(os.path.join(ROOT, "results", "b17-public-trades-20261018", "BUILD_RECEIPTS.json")))
        env = receipt["environment"]
        self.assertFalse(env["commit_dirty"])
        probe = subprocess.run(["git", "cat-file", "-e", env["commit"] + "^{commit}"], cwd=ROOT, capture_output=True)
        if probe.returncode != 0:
            self.skipTest("the measured commit is not available in this checkout (shallow clone?)")
        ancestor = subprocess.run(["git", "merge-base", "--is-ancestor", env["commit"], "HEAD"], cwd=ROOT, capture_output=True)
        self.assertEqual(ancestor.returncode, 0, "the receipt commit must be an ancestor of HEAD")

    def test_run_2_assessment_is_clean(self):
        text = open(os.path.join(ROOT, "results", "b17-public-trades-20261018", "ASSESSMENT.md")).read()
        self.assertNotRegex(text, r"(?i)another (agent|project)|coordinator|fuzz(ing)? (job|campaign)")
        self.assertIn("Binance Vision", text)
        self.assertIn("UNDERPOWERED", text)

    def test_protocol_is_pinned(self):
        pinned = open(os.path.join(ROOT, "docs", "B17_PUBLIC_TRADES_PROTOCOL.sha256")).read().split()[0]
        self.assertEqual(verify_b17.sha256(os.path.join(ROOT, "docs", "B17_PUBLIC_TRADES_PROTOCOL.md")), pinned)

    def test_no_generic_leak_pattern_in_the_published_text(self):
        for base in ("results", "README.md", "benchmarks/README.md", "benchmarks/public_trades/README.md"):
            path = os.path.join(ROOT, base)
            files = [path] if os.path.isfile(path) else [os.path.join(d, f) for d, _, fs in os.walk(path) for f in fs]
            for f in files:
                if f.endswith((".json", ".md")):
                    found = verify_b17.LEAK.search(open(f, errors="replace").read())
                    self.assertIsNone(found, f"{f}: {found and found.group(0)!r}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
