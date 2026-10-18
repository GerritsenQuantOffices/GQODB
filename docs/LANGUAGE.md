# The GQODB command language

**0.x preview, no stability promise.** The grammar, the error codes and texts, the exit codes
and the output of the `gqodb` command may change in any 0.x release (see
[API stability](API_STABILITY.md)). This page describes what the code in this repository does.
Where the code has a known flaw, section 13 says so.

The language asks `.gqodb.tick` and `.gqodb.ob` files what they hold. One command is one line, the
verb comes first, and steps are joined with `|`:

```text
show trades SYNTH 1..3 on sequence in ./demo | columns sequence, price, quantity
```

It is implemented by two crates: `gqodb-lang` (lexer, parser, diagnostics and planner; no I/O) and
`gqodb-cli` (the `gqodb` binary, which reads the files and writes the result).

## 1. Overview and a first example

The examples on this page use two small files written by the `demo` helper of `gqodb-codec`. They hold
three synthetic rows of the symbol `SYNTH` each, stamped a few nanoseconds after 1970-01-01, and carry
no venue label. The second row has no source time.

```sh
mkdir demo
gqodb-codec demo tick-segment demo/demo.gqodb.tick
gqodb-codec demo book-segment demo/demo.gqodb.ob
```

(If the tools are not installed, use `target/release/gqodb-codec` and `target/release/gqodb` after
`cargo build --workspace --bins --release --locked`.) Then:

```text
$ gqodb "show trades SYNTH 1..3 on sequence in ./demo | columns sequence, price, quantity"
sequence,price,quantity
1,1.00,2
2,1.00,2
-- clock sequence, range [1, 3)
-- files: 1 read, 1 skipped · blocks: 1 of 1 read · bytes: 2455 · rows: 3 scanned, 2 out
-- skipped demo.gqodb.ob: other stream
```

The rows go to standard output; the three lines that start with `--` are the footer and go to standard
error. The command selected the rows of `SYNTH` whose sequence number is in the half-open range `[1, 3)`,
read the directory `./demo`, skipped the order-book file because it holds another stream, and said
what it scanned. Lines that start with `$ ` are commands to type, written so that the test of this page can parse them;
command lines that carry flags, or that cannot parse, are shown without the `$ `. The examples on this page
show what a pipe or a file receives (CSV); on a terminal
the default format is an aligned table (section 9).

Three rules shape the whole language:

- **There is no default clock.** A range names the clock it is measured on (section 5).
- **Output is exact.** Prices and quantities are integers with a scale and are written as decimal text,
  never through a float (section 9).
- **Nothing is cut silently.** Every budget that stops a read is reported, and `--strict` turns it
  into an error (section 8).

## 2. Commands

| Command | Does |
| --- | --- |
| `show` | Reads rows and writes them. |
| `explain show ...` | Prints what a `show` would read, per file, without reading data blocks. |
| `use clock <clock>` / `use path <path>` | Sets a clock or a path for the following commands of the same run. |
| `help [<topic>]` | Prints the usage text, or one topic: `clocks`, `ranges`, `steps`, `flags`, `errors`. |
| `quit` / `exit` | Ends the script or command line: the commands after it are not run. (The interactive loop is a special case, see section 13.) |

**Running commands.** The `gqodb` binary takes a command in three ways:

```sh
gqodb "show trades SYNTH in ./demo | take 2"     # one argument: the whole command line
gqodb show trades SYNTH in ./demo                # separate words are joined into one command
gqodb run queries.gq                             # the commands in a file, one per line
gqodb                                            # commands from the terminal, or from a pipe
```

A single argument is one whole command line. Among several arguments, one that holds whitespace is
treated as quoted. `gqodb run <file>` and a command line may hold several commands separated by a
newline or `;`; `#` starts a comment to the end of the line. A script is parsed completely before
anything runs. The parse reports the first error of each command, so one script shows all its broken
lines at once, except that a lexical error (an unclosed quote) stops the parse and is the only one
reported; the exit code is 2 and nothing runs. After that the commands run in order and the run stops
at the first one that fails, with that command's exit code.
`use clock` and `use path` last until the end of the run; they are not kept between runs. A script
file, or one command line given to the binary, may not be longer than 1 MiB (1,048,576 bytes).

