//! Diagnostics: a code, a message, the place in the source and an optional hint.
use crate::span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code {
    Syntax,
    Clock,
    Range,
    Stream,
    Column,
    Path,
    Venue,
    Engine,
    Budget,
    File,
}

impl Code {
    pub fn as_str(self) -> &'static str {
        match self {
            Code::Syntax => "E-SYNTAX",
            Code::Clock => "E-CLOCK",
            Code::Range => "E-RANGE",
            Code::Stream => "E-STREAM",
            Code::Column => "E-COLUMN",
            Code::Path => "E-PATH",
            Code::Venue => "E-VENUE",
            Code::Engine => "E-ENGINE",
            Code::Budget => "E-BUDGET",
            Code::File => "E-FILE",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: Code,
    pub message: String,
    pub span: Span,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn new(code: Code, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            span,
            help: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Multi-line text for a terminal:
    /// ```text
    /// error[E-CLOCK]: a range needs a clock
    ///   show trades EURUSD 2026-03-01..2026-03-02 in ./data
    ///                      ^^^^^^^^^^^^^^^^^^^^^^^
    ///   help: add one of: on available | on received | on source | on sequence
    /// ```
    pub fn render(&self, source: &str) -> String {
        let start = self.span.start.min(source.len());
        let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = source[start..]
            .find('\n')
            .map_or(source.len(), |i| start + i);
        let line = &source[line_start..line_end];
        let end = self.span.end.clamp(start, line_end);
        let column = source[line_start..start].chars().count();
        let width = source[start..end].chars().count().max(1);
        let mut out = format!(
            "error[{}]: {}\n  {}\n  {}{}\n",
            self.code.as_str(),
            self.message,
            line,
            " ".repeat(column),
            "^".repeat(width)
        );
        if let Some(help) = &self.help {
            out.push_str(&format!("  help: {help}\n"));
        }
        out
    }
}
