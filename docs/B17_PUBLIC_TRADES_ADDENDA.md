# B17 addenda (dated; the protocol file itself is never edited)

Each entry says when it was written and what had already been seen. Entries marked
"pre-measurement" were made before any B17 candidate ran on the real input.

## A-001, 2026-10-17, pre-measurement: clarifications found while implementing

State of knowledge: the protocol was frozen (sha256 pinned) and committed first. Candidates
were then exercised only on synthetic and fixture data (a few hundred rows). No size or
timing on the real input exists. None of the points below changes a choice in a direction
that could favour GQODB; they are listed so a reviewer can see every difference between
the frozen text and the code.

1. Tie order of the decision rules. Section 10 does not say what happens when a ratio sits
   exactly on a band edge. The code tests BETTER, then WORSE, then EQUIVALENT, then
   INCONCLUSIVE (`benchmarks/public_trades/stats.py`).
2. ClickHouse ingest settings. Section 4.1 lists `max_insert_block_size` and
   `min_insert_block_size_rows`. A third setting, `min_insert_block_size_bytes = 2^34`, is
   also set, because the 256 MiB default would split the full-month inputs into several
   parts. It applies to both ClickHouse candidates and is a layout setting, not tuning.
3. ClickHouse startup subtraction. The `SELECT 1 FORMAT Null` startup process uses a fresh
   empty path for `write` and the existing store for the read phases, so that it is in the
   same state as the process it is subtracted from. The write window ends after the common
   fsync pass, like for every other candidate.
4. `noop` is run once per candidate and round (not once per tool); resource figures are
   reported above that candidate's own `noop` of the same round.
5. `normalise` runs once per round inside the V2 loop, so "write incl. normalisation" pairs
   rounds of the same loop.
6. The Rust Parquet read path re-attaches the protocol schema to the decoded batches inside
   the timed region (a zero-copy operation); the range read includes the exact row filter, as in B12.
7. The low CPU ratio flag (below 0.85 in a warm read) uses the `wait4` rusage of the
   adapter process divided by its end-to-end wall time; the `noop` phase is not subject to it.
8. The fetch receipt stores the host and the object key as two fields (no joined URL).
9. Datasets named `FX1`, `FX2`, `FX3` are synthetic fixtures used by `smoke` only; their
   output is never evidence and the verifier rejects a receipt whose purpose is not
   "published run".

## A-002, 2026-10-17, pre-measurement: tooling validated on the real input

State of knowledge: after the protocol was frozen, `harness.py prepare` was run once on the
two registered slices (D1, D2) and on the full month S1 (3,132,841 rows, 96 blocks, references agree) to validate the converter, the two
independent reference derivations and the V1/V2 normalisation check on real rows. No
candidate (GQODB, Parquet, DuckDB, ClickHouse) was run on real data and no size or time of
any candidate exists. Facts learned about the input itself: both slices are 32 blocks of
32,768 rows; the 60-second window of the registered rule holds 45 rows in D1 and 468 rows in
D2; for both slices the V2 values obtained from V1 floats equal those parsed from the text
(float path agrees with text path).

## A-003, 2026-10-17, pre-measurement: owner decisions and run set-up

State of knowledge: nothing was measured yet. The `prepare` checks of A-002 are the only runs
on the real input. The statistical rules, bands, thresholds, windows, candidate settings and
the headline definition of the protocol are **unchanged** for GQODB, the Parquet
configurations, DuckDB and ClickHouse.