Without arguments, `gqodb` reads commands line by line. On a terminal, an error is shown and the
loop goes on. When the input is a pipe, the first failing line ends the run with its exit code.
There is no line editing; `rlwrap gqodb` adds history.

`help` prints a usage text; `help <topic>` prints one of the five topics, and any other topic is an
error (exit code 2) that lists them. `gqodb --help` and `gqodb -h` print the same usage text as
`help`.

Keywords are case-insensitive: verbs, stream names, clock names, step words and the names in the
keyword form (`stream=`, `symbol=`). Symbols, venues and paths are compared exactly as written.

## 3. Grammar

```text
script      = { command , ( NEWLINE | ";" ) } ;               (* "#" starts a comment to the end of the line *)
command     = show | explain | use | help | quit ;
show        = "show" , ( positional | named ) , { "|" , step } ;
positional  = stream , symbol , [ venue ] , [ range , [ "on" , clock ] ] , [ "in" , path ] ;
named       = pair , { pair } ;
pair        = ( "stream" | "symbol" | "venue" | "range" | "clock" | "path" ) , "=" , value ;
explain     = "explain" , show ;
use         = "use" , ( "clock" , clock | "path" , path ) ;
help        = "help" , [ topic ] ;
quit        = "quit" | "exit" ;
step        = "take" , number | "skip" , number | "columns" , name , { "," , name }
            | "order" , [ "on" , clock ] , [ "desc" ] ;
stream      = "trades" | "quotes" | "book" ;
clock       = "available" | "received" | "local" | "wall" | "source" | "broker" | "sequence" ;
range       = [ bound ] , ".." , [ bound ] ;                  (* UTC instants, or whole numbers for the sequence clock *)
symbol      = bare | quoted ;   venue = bare | quoted ;        (* bare: starts with a letter, then letters digits . _ : / - ; never ".." *)
```

Lexical rules, as the code applies them:

- **Words.** A run of characters that holds no whitespace and none of `| , = ; # ' "` is one word.
  A word that holds one of those characters must be quoted.
- **Quoted text.** `'...'` or `"..."`. There are no escape sequences, and the text must close on
  the same line. A quoted name cannot be empty.
- **Symbol and venue.** A bare name starts with an ASCII letter and holds ASCII letters, digits and `. _ : / -`,
  never `..`. Anything else (a leading digit, `+`, a space) must be quoted: `'1INCHUSDT'`,
  `"EUR/USD"`. A bare name that contains characters outside that set is an E-SYNTAX error that says so.
- **Venue position.** A bare word that follows the symbol and starts with a letter is the venue,
  unless it is one of `on in take skip columns order desc show explain use help quit exit`, which can
  only be a venue when quoted. A reserved word there gives E-ENGINE (section 11).
- **Range.** In the positional form, a word after the symbol (and venue) that starts with a digit or a dot
  is read as a range and must be of the form `<begin>..<end>`. In the keyword form (`range=...`) the value must start with a digit or a dot as well; anything else is
  E-RANGE. This rule exists because the canonical text of a command writes the range positionally, where
  only such a word is read as a range; a fuzz run found `range=` accepting values that could not be
  printed back.
- **Number.** A run of ASCII digits that fits an unsigned 64-bit integer (no sign).
- **Path.** A bare word (`./data`, `/mnt/x`) or quoted text. A path that contains a space, `|`, `,`,
  `=`, `;` or `#` must be quoted.
- **Keyword form.** `show stream=trades symbol=SYNTH ...`: the pairs may come in any order, each at most
  once; `stream=` and `symbol=` are required. The first word after `show` decides the form: a word followed
  by `=` starts the keyword form.

The keyword-form rule for ranges, as the executor shows it (this command does not parse):

```text
gqodb "show stream=trades symbol=SYNTH range=x..y clock=sequence path=./demo"
error[E-RANGE]: `x..y` is not a range
  show stream=trades symbol=SYNTH range=x..y clock=sequence path=./demo
                                        ^^^^
  help: a range is <begin>..<end>, e.g. 2026-03-01..2026-03-02; leave a side empty for open
```

