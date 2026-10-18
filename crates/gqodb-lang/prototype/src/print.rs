//! The canonical text of a parsed script: aliases replaced by canonical clock names, quotes only where needed,
//! one command per line. `parse(to_canonical(parse(x))) == parse(x)` up to source positions.
use crate::ast::{Bound, Command, Script, Show, Step};
use crate::parser::RESERVED;

const KEYWORDS: &[&str] = &[
    "on", "in", "take", "skip", "columns", "order", "desc", "show", "explain", "use", "help",
    "quit", "exit",
];

fn quote(text: &str) -> String {
    if text.contains('\'') {
        format!("\"{text}\"")
    } else {
        format!("'{text}'")
    }
}

fn bare_ok(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._:/-".contains(c))
        && !name.contains("..")
}

fn name(text: &str, venue: bool) -> String {
    let lower = text.to_ascii_lowercase();
    let reserved_word = KEYWORDS.contains(&lower.as_str()) || RESERVED.contains(&lower.as_str());
    if bare_ok(text) && !(venue && reserved_word) {
        text.to_string()
    } else {
        quote(text)
    }
}

fn path(text: &str) -> String {
    if text.is_empty()
        || text
            .chars()
            .any(|c| c.is_whitespace() || "|,=;#'\"".contains(c))
    {
        quote(text)
    } else {
        text.to_string()
    }
}

fn bound(b: &Bound) -> &str {
    match b {
        Bound::Open => "",
        Bound::Value(v) => v,
    }
}

fn show(s: &Show) -> String {
    let mut out = String::new();
    if s.clock.is_some() && s.range.is_none() {
        out.push_str(&format!(
            "stream={} symbol={}",
            s.stream.node.name(),
            name(&s.symbol.node, false)
        ));
        if let Some(v) = &s.venue {
            out.push_str(&format!(" venue={}", name(&v.node, false)));
        }
        if let Some(c) = &s.clock {
            out.push_str(&format!(" clock={}", c.node.name()));
        }
        if let Some(p) = &s.path {
            out.push_str(&format!(" path={}", path(&p.node)));
        }
    } else {
        out.push_str(&format!(
            "{} {}",
            s.stream.node.name(),
            name(&s.symbol.node, false)
        ));
        if let Some(v) = &s.venue {
            out.push_str(&format!(" {}", name(&v.node, true)));
        }
        if let Some(r) = &s.range {
            out.push_str(&format!(
                " {}..{}",
                bound(&r.node.start.node),
                bound(&r.node.end.node)
            ));
            if let Some(c) = &s.clock {
                out.push_str(&format!(" on {}", c.node.name()));
            }
        }
        if let Some(p) = &s.path {
            out.push_str(&format!(" in {}", path(&p.node)));
        }
    }
    for step in &s.steps {
        match &step.node {
            Step::Take(n) => out.push_str(&format!(" | take {n}")),
            Step::Skip(n) => out.push_str(&format!(" | skip {n}")),
            Step::Columns(cols) => {
                let list: Vec<&str> = cols.iter().map(|c| c.node.as_str()).collect();
                out.push_str(&format!(" | columns {}", list.join(", ")));
            }
            Step::Order { clock, desc } => {
                out.push_str(" | order");
                if let Some(c) = clock {
                    out.push_str(&format!(" on {}", c.node.name()));
                }
                if *desc {
                    out.push_str(" desc");
                }
            }
        }
    }
    out
}

pub fn to_canonical(script: &Script) -> String {
    let mut lines = Vec::new();
    for c in &script.commands {
        lines.push(match &c.node {
            Command::Show(s) => format!("show {}", show(s)),
            Command::Explain(s) => format!("explain show {}", show(s)),
            Command::UseClock(c) => format!("use clock {}", c.name()),
            Command::UsePath(p) => format!("use path {}", path(p)),
            Command::Help(None) => "help".to_string(),
            Command::Help(Some(t)) => format!("help {}", path(t)),
            Command::Quit => "quit".to_string(),
        });
    }
    lines.join("\n")
}
