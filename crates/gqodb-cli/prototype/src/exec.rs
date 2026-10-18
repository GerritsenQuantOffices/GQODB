//! Runs a `Plan` over files. The `Executor` trait is the seam the paid engine plugs into; `FileExecutor` is the free one.
use crate::select::pick_rows;
use crate::sink::Sink;
use crate::value::{Cell, TimeFormat, cell, kind_of};
use arrow_array::{Array, Int64Array, RecordBatch};
use arrow_schema::DataType;
use gqodb_lang::plan::Plan;
use gqodb_lang::{ClockName, Stream};
use std::fmt;
use std::path::{Path, PathBuf};

pub const HARD_MAX_ROWS: u64 = 1_000_000;
pub const HARD_MAX_SCAN_ROWS: u64 = 25_600_000;
pub const HARD_MAX_BLOCKS: u64 = 100_000;
/// Files read from one directory.
pub const MAX_FILES: usize = 10_000;
/// Rows `order` may hold in memory at once.
pub const ORDER_BUFFER_ROWS: usize = 2_000_000;
const ALL_CLOCKS: [ClockName; 4] = [
    ClockName::Available,
    ClockName::Received,
    ClockName::Source,
    ClockName::Sequence,
];
/// The largest decimal scale the codec accepts for prices and quantities.
pub const MAX_SCALE: u8 = 12;

#[derive(Clone, Debug)]
pub struct Options {
    pub max_rows: u64,
    pub max_scan_rows: u64,
    pub max_blocks: u64,
    pub strict: bool,
    pub skip_bad: bool,
    pub time: TimeFormat,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_rows: 1_000,
            max_scan_rows: 1_048_576,
            max_blocks: 4_096,
            strict: false,
            skip_bad: false,
            time: TimeFormat::Iso,
        }
    }
}

impl Options {
    pub fn validate(&self) -> Result<(), String> {
        let check = |name: &str, v: u64, max: u64| {
            if (1..=max).contains(&v) {
                Ok(())
            } else {
                Err(format!("--{name} must be between 1 and {max}"))
            }
        };
        check("max-rows", self.max_rows, HARD_MAX_ROWS)?;
        check("max-scan-rows", self.max_scan_rows, HARD_MAX_SCAN_ROWS)?;
        check("max-blocks", self.max_blocks, HARD_MAX_BLOCKS)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Truncation {
    OutputRows,
    ScanRows,
    Blocks,
}

impl fmt::Display for Truncation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Truncation::OutputRows => "output rows",
            Truncation::ScanRows => "scanned rows",
            Truncation::Blocks => "blocks read",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkipReason {
    WrongStream,
    NotNormalized,
    NoVenueLabel,
    VenueMismatch,
    /// A symbolic link whose target lies outside the directory.
    LinkOutside,
    /// A symbolic link whose target does not exist.
    BrokenLink,
    /// A name that fits, but the entry is not a regular file (a directory, a pipe, a device) or the link leads to one.
    NotRegular,
    /// A symbolic link to a file that is read under its own name; the name of that file.
    DuplicateOf(String),
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SkipReason::WrongStream => "other stream",
            SkipReason::NotNormalized => "not a normalized tick or book file",
            SkipReason::NoVenueLabel => "no venue label",
            SkipReason::VenueMismatch => "other venue",
            SkipReason::LinkOutside => "symbolic link to outside the directory",
            SkipReason::BrokenLink => "broken symbolic link",
            SkipReason::NotRegular => "not a regular file",
            SkipReason::DuplicateOf(name) => return write!(f, "duplicate of {name}"),
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub columns: Vec<String>,
    pub files_read: u64,
    pub skipped: Vec<(String, SkipReason)>,
    /// Files that could not be read completely, with `--skip-bad`.
    pub bad_files: Vec<(String, String)>,
    pub blocks_read: u64,
    /// Blocks of the files that were read; files left unopened (after a limit or `take`) are not in it.
    pub blocks_total: u64,
    pub bytes_read: u64,
    pub rows_scanned: u64,
    pub rows_out: u64,
    pub null_clock_excluded: u64,
    /// Rows that `order` put last because they have no value on the order clock.
    pub null_order_last: u64,
    pub truncated: Option<Truncation>,
}

#[derive(Debug)]
pub enum ExecError {
    Path(String),
    File {
        path: String,
        reason: String,
    },
    Budget(String),
    Output(String),
    /// The reader of our output went away (for example `| head`). Not an error to report.
    /// Carries the `--skip-bad` notices collected so far, which are still reported.
    Closed(Vec<(String, String)>),
}

fn out_err(e: std::io::Error) -> ExecError {
    if e.kind() == std::io::ErrorKind::BrokenPipe {
        ExecError::Closed(Vec::new())
    } else {
        ExecError::Output(e.to_string())
    }
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecError::Path(m) | ExecError::Budget(m) | ExecError::Output(m) => f.write_str(m),
            ExecError::File { path, reason } => write!(f, "{path}: {reason}"),
            ExecError::Closed(_) => f.write_str("output closed"),
        }
    }
}

