# gqodb-cli

The `gqodb` command: runs the GQODB command language over native tick and order-book files
(0.x preview).

> **0.1.0 preview.** No stability promise: the command language, its error texts, the footer and the
> output formats may change in any 0.x release (see
> [API stability](https://github.com/GerritsenQuantOffices/GQODB/blob/main/docs/API_STABILITY.md)).
> Part of the [GQODB open layer](https://github.com/GerritsenQuantOffices/GQODB/blob/main/README.md).

```sh
gqodb "show trades EURUSD 2026-03-01..2026-03-02 on received in ./data | take 20"
gqodb explain show trades EURUSD in ./data      # what would be read, without reading it
gqodb run queries.gq                            # a file of commands, stopped at the first failure
gqodb                                           # commands from the terminal or from a pipe
```

The language, the clocks, the budgets, the output formats, the error codes and the exit codes are
described in
[docs/LANGUAGE.md](https://github.com/GerritsenQuantOffices/GQODB/blob/main/docs/LANGUAGE.md), together
with the flaws that are known today.

What the command does and does not do:

- It reads `.gqodb.tick` and `.gqodb.ob` files, one file or the direct children of a directory, through
  the bounded read route of the store. It writes no data file; the only file it creates is the result
  of `--out`, which is never overwritten.
- Every range names its clock. Rows that are empty on that clock are left out and counted.
- Results are cut at 1,000 output rows, 1,048,576 scanned rows and 4,096 blocks unless the flags
  `--max-rows`, `--max-scan-rows` and `--max-blocks` say otherwise, up to hard maxima. A cut is reported
  on standard error; with `--strict` it is exit code 3. Exit code 1 means a file could not be read
  (or `--skip-bad` skipped one) and 2 means the command is wrong.
- Prices and quantities are written exactly from their integer and scale, never through a float.
- The engine words (`as of`, provenance, quality, uncertainty, join and others) are refused with
  `E-ENGINE`; this crate does not run them.

The library part (`gqodb_cli`) holds the executor, the `Executor` trait, the output sinks and the
front door; the language itself is in `gqodb-lang`. `install.sh` installs the binary with
`gqodb-codec` and `ob_store`.

```sh
cargo test --locked -p gqodb-cli
```
