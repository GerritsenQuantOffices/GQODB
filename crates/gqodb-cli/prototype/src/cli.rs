//! The front door: flags, joining arguments into one command, running scripts and the REPL, exit codes.
//! Exit codes: 0 done (also when truncated, which is reported), 1 could not read or write, 2 wrong command, 3 truncated with `--strict`.
use crate::exec::{ExecError, Executor, FileExecutor, Options, Report};
use crate::sink::{Format, make_sink};
use crate::value::TimeFormat;
use gqodb_lang::{Command, Session, parse, plan};
use std::io::{BufRead, BufWriter, Write};

/// Longest accepted command text.
pub const MAX_COMMAND_BYTES: usize = 1 << 20;

pub const HELP: &str = "\
gqodb: ask native tick and order-book files what they hold

usage:
  gqodb [flags] <command>          run one command (quote it if it holds | or spaces)
  gqodb [flags] run <script>       run the commands in a file, stop at the first failure
  gqodb [flags]                    read commands from the terminal (or from a pipe)

commands:
  show <stream> <symbol> [<venue>] [<begin>..<end> [on <clock>]] [in <path>] [| <step>]...
  show stream=<s> symbol=<x> [venue=<v>] [range=<b>..<e>] [clock=<c>] [path=<p>] [| <step>]...
  explain show ...                 what would be read, without reading it
  use clock <clock> | use path <path>
  help | quit

streams: trades, quotes, book
clocks:  available | received (local, wall) | source (broker) | sequence
steps:   take <n> | skip <n> | columns <a>,<b> | order [on <clock>] [desc]
ranges:  <begin>..<end> in UTC, the end is not included: 2026-03-01..2026-03-02, ..2026-03-01T12:00:00Z

examples:
  gqodb \"show trades EURUSD 2026-03-01..2026-03-02 on received in ./data | take 20\"
  gqodb \"show book 'BTC-USDT' bybit 100..200 on sequence in ./data | order desc\"

flags: --format table|csv|json|ndjson  --out <new file>  --time iso|ns
       --max-rows <n> (default 1000)  --max-scan-rows <n> (default 1048576)  --max-blocks <n> (default 4096)
       --strict (a truncated result is an error)  --skip-bad (skip unreadable files, with a notice)
";

#[derive(Clone, Debug, Default)]
pub struct Flags {
    pub format: Option<Format>,
    pub out: Option<String>,
    pub opts: Options,
}

/// Separates the flags from the words of the command.
pub fn split_flags(args: &[String]) -> Result<(Flags, Vec<String>), String> {
    let mut flags = Flags::default();
    let mut words = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        let Some(flag) = arg.strip_prefix("--") else {
            words.push(arg.clone());
            continue;
        };
        let (name, inline) = match flag.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (flag, None),
        };
        let mut value = |name: &str| -> Result<String, String> {
            if let Some(v) = inline.clone() {
                return Ok(v);
            }
            let v = args
                .get(i)
                .cloned()
                .ok_or_else(|| format!("--{name} needs a value"))?;
            i += 1;
            Ok(v)
        };
        let number = |name: &str, text: String| {
            text.parse::<u64>()
                .map_err(|_| format!("--{name} needs a whole number"))
        };
        match name {
            "format" => {
                let v = value(name)?;
                flags.format =
                    Some(Format::parse(&v).ok_or_else(|| {
                        format!("unknown format `{v}`: table, csv, json or ndjson")
                    })?);
            }
            "out" => flags.out = Some(value(name)?),
            "time" => {
                flags.opts.time = match value(name)?.as_str() {
                    "iso" => TimeFormat::Iso,
                    "ns" => TimeFormat::Ns,
                    other => return Err(format!("unknown time format `{other}`: iso or ns")),
                }
            }
            "max-rows" => flags.opts.max_rows = number(name, value(name)?)?,
            "max-scan-rows" => flags.opts.max_scan_rows = number(name, value(name)?)?,
            "max-blocks" => flags.opts.max_blocks = number(name, value(name)?)?,
            "strict" => flags.opts.strict = true,
            "skip-bad" => flags.opts.skip_bad = true,
            "help" => words.push("help".to_string()),
            other => return Err(format!("unknown flag `--{other}`")),
        }
    }
    Ok((flags, words))
}

/// Joins shell words into one command text. A single word is a whole command line (`gqodb "show trades A | take 5"`) and
/// stays as it is. Among several words, one with whitespace was quoted by the user, so it is quoted again.
pub fn join_args(words: &[String]) -> String {
    if let [only] = words {
        return only.clone();
    }
    let parts: Vec<String> = words
        .iter()
        .map(|w| {
            if w.is_empty() {
                "''".to_string()
            } else if w.chars().any(char::is_whitespace) {
                if w.contains('\'') {
                    format!("\"{w}\"")
                } else {
                    format!("'{w}'")
                }
            } else {
                w.clone()
            }
        })
        .collect();
    parts.join(" ")
}

