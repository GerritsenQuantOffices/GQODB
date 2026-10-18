//! Recursive-descent parser. One command per line (or per `;`); the first error of a command is reported,
//! the next command is still parsed, so a script shows all its broken lines at once.
use crate::ast::{Bound, CLOCK_CHOICES, ClockName, Command, RangeExpr, Script, Show, Step, Stream};
use crate::diag::{Code, Diagnostic};
use crate::lexer::{Tok, Token, lex};
use crate::span::{Span, Spanned};

/// Constructs of the paid engine. The free executor recognises them and says so (E-ENGINE) instead of failing on syntax.
pub const RESERVED: &[&str] = &[
    "as",
    "provenance",
    "quality",
    "uncertainty",
    "join",
    "asof",
    "import",
    "capture",
    "ledger",
];
/// Words that can never be an unquoted venue.
const KEYWORDS: &[&str] = &[
    "on", "in", "take", "skip", "columns", "order", "desc", "show", "explain", "use", "help",
    "quit", "exit",
];

pub fn parse(src: &str) -> Result<Script, Vec<Diagnostic>> {
    let tokens = lex(src).map_err(|d| vec![d])?;
    let (mut commands, mut diags) = (Vec::new(), Vec::new());
    for chunk in tokens.split(|t| t.tok == Tok::Sep) {
        if chunk.is_empty() {
            continue;
        }
        match parse_command(chunk) {
            Ok(c) => commands.push(c),
            Err(d) => diags.push(d),
        }
    }
    if diags.is_empty() {
        Ok(Script { commands })
    } else {
        Err(diags)
    }
}

fn syntax(msg: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(Code::Syntax, msg, span)
}

fn engine(word: &str, span: Span) -> Diagnostic {
    Diagnostic::new(
        Code::Engine,
        format!("`{word}` needs the GQODB engine"),
        span,
    )
    .with_help("this construct belongs to the paid engine; the free executor does not run it")
}

struct Cur<'a> {
    t: &'a [Token],
    i: usize,
}

impl<'a> Cur<'a> {
    fn peek(&self) -> Option<&'a Token> {
        self.t.get(self.i)
    }
    fn peek_n(&self, n: usize) -> Option<&'a Token> {
        self.t.get(self.i + n)
    }
    fn next(&mut self) -> Option<&'a Token> {
        let t = self.t.get(self.i);
        if t.is_some() {
            self.i += 1;
        }
        t
    }
    /// A zero-width span just after the last token: where something is missing.
    fn end_span(&self) -> Span {
        let end = self.t.last().map_or(0, |t| t.span.end);
        Span::new(end, end)
    }
    fn at_atom(&self, word: &str) -> bool {
        matches!(self.peek().map(|t| &t.tok), Some(Tok::Atom(a)) if a.eq_ignore_ascii_case(word))
    }
}

fn parse_command(t: &[Token]) -> Result<Spanned<Command>, Diagnostic> {
    let first = &t[0];
    let all = Span::new(first.span.start, t[t.len() - 1].span.end);
    let Tok::Atom(verb) = &first.tok else {
        return Err(syntax(
            "a command starts with a word such as `show`",
            first.span,
        ));
    };
    let lower = verb.to_ascii_lowercase();
    let mut cur = Cur { t, i: 1 };
    let node = match lower.as_str() {
        "show" => Command::Show(parse_show(&mut cur, first.span)?),
        "explain" => {
            if cur.at_atom("show") {
                cur.i += 1;
                Command::Explain(parse_show(&mut cur, first.span)?)
            } else {
                return Err(syntax(
                    "`explain` is followed by a `show` command",
                    cur.end_span(),
                ));
            }
        }
        "use" => parse_use(&mut cur)?,
        "help" => {
            let topic = match cur.next() {
                None => None,
                Some(tok) => Some(text_of(tok)?),
            };
            Command::Help(topic)
        }
        "quit" | "exit" => Command::Quit,
        w if RESERVED.contains(&w) => return Err(engine(w, first.span)),
        _ => {
            return Err(syntax(format!("unknown command `{verb}`"), first.span)
                .with_help("commands: show, explain, use, help, quit"));
        }
    };
    if let Some(tok) = cur.peek() {
        return Err(unexpected(tok));
    }
    Ok(Spanned::new(node, all))
}

