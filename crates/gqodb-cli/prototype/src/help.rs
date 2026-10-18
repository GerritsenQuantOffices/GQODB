//! Help topics for `help <topic>`.
pub const TOPICS: &[&str] = &["clocks", "ranges", "steps", "flags", "errors"];

pub fn topic(name: &str) -> Option<String> {
    let text = match name {
        "clocks" => {
            "clocks (choose one; there is no default):\n  available   when the record became available (indexed in files from the codec)\n  received    when it was received on the local clock (also: local, wall); can be empty\n  source      the time of the source, exchange or broker (also: broker); can be empty\n  sequence    the capture sequence number (whole numbers, not a time)\nA row whose value on the chosen clock is empty is outside every range on that clock; the count is reported.\nSet one for the session with: use clock <clock>\n"
        }
        "ranges" => {
            "ranges: <begin>..<end>, UTC, the end is not included.\n  2026-03-01..2026-03-02                  one day\n  2026-03-01T12:30:00.123456789Z..        from a moment on (up to 9 fraction digits)\n  ..2026-03-01                            up to a moment\n  100..200 (with clock sequence)          sequence numbers\n"
        }
        "steps" => {
            "steps (after |, each at most once; they apply after selection):\n  order [on <clock>] [desc]   sort; rows with an empty clock value come last\n  skip <n>                    pass over the first n rows\n  take <n>                    at most n rows\n  columns <a>,<b>             choose and order columns\n"
        }
        "flags" => {
            "flags: --format table|csv|json|ndjson  --out <new file, never overwritten>  --time iso|ns\n       --max-rows <n> (1000, at most 1000000)  --max-scan-rows <n> (1048576, at most 25600000)  --max-blocks <n> (4096, at most 100000)\n       --strict (a truncated result exits 3)  --skip-bad (skip unreadable files with a notice, exit 1)\n"
        }
        "errors" => {
            "errors: E-SYNTAX E-CLOCK E-RANGE E-STREAM E-COLUMN E-PATH E-VENUE E-ENGINE E-BUDGET E-FILE\nexit codes: 0 done (also when truncated; see the notice), 1 could not read or write, 2 wrong command, 3 truncated with --strict\n"
        }
        _ => return None,
    };
    Some(text.to_string())
}