fn footer(err: &mut dyn Write, plan: &gqodb_lang::Plan, r: &Report, opts: &Options) {
    let range = plan.range.map(|rg| {
        let show = |v: Option<i64>| match v {
            None => String::new(),
            Some(v) if rg.clock == gqodb_lang::ClockName::Sequence => v.to_string(),
            Some(v) => gqodb_lang::time::format_instant(v),
        };
        format!(
            "clock {}, range [{}, {})",
            rg.clock.name(),
            show(rg.start),
            show(rg.end)
        )
    });
    let _ = writeln!(
        err,
        "-- {}",
        range.unwrap_or_else(|| "no time range".to_string())
    );
    let _ = writeln!(
        err,
        "-- files: {} read, {} skipped · blocks: {} of {} read · bytes: {} · rows: {} scanned, {} out",
        r.files_read,
        r.skipped.len(),
        r.blocks_read,
        r.blocks_total,
        r.bytes_read,
        r.rows_scanned,
        r.rows_out
    );
    for (file, reason) in &r.skipped {
        let _ = writeln!(err, "-- skipped {file}: {reason}");
    }
    for (file, why) in &r.bad_files {
        let _ = writeln!(err, "-- could not read {file}: {why}");
    }
    if r.null_clock_excluded > 0 {
        let clock = plan.range.map_or("the chosen clock", |rg| rg.clock.name());
        let _ = writeln!(
            err,
            "-- excluded: {} rows without a {} value",
            r.null_clock_excluded, clock
        );
    }
    if let Some(t) = r.truncated {
        let _ = writeln!(
            err,
            "-- truncated: {t} (limits: {} output rows, {} scanned rows, {} blocks)",
            opts.max_rows, opts.max_scan_rows, opts.max_blocks
        );
    }
}

fn exec_error(err: &mut dyn Write, e: &ExecError) {
    let code = match e {
        ExecError::Closed(_) => return,
        ExecError::Path(_) => "E-PATH",
        ExecError::File { .. } | ExecError::Output(_) => "E-FILE",
        ExecError::Budget(_) => "E-BUDGET",
    };
    let _ = writeln!(err, "error[{code}]: {e}");
}

fn run_show(
    cmd: &gqodb_lang::Spanned<Command>,
    src: &str,
    session: &Session,
    flags: &Flags,
    tty: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> (u8, bool) {
    let p = match plan(cmd, session) {
        Ok(p) => p,
        Err(diags) => {
            for d in diags {
                let _ = write!(err, "{}", d.render(src));
            }
            return (2, false);
        }
    };
    if let Err(m) = flags.opts.validate() {
        let _ = writeln!(err, "error[E-BUDGET]: {m}");
        return (2, false);
    }
    let exec = FileExecutor;
    if p.explain {
        return match exec.explain(&p, &flags.opts) {
            Ok(lines) => {
                for l in lines {
                    let _ = writeln!(out, "{l}");
                }
                (0, false)
            }
            Err(e) => {
                exec_error(err, &e);
                (1, false)
            }
        };
    }
    let format = flags
        .format
        .unwrap_or(if tty { Format::Table } else { Format::Csv });
    let result = match &flags.out {
        Some(path) => match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => {
                let mut w = BufWriter::new(file);
                let r = {
                    let mut sink = make_sink(format, &mut w);
                    exec.execute(&p, &flags.opts, &mut *sink)
                };
                if let Err(e) = w.flush() {
                    let _ = writeln!(err, "error[E-FILE]: cannot write `{path}`: {e}");
                    return (1, false);
                }
                r
            }
            Err(e) => {
                let _ = writeln!(
                    err,
                    "error[E-FILE]: cannot create `{path}` (an existing file is never overwritten): {e}"
                );
                return (1, false);
            }
        },
        None => {
            let mut sink = make_sink(format, &mut *out);
            exec.execute(&p, &flags.opts, &mut *sink)
        }
    };
    match result {
        Err(ExecError::Closed(bad_files)) => {
            for (file, why) in &bad_files {
                let _ = writeln!(err, "-- could not read {file}: {why}");
            }
            (u8::from(!bad_files.is_empty()), true)
        }
        Err(e) => {
            exec_error(err, &e);
            (1, false)
        }
        Ok(report) => {
            footer(err, &p, &report, &flags.opts);
            let code = if flags.opts.strict && report.truncated.is_some() {
                3
            } else if !report.bad_files.is_empty() {
                1
            } else {
                0
            };
            (code, false)
        }
    }
}

/// Runs every command in `src`. Stops at the first command that fails and returns its exit code.
pub fn run_text(
    src: &str,
    session: &mut Session,
    flags: &Flags,
    tty: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    run_commands(src, session, flags, tty, out, err, &mut false)
}