More commands (shown without output, because `./data` is not part of this repository):

```text
$ gqodb "show trades EURUSD bybit 2026-03-01..2026-03-02 on received in ./data"
$ gqodb "show trades EURUSD 2026-03-01T09:00:00Z..2026-03-01T09:05:00Z on available in ./data | take 20"
$ gqodb "show quotes EURUSD 2026-03-01.. on source in ./data/eurusd.gqodb.tick | columns source_utc_ns, bid, ask"
$ gqodb "show book 'BTC-USDT' bybit 100..200 on sequence in ./data | order desc"
$ gqodb "show trades '1INCHUSDT' 2026-03-01..2026-03-02 on received in ./data | skip 10 | take 5"
$ gqodb "show stream=trades symbol=EURUSD venue=bybit range=2026-03-01..2026-03-02 clock=received path=./data"
$ gqodb "explain show trades EURUSD 2026-03-01..2026-03-02 on received in ./data"
```

A command can also use a session setting instead of repeating it:

```text
$ gqodb "use path ./demo; use clock sequence; show trades SYNTH 2.. | columns sequence"
sequence
2
3
-- clock sequence, range [2, )
-- files: 1 read, 1 skipped · blocks: 1 of 1 read · bytes: 2455 · rows: 3 scanned, 2 out
-- skipped demo.gqodb.ob: other stream
```

## 4. Streams and columns per stream

| Stream | Reads files with | Columns |
| --- | --- | --- |
| `trades` | `gqodb.family` = `tick` and `gqodb.subtype` = `trade` | `symbol`, `source_utc_ns`, `received_utc_ns`, `clock_error_ns`, `available_utc_ns`, `sequence`, `price`, `quantity`, `side` |
| `quotes` | `gqodb.family` = `tick` and `gqodb.subtype` = `quote` | `symbol`, `source_utc_ns`, `received_utc_ns`, `clock_error_ns`, `available_utc_ns`, `sequence`, `bid`, `ask`, `bid_quantity`, `ask_quantity` |
| `book` | `gqodb.family` = `orderbook` | `symbol`, `source_utc_ns`, `received_utc_ns`, `clock_error_ns`, `available_utc_ns`, `sequence`, `price`, `quantity`, `level`, `side`, `action` |

The stream of a file is decided by that schema metadata, not by its name. A file that has no such metadata is skipped as "not a normalized tick or book file"; a file of another
stream is skipped as "other stream". Both are counted and named in the footer.

**Paths.** `in <path>` (or `use path`) names a file or a directory. A file path is read whatever its
name is. A directory means its direct children whose names end in `.gqodb.tick` or `.gqodb.ob`, sorted
by name; it is not searched recursively. Other files and subdirectories are ignored, and they are not
named in the footer or in `explain` (only the files that qualify by name are counted). A command with
no path at all is E-PATH.

What the columns hold:

- `symbol` is text. The `symbol` of a command is compared with it exactly, case-sensitively, with no
  normalisation.
- `source_utc_ns`, `received_utc_ns` and `available_utc_ns` are UTC times in whole nanoseconds. The
  first two can be empty; `available_utc_ns` cannot.
- `clock_error_ns` is a whole number of nanoseconds and can be empty. `sequence` is the capture sequence
  number and cannot be empty.
- `price`, `bid`, `ask`, `quantity`, `bid_quantity` and `ask_quantity` are stored as integers with a scale
  (`gqodb.price_decimals` and `gqodb.quantity_decimals` in the file metadata). A file that lacks either,
  or holds a value that is not a number, is an E-FILE error (exit code 1).
- `side`, `level` and `action` are small whole numbers as stored. This repository does not interpret
  them; the meaning of `side` and `action` stays with the writer.

The table lists the columns of each stream; the order inside a file can differ (in the demo file,
`symbol` comes after `sequence`). Without a `columns` step, every column of the file is written, in the
order of the file's schema. With
`columns`, the named columns are written in the order given. A name that is not a column of the
stream is E-COLUMN, and the error lists the columns of that stream:

