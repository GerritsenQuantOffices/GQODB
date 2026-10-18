#![forbid(unsafe_code)]
use std::io::IsTerminal;
use std::process::ExitCode;

fn main() -> ExitCode {
    // `args_os`, because `args` panics on an argument that is not valid UTF-8; such a byte is shown as U+FFFD instead.
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let code = gqodb_cli::cli::main_with(args, std::io::stdin().is_terminal());
    ExitCode::from(code)
}
