//! The `gqodb` command line: runs the GQODB command language (`gqodb-lang`) over native tick and order-book files.
//! `exec` reads files, `sink` writes results, `cli` is the front door (arguments, scripts, the REPL).
#![forbid(unsafe_code)]

pub mod cli;
pub mod exec;
pub mod help;
pub mod select;
pub mod sink;
pub mod value;