```text
$ gqodb "show trades SYNTH in ./demo | columns nope"
error[E-COLUMN]: unknown column `nope`
  show trades SYNTH in ./demo | columns nope
                                        ^^^^
  help: columns of trades: symbol, source_utc_ns, received_utc_ns, clock_error_ns, available_utc_ns, sequence, price, quantity, side
```

## 5. Clocks

There is no default clock. A range without `on <clock>` and without `use clock` is E-CLOCK.

| Clock | Column | Can be empty | Indexed in files from the codec facade | Meaning |
| --- | --- | --- | --- | --- |
| `available` | `available_utc_ns` | no | yes | when the record was available |
| `received` (also `local`, `wall`) | `received_utc_ns` | yes | no | the receive moment on the local clock, as UTC |
| `source` (also `broker`) | `source_utc_ns` | yes | no | the time the source (exchange or broker) gave |
| `sequence` | `sequence` | no | no | capture sequence number; not a time |

**Empty values.** When a command has a range, a row whose value on the chosen clock is empty is outside
every range on that clock, including an open range (`..`). Such rows are counted, and the footer says how many. No value
is guessed and none is taken from another clock:

```text
$ gqodb "show trades SYNTH 1970-01-01.. on source in ./demo | columns sequence, source_utc_ns"
sequence,source_utc_ns
1,1970-01-01T00:00:00.000000008Z
3,1970-01-01T00:00:00.000000010Z
-- clock source, range [1970-01-01T00:00:00.000000000Z, )
-- files: 1 read, 1 skipped · blocks: 1 of 1 read · bytes: 2455 · rows: 3 scanned, 2 out
-- skipped demo.gqodb.ob: other stream
-- excluded: 1 rows without a source value
```

**Indexed or not.** A segment file has one index column for time. Only on that column can a block be
skipped from its minimum and maximum. Which column it is, is decided per file from its header; the
table above is what files written through the codec facade have. The executor skips blocks only when
the chosen clock is the file's index column and that column cannot be empty (it is declared non-nullable in the file's schema). On any other clock it
reads every block that holds the symbol, which uses the scan budget faster. `explain` shows this
before anything is read:

```text
$ gqodb "explain show trades SYNTH 1970-01-01T00:00:00.000000010Z..1970-01-01T00:00:00.000000012Z on available in ./demo"
plan: trades SYNTH on available in ./demo (budgets: 1000 output rows, 1048576 scanned rows, 4096 blocks)
demo.gqodb.ob: skipped (other stream)
demo.gqodb.tick: 1 of 1 blocks to read (index on available skips blocks outside the range); at most 2455 bytes
```

```text
$ gqodb "explain show trades SYNTH 1..3 on sequence in ./demo"
plan: trades SYNTH on sequence in ./demo (budgets: 1000 output rows, 1048576 scanned rows, 4096 blocks)
demo.gqodb.ob: skipped (other stream)
demo.gqodb.tick: 1 of 1 blocks to read (clock sequence is not indexed in this file: every block with the symbol is read); at most 2455 bytes
```

`explain` opens each file and reads its header and block index; it reads no data blocks. It does not
write the footer and ignores `--format` and `--out`. The byte count is an upper bound: it adds up every
block that holds the symbol, also those the index would skip.