impl std::error::Error for ExecError {}

pub trait Executor {
    fn execute(
        &self,
        plan: &Plan,
        opts: &Options,
        sink: &mut dyn Sink,
    ) -> Result<Report, ExecError>;
    /// What `execute` would do, one line per file, without reading data.
    fn explain(&self, plan: &Plan, opts: &Options) -> Result<Vec<String>, ExecError>;
}

pub struct FileExecutor;

/// The metadata that says which stream a file holds: `gqodb.family` and, for ticks, `gqodb.subtype`.
fn stream_of(meta: &std::collections::HashMap<String, String>) -> Option<Stream> {
    match (
        meta.get("gqodb.family").map(String::as_str),
        meta.get("gqodb.subtype").map(String::as_str),
    ) {
        (Some("tick"), Some("trade")) => Some(Stream::Trades),
        (Some("tick"), Some("quote")) => Some(Stream::Quotes),
        (Some("orderbook"), _) => Some(Stream::Book),
        _ => None,
    }
}

fn name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The files to read and the entries with a fitting name that are left out, with the reason.
#[derive(Debug, Default)]
pub struct Discovered {
    pub files: Vec<PathBuf>,
    pub skipped: Vec<(String, SkipReason)>,
}

/// A file, or the direct children of a directory with the extension `.gqodb.tick` or `.gqodb.ob`, sorted by name.
/// An entry with a fitting name that is not read is reported, not dropped: a symbolic link that leads out of the
/// directory or nowhere, an entry that is not a regular file (a directory, a pipe, a device), and a symbolic link to
/// a file that is read already (it is read once, under the name of the file itself, or else of the first link).
pub fn scan(path: &str) -> Result<Discovered, ExecError> {
    let root = Path::new(path);
    let meta = std::fs::metadata(root)
        .map_err(|e| ExecError::Path(format!("cannot read `{path}`: {e}")))?;
    if meta.is_file() {
        return Ok(Discovered {
            files: vec![root.to_path_buf()],
            skipped: Vec::new(),
        });
    }
    if !meta.is_dir() {
        return Err(ExecError::Path(format!(
            "`{path}` is neither a file nor a directory"
        )));
    }
    let canonical = std::fs::canonicalize(root)
        .map_err(|e| ExecError::Path(format!("cannot read `{path}`: {e}")))?;
    let entries = std::fs::read_dir(root)
        .map_err(|e| ExecError::Path(format!("cannot list `{path}`: {e}")))?;
    let mut regular: Vec<(String, PathBuf)> = Vec::new();
    let mut links: Vec<(String, PathBuf, PathBuf)> = Vec::new();
    let mut skipped = Vec::new();
    let mut matching = 0;
    for entry in entries {
        let entry = entry.map_err(|e| ExecError::Path(format!("cannot list `{path}`: {e}")))?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if !(file_name.ends_with(".gqodb.tick") || file_name.ends_with(".gqodb.ob")) {
            continue;
        }
        matching += 1;
        if matching > MAX_FILES {
            return Err(ExecError::Budget(format!(
                "more than {MAX_FILES} files in `{path}`; point at a smaller directory"
            )));
        }
        let ty = entry
            .file_type()
            .map_err(|e| ExecError::Path(format!("cannot inspect `{file_name}`: {e}")))?;
        // Nothing here opens the entry: a pipe or a device must never be able to block the run.
        if ty.is_file() {
            regular.push((file_name, entry.path()));
        } else if ty.is_symlink() {
            match std::fs::canonicalize(entry.path()) {
                Err(_) => skipped.push((file_name, SkipReason::BrokenLink)),
                Ok(target) if !target.starts_with(&canonical) => {
                    skipped.push((file_name, SkipReason::LinkOutside));
                }
                Ok(target) if std::fs::metadata(&target).is_ok_and(|m| m.is_file()) => {
                    links.push((file_name, entry.path(), target));
                }
                Ok(_) => skipped.push((file_name, SkipReason::NotRegular)),
            }
        } else {
            skipped.push((file_name, SkipReason::NotRegular));
        }
    }
    let mut files = regular;
    if !links.is_empty() {
        let mut seen: std::collections::HashMap<PathBuf, String> = files
            .iter()
            .map(|(name, p)| {
                (
                    std::fs::canonicalize(p).unwrap_or_else(|_| p.clone()),
                    name.clone(),
                )
            })
            .collect();
        links.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, p, target) in links {
            match seen.get(&target) {
                Some(original) => skipped.push((name, SkipReason::DuplicateOf(original.clone()))),
                None => {
                    seen.insert(target, name.clone());
                    files.push((name, p));
                }
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    skipped.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(Discovered {
        files: files.into_iter().map(|(_, p)| p).collect(),
        skipped,
    })
}

/// The files `scan` would read.
pub fn discover(path: &str) -> Result<Vec<PathBuf>, ExecError> {
    Ok(scan(path)?.files)
}

struct Opened {
    reader: gqodb_store::Reader,
    schema: arrow_schema::SchemaRef,
    scales: (u8, u8),
}

/// Opens a file and decides whether it takes part: right stream, right venue.
fn open_for(path: &Path, plan: &Plan) -> Result<Result<Opened, SkipReason>, ExecError> {
    let bad = |reason: String| ExecError::File {
        path: name_of(path),
        reason,
    };
    let reader = gqodb_store::Reader::open(path).map_err(|e| bad(format!("{e:#}")))?;
    let schema = reader.schema();
    let meta = schema.metadata();
    match stream_of(meta) {
        None => return Ok(Err(SkipReason::NotNormalized)),
        Some(s) if s != plan.stream => return Ok(Err(SkipReason::WrongStream)),
        Some(_) => {}
    }
    if let Some(venue) = &plan.venue {
        match meta.get("gqodb.venue") {
            None => return Ok(Err(SkipReason::NoVenueLabel)),
            Some(v) if v != venue => return Ok(Err(SkipReason::VenueMismatch)),
            Some(_) => {}
        }
    }
    // Files that did not pass through the codec's own writer are not checked by it: check what reading relies on.
    let scale = |key: &str| -> Result<u8, ExecError> {
        let n: u8 = meta
            .get(key)
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| bad(format!("missing or invalid `{key}`")))?;
        if n > MAX_SCALE {
            return Err(bad(format!(
                "`{key}` is {n}, more than the largest scale {MAX_SCALE}"
            )));
        }
        Ok(n)
    };
    let scales = (
        scale("gqodb.price_decimals")?,
        scale("gqodb.quantity_decimals")?,
    );
    for clock in ALL_CLOCKS {
        let column = clock.column();
        if let Ok(field) = schema.field_with_name(column) {
            if *field.data_type() != DataType::Int64 {
                return Err(bad(format!("column `{column}` is not a 64-bit integer")));
            }
        }
    }
    let used = plan
        .range
        .map(|r| r.clock)
        .into_iter()
        .chain(plan.order.map(|o| o.clock));
    for clock in used {
        if schema.field_with_name(clock.column()).is_err() {
            return Err(bad(format!("has no column `{}`", clock.column())));
        }
    }
    Ok(Ok(Opened {
        reader,
        schema,
        scales,
    }))
}

/// Where the columns of the answer are in a file, and how each is written.
type Indexes = Vec<(usize, crate::value::Kind)>;

/// The columns the answer has, and where they are in the file. Checked before anything is written, so that a file
/// that cannot take part (a column is missing, or the columns differ from the first file) leaves no header behind.
fn check_columns(
    path: &Path,
    plan: &Plan,
    opened: &Opened,
    first: Option<&Vec<String>>,
) -> Result<(Vec<String>, Indexes), ExecError> {
    let bad = |reason: String| ExecError::File {
        path: name_of(path),
        reason,
    };
    let columns = columns_of(plan, &opened.schema);
    let mut indexes = Vec::new();
    for name in &columns {
        let i = opened
            .schema
            .index_of(name)
            .map_err(|_| bad(format!("has no column `{name}`")))?;
        indexes.push((i, kind_of(name)));
    }
    if let Some(first) = first {
        if &columns != first {
            return Err(bad(format!(
                "columns differ from the first file ({})",
                first.join(", ")
            )));
        }
    }
    Ok((columns, indexes))
}

/// Why a file could not be read, without the file name (the notice names the file itself).
fn reason_of(e: &ExecError) -> String {
    match e {
        ExecError::File { reason, .. } => reason.clone(),
        other => other.to_string(),
    }
}

/// A block can be skipped by its time bounds only when the file indexes the chosen clock and that column has no empty values.
fn prunes(opened: &Opened, clock: ClockName) -> bool {
    let (time_column, _) = opened.reader.index_columns();
    clock.column() == time_column
        && opened
            .schema
            .field_with_name(time_column)
            .is_ok_and(|f| !f.is_nullable())
}

struct State<'a> {
    plan: &'a Plan,
    opts: &'a Options,
    report: Report,
    header: Option<Vec<String>>,
    /// Rows passed over by `skip`.
    skipped: u64,
    emitted: u64,
    stop: bool,
    ordered: Vec<(Option<i64>, Vec<Cell>)>,
}