fn unexpected(tok: &Token) -> Diagnostic {
    match &tok.tok {
        Tok::Atom(a) if RESERVED.contains(&a.to_ascii_lowercase().as_str()) => {
            engine(&a.to_ascii_lowercase(), tok.span)
        }
        Tok::Atom(a) | Tok::Quoted(a) => syntax(format!("unexpected `{a}`"), tok.span),
        Tok::Pipe => syntax("unexpected `|`", tok.span),
        Tok::Comma => syntax("unexpected `,`", tok.span),
        Tok::Eq => syntax("unexpected `=`", tok.span),
        Tok::Sep => syntax("unexpected end of command", tok.span),
    }
}

fn text_of(tok: &Token) -> Result<String, Diagnostic> {
    match &tok.tok {
        Tok::Atom(a) | Tok::Quoted(a) => Ok(a.clone()),
        _ => Err(unexpected(tok)),
    }
}

fn parse_use(cur: &mut Cur) -> Result<Command, Diagnostic> {
    let Some(what) = cur.next() else {
        return Err(syntax(
            "`use` is followed by `clock` or `path`",
            cur.end_span(),
        ));
    };
    let Tok::Atom(w) = &what.tok else {
        return Err(unexpected(what));
    };
    match w.to_ascii_lowercase().as_str() {
        "clock" => {
            let tok = cur
                .next()
                .ok_or_else(|| syntax("a clock name is missing", cur.end_span()))?;
            Ok(Command::UseClock(clock_of(tok)?))
        }
        "path" => {
            let tok = cur
                .next()
                .ok_or_else(|| syntax("a path is missing", cur.end_span()))?;
            Ok(Command::UsePath(text_of(tok)?))
        }
        other => Err(syntax(format!("cannot `use {other}`"), what.span)
            .with_help("use clock <clock> | use path <path>")),
    }
}

fn clock_of(tok: &Token) -> Result<ClockName, Diagnostic> {
    let Tok::Atom(a) = &tok.tok else {
        return Err(unexpected(tok));
    };
    ClockName::parse(a).ok_or_else(|| {
        Diagnostic::new(Code::Clock, format!("unknown clock `{a}`"), tok.span).with_help(format!(
            "clocks: {CLOCK_CHOICES} (aliases: local, wall = received; broker = source)"
        ))
    })
}

/// A bare name (symbol or venue): starts with a letter, then letters, digits and `. _ : / -`, never `..`.
fn check_bare(name: &str, span: Span) -> Result<(), Diagnostic> {
    let mut chars = name.chars();
    let first_ok = chars.next().is_some_and(|c| c.is_ascii_alphabetic());
    let rest_ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "._:/-".contains(c));
    if !first_ok || !rest_ok || name.contains("..") {
        return Err(syntax(format!("`{name}` cannot be written without quotes"), span)
            .with_help("an unquoted name starts with a letter and holds letters, digits and . _ : / - ; otherwise put it in quotes, e.g. '1INCHUSDT'"));
    }
    Ok(())
}

fn name_of(tok: &Token) -> Result<Spanned<String>, Diagnostic> {
    match &tok.tok {
        Tok::Atom(a) => {
            check_bare(a, tok.span)?;
            Ok(Spanned::new(a.clone(), tok.span))
        }
        Tok::Quoted(q) if !q.is_empty() => Ok(Spanned::new(q.clone(), tok.span)),
        Tok::Quoted(_) => Err(syntax("a name cannot be empty", tok.span)),
        _ => Err(unexpected(tok)),
    }
}

fn stream_of(tok: &Token) -> Result<Spanned<Stream>, Diagnostic> {
    let Tok::Atom(a) = &tok.tok else {
        return Err(unexpected(tok));
    };
    Stream::parse(a)
        .map(|s| Spanned::new(s, tok.span))
        .ok_or_else(|| {
            Diagnostic::new(Code::Stream, format!("unknown stream `{a}`"), tok.span)
                .with_help("streams: trades, quotes, book")
        })
}