**Choosing a clock for the session.** `use clock <clock>` sets the clock for the following commands
of the run; `on <clock>` after a range overrides it for that command. `order` without `on` uses the
clock of the command (its `on`, else the session's); with neither it is E-CLOCK.

## 6. Ranges and times

A range is `<begin>..<end>`. It is **half-open**: the begin is in, the end is out. A side that is left
empty is open: `2026-03-01..` has no end, `..2026-03-02` has no begin and `..` has neither. Begin
and end must be in order (begin before end); equal or reversed bounds are E-RANGE.

On the time clocks (`available`, `received`, `source`) a bound is a UTC time:

- `2026-03-01`, midnight UTC;
- `2026-03-01T12:30:00Z`;
- `2026-03-01T12:30:00.123456789Z`, with one to nine fraction digits.

The form is strict: four-digit year, two-digit month, day, hour, minute and second, an upper-case `T` and
`Z`. A time zone other than `Z` is an error (E-RANGE, "not a UTC time"), as is an impossible date or
time (30 February, hour 24). Times are whole nanoseconds in a signed 64-bit number, so only instants from
1677-09-21T00:12:43.145224192Z to 2262-04-11T23:47:16.854775807Z can be written: 1677-09-22 and
2262-04-11 are accepted, 1677-09-21 and 2262-04-12 are refused. (The error text for a refused date says
"outside the supported years 1677 to 2262".)

On the `sequence` clock a bound is a whole number (digits only, no sign) that fits a signed 64-bit
number, for example `100..200`. A date there, or a number on a time clock that is not a date, is E-RANGE.
The range is typed only when the clock is known, so `show trades A 1..3 in ./d` without a clock is E-CLOCK
and not an error about `1..3`.

The footer repeats the clock and the range as the executor understood them, with times written to
nine fraction digits.

## 7. Steps

Steps follow the selection after `|`. Each step may be given at most once (a second `take` is
E-SYNTAX).

| Step | Does |
| --- | --- |
| `take <n>` | At most `n` rows. `take 0` is allowed and gives no rows. |
| `skip <n>` | Passes over the first `n` rows. |
| `columns <a>, <b>` | Chooses and orders the columns. A column named twice is E-COLUMN. |
| `order [on <clock>] [desc]` | Sorts on a clock. The sort is stable. Rows with an empty value on that clock come last, also with `desc`, and are not dropped. |

The order in which the steps are written does not change the result: the rows are selected (symbol,
venue, clock and range), then ordered, then `skip` and `take` apply, and `columns` only decides what is
written. Without `order`, rows come in the order of storage: files sorted by name, blocks in file
order, rows in block order, late events included.

```text
$ gqodb "show book SYNTH 1..4 on sequence in ./demo | order desc | columns sequence, level, action"
sequence,level,action
3,1,1
2,1,1
1,1,0
-- clock sequence, range [1, 4)
-- files: 1 read, 1 skipped · blocks: 1 of 1 read · bytes: 2627 · rows: 3 scanned, 3 out
-- skipped demo.gqodb.tick: other stream
```

`order` holds the selected rows in memory, at most 2,000,000. More than that is an E-BUDGET error
(exit code 1) that tells you to narrow the range or drop `order`. With the default scan budget
(1,048,576 rows) that limit is not reached.

## 8. Budgets and truncation

Every `show` has three budgets:

| Budget | Flag | Default | Hard maximum |
| --- | --- | ---: | ---: |
| Output rows | `--max-rows <n>` | 1,000 | 1,000,000 |
| Scanned rows | `--max-scan-rows <n>` | 1,048,576 | 25,600,000 |
| Blocks read | `--max-blocks <n>` | 4,096 | 100,000 |

A value of 0, or above the hard maximum, is rejected before anything is read (E-BUDGET, exit code 2).

When a budget stops the read, the command ends normally and the footer says which one:

```text
gqodb --max-rows 2 "show trades SYNTH in ./demo | columns sequence"
sequence
1
2
-- no time range
-- files: 1 read, 1 skipped · blocks: 1 of 1 read · bytes: 2455 · rows: 3 scanned, 2 out
-- skipped demo.gqodb.ob: other stream
-- truncated: output rows (limits: 2 output rows, 1048576 scanned rows, 4096 blocks)
```

Exit code 0 means the command ran and reported; it does not mean the result is complete. With
`--strict`, a truncated result exits with code 3.

- **Output rows.** Reported when another row would have been written beyond the limit. A `take` that
  is at most the limit is not a truncation, since you asked for it. Exactly as many rows as the limit
  is not a truncation either.
- **Scanned rows.** Blocks are read whole. A block that would take the total of scanned rows past the
  limit is not read at all. The count includes the rows of other symbols in the blocks that were read.
- **Blocks read.** Blocks that are skipped, by the index or because they hold no row of the symbol, do
  not count.
- With `order`, a result that was truncated by the scan or block budget is the order of the rows that
  were read, not of all rows.
- Other limits: at most 10,000 files in one directory (E-BUDGET, exit code 1); a block is decoded
  only through the bounded read route of the store, with a size bound on what a block may expand to.

## 9. Output, formats and exactness

**Format.** `--format table|csv|json|ndjson`. The default is `table` when standard output is a
terminal and `csv` otherwise.

The format examples below show standard output only; the footer follows on standard error as in section 1.

```text
gqodb --format table "show trades SYNTH in ./demo | columns sequence, source_utc_ns, price, quantity"
sequence  source_utc_ns                   price  quantity
--------  ------------------------------  -----  --------
1         1970-01-01T00:00:00.000000008Z  1.00   2
2                                         1.00   2
3         1970-01-01T00:00:00.000000010Z  1.01   3
```

```text
gqodb --format json "show trades SYNTH in ./demo | take 2 | columns sequence, source_utc_ns, price"
[
{"sequence":1,"source_utc_ns":"1970-01-01T00:00:00.000000008Z","price":"1.00"},
{"sequence":2,"source_utc_ns":null,"price":"1.00"}
]
```

- An empty value is empty in `table` and `csv`, and `null` in `json` and `ndjson`.
- `csv` quotes a field only where needed. `json` is one array; `ndjson` is one object per line.
- A result with no rows still writes its header (CSV, table) or `[]`.
- The table is written when the read is done; the other formats write rows while they are read.

**Exactness.** A price or quantity is written from its integer and scale as decimal text with exactly
the file's number of decimals: 108342 with scale 5 is `1.08342`, and 100 with scale 2 is `1.00`. No
value passes through a float. In JSON a decimal is a string (`"1.00"`), so it cannot be rounded by
the reader. Two files with different scales stay exact per row, also when they are sorted together.

**Times.** Time columns are written as UTC with nine fraction digits
(`1970-01-01T00:00:00.000000008Z`), or as whole nanoseconds with `--time ns`:

```text
gqodb --format ndjson --time ns "show trades SYNTH in ./demo | take 2 | columns sequence, available_utc_ns, clock_error_ns"
{"sequence":1,"available_utc_ns":10,"clock_error_ns":2}
{"sequence":2,"available_utc_ns":11,"clock_error_ns":null}
```

`sequence`, `clock_error_ns`, `side`, `level` and `action` are always whole numbers.

**Where output goes.** Rows go to standard output, or with `--out <file>` into a new file. `--out` creates
the file and never overwrites: if it exists, the command fails with E-FILE (exit code 1) and the file is
untouched. The footer and every error go to standard error.

**The footer.** After a `show` that finished, standard error holds:

- `-- clock <clock>, range [<begin>, <end>)`, or `-- no time range`;
- `-- files: <n> read, <n> skipped · blocks: <read> of <total> read · bytes: <n> · rows: <n> scanned, <n> out`
  (`bytes` is the encoded size of the blocks that were read);
- one `-- skipped <file>: <reason>` per skipped file (other stream, not a normalized tick or book file,
  no venue label, other venue);
- one `-- could not read <file>: <reason>` per file that `--skip-bad` skipped;
- `-- excluded: <n> rows without a <clock> value`, when the empty-clock rule dropped rows;
- `-- truncated: <which budget> (limits: ...)`, when a budget stopped the read.

**Damaged files.** A file that cannot be read, or a block that does not decode, is an E-FILE error with
the file name and the reason (exit code 1). The error comes at the point of failure: the table is not
written at all, but `csv`, `json` and `ndjson` may already have written the rows of the files before it
(a `json` array is then left unclosed). Files
are never repaired. With `--skip-bad` an unreadable file is skipped, and a block that does not decode ends the reading
of that file (the rows of its earlier blocks stay in the output); either is named in the footer
(`-- could not read ...`), the other files are read, and the exit code is 1 instead of 0. Other
failures still end the run with E-FILE (section 13). Without the
flag, a damaged file is never skipped silently.

**A closed output pipe.** If the reader of standard output goes away (`gqodb ... | head`), the whole run
ends quietly with exit code 0: no error text, no later commands of a script or of the loop. The one
exception is `--skip-bad`: files that had already been skipped are still reported on standard error,
and the exit code is then 1.

## 10. Errors and exit codes

An error has a code, the place in the source (with a caret line) and, where it can, a hint:

```text
$ gqodb "show trades SYNTH 1970-01-01..1970-01-02 in ./demo"
error[E-CLOCK]: a range needs a clock
  show trades SYNTH 1970-01-01..1970-01-02 in ./demo
                    ^^^^^^^^^^^^^^^^^^^^^^
  help: add one of: on available | on received | on source | on sequence
```

Errors that arise while files are read have no caret line: `error[E-FILE]: c.gqodb.tick: invalid footer`.

| Code | When | Exit code |
| --- | --- | --- |
| E-SYNTAX | The command does not follow the grammar; an unknown flag or flag value; an unknown help topic; input longer than 1 MiB; a step given twice | 2 |
| E-CLOCK | A range or `order` without a clock; an unknown clock name | 2 |
| E-RANGE | A range that is malformed, impossible, out of order or of the wrong kind for its clock | 2 |
| E-STREAM | A stream other than `trades`, `quotes`, `book` | 2 |
| E-COLUMN | A column that does not exist for the stream, or one named twice | 2 |
| E-PATH | No path given (2); the path cannot be read or listed (1) | 2 or 1 |
| E-VENUE | Defined and listed by `help errors`; no command produces it in this version (section 12) | none |
| E-ENGINE | A word reserved for the engine (section 11) | 2 |
| E-BUDGET | A budget flag outside its limits (2); more than 10,000 files, or more rows than `order` can hold (1) | 2 or 1 |
| E-FILE | A damaged or unreadable data file; a file that cannot be created or written for `--out`; a script that cannot be read; a file with a missing or invalid scale | 1 |

Truncation is not an error code: it is a footer line (section 8).

The **exit codes** of the `gqodb` binary:

| Code | Meaning |
| --- | --- |
| 0 | The command ran. Also when the result was truncated (it says so), and after a closed output pipe. |
| 1 | It could not read or write: a data file, a path, the `--out` file; also when `--skip-bad` skipped a file (files skipped for another stream or venue do not count). |
| 2 | The command or its flags are wrong: nothing was read. |
| 3 | The result was truncated and `--strict` was given. |

If a result is both truncated under `--strict` and has files skipped by `--skip-bad`, the code is 3. In a script,
the code is that of the first command that failed. Parse errors (the first error of each command, or
the one lexical error) are reported for the whole script before anything runs. Other errors appear
when the run reaches the command, after the earlier commands have run and written their output: those
found while planning it (a missing clock, an unknown column, a missing path, a step given twice, which is
an E-SYNTAX error) and an unknown help topic (also E-SYNTAX).

## 11. Reserved words (engine)

`as`, `provenance`, `quality`, `uncertainty`, `join`, `asof`, `import`, `capture`, `ledger`.

These words belong to the GQODB engine, which is not part of this repository. This repository contains
no implementation of them. The parser recognises them so that a command written for the engine fails
here with a clear message instead of a syntax error. This command does not parse, which is the point:

```text
gqodb "show trades SYNTH as of 2026-03-05 in ./demo"
error[E-ENGINE]: `as` needs the GQODB engine
  show trades SYNTH as of 2026-03-05 in ./demo
                    ^^
  help: this construct belongs to the paid engine; the free executor does not run it
```

The error is given when a reserved word stands as the first word of a command, in the venue position,
as a step name (`| join ...`), or at another place where the grammar does not expect a word. Written as a
symbol, or inside quotes, a reserved word is plain text: `show trades as in ./d` reads the symbol
`as`. The grammar is the same with or without the engine.

E-ENGINE is not given everywhere. A reserved word in the stream position (`show quality SYNTH ...`) is
E-STREAM, and after `use`, as a help topic or as a name in the keyword form (`show join=1 ...`) it is
E-SYNTAX.

## 12. Venue

The files carry no venue column. In this version a venue is a label in the file's schema metadata,
under the key `gqodb.venue`, set by whoever wrote the file.

- A command without a venue reads every file, labelled or not.
- A command with a venue reads only files whose `gqodb.venue` equals it exactly (case-sensitive).
- A file **without** the key never matches a venue: it is skipped and named in the footer
  (`no venue label`). A file with another value is skipped as `other venue`.

```text
$ gqodb "show trades SYNTH bybit in ./demo | columns sequence"
sequence
-- no time range
-- files: 0 read, 2 skipped · blocks: 0 of 0 read · bytes: 0 · rows: 0 scanned, 0 out
-- skipped demo.gqodb.ob: other stream
-- skipped demo.gqodb.tick: no venue label
```

Files written before this convention, and the demo files, have no label and cannot be selected by venue
until they are written again with one. A venue that matches nothing is an empty result with a footer, not
an error; the error code E-VENUE exists but is not used.

## 13. What is not in phase 1, and what is known to be wrong

**Not in this version of the language.** No calculations (sums, averages, expressions), no filter on
columns other than symbol, venue and the clock range, no joins, no `as of` queries, no provenance or
quality or uncertainty (the reserved words, section 11), no `import`, `capture` or `ledger`. The
language reads files; it writes none except the `--out` result and changes none. It makes no network
call. `inspect`, `verify` and `recover` are not commands of the language: they stay in `gqodb-codec`
and `ob_store`. There is no line editor.

**Known limits.** These were found by reading the code and, where it was possible, by running it. They
are known flaws, not design. They are expected to be fixed, and this list changes then.

- **`--out` is created once per `show`.** A script or loop with a second `show` and `--out` fails at
  the second one with E-FILE, because the file now exists. Use one `show` per `--out` file.
- **The interactive loop stops only on a line that is exactly `quit` or `exit`** (case and surrounding
  spaces do not matter). `quit;`, or `show ...; quit` on one line, runs the command and then goes on
  reading. In a script file given to `gqodb run`, `quit` does end the script.
- **The prompt `gqodb> ` is written to standard output** when standard input is a terminal, also when
  standard output is redirected to a file, so it ends up in that file.
- **`gqodb run <file>` reads the whole file into memory before it checks the 1 MiB limit**, and the
  interactive loop reads a line without a bound. A very large or endless input (for example
  `/dev/zero`) is read until memory runs out instead of being refused.
- **JSON writes whole numbers as bare numbers.** `sequence`, `clock_error_ns`, `side`, `level`, `action`
  and, with `--time ns`, the time columns can exceed 2^53 (a nanosecond time today is about 1.8e18).
  A reader that parses JSON numbers as floating point, as JavaScript does, loses precision above 2^53.
  Decimals and ISO times are strings and are exact. CSV is exact.
- **`explain` ignores `--skip-bad`.** One unreadable file makes `explain` fail with E-FILE, while the
  `show` would have skipped it. Its byte count is an upper bound over all blocks with the symbol.
- **Files that do not qualify by name are ignored silently.** In a directory, a file whose name does
  not end in `.gqodb.tick` or `.gqodb.ob`, and every subdirectory, is not listed or counted anywhere.
- **An outside or broken symbolic link is dropped silently.** A symbolic link in a directory is followed
  only when its target is a file inside that directory; any other link is left out of the file list
  and is not named in the footer or in `explain`.
- **The footer's `blocks: <read> of <total>` counts the total only over files that were opened.** When a
  read stops early (`take`, a budget), the files after the stop are not counted, so the total can be
  lower than the number of blocks the directory holds.
- **Files of one stream with different columns stop the run.** If a later file has other columns than
  the first file that was read (no `columns` step given), the run ends with E-FILE, also with
  `--skip-bad`. The same holds when a file has a column of the wrong type, or lacks a column named in
  a `columns` step. (Found by reading the code; not reproduced with a test file.)
- **A closed output pipe is noticed only by `show`.** `explain` and `help` ignore write errors, so a pipe
  that closes while they write is not noticed. A following command that fails before it writes (for
  example `explain ...; show ... in ./missing`) still prints its error and exits 1.
- **`help` does not list the topics.** The general usage text does not mention `help <topic>`; the
  five topics are `clocks`, `ranges`, `steps`, `flags` and `errors`.
- **The hint of E-RANGE for a keyword range is misleading.** `range=x..y` is refused because it does not
  start with a digit or a dot, but the hint then says "a range is `<begin>..<end>`", which `x..y` is.
- **E-VENUE is never produced.** A venue that does not match gives skipped files and an empty result.
