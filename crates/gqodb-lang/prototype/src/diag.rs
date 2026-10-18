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
    /// A source line longer than 120 characters is cut around the caret (`...` marks the cut side), so that the
    /// text stays short whatever the input.
    pub fn render(&self, source: &str) -> String {
        let start = floor_boundary(source, self.span.start.min(source.len()));
        let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = source[start..]
            .find('\n')
            .map_or(source.len(), |i| start + i);
        let line = &source[line_start..line_end];
        let end = floor_boundary(source, self.span.end.clamp(start, line_end));
        let column = source[line_start..start].chars().count();
        let width = source[start..end].chars().count().max(1);
        let (shown, column, width) = cut_around(line, column, width);
        let mut out = format!(
            "error[{}]: {}\n  {}\n  {}{}\n",
            self.code.as_str(),
            self.message,
            shown,
            " ".repeat(column),
            "^".repeat(width)
        );
        if let Some(help) = &self.help {
            out.push_str(&format!("  help: {help}\n"));
        }
        out
    }
}

/// Most characters of a source line that are shown, and how many of them come before the caret.
const SHOWN_CHARS: usize = 120;
const CHARS_BEFORE_CARET: usize = 60;
/// Diagnostics printed by `render_all`; the rest is counted.
pub const MAX_DIAGNOSTICS_SHOWN: usize = 20;

/// The largest index not above `i` that is on a character boundary.
fn floor_boundary(s: &str, mut i: usize) -> usize {
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// The part of `line` that is shown, with the caret column and width inside it. Counted in characters.
fn cut_around(line: &str, column: usize, width: usize) -> (String, usize, usize) {
    let total = line.chars().count();
    if total <= SHOWN_CHARS {
        return (line.to_string(), column, width);
    }
    let to = (column.saturating_sub(CHARS_BEFORE_CARET) + SHOWN_CHARS).min(total);
    let from = to - SHOWN_CHARS;
    let byte_at = |n: usize| line.char_indices().nth(n).map_or(line.len(), |(i, _)| i);
    let lead = if from > 0 { "..." } else { "" };
    let tail = if to < total { "..." } else { "" };
    let shown = format!("{lead}{}{tail}", &line[byte_at(from)..byte_at(to)]);
    let width = width.min(to.saturating_sub(column)).max(1);
    (shown, column - from + lead.len(), width)
}

/// The text for a list of diagnostics: the first 20, then one line `... and N more` when there are others.
pub fn render_all(diags: &[Diagnostic], source: &str) -> String {
    let mut out: String = diags
        .iter()
        .take(MAX_DIAGNOSTICS_SHOWN)
        .map(|d| d.render(source))
        .collect();
    if diags.len() > MAX_DIAGNOSTICS_SHOWN {
        out.push_str(&format!(
            "... and {} more\n",
            diags.len() - MAX_DIAGNOSTICS_SHOWN
        ));
    }
    out
}