fn range_of(tok: &Token) -> Result<Spanned<RangeExpr>, Diagnostic> {
    let Tok::Atom(a) = &tok.tok else {
        return Err(unexpected(tok));
    };
    // A range starts with a digit or a dot: the canonical text writes it positionally, where only
    // such an atom is read as a range.
    let Some((begin, end)) = a
        .starts_with(|c: char| c.is_ascii_digit() || c == '.')
        .then(|| a.split_once(".."))
        .flatten()
    else {
        return Err(Diagnostic::new(Code::Range, format!("`{a}` is not a range"), tok.span)
            .with_help("a range is <begin>..<end>, e.g. 2026-03-01..2026-03-02; leave a side empty for open"));
    };
    if end.contains("..") {
        return Err(Diagnostic::new(
            Code::Range,
            "a range has one `..`",
            tok.span,
        ));
    }
    let bound = |text: &str, start: usize| {
        let node = if text.is_empty() {
            Bound::Open
        } else {
            Bound::Value(text.to_string())
        };
        Spanned::new(node, Span::new(start, start + text.len()))
    };
    let s = tok.span.start;
    let range = RangeExpr {
        start: bound(begin, s),
        end: bound(end, s + begin.len() + 2),
    };
    Ok(Spanned::new(range, tok.span))
}

fn parse_show(cur: &mut Cur, verb: Span) -> Result<Show, Diagnostic> {
    let named = matches!(cur.peek().map(|t| &t.tok), Some(Tok::Atom(_)))
        && matches!(cur.peek_n(1).map(|t| &t.tok), Some(Tok::Eq));
    if named {
        return parse_show_named(cur, verb);
    }
    let stream = stream_of(cur.next().ok_or_else(|| {
        syntax(
            "a stream is missing: trades, quotes or book",
            cur.end_span(),
        )
    })?)?;
    let symbol = name_of(
        cur.next()
            .ok_or_else(|| syntax("a symbol is missing", cur.end_span()))?,
    )?;
    let venue = match cur.peek() {
        Some(tok) => match &tok.tok {
            Tok::Quoted(_) => {
                cur.i += 1;
                Some(name_of(tok)?)
            }
            Tok::Atom(a) if a.starts_with(|c: char| c.is_ascii_alphabetic()) => {
                let lower = a.to_ascii_lowercase();
                if RESERVED.contains(&lower.as_str()) {
                    return Err(engine(&lower, tok.span));
                }
                if KEYWORDS.contains(&lower.as_str()) {
                    None
                } else {
                    cur.i += 1;
                    Some(name_of(tok)?)
                }
            }
            _ => None,
        },
        None => None,
    };
    let mut range = None;
    let mut clock = None;
    if let Some(tok) = cur.peek() {
        if matches!(&tok.tok, Tok::Atom(a) if a.starts_with(|c: char| c.is_ascii_digit() || c == '.'))
        {
            cur.i += 1;
            range = Some(range_of(tok)?);
            if cur.at_atom("on") {
                cur.i += 1;
                let c = cur
                    .next()
                    .ok_or_else(|| syntax("a clock name is missing after `on`", cur.end_span()))?;
                clock = Some(Spanned::new(clock_of(c)?, c.span));
            }
        } else if cur.at_atom("on") {
            return Err(syntax("`on <clock>` follows a range", tok.span)
                .with_help("e.g. show trades EURUSD 2026-03-01..2026-03-02 on received"));
        }
    }
    let mut path = None;
    if cur.at_atom("in") {
        cur.i += 1;
        let p = cur
            .next()
            .ok_or_else(|| syntax("a path is missing after `in`", cur.end_span()))?;
        path = Some(Spanned::new(text_of(p)?, p.span));
    }
    let steps = parse_steps(cur)?;
    Ok(Show {
        stream,
        symbol,
        venue,
        range,
        clock,
        path,
        steps,
    })
}