/// Like `run_text`; sets `closed_out` when the reader of `out` went away, which ends the whole run.
fn run_commands(
    src: &str,
    session: &mut Session,
    flags: &Flags,
    tty: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
    closed_out: &mut bool,
) -> u8 {
    if src.len() > MAX_COMMAND_BYTES {
        let _ = writeln!(
            err,
            "error[E-SYNTAX]: the input is longer than {MAX_COMMAND_BYTES} bytes"
        );
        return 2;
    }
    let script = match parse(src) {
        Ok(s) => s,
        Err(diags) => {
            for d in diags {
                let _ = write!(err, "{}", d.render(src));
            }
            return 2;
        }
    };
    for cmd in &script.commands {
        match &cmd.node {
            Command::UseClock(_) | Command::UsePath(_) => {
                session.apply(&cmd.node);
            }
            Command::Help(topic) => match topic.as_deref() {
                None => {
                    let _ = write!(out, "{HELP}");
                }
                Some(name) => match crate::help::topic(name) {
                    Some(text) => {
                        let _ = write!(out, "{text}");
                    }
                    None => {
                        let _ = writeln!(
                            err,
                            "error[E-SYNTAX]: no help for `{name}`; topics: {}",
                            crate::help::TOPICS.join(", ")
                        );
                        return 2;
                    }
                },
            },
            Command::Quit => return 0,
            Command::Show(_) | Command::Explain(_) => {
                let (code, closed) = run_show(cmd, src, session, flags, tty, out, err);
                if code != 0 || closed {
                    *closed_out = closed;
                    return code;
                }
            }
        }
    }
    0
}

fn repl(
    input: &mut dyn BufRead,
    tty_in: bool,
    tty_out: bool,
    flags: &Flags,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let mut session = Session::default();
    loop {
        if tty_in {
            let _ = write!(out, "gqodb> ");
            let _ = out.flush();
        }
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        if text.eq_ignore_ascii_case("quit") || text.eq_ignore_ascii_case("exit") {
            break;
        }
        let mut closed = false;
        let code = run_commands(&line, &mut session, flags, tty_out, out, err, &mut closed);
        if closed {
            return code;
        }
        if code != 0 && !tty_in {
            return code;
        }
    }
    0
}

pub fn main_with(args: Vec<String>, stdin_is_tty: bool) -> u8 {
    use std::io::IsTerminal;
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let (mut out, mut err) = (stdout.lock(), stderr.lock());
    let tty_out = std::io::stdout().is_terminal();
    let (flags, words) = match split_flags(&args) {
        Ok(v) => v,
        Err(m) => {
            let _ = writeln!(err, "error[E-SYNTAX]: {m}");
            return 2;
        }
    };
    let mut session = Session::default();
    match words.first().map(String::as_str) {
        None => {
            let stdin = std::io::stdin();
            repl(
                &mut stdin.lock(),
                stdin_is_tty,
                tty_out,
                &flags,
                &mut out,
                &mut err,
            )
        }
        Some("run") if words.len() == 2 => match std::fs::read_to_string(&words[1]) {
            Ok(text) => run_text(&text, &mut session, &flags, tty_out, &mut out, &mut err),
            Err(e) => {
                let _ = writeln!(err, "error[E-FILE]: cannot read `{}`: {e}", words[1]);
                1
            }
        },
        Some("-h") if words.len() == 1 => {
            let _ = write!(out, "{HELP}");
            0
        }
        Some(_) => run_text(
            &join_args(&words),
            &mut session,
            &flags,
            tty_out,
            &mut out,
            &mut err,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|i| (*i).to_string()).collect()
    }

    #[test]
    fn flags_are_split_from_the_command() {
        let (f, w) = split_flags(&s(&[
            "--format",
            "json",
            "show",
            "trades",
            "A",
            "--max-rows=5",
            "--strict",
        ]))
        .unwrap();
        assert_eq!(f.format, Some(Format::Json));
        assert_eq!(f.opts.max_rows, 5);
        assert!(f.opts.strict);
        assert_eq!(w, ["show", "trades", "A"]);
        assert!(split_flags(&s(&["--nope"])).is_err());
        assert!(split_flags(&s(&["--max-rows"])).is_err());
        assert!(split_flags(&s(&["--max-rows", "x"])).is_err());
        assert!(split_flags(&s(&["--format", "xml"])).is_err());
        assert!(split_flags(&s(&["--time", "utc"])).is_err());
    }

    #[test]
    fn words_with_spaces_are_quoted_again() {
        assert_eq!(
            join_args(&s(&["show", "trades", "A", "in", "my data"])),
            "show trades A in 'my data'"
        );
        assert_eq!(
            join_args(&s(&["use", "path", "it's here"])),
            "use path \"it's here\""
        );
        assert_eq!(
            join_args(&s(&["show", "trades", "A", "|", "take", "5"])),
            "show trades A | take 5"
        );
        assert_eq!(join_args(&s(&["columns", "a,b"])), "columns a,b");
        assert_eq!(join_args(&s(&["show", ""])), "show ''");
        assert_eq!(
            join_args(&s(&["show trades 'my data' | take 5"])),
            "show trades 'my data' | take 5",
            "one word is a whole command line"
        );
    }
}
