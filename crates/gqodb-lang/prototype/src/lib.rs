//! The GQODB command language: lexer, parser, diagnostics and the planner. No I/O and no dependencies.
//! One command per line, verb first:
//! `show trades EURUSD bybit 2026-03-01..2026-03-02 on received in ./data | take 100`.
#![forbid(unsafe_code)]

pub mod ast;
pub mod diag;
pub mod lexer;
pub mod parser;
pub mod plan;
pub mod print;
pub mod span;
pub mod time;

pub use ast::{ClockName, Command, Script, Show, Step, Stream};
pub use diag::{Code, Diagnostic, render_all};
pub use parser::parse;
pub use plan::{Plan, Session, plan};
pub use print::to_canonical;
pub use span::{Span, Spanned};