fn parse_show_named(cur: &mut Cur, verb: Span) -> Result<Show, Diagnostic> {
    let (mut stream, mut symbol, mut venue, mut range, mut clock, mut path) =
        (None, None, None, None, None, None);
    while let (Some(k), Some(eq)) = (cur.peek(), cur.peek_n(1)) {
        let (Tok::Atom(key), Tok::Eq) = (&k.tok, &eq.tok) else {
            break;
        };
        let key_l = key.to_ascii_lowercase();
        cur.i += 2;
        let v = cur
            .next()
            .ok_or_else(|| syntax(format!("a value is missing after `{key}=`"), cur.end_span()))?;
        let dup = |present: bool| {
            if present {
                Err(syntax(format!("`{key}` is given twice"), k.span))
            } else {
                Ok(())
            }
        };
        match key_l.as_str() {
            "stream" => {
                dup(stream.is_some())?;
                stream = Some(stream_of(v)?);
            }
            "symbol" => {
                dup(symbol.is_some())?;
                symbol = Some(name_of(v)?);
            }
            "venue" => {
                dup(venue.is_some())?;
                venue = Some(name_of(v)?);
            }
            "range" => {
                dup(range.is_some())?;
                range = Some(range_of(v)?);
            }
            "clock" => {
                dup(clock.is_some())?;
                clock = Some(Spanned::new(clock_of(v)?, v.span));
            }
            "path" => {
                dup(path.is_some())?;
                path = Some(Spanned::new(text_of(v)?, v.span));
            }
            other => {
                return Err(syntax(format!("unknown name `{other}`"), k.span)
                    .with_help("names: stream, symbol, venue, range, clock, path"));
            }
        }
    }
    let stream = stream.ok_or_else(|| syntax("`stream=` is missing", verb))?;
    let symbol = symbol.ok_or_else(|| syntax("`symbol=` is missing", verb))?;
    let steps = parse_steps(cur)?;
    Ok(Show {
        stream,
        symbol,
        venue,
        range,
        clock,
        path,
        steps,
    })
}

fn number_of(tok: Option<&Token>, at: Span) -> Result<u64, Diagnostic> {
    let Some(tok) = tok else {
        return Err(syntax(
            "a whole number is missing",
            Span::new(at.end, at.end),
        ));
    };
    match &tok.tok {
        Tok::Atom(a) if !a.is_empty() && a.bytes().all(|b| b.is_ascii_digit()) => a
            .parse::<u64>()
            .map_err(|_| syntax(format!("`{a}` is too large"), tok.span)),
        _ => Err(syntax("a whole number is expected", tok.span)),
    }
}

fn parse_steps(cur: &mut Cur) -> Result<Vec<Spanned<Step>>, Diagnostic> {
    let mut steps = Vec::new();
    while let Some(pipe) = cur.peek() {
        if pipe.tok != Tok::Pipe {
            break;
        }
        cur.i += 1;
        let word = cur
            .next()
            .ok_or_else(|| syntax("a step is missing after `|`", pipe.span))?;
        let Tok::Atom(w) = &word.tok else {
            return Err(syntax(
                "a step starts with a word such as `take`",
                word.span,
            ));
        };
        let mut end = word.span.end;
        let step = match w.to_ascii_lowercase().as_str() {
            "take" | "skip" => {
                let tok = cur.next();
                let n = number_of(tok, word.span)?;
                end = tok.map_or(end, |t| t.span.end);
                if w.eq_ignore_ascii_case("take") {
                    Step::Take(n)
                } else {
                    Step::Skip(n)
                }
            }
            "columns" => {
                let mut cols = Vec::new();
                loop {
                    let tok = cur
                        .next()
                        .ok_or_else(|| syntax("a column name is missing", cur.end_span()))?;
                    let Tok::Atom(name) = &tok.tok else {
                        return Err(syntax("a column name is expected", tok.span));
                    };
                    cols.push(Spanned::new(name.clone(), tok.span));
                    end = tok.span.end;
                    if matches!(cur.peek().map(|t| &t.tok), Some(Tok::Comma)) {
                        cur.i += 1;
                    } else {
                        break;
                    }
                }
                Step::Columns(cols)
            }
            "order" => {
                let mut clock = None;
                let mut desc = false;
                if cur.at_atom("on") {
                    cur.i += 1;
                    let c = cur.next().ok_or_else(|| {
                        syntax("a clock name is missing after `on`", cur.end_span())
                    })?;
                    clock = Some(Spanned::new(clock_of(c)?, c.span));
                    end = c.span.end;
                }
                if cur.at_atom("desc") {
                    let d = cur.next().expect("peeked");
                    desc = true;
                    end = d.span.end;
                }
                Step::Order { clock, desc }
            }
            r if RESERVED.contains(&r) => return Err(engine(r, word.span)),
            _ => {
                return Err(syntax(format!("unknown step `{w}`"), word.span)
                    .with_help("steps: take, skip, columns, order"));
            }
        };
        steps.push(Spanned::new(step, Span::new(word.span.start, end)));
    }
    Ok(steps)
}