impl State<'_> {
    fn limit(&self) -> u64 {
        self.plan
            .take
            .map_or(self.opts.max_rows, |t| t.min(self.opts.max_rows))
    }

    fn truncate(&mut self, why: Truncation) {
        self.report.truncated.get_or_insert(why);
        self.stop = true;
    }

    /// True when the row was taken; false when it was passed over or the limit is reached.
    fn offer(&mut self, cells: Vec<Cell>, sink: &mut dyn Sink) -> Result<(), ExecError> {
        if self.skipped < self.plan.skip {
            self.skipped += 1;
            return Ok(());
        }
        if self.emitted >= self.limit() {
            if self.plan.take.is_none_or(|t| t > self.opts.max_rows) {
                self.truncate(Truncation::OutputRows);
            }
            self.stop = true;
            return Ok(());
        }
        sink.row(&cells).map_err(out_err)?;
        self.emitted += 1;
        self.report.rows_out += 1;
        Ok(())
    }
}

fn begin(st: &mut State, sink: &mut dyn Sink, columns: Vec<String>) -> Result<(), ExecError> {
    if st.header.is_none() {
        sink.begin(&columns).map_err(out_err)?;
        st.report.columns.clone_from(&columns);
        st.header = Some(columns);
    }
    Ok(())
}

