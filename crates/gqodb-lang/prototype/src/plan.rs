//! Resolves a parsed `show` or `explain` into a `Plan`: clocks are chosen, the range is typed, the path is known,
//! names are checked. Nothing is read from disk here; that is the executor's job.
use crate::ast::{Bound, CLOCK_CHOICES, ClockName, Command, Show, Step, Stream};
use crate::diag::{Code, Diagnostic};
use crate::span::{Span, Spanned};
use crate::time::{TimeError, parse_instant};

/// Settings that `use clock` and `use path` keep between commands (REPL and scripts).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Session {
    pub clock: Option<ClockName>,
    pub path: Option<String>,
}

impl Session {
    /// Applies `use clock` / `use path`. Returns true when the command was one of those.
    pub fn apply(&mut self, cmd: &Command) -> bool {
        match cmd {
            Command::UseClock(c) => {
                self.clock = Some(*c);
                true
            }
            Command::UsePath(p) => {
                self.path = Some(p.clone());
                true
            }
            _ => false,
        }
    }
}

/// Half-open range `[start, end)` on one clock. `None` is open. For `Sequence` the numbers are sequence numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedRange {
    pub clock: ClockName,
    pub start: Option<i64>,
    pub end: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order {
    pub clock: ClockName,
    pub desc: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub explain: bool,
    pub stream: Stream,
    pub symbol: String,
    pub venue: Option<String>,
    pub range: Option<ResolvedRange>,
    pub path: String,
    /// `None`: every column of the file, in schema order.
    pub columns: Option<Vec<String>>,
    pub order: Option<Order>,
    pub skip: u64,
    pub take: Option<u64>,
}

/// The columns every normalized file has, plus those of the stream.
pub fn known_columns(stream: Stream) -> &'static [&'static str] {
    match stream {
        Stream::Trades => &[
            "symbol",
            "source_utc_ns",
            "received_utc_ns",
            "clock_error_ns",
            "available_utc_ns",
            "sequence",
            "price",
            "quantity",
            "side",
        ],
        Stream::Quotes => &[
            "symbol",
            "source_utc_ns",
            "received_utc_ns",
            "clock_error_ns",
            "available_utc_ns",
            "sequence",
            "bid",
            "ask",
            "bid_quantity",
            "ask_quantity",
        ],
        Stream::Book => &[
            "symbol",
            "source_utc_ns",
            "received_utc_ns",
            "clock_error_ns",
            "available_utc_ns",
            "sequence",
            "price",
            "quantity",
            "level",
            "side",
            "action",
        ],
    }
}

/// The first and the last instant a time can have: the ends of a 64-bit count of nanoseconds since 1970.
const EARLIEST: &str = "1677-09-21T00:12:43.145224192Z";
const LATEST: &str = "2262-04-11T23:47:16.854775807Z";

fn clock_help() -> String {
    "add one of: on available | on received | on source | on sequence".to_string()
}

fn resolve_bound(bound: &Spanned<Bound>, clock: ClockName) -> Result<Option<i64>, Diagnostic> {
    let Bound::Value(text) = &bound.node else {
        return Ok(None);
    };
    if clock == ClockName::Sequence {
        if !text.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Diagnostic::new(
                Code::Range,
                format!("`{text}` is not a sequence number"),
                bound.span,
            )
            .with_help("a range on the sequence clock uses whole numbers, e.g. 100..200"));
        }
        return text.parse::<i64>().map(Some).map_err(|_| {
            Diagnostic::new(Code::Range, format!("`{text}` is too large"), bound.span)
        });
    }
    parse_instant(text).map(Some).map_err(|e| {
        let (msg, help) = match e {
            TimeError::Syntax => (format!("`{text}` is not a UTC time"), "use 2026-03-01 or 2026-03-01T12:30:00Z (fraction of a second up to 9 digits; always UTC)"),
            TimeError::Calendar => (format!("`{text}` is not a real date or time"), "check month, day, hour, minute and second"),
            TimeError::OutOfRange => (format!("`{text}` is outside the supported times {EARLIEST} to {LATEST}"), "times are whole nanoseconds in a 64-bit number; a date alone is the start of that day"),
        };
        Diagnostic::new(Code::Range, msg, bound.span).with_help(help)
    })
}

