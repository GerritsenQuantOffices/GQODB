//! Splits the source into tokens. No meaning is decided here: a run of ordinary characters is one `Atom`
//! (a word, a symbol, a number, a time or a range like `2026-03-01..2026-03-02`); the parser decides what it is.
use crate::diag::{Code, Diagnostic};
use crate::span::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tok {
    /// Any run of characters that is not whitespace and not one of `| , = ; # ' "`.
    Atom(String),
    /// Text between matching single or double quotes, without the quotes. No escapes; no newline inside.
    Quoted(String),
    Pipe,
    Comma,
    Eq,
    /// A newline or `;`. Several in a row are allowed; the parser ignores empty commands.
    Sep,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

fn is_special(c: char) -> bool {
    matches!(c, '|' | ',' | '=' | ';' | '#' | '\'' | '"') || c.is_whitespace()
}

pub fn lex(src: &str) -> Result<Vec<Token>, Diagnostic> {
    let mut out = Vec::new();
    let mut chars = src.char_indices().peekable();
    while let Some(&(i, c)) = chars.peek() {
        let single = |tok: Tok, len: usize| Token {
            tok,
            span: Span::new(i, i + len),
        };
        match c {
            '\n' | ';' => {
                chars.next();
                out.push(single(Tok::Sep, 1));
            }
            c if c.is_whitespace() => {
                chars.next();
            }
            '#' => {
                while let Some(&(_, d)) = chars.peek() {
                    if d == '\n' {
                        break;
                    }
                    chars.next();
                }
            }
            '|' => {
                chars.next();
                out.push(single(Tok::Pipe, 1));
            }
            ',' => {
                chars.next();
                out.push(single(Tok::Comma, 1));
            }
            '=' => {
                chars.next();
                out.push(single(Tok::Eq, 1));
            }
            '\'' | '"' => {
                chars.next();
                let mut text = String::new();
                let mut closed_at = None;
                for (j, d) in chars.by_ref() {
                    if d == c {
                        closed_at = Some(j);
                        break;
                    }
                    if d == '\n' {
                        return Err(Diagnostic::new(
                            Code::Syntax,
                            "quoted text must close on the same line",
                            Span::new(i, j),
                        ));
                    }
                    text.push(d);
                }
                match closed_at {
                    Some(j) => out.push(Token {
                        tok: Tok::Quoted(text),
                        span: Span::new(i, j + 1),
                    }),
                    None => {
                        return Err(Diagnostic::new(
                            Code::Syntax,
                            "quoted text is not closed",
                            Span::new(i, src.len()),
                        ));
                    }
                }
            }
            _ => {
                let mut end = i;
                let mut text = String::new();
                while let Some(&(j, d)) = chars.peek() {
                    if is_special(d) {
                        break;
                    }
                    text.push(d);
                    end = j + d.len_utf8();
                    chars.next();
                }
                out.push(Token {
                    tok: Tok::Atom(text),
                    span: Span::new(i, end),
                });
            }
        }
    }
    Ok(out)
}
