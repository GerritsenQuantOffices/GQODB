//! Runs a `Plan` over files. The `Executor` trait is the seam the paid engine plugs into; `FileExecutor` is the free one.
use crate::select::pick_rows;
use crate::sink::Sink;
use crate::value::{Cell, TimeFormat, cell, kind_of};
use arrow_array::{Array, Int64Array, RecordBatch};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    WrongStream,
    NotNormalized,
    NoVenueLabel,
    VenueMismatch,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SkipReason::WrongStream => "other stream",
            SkipReason::NotNormalized => "not a normalized tick or book file",
            SkipReason::NoVenueLabel => "no venue label",
            SkipReason::VenueMismatch => "other venue",
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
    pub blocks_total: u64,
    pub bytes_read: u64,
    pub rows_scanned: u64,
    pub rows_out: u64,
    pub null_clock_excluded: u64,
    pub truncated: Option<Truncation>,
}

#[derive(Debug)]
pub enum ExecError {
    Path(String),
    File { path: String, reason: String },
    Budget(String),
    Output(String),
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecError::Path(m) | ExecError::Budget(m) | ExecError::Output(m) => f.write_str(m),
            ExecError::File { path, reason } => write!(f, "{path}: {reason}"),
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

/// A file, or the direct children of a directory with the extension `.gqodb.tick` or `.gqodb.ob`, sorted by name.
/// A symbolic link is followed only when it stays inside the directory.
pub fn discover(path: &str) -> Result<Vec<PathBuf>, ExecError> {
    let root = Path::new(path);
    let meta = std::fs::metadata(root)
        .map_err(|e| ExecError::Path(format!("cannot read `{path}`: {e}")))?;
    if meta.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }
    let canonical = std::fs::canonicalize(root)
        .map_err(|e| ExecError::Path(format!("cannot read `{path}`: {e}")))?;
    let mut files = Vec::new();
    let entries = std::fs::read_dir(root)
        .map_err(|e| ExecError::Path(format!("cannot list `{path}`: {e}")))?;
    for entry in entries {
        let entry = entry.map_err(|e| ExecError::Path(format!("cannot list `{path}`: {e}")))?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if !(file_name.ends_with(".gqodb.tick") || file_name.ends_with(".gqodb.ob")) {
            continue;
        }
        let ty = entry
            .file_type()
            .map_err(|e| ExecError::Path(format!("cannot inspect `{file_name}`: {e}")))?;
        let keep = if ty.is_symlink() {
            std::fs::canonicalize(entry.path())
                .is_ok_and(|t| t.starts_with(&canonical) && t.is_file())
        } else {
            ty.is_file()
        };
        if keep {
            files.push(entry.path());
            if files.len() > MAX_FILES {
                return Err(ExecError::Budget(format!(
                    "more than {MAX_FILES} files in `{path}`; point at a smaller directory"
                )));
            }
        }
    }
    files.sort_by_key(|p| name_of(p));
    Ok(files)
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
    let scale = |key: &str| -> Result<u8, ExecError> {
        meta.get(key)
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| bad(format!("missing or invalid `{key}`")))
    };
    let scales = (
        scale("gqodb.price_decimals")?,
        scale("gqodb.quantity_decimals")?,
    );
    Ok(Ok(Opened {
        reader,
        schema,
        scales,
    }))
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
        sink.row(&cells)
            .map_err(|e| ExecError::Output(e.to_string()))?;
        self.emitted += 1;
        self.report.rows_out += 1;
        Ok(())
    }
}

fn begin(st: &mut State, sink: &mut dyn Sink, columns: Vec<String>) -> Result<(), ExecError> {
    if st.header.is_none() {
        sink.begin(&columns)
            .map_err(|e| ExecError::Output(e.to_string()))?;
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
    st: &mut State,
    sink: &mut dyn Sink,
) -> Result<(), ExecError> {
    let plan = st.plan;
    let prune = plan
        .range
        .as_ref()
        .is_some_and(|r| prunes(&opened, r.clock));
    let Opened {
        mut reader,
        schema,
        scales,
    } = opened;
    let file_label = name_of(path);
    let columns = columns_of(plan, &schema);
    begin(st, sink, columns.clone())?;
    let header = st.header.clone().expect("begun");
    if columns != header {
        return Err(ExecError::File {
            path: file_label,
            reason: format!("columns differ from the first file ({})", header.join(", ")),
        });
    }
    let mut indexes = Vec::new();
    for name in &columns {
        let i = schema.index_of(name).map_err(|_| ExecError::File {
            path: file_label.clone(),
            reason: format!("has no column `{name}`"),
        })?;
        indexes.push((i, kind_of(name)));
    }
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

impl Executor for FileExecutor {
    fn execute(
        &self,
        plan: &Plan,
        opts: &Options,
        sink: &mut dyn Sink,
    ) -> Result<Report, ExecError> {
        let files = discover(&plan.path)?;
        let mut st = State {
            plan,
            opts,
            report: Report::default(),
            header: None,
            skipped: 0,
            emitted: 0,
            stop: false,
            ordered: Vec::new(),
        };
        for path in &files {
            if st.stop {
                break;
            }
            match open_for(path, plan) {
                Ok(Ok(opened)) => read_file(path, opened, &mut st, sink)?,
                Ok(Err(reason)) => st.report.skipped.push((name_of(path), reason)),
                Err(e) if opts.skip_bad => st.report.bad_files.push((name_of(path), e.to_string())),
                Err(e) => return Err(e),
            }
        }
        if let Some(order) = plan.order {
            let mut rows = std::mem::take(&mut st.ordered);
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
            begin(&mut st, sink, columns)?;
        }
        sink.finish()
            .map_err(|e| ExecError::Output(e.to_string()))?;
        Ok(st.report)
    }

    fn explain(&self, plan: &Plan, opts: &Options) -> Result<Vec<String>, ExecError> {
        let files = discover(&plan.path)?;
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
        for path in &files {
            match open_for(path, plan)? {
                Err(reason) => lines.push(format!("{}: skipped ({reason})", name_of(path))),
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
                    let candidates = if use_index {
                        let r = plan.range.as_ref().expect("checked");
                        with_symbol
                            .iter()
                            .filter(|b| {
                                !(r.end.is_some_and(|e| b.min_time >= e)
                                    || r.start.is_some_and(|s| b.max_time < s))
                            })
                            .count()
                    } else {
                        with_symbol.len()
                    };
                    let bytes: u64 = with_symbol.iter().map(|b| b.length).sum();
                    lines.push(format!(
                        "{}: {} of {} blocks to read ({}); at most {} bytes",
                        name_of(path),
                        candidates,
                        blocks.len(),
                        why,
                        bytes
                    ));
                }
            }
        }
        Ok(lines)
    }
}