fn plan_show(
    show: &Show,
    session: &Session,
    cmd_span: Span,
    explain: bool,
) -> Result<Plan, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    let clock = show.clock.as_ref().map(|c| c.node).or(session.clock);

    let mut range = None;
    if let Some(r) = &show.range {
        match clock {
            None => diags.push(
                Diagnostic::new(Code::Clock, "a range needs a clock", r.span)
                    .with_help(clock_help()),
            ),
            Some(clock) => {
                let start = resolve_bound(&r.node.start, clock);
                let end = resolve_bound(&r.node.end, clock);
                match (start, end) {
                    (Ok(start), Ok(end)) => {
                        if let (Some(a), Some(b)) = (start, end) {
                            if a >= b {
                                diags.push(Diagnostic::new(
                                    Code::Range,
                                    "the begin must be before the end (the end is not included)",
                                    r.span,
                                ));
                            }
                        }
                        range = Some(ResolvedRange { clock, start, end });
                    }
                    (s, e) => {
                        diags.extend(s.err());
                        diags.extend(e.err());
                    }
                }
            }
        }
    }

    let path = show
        .path
        .as_ref()
        .map(|p| p.node.clone())
        .or_else(|| session.path.clone());
    if path.is_none() {
        diags.push(
            Diagnostic::new(Code::Path, "no path to read from", cmd_span)
                .with_help("add `in <path>` or set one with `use path <path>`"),
        );
    }

    let (mut columns, mut order, mut skip, mut take) = (None, None, None, None);
    for step in &show.steps {
        let twice = |name: &str| {
            Diagnostic::new(Code::Syntax, format!("`{name}` is given twice"), step.span)
        };
        match &step.node {
            Step::Take(n) => {
                if take.replace(*n).is_some() {
                    diags.push(twice("take"));
                }
            }
            Step::Skip(n) => {
                if skip.replace(*n).is_some() {
                    diags.push(twice("skip"));
                }
            }
            Step::Columns(cols) => {
                if columns.is_some() {
                    diags.push(twice("columns"));
                    continue;
                }
                let mut names: Vec<String> = Vec::new();
                for c in cols {
                    if !known_columns(show.stream.node).contains(&c.node.as_str()) {
                        diags.push(
                            Diagnostic::new(
                                Code::Column,
                                format!("unknown column `{}`", c.node),
                                c.span,
                            )
                            .with_help(format!(
                                "columns of {}: {}",
                                show.stream.node.name(),
                                known_columns(show.stream.node).join(", ")
                            )),
                        );
                    } else if names.contains(&c.node) {
                        diags.push(Diagnostic::new(
                            Code::Column,
                            format!("column `{}` is given twice", c.node),
                            c.span,
                        ));
                    } else {
                        names.push(c.node.clone());
                    }
                }
                columns = Some(names);
            }
            Step::Order { clock: own, desc } => {
                if order.is_some() {
                    diags.push(twice("order"));
                    continue;
                }
                match own.as_ref().map(|c| c.node).or(clock) {
                    Some(c) => order = Some(Order { clock: c, desc: *desc }),
                    None => diags.push(Diagnostic::new(Code::Clock, "`order` needs a clock", step.span)
                        .with_help("write `order on <clock>`, give the range a clock, or `use clock <clock>`")),
                }
            }
        }
    }

    if !diags.is_empty() {
        return Err(diags);
    }
    Ok(Plan {
        explain,
        stream: show.stream.node,
        symbol: show.symbol.node.clone(),
        venue: show.venue.as_ref().map(|v| v.node.clone()),
        range,
        path: path.expect("checked above"),
        columns,
        order,
        skip: skip.unwrap_or(0),
        take,
    })
}

/// Plans a `show` or `explain`. Other commands do not read data: `Err` with an E-SYNTAX diagnostic.
pub fn plan(cmd: &Spanned<Command>, session: &Session) -> Result<Plan, Vec<Diagnostic>> {
    match &cmd.node {
        Command::Show(s) => plan_show(s, session, cmd.span, false),
        Command::Explain(s) => plan_show(s, session, cmd.span, true),
        _ => Err(vec![Diagnostic::new(
            Code::Syntax,
            "this command does not read data",
            cmd.span,
        )]),
    }
}

/// All clock names, for help texts.
pub fn clock_choices() -> &'static str {
    CLOCK_CHOICES
}