1. Candidate set. The owner dropped one optional baseline adapter that had been planned from the
   project (no reason is given here). Its code, tests and mentions were removed. The protocol file
   was edited once for this, before any measurement and with this entry as the record: the table row
   for that adapter and the words "; ... excluded" in the cell-count sentence were deleted, nothing
   else. The sha256 of the protocol changed from
   `a9abed36488030ef7e449ccb2e007be164da774f22962c132f6de55ad512b68b` (a version that is not in
   this repository's history) to
   `421966afb82a5fd21293287e042bbde45415fb2295bee33e0c26e66766fe0b98` and the pin file was updated
   in the same commit. Every candidate in the protocol, the receipt and the results was measured.
2. Input data. On 2026-10-17 the owner decided to use Binance Vision data (CC BY-NC-SA 4.0,
   Vision Dataset Terms v1.0 of 26 August 2026) for the public benchmark at his own risk,
   accepting the legal uncertainty described in `docs/BASELINE_LICENCES.md`. This is not legal
   advice. Consequences: prominent attribution in the benchmark README and in every
   `ASSESSMENT.md`; no data committed; published results are aggregates only. To honour the
   last point the published `inputs.json` omits the first and last `agg_id` and timestamp and the
   window bounds of each dataset (they are recomputed from the data by anyone who fetches it);
   row counts, block counts, window row counts, hashes and whole-table digests are kept. This
   changes what is published, not what is measured.
3. Run set-up and harness changes made before the run (committed first, receipt names that
   commit): the matrix is written to an append-only checkpoint after every round and a restarted
   run resumes at the next round of the same cell (refused if the commit or protocol hash
   differ); the box-wide background busy figure measured before each round is stored with the
   samples of that round; a process failure in the very first attempt of a cell stops the run
   so that a systematic error is fixed instead of consuming the repeat budget. The environment
   record is taken at the start of the run.
4. MEASURE-GO. Given by the owner on 2026-10-17. Pinned core 10 (sibling
   24 idle), harness core 11. The fuzzing campaign of this release (`docs/FUZZING.md`) runs on CPUs
   8, 9, 22 and 23 at nice 19 during the measurement, so the box can never satisfy "at most 4 busy CPU-equivalents outside
   the benchmark cores" while it runs. Before the first measured round the quiet rule is
   therefore changed in one respect: CPUs 8, 9, 22 and 23 are declared foreign cores like CPUs 6
   and 20 (not counted by the quiet rule). Everything else counts as before with the same limit
   of 4, and the busy time on the declared foreign cores is stored with every round
   (`foreign_busy`). The pinned core and its sibling are not affected by the declaration: a
   foreign process touching the sibling CPU still flags the sample. The fuzzer shares the
   last-level cache and memory bandwidth with the benchmark core; this is not controlled and
   is a limitation of the result. The declaration treats all candidates identically (they are
   alternated round by round); it is an environment fact, not a choice among results. A
   long-running unpinned research job (nice 15) also exists on the box and may wander onto the
   benchmark cores; the sibling-busy flag and the repeat rule are the only protection against it.

## A-004, 2026-10-17, after a first start of the run was aborted: interference thresholds and repeat budget (an amendment)

This entry is an amendment of the registered interference rule and repeat budget, not a
clarification of the frozen text. Sections 5 and 6 below, added on 2026-10-18, state what it
changed in the labels.

State of knowledge, stated exactly. The measurement was started once under A-003 and stopped
after five rounds of D1/v1 (rounds 0 to 2 warm-up, rounds 3 and 4 measured) because nearly every
round was flagged. That checkpoint was set aside (not published, not used for any number) and the
run was restarted from an empty checkpoint. What was seen of that attempt: the flag counts per
sample, and the printed timings of the first round of `gqodb` on D1/v1 (a few lines of output,
no comparison). No ratio, interval or verdict was computed. The following changes were decided
from the flag statistics (the registered rule flagged 17 of 345 samples, 14 sibling-busy and 3 low
CPU ratio; the new rule flags 4, 3 and 1. This entry first counted only the sibling flag, 14 and 3;
the totals were recomputed on 2026-10-18 from the set-aside checkpoint) and a 60-second look at the busy time of the idle sibling CPU, not from any timing value.

1. Sibling flag. The registered rule (sibling busy more than max(2 jiffies, 2% of the sample)) sits
   below the normal housekeeping noise of an idle CPU on this shared machine (about 1 to 2
   jiffies per second, with bursts), so with 66 to 69 samples per round almost every round
   would be flagged. The threshold becomes max(5 jiffies, 10% of the sample's wall time). Bursts
   of 12 to 20 jiffies seen in the first attempt (a foreign process on the sibling) are still
   flagged.
2. The CPU-to-wall ratio flag in warm reads (below 0.85) is only evaluated for samples of at least
   100 ms end-to-end; below that the accounting resolution of the kernel makes it meaningless
   (processes of 10 ms were flagged in the first attempt).
3. Repeat budget. Because one flagged sample still repeats the whole round, the number of extra
   rounds allowed per cell rises from 5 to 25 (at most 3 + 15 + 25 attempts). The 15 measured
   rounds that enter the statistics are still only unflagged rounds; if fewer than 15 can be
   obtained the cell is labelled UNDERPOWERED as before.
4. The sensitivity analysis over all measured rounds (including flagged ones) is part of every
   summary and is the check on whether these changes matter.

These changes apply to all candidates identically. They loosen the registered interference rule;
a reader who prefers the registered thresholds can recompute from the stored per-sample
`sibling_busy_jiffies` and `e2e_ns` fields.

5. What this changed in the labels (added 2026-10-18, recomputed from the stored samples with the
   registered rules). With the registered thresholds (sibling busy above max(2 jiffies, 2% of the
   sample), CPU-ratio flag without the 100 ms floor) a round counts as flagged when any of its
   samples is flagged. Over all rounds attempted, the number of unflagged measured rounds per cell
   would have been 8 (D1/v1), 1 (D1/v2), 3 (D2/v1), 1 (D2/v2), 3 (S1/v1), 0 (S1/v2), 4 (S2/v1) and
   6 (S2/v2): every cell would have been labelled UNDERPOWERED. With the amended thresholds but the
   registered repeat budget (5 extra rounds, so at most 20 measured rounds per cell) the numbers
   would have been 15, 13, 15, 12, 11, 15, 15 and 15: three cells (D1/v2, D2/v2, S1/v1) would have
   been UNDERPOWERED. "No cell is UNDERPOWERED" and the 15 unflagged rounds per cell therefore rest
   on A-004. The sensitivity analysis in `summary.json` (all measured rounds, flagged included)
   changes 6 verdicts out of 864 comparisons (640 on the ten core metrics, 192 on the three end-to-end
   columns and 32 on write-including-normalisation), none on a headline axis; this is the check on whether the amendment
   matters for the conclusions.
6. What cannot be checked. The entry says the decision used flag statistics and not timings. The
   set-aside checkpoint also holds the timings of the first rounds, it is not published, and
   nothing in this repository can show that they were not looked at.

## A-005, 2026-10-18, after the measurement: the receipt commit and the measured code in this history

State of knowledge: the B17 run (A-003, A-004) was finished and sealed on 2026-10-17. Afterwards the
public history was rebuilt as a short series of commits from the final tree. Nothing was measured
again.

1. The receipt (`results/b17-public-trades-20261017/BUILD_RECEIPTS.json`) names the commit the run was
   built from, `3323f73c06f8076b6f5054783e02043a6c3c55a3`. That commit is part of the private development
   history and is not in this repository (the same holds for the commit ids in the B13 and B15
   receipts). The measured code is identified here by git object hashes instead. One identity can be
   checked without that history: `sha256sum Cargo.lock` equals `cargo_lock_sha256` in the receipt.
2. Paths that were equal at the measured commit. In this history, the commit with the subject
   `feat(bench): B17 public-trades tooling ...` (the tooling commit; the next commit only adds the
   results, the addenda, the manifest and the evidence-verifier script) has exactly these object hashes, taken from the measured
   commit; `git rev-parse <commit>:<path>` shows them:

   | Path | Object hash at the measured commit | Tooling commit | Final tree |
   | --- | --- | --- | --- |
   | `Cargo.toml` | `27fc0b1a2bd2f43642a3da3e687c43c613c1ab10` | equal | equal |
   | `Cargo.lock` | `13a7a009a3a086e79ed6e668b1b2247604bfa89a` | equal | equal |
   | `crates` | `4d78f7a2f33beccd2d67d45025b96e99feca2587` | equal | **differs** (`f6778cdffa88450f3604766997b0386e968f34b8`) |
   | `benchmarks/baselines` | `9eeebd205969b4465c71eab51d32d4264ce7abde` | equal | equal |
   | `benchmarks/public_trades/common.py` | `86094f91814b01f049a7a843d5010703c12372f8` | equal | equal |
   | `benchmarks/public_trades/harness.py` | `a6a00069031e8c31dc5ef63ffb160c8074e9a778` | equal | equal |
   | `benchmarks/public_trades/report.py` | `86a1db94cfd3834b820caae73a4913d9dd67b14d` | equal | equal |
   | `benchmarks/public_trades/stats.py` | `27567e6fcb4e6bc090da3ae41cea009f30e19a72` | equal | equal |
   | `benchmarks/public_trades/fetch.sh` | `29688dd7ce4147fdc942ee3dabdaa682600a24c3` | equal | equal |

   The tooling commit also carries the B17 Python tests, the fixtures, the runner script and the
   workflow as they were at the measured commit (the manifest and its checker follow in the next
   commit), with two exceptions: `verify_b17.py` and `tests/test_b17_public_trades.py`
   no longer contain the name patterns that the first version of the verifier used; they check
   generic leak patterns instead (absolute home paths, carrier-grade-NAT addresses of the kind VPN
   overlays use, e-mail addresses, private-key blocks).
3. Paths that differ between the measured commit and the final tree. Under the paths in the table:
   - `benchmarks/public_trades/verify_b17.py` (generic leak patterns; accepts a redacted result file that
     is listed in `results/PUBLIC_REDACTION.json`) and `benchmarks/public_trades/README.md` (status and
     wording). Neither runs during a measurement.
   - `crates/*/Cargo.toml` (crates.io metadata; `gqodb-blocks` also registers the `malformed` test
     target), new `LICENSE` and `NOTICE` files per crate, `crates/*/README.md`, and comments and doc
     examples in `crates/*/prototype/src/lib.rs` and `gqodb-types/.../timing.rs`. After removing comment
     lines, the code of those five source files is identical to the measured one.
   - New tests: `crates/gqodb-blocks/prototype/tests/malformed.rs` and ten inputs under
     `crates/gqodb-blocks/prototype/tests/regressions/`.
   - **Code changed after the measurement:** `crates/gqodb-blocks/prototype/src/parquet.rs` and
     `crates/gqodb-blocks/prototype/src/real.rs`, by the fixes found by fuzzing (`docs/FUZZING.md`,
     fixes 1 to 8). Reading the call graph of the B17 adapters: `trades_candidate` writes and reads its
     four Rust Parquet configurations with the `parquet` crate directly and imports only `Mode` from
     `gqodb_blocks`, so `parquet.rs` and the Parquet branches of `real.rs` are not executed by those
     candidates; the GQODB candidate goes through `gqodb_store::Writer` and `Reader`, which call
     `real::encode` and `real::Decoder` with the GQODB mode, and on that path the only change is the
     added condition `end >= 12 &&` before a subtraction in `decode_with_scratch` (fix 1). That is a
     reading of the code and not a measurement: the effect on timings was not measured, and the B17
     figures were not re-measured on the final tree.
4. The A-004 numbers and labels were recomputed on the same day (see A-004, items 5 and 6), and the
   wording of `ASSESSMENT.md` was corrected; both are listed under `text_corrections` in
   `results/PUBLIC_REDACTION.json`.
