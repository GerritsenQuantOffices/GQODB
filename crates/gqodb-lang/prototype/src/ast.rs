//! The syntax tree. Aliases are resolved while parsing: `local` and `wall` are `Received`, `broker` is `Source`.
use crate::span::Spanned;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Trades,
    Quotes,
    Book,
}

impl Stream {
    pub fn name(self) -> &'static str {
        match self {
            Stream::Trades => "trades",
            Stream::Quotes => "quotes",
            Stream::Book => "book",
        }
    }

    pub fn parse(word: &str) -> Option<Stream> {
        match word.to_ascii_lowercase().as_str() {
            "trades" => Some(Stream::Trades),
            "quotes" => Some(Stream::Quotes),
            "book" => Some(Stream::Book),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockName {
    Available,
    Received,
    Source,
    Sequence,
}

/// Shown in help texts and error hints.
pub const CLOCK_CHOICES: &str = "available | received | source | sequence";

impl ClockName {
    /// The canonical name (never an alias).
    pub fn name(self) -> &'static str {
        match self {
            ClockName::Available => "available",
            ClockName::Received => "received",
            ClockName::Source => "source",
            ClockName::Sequence => "sequence",
        }
    }

    pub fn parse(word: &str) -> Option<ClockName> {
        match word.to_ascii_lowercase().as_str() {
            "available" => Some(ClockName::Available),
            "received" | "local" | "wall" => Some(ClockName::Received),
            "source" | "broker" => Some(ClockName::Source),
            "sequence" => Some(ClockName::Sequence),
            _ => None,
        }
    }

    /// The column of the normalized file schema that this clock reads.
    pub fn column(self) -> &'static str {
        match self {
            ClockName::Available => "available_utc_ns",
            ClockName::Received => "received_utc_ns",
            ClockName::Source => "source_utc_ns",
            ClockName::Sequence => "sequence",
        }
    }
}

/// One side of `begin..end`: empty (open) or the text as typed (typed later, once the clock is known).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bound {
    Open,
    Value(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RangeExpr {
    pub start: Spanned<Bound>,
    pub end: Spanned<Bound>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Take(u64),
    Skip(u64),
    Columns(Vec<Spanned<String>>),
    Order {
        clock: Option<Spanned<ClockName>>,
        desc: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Show {
    pub stream: Spanned<Stream>,
    pub symbol: Spanned<String>,
    pub venue: Option<Spanned<String>>,
    pub range: Option<Spanned<RangeExpr>>,
    pub clock: Option<Spanned<ClockName>>,
    pub path: Option<Spanned<String>>,
    pub steps: Vec<Spanned<Step>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Show(Show),
    Explain(Show),
    UseClock(ClockName),
    UsePath(String),
    Help(Option<String>),
    Quit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Script {
    pub commands: Vec<Spanned<Command>>,
}