fn columns_of(plan: &Plan, schema: &arrow_schema::SchemaRef) -> Vec<String> {
    plan.columns
        .clone()
        .unwrap_or_else(|| schema.fields().iter().map(|f| f.name().clone()).collect())
}

fn read_file(
    path: &Path,
    opened: Opened,
    columns: Vec<String>,
    indexes: Indexes,
    st: &mut State,
    sink: &mut dyn Sink,
) -> Result<(), ExecError> {
    let plan = st.plan;
    let prune = plan
        .range
        .as_ref()
        .is_some_and(|r| prunes(&opened, r.clock));
    let Opened {
        mut reader, scales, ..
    } = opened;
    let file_label = name_of(path);
    begin(st, sink, columns)?;
    let order_key = plan.order.map(|o| o.clock.column());
    st.report.files_read += 1;
    let blocks: Vec<_> = reader.blocks().to_vec();
    for (i, block) in blocks.iter().enumerate() {
        st.report.blocks_total += 1;
        if st.stop || !block.symbols.iter().any(|s| s == &plan.symbol) {
            continue;
        }
        if let (true, Some(r)) = (prune, plan.range.as_ref()) {
            if r.end.is_some_and(|e| block.min_time >= e)
                || r.start.is_some_and(|s| block.max_time < s)
            {
                continue;
            }
        }
        if st.report.blocks_read >= st.opts.max_blocks {
            st.truncate(Truncation::Blocks);
            continue;
        }
        if st.report.rows_scanned + block.rows as u64 > st.opts.max_scan_rows {
            st.truncate(Truncation::ScanRows);
            continue;
        }
        let batch =
            match reader.read_block_bounded(i, block.rows.max(1), gqodb_codec::MAX_DECODED_BYTES) {
                Ok(b) => b,
                Err(e) if st.opts.skip_bad => {
                    st.report
                        .bad_files
                        .push((file_label.clone(), format!("block {i}: {e:#}")));
                    return Ok(());
                }
                Err(e) => {
                    return Err(ExecError::File {
                        path: file_label,
                        reason: format!("block {i}: {e:#}"),
                    });
                }
            };
        st.report.blocks_read += 1;
        st.report.rows_scanned += batch.num_rows() as u64;
        st.report.bytes_read += block.length;
        let picked =
            pick_rows(&batch, &plan.symbol, plan.range.as_ref()).map_err(|e| ExecError::File {
                path: file_label.clone(),
                reason: format!("{e:#}"),
            })?;
        st.report.null_clock_excluded += picked.null_clock;
        for row in picked.rows {
            let cells = render_row(&batch, row, &indexes, scales, st.opts.time).map_err(|e| {
                ExecError::File {
                    path: file_label.clone(),
                    reason: format!("{e:#}"),
                }
            })?;
            if let Some(key_column) = order_key {
                let key = batch
                    .column_by_name(key_column)
                    .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
                    .and_then(|a| (!a.is_null(row)).then(|| a.value(row)));
                if st.ordered.len() >= ORDER_BUFFER_ROWS {
                    return Err(ExecError::Budget(format!(
                        "`order` would hold more than {ORDER_BUFFER_ROWS} rows; narrow the range or drop `order`"
                    )));
                }
                st.ordered.push((key, cells));
            } else {
                st.offer(cells, sink)?;
                if st.stop {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn render_row(
    batch: &RecordBatch,
    row: usize,
    indexes: &[(usize, crate::value::Kind)],
    scales: (u8, u8),
    time: TimeFormat,
) -> anyhow::Result<Vec<Cell>> {
    indexes
        .iter()
        .map(|(i, kind)| cell(batch.column(*i).as_ref(), row, *kind, scales, time))
        .collect()
}

fn drive(
    plan: &Plan,
    opts: &Options,
    files: &[PathBuf],
    st: &mut State,
    sink: &mut dyn Sink,
) -> Result<(), ExecError> {
    for path in files {
        if st.stop {
            break;
        }
        let prepared = match open_for(path, plan) {
            Ok(Ok(opened)) => {
                check_columns(path, plan, &opened, st.header.as_ref()).map(|c| Ok((opened, c)))
            }
            Ok(Err(reason)) => Ok(Err(reason)),
            Err(e) => Err(e),
        };
        match prepared {
            Ok(Ok((opened, (columns, indexes)))) => {
                read_file(path, opened, columns, indexes, st, sink)?;
            }
            Ok(Err(reason)) => st.report.skipped.push((name_of(path), reason)),
            Err(e) if opts.skip_bad => st.report.bad_files.push((name_of(path), reason_of(&e))),
            Err(e) => return Err(e),
        }
    }
    if let Some(order) = plan.order {
        let mut rows = std::mem::take(&mut st.ordered);
        st.report.null_order_last = rows.iter().filter(|r| r.0.is_none()).count() as u64;
        rows.sort_by(|a, b| match (a.0, b.0) {
            (Some(x), Some(y)) => {
                if order.desc {
                    y.cmp(&x)
                } else {
                    x.cmp(&y)
                }
            }
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
        st.stop = false;
        for (_, cells) in rows {
            st.offer(cells, sink)?;
            if st.stop {
                break;
            }
        }
    }
    if st.header.is_none() {
        let columns = plan.columns.clone().unwrap_or_else(|| {
            gqodb_lang::plan::known_columns(plan.stream)
                .iter()
                .map(|c| (*c).to_string())
                .collect()
        });
        begin(st, sink, columns)?;
    }
    sink.finish().map_err(out_err)?;
    Ok(())
}

impl Executor for FileExecutor {
    fn execute(
        &self,
        plan: &Plan,
        opts: &Options,
        sink: &mut dyn Sink,
    ) -> Result<Report, ExecError> {
        let found = scan(&plan.path)?;
        let mut st = State {
            plan,
            opts,
            report: Report {
                skipped: found.skipped,
                ..Report::default()
            },
            header: None,
            skipped: 0,
            emitted: 0,
            stop: false,
            ordered: Vec::new(),
        };
        match drive(plan, opts, &found.files, &mut st, sink) {
            Ok(()) => Ok(st.report),
            Err(ExecError::Closed(_)) => Err(ExecError::Closed(st.report.bad_files)),
            Err(e) => Err(e),
        }
    }

    fn explain(&self, plan: &Plan, opts: &Options) -> Result<Vec<String>, ExecError> {
        let found = scan(&plan.path)?;
        let mut lines = vec![format!(
            "plan: {} {}{}{} in {} (budgets: {} output rows, {} scanned rows, {} blocks)",
            plan.stream.name(),
            plan.symbol,
            plan.venue
                .as_ref()
                .map(|v| format!(" venue {v}"))
                .unwrap_or_default(),
            plan.range
                .map(|r| format!(" on {}", r.clock.name()))
                .unwrap_or_default(),
            plan.path,
            opts.max_rows,
            opts.max_scan_rows,
            opts.max_blocks
        )];
        // One line per entry, in name order: the files and the entries that are left out.
        let mut entries: Vec<(String, Option<&PathBuf>, Option<&SkipReason>)> = found
            .files
            .iter()
            .map(|p| (name_of(p), Some(p), None))
            .chain(
                found
                    .skipped
                    .iter()
                    .map(|(n, r)| (n.clone(), None, Some(r))),
            )
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, path, left_out) in entries {
            if let Some(reason) = left_out {
                lines.push(format!("{name}: skipped ({reason})"));
                continue;
            }
            let path = path.expect("a file or a reason");
            let opened = match open_for(path, plan) {
                Err(e) if opts.skip_bad => {
                    lines.push(format!(
                        "{name}: could not be read ({}); skipped with --skip-bad",
                        reason_of(&e)
                    ));
                    continue;
                }
                other => other?,
            };
            match opened {
                Err(reason) => lines.push(format!("{name}: skipped ({reason})")),
                Ok(opened) => {
                    let blocks = opened.reader.blocks();
                    let with_symbol: Vec<_> = blocks
                        .iter()
                        .filter(|b| b.symbols.iter().any(|s| s == &plan.symbol))
                        .collect();
                    let (use_index, why) = match plan.range.as_ref() {
                        None => (false, "no range: every block with the symbol".to_string()),
                        Some(r) if prunes(&opened, r.clock) => (
                            true,
                            format!("index on {} skips blocks outside the range", r.clock.name()),
                        ),
                        Some(r) => (
                            false,
                            format!(
                                "clock {} is not indexed in this file: every block with the symbol is read",
                                r.clock.name()
                            ),
                        ),
                    };
                    let candidates: Vec<_> = if use_index {
                        let r = plan.range.as_ref().expect("checked");
                        with_symbol
                            .into_iter()
                            .filter(|b| {
                                !(r.end.is_some_and(|e| b.min_time >= e)
                                    || r.start.is_some_and(|s| b.max_time < s))
                            })
                            .collect()
                    } else {
                        with_symbol
                    };
                    let bytes: u64 = candidates.iter().map(|b| b.length).sum();
                    lines.push(format!(
                        "{name}: {} of {} blocks to read ({why}); at most {bytes} bytes",
                        candidates.len(),
                        blocks.len(),
                    ));
                }
            }
        }
        Ok(lines)
    }
}
