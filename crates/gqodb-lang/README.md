# gqodb-lang

The GQODB command language (0.x preview): lexer, parser, diagnostics and planner. No I/O and no
dependencies.

> **0.x preview, not yet released.** No stability promise: the grammar, the error codes and texts and this API may
> change in any 0.x release (see [API stability](https://github.com/GerritsenQuantOffices/GQODB/blob/main/docs/API_STABILITY.md)).
> Part of the [GQODB open layer](https://github.com/GerritsenQuantOffices/GQODB/blob/main/README.md).

One command is one line, the verb comes first and steps are joined with `|`:

```text
show trades EURUSD bybit 2026-03-01..2026-03-02 on received in ./data | take 100
```

The language is described in
[docs/LANGUAGE.md](https://github.com/GerritsenQuantOffices/GQODB/blob/main/docs/LANGUAGE.md): the
grammar, the clocks (a range always names one; there is no default), the budgets, the ten error codes
and the words reserved for the GQODB engine.

This crate only parses and plans. It reads no file and runs nothing; the `gqodb-cli` crate reads the
files. Use it to turn text into a command and a plan, or into an error with a place in the source:

```rust
use gqodb_lang::{parse, plan, Session};

let src = "show trades EURUSD 2026-03-01..2026-03-02 in ./data";
let script = parse(src).expect("a valid command");
// the range has no clock, so planning fails with E-CLOCK
let diagnostics = plan(&script.commands[0], &Session::default()).unwrap_err();
print!("{}", diagnostics[0].render(src));
// error[E-CLOCK]: a range needs a clock
//   show trades EURUSD 2026-03-01..2026-03-02 in ./data
//                      ^^^^^^^^^^^^^^^^^^^^^^
//   help: add one of: on available | on received | on source | on sequence
```

- `parse(&str)` returns a `Script`, or the first error of each command with its place; a lexical error (an unclosed quote) stops the parse and is the only one returned.
- `plan(&command, &Session)` resolves clocks, the range (UTC nanoseconds, or sequence numbers) and the
  path into a `Plan`. `Session` holds what `use clock` and `use path` set.
- `to_canonical` prints a parsed script back to canonical text.
- `time::parse_instant` and `time::format_instant` convert between UTC text and whole nanoseconds.

`#![forbid(unsafe_code)]`. A libFuzzer target (`lang_parse`) covers `parse`, `to_canonical` and `plan`;
see [docs/FUZZING.md](https://github.com/GerritsenQuantOffices/GQODB/blob/main/docs/FUZZING.md) for
what ran.

```sh
cargo test --locked -p gqodb-lang
```
