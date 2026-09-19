//! Experimental .gqodb.ob segments. Independent of ingestion, bus and replay.
#![forbid(unsafe_code)]

use anyhow::{Context, Result, ensure};
use arrow_array::{Array, BooleanArray, Int64Array, LargeStringArray, RecordBatch};
use arrow_schema::{DataType, Schema, SchemaRef};
use arrow_select::filter::filter_record_batch;
use bytes::Bytes;
use gqodb_blocks::real::{self, Decoder, Mode};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::Arc,
    time::Instant,
};

const HEADER: &[u8; 8] = b"GQOBHDR1";
const BLOCK: &[u8; 8] = b"GQOBBLK1";
const INDEX: &[u8; 8] = b"GQOBIDX1";
const END: &[u8; 8] = b"GQOBEND1";
const FRAME: u64 = 24;
const MAX_META: usize = 16 * 1024 * 1024;
const MAX_PAYLOAD: usize = 128 * 1024 * 1024;
const MAX_BLOCKS: usize = 100_000;

#[derive(Clone, Serialize, Deserialize)]
struct Header {
    schema: Schema,
    time_column: String,
    symbol_column: String,
    codec: String,
}

fn native_mode(codec: &str) -> Result<Mode> {
    match codec {
        "GQOREAL3/GQOADP02" => Ok(Mode::Gqodb),
        "GQOREAL4/GQOBOOK1" => Ok(Mode::GqodbBook),
        _ => anyhow::bail!("unsupported native segment codec"),
    }
}

fn bounded_expansion(
    bytes: &[u8],
    indexed_rows: usize,
    max_rows: usize,
    max_bytes: usize,
    mode: Mode,
) -> Result<()> {
    ensure!(
        bytes.len() >= 32 && indexed_rows <= max_rows,
        "bounded block row/length limit"
    );
    let end = bytes.len() - 4;
    let metadata = u32::from_le_bytes(bytes[8..12].try_into()?) as usize;
    ensure!(
        metadata <= MAX_META && metadata <= end - 12,
        "bounded schema length"
    );
    let payload = &bytes[12 + metadata..end];
    ensure!(payload.len() >= 20, "bounded payload length");
    let (rows, columns) = match mode {
        Mode::Gqodb => {
            ensure!(
                &bytes[..8] == b"GQOREAL3" && &payload[..8] == b"GQOADP02",
                "bounded tick codec mismatch"
            );
            (
                u32::from_le_bytes(payload[12..16].try_into()?) as usize,
                u16::from_le_bytes(payload[10..12].try_into()?) as usize,
            )
        }
        Mode::GqodbBook => {
            ensure!(
                &bytes[..8] == b"GQOREAL4" && &payload[..8] == b"GQOBOOK1",
                "bounded book codec mismatch"
            );
            (
                u32::from_le_bytes(payload[8..12].try_into()?) as usize,
                u32::from_le_bytes(payload[12..16].try_into()?) as usize,
            )
        }
        _ => anyhow::bail!("non-native bounded codec"),
    };
    ensure!(
        rows == indexed_rows && rows <= max_rows && (1..=64).contains(&columns),
        "bounded dimensions mismatch"
    );
    let layout: serde_json::Value = serde_json::from_slice(&bytes[12..12 + metadata])?;
    let mappings = layout["columns"]
        .as_array()
        .context("missing bounded column mapping")?;
    ensure!(mappings.len() <= 64, "bounded column mapping limit");
    let mut estimate = rows
        .checked_mul(columns + mappings.len())
        .and_then(|n| n.checked_mul(8))
        .context("bounded expansion overflow")?;
    for mapping in mappings {
        let dictionary = mapping["dictionary"]
            .as_array()
            .context("missing bounded dictionary")?;
        let mut maximum = 0;
        for value in dictionary {
            maximum = maximum.max(value.as_str().context("invalid bounded dictionary")?.len());
        }
        estimate = estimate
            .checked_add(
                rows.checked_mul(maximum)
                    .context("bounded dictionary overflow")?,
            )
            .context("bounded expansion overflow")?;
    }
    ensure!(estimate <= max_bytes, "bounded decoded expansion limit");
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockInfo {
    pub offset: u64,
    pub length: u64,
    pub rows: usize,
    pub min_time: i64,
    pub max_time: i64,
    pub symbols: Vec<String>,
}

fn fields<'a>(
    batch: &'a RecordBatch,
    header: &Header,
) -> Result<(&'a Int64Array, &'a LargeStringArray)> {
    ensure!(batch.schema().as_ref() == &header.schema, "schema changed");
    let t = batch
        .column_by_name(&header.time_column)
        .context("time column missing")?
        .as_any()
        .downcast_ref::<Int64Array>()
        .context("time must be Int64 nanoseconds")?;
    let s = batch
        .column_by_name(&header.symbol_column)
        .context("symbol column missing")?
        .as_any()
        .downcast_ref::<LargeStringArray>()
        .context("symbol must be LargeUtf8")?;
    ensure!(
        t.null_count() == 0 && s.null_count() == 0,
        "null indexed fields"
    );
    Ok((t, s))
}

fn info(batch: &RecordBatch, header: &Header, offset: u64, length: u64) -> Result<BlockInfo> {
    ensure!(batch.num_rows() > 0, "empty block");
    let (t, s) = fields(batch, header)?;
    // Validate every symbol, but insert only at a run transition. Bulk collecting
    // all row strings sorts a large duplicate-heavy temporary vector unnecessarily.
    let mut symbols = BTreeSet::new();
    let mut previous = None;
    for symbol in s.iter().flatten() {
        if previous != Some(symbol) {
            symbols.insert(symbol);
            previous = Some(symbol);
        }
    }
    Ok(BlockInfo {
        offset,
        length,
        rows: batch.num_rows(),
        min_time: *t.values().iter().min().unwrap(),
        max_time: *t.values().iter().max().unwrap(),
        symbols: symbols.into_iter().map(str::to_owned).collect(),
    })
}

fn frame(file: &mut File, magic: &[u8; 8], meta: &[u8], payload: &[u8]) -> Result<u64> {
    ensure!(
        meta.len() <= MAX_META && payload.len() <= MAX_PAYLOAD,
        "frame too large"
    );
    let mut h = Vec::from(*magic);
    h.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    h.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    let mut hash = crc32fast::Hasher::new();
    hash.update(meta);
    hash.update(payload);
    h.extend_from_slice(&hash.finalize().to_le_bytes());
    h.extend_from_slice(&crc32fast::hash(&h).to_le_bytes());
    file.write_all(&h)?;
    file.write_all(meta)?;
    file.write_all(payload)?;
    Ok(FRAME + meta.len() as u64 + payload.len() as u64)
}

struct DecodedFrame {
    magic: [u8; 8],
    meta: Vec<u8>,
    payload: Vec<u8>,
    length: u64,
}

/// A physically incomplete tail is distinguishable from a complete corrupt frame.
fn read_frame(file: &mut File, offset: u64, end: u64) -> Result<Option<DecodedFrame>> {
    ensure!(offset <= end, "frame offset past EOF");
    if end - offset < FRAME {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut h = [0u8; 24];
    file.read_exact(&mut h)?;
    ensure!(
        crc32fast::hash(&h[..20]) == u32::from_le_bytes(h[20..].try_into()?),
        "frame header CRC"
    );
    let m = u32::from_le_bytes(h[8..12].try_into()?) as usize;
    let p = u32::from_le_bytes(h[12..16].try_into()?) as usize;
    ensure!(m <= MAX_META && p <= MAX_PAYLOAD, "frame allocation limit");
    let length = FRAME + m as u64 + p as u64;
    if length > end - offset {
        return Ok(None);
    }
    let mut meta = vec![0; m];
    let mut payload = vec![0; p];
    file.read_exact(&mut meta)?;
    file.read_exact(&mut payload)?;
    let mut hash = crc32fast::Hasher::new();
    hash.update(&meta);
    hash.update(&payload);
    ensure!(
        hash.finalize() == u32::from_le_bytes(h[16..20].try_into()?),
        "frame body CRC"
    );
    Ok(Some(DecodedFrame {
        magic: h[..8].try_into()?,
        meta,
        payload,
        length,
    }))
}

fn read_header(file: &mut File, end: u64) -> Result<(Header, u64)> {
    let f = read_frame(file, 0, end)?.context("incomplete segment header")?;
    ensure!(
        &f.magic == HEADER && f.payload.is_empty(),
        "wrong segment header/version"
    );
    let header: Header = serde_json::from_slice(&f.meta)?;
    native_mode(&header.codec)?;
    ensure!(
        header
            .schema
            .field_with_name(&header.time_column)?
            .data_type()
            == &DataType::Int64,
        "time schema must be Int64 nanoseconds"
    );
    ensure!(
        header
            .schema
            .field_with_name(&header.symbol_column)?
            .data_type()
            == &DataType::LargeUtf8,
        "symbol schema must be LargeUtf8"
    );
    Ok((header, f.length))
}

/// Creates a new path only. No existing file can be overwritten.
pub struct Writer {
    file: File,
    header: Header,
    blocks: Vec<BlockInfo>,
    poisoned: bool,
}

impl Writer {
    pub fn create(
        path: impl AsRef<Path>,
        schema: SchemaRef,
        time_column: &str,
        symbol_column: &str,
    ) -> Result<Self> {
        Self::create_with_codec(path, schema, time_column, symbol_column, Mode::GqodbBook)
    }

    /// Select an existing native codec while preserving the indexed segment framing.
    pub fn create_with_codec(
        path: impl AsRef<Path>,
        schema: SchemaRef,
        time_column: &str,
        symbol_column: &str,
        mode: Mode,
    ) -> Result<Self> {
        let codec = match mode {
            Mode::Gqodb => "GQOREAL3/GQOADP02",
            Mode::GqodbBook => "GQOREAL4/GQOBOOK1",
            _ => anyhow::bail!("segment writer requires a native tick or book codec"),
        };
        let header = Header {
            schema: schema.as_ref().clone(),
            time_column: time_column.into(),
            symbol_column: symbol_column.into(),
            codec: codec.into(),
        };
        ensure!(
            header.schema.field_with_name(time_column)?.data_type() == &DataType::Int64,
            "invalid time type"
        );
        ensure!(
            header.schema.field_with_name(symbol_column)?.data_type() == &DataType::LargeUtf8,
            "invalid symbol type"
        );
        let meta = serde_json::to_vec(&header)?;
        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(path)?;
        frame(&mut file, HEADER, &meta, &[])?;
        Ok(Self {
            file,
            header,
            blocks: Vec::new(),
            poisoned: false,
        })
    }

    /// Append success is buffered IO, not a durable acknowledgement. Use checkpoint.
    pub fn append(&mut self, batch: &RecordBatch) -> Result<()> {
        ensure!(
            !self.poisoned && self.blocks.len() < MAX_BLOCKS,
            "writer failed or block limit"
        );
        let offset = self.file.stream_position()?;
        let mut entry = info(batch, &self.header, offset, 0)?;
        let payload = real::encode(batch, native_mode(&self.header.codec)?)?;
        // Frame metadata omits the self-referential length; index records the actual length.
        let meta = serde_json::to_vec(&entry)?;
        self.poisoned = true;
        entry.length = frame(&mut self.file, BLOCK, &meta, &payload)?;
        self.blocks.push(entry);
        self.poisoned = false;
        Ok(())
    }

    pub fn checkpoint(&mut self) -> Result<()> {
        ensure!(!self.poisoned, "writer failed");
        if let Err(error) = self.file.sync_all() {
            self.poisoned = true;
            return Err(error.into());
        }
        Ok(())
    }

    /// Seals the index and synchronizes file contents. Directory durability/publication
    /// is deliberately not claimed; an external catalog/atomic rename is future work.
    pub fn finish(mut self) -> Result<()> {
        ensure!(!self.poisoned, "writer failed");
        let index = serde_json::to_vec(&self.blocks)?;
        ensure!(index.len() <= MAX_META, "index too large");
        let offset = self.file.stream_position()?;
        frame(&mut self.file, INDEX, &[], &index)?;
        let mut trailer = END.to_vec();
        trailer.extend_from_slice(&offset.to_le_bytes());
        trailer.extend_from_slice(&crc32fast::hash(&trailer).to_le_bytes());
        self.file.write_all(&trailer)?;
        self.file.sync_all()?;
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct QueryStats {
    pub blocks_read: usize,
    pub bytes_read: u64,
}
pub struct QueryResult {
    pub batches: Vec<RecordBatch>,
    pub stats: QueryStats,
}

/// Optional diagnostic timings; normal reads compile without these clock calls.
#[derive(Default, Debug, Serialize)]
pub struct ReadProfile {
    pub frame_and_metadata_ns: u128,
    pub decode_ns: u128,
    pub decoded_index_check_ns: u128,
}

pub struct Reader {
    file: File,
    header: Header,
    blocks: Vec<BlockInfo>,
    decoder: Decoder,
    pub recovered: bool,
    /// End of verified block prefix (or final file size for a normally opened segment).
    pub valid_bytes: u64,
}

impl Reader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path)?;
        let end = file.metadata()?.len();
        let (header, first) = read_header(&mut file, end)?;
        ensure!(end >= first + 20, "missing footer");
        file.seek(SeekFrom::Start(end - 20))?;
        let mut tail = [0; 20];
        file.read_exact(&mut tail)?;
        ensure!(
            &tail[..8] == END
                && crc32fast::hash(&tail[..16]) == u32::from_le_bytes(tail[16..].try_into()?),
            "invalid footer"
        );
        let offset = u64::from_le_bytes(tail[8..16].try_into()?);
        ensure!(offset >= first && offset < end - 20, "invalid index offset");
        let f = read_frame(&mut file, offset, end - 20)?.context("incomplete index")?;
        ensure!(
            &f.magic == INDEX
                && f.meta.is_empty()
                && f.payload.len() <= MAX_META
                && offset + f.length == end - 20,
            "invalid index frame"
        );
        let blocks: Vec<BlockInfo> = serde_json::from_slice(&f.payload)?;
        ensure!(blocks.len() <= MAX_BLOCKS, "too many blocks");
        let mut pos = first;
        for b in &blocks {
            ensure!(
                b.offset == pos && b.length >= FRAME && b.length <= offset - pos,
                "noncontiguous/out-of-bounds index"
            );
            ensure!(
                b.rows > 0
                    && b.rows <= gqodb_blocks::data::MAX_ROWS
                    && b.min_time <= b.max_time
                    && !b.symbols.is_empty()
                    && b.symbols.windows(2).all(|w| w[0] < w[1]),
                "invalid index statistics"
            );
            pos += b.length;
        }
        ensure!(pos == offset, "index omits blocks");
        Ok(Self {
            file,
            header,
            blocks,
            decoder: Decoder::default(),
            recovered: false,
            valid_bytes: end,
        })
    }

    /// Read-only recovery of complete blocks. Complete corrupt frames fail closed.
    /// Does not truncate, repair in place, or make lost unsynced data durable.
    pub fn recover(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path)?;
        let end = file.metadata()?.len();
        let (header, mut pos) = read_header(&mut file, end)?;
        let mut blocks = Vec::new();
        let mut decoder = Decoder::default();
        while let Some(f) = read_frame(&mut file, pos, end)? {
            if &f.magic == INDEX {
                break;
            }
            ensure!(
                &f.magic == BLOCK && blocks.len() < MAX_BLOCKS,
                "unknown frame or block limit"
            );
            let mut entry: BlockInfo = serde_json::from_slice(&f.meta)?;
            ensure!(entry.length == 0, "invalid frame metadata length");
            entry.length = f.length;
            let batch = decoder.decode(Bytes::from(f.payload), native_mode(&header.codec)?)?;
            ensure!(
                entry == info(&batch, &header, pos, f.length)?,
                "block metadata mismatch"
            );
            blocks.push(entry);
            pos += f.length;
        }
        Ok(Self {
            file,
            header,
            blocks,
            decoder,
            recovered: true,
            valid_bytes: pos,
        })
    }

    pub fn schema(&self) -> SchemaRef {
        Arc::new(self.header.schema.clone())
    }
    pub fn codec(&self) -> Mode {
        // All constructors validate the private header before constructing a reader.
        native_mode(&self.header.codec).expect("validated native codec")
    }
    pub fn index_columns(&self) -> (&str, &str) {
        (&self.header.time_column, &self.header.symbol_column)
    }
    /// Decode/validate one block at a time. Visitor errors stop the scan.
    pub fn visit_blocks(
        &mut self,
        mut visitor: impl FnMut(&RecordBatch) -> Result<()>,
    ) -> Result<()> {
        for i in 0..self.blocks.len() {
            visitor(&self.read_block(i)?)?;
        }
        Ok(())
    }
    pub fn blocks(&self) -> &[BlockInfo] {
        &self.blocks
    }

    pub fn read_block(&mut self, index: usize) -> Result<RecordBatch> {
        self.read_block_impl::<false>(index, &mut ReadProfile::default(), None)
    }

    /// Decode one indexed block with additional caller limits checked before expansion.
    pub fn read_block_bounded(
        &mut self,
        index: usize,
        max_rows: usize,
        max_decoded_bytes: usize,
    ) -> Result<RecordBatch> {
        ensure!(
            (1..=gqodb_blocks::data::MAX_ROWS).contains(&max_rows)
                && (1..=MAX_PAYLOAD).contains(&max_decoded_bytes),
            "invalid block decode budget"
        );
        self.read_block_impl::<false>(
            index,
            &mut ReadProfile::default(),
            Some((max_rows, max_decoded_bytes)),
        )
    }

    pub fn profile_read_all(&mut self) -> Result<(Vec<RecordBatch>, ReadProfile)> {
        let mut profile = ReadProfile::default();
        let batches = (0..self.blocks.len())
            .map(|i| self.read_block_impl::<true>(i, &mut profile, None))
            .collect::<Result<Vec<_>>>()?;
        Ok((batches, profile))
    }

    fn read_block_impl<const PROFILE: bool>(
        &mut self,
        index: usize,
        profile: &mut ReadProfile,
        limits: Option<(usize, usize)>,
    ) -> Result<RecordBatch> {
        let begin = if PROFILE { Some(Instant::now()) } else { None };
        let b = self
            .blocks
            .get(index)
            .context("block index out of bounds")?;
        let f = read_frame(&mut self.file, b.offset, b.offset + b.length)?
            .context("incomplete block")?;
        ensure!(
            &f.magic == BLOCK && f.length == b.length,
            "block frame mismatch"
        );
        let mut entry: BlockInfo = serde_json::from_slice(&f.meta)?;
        ensure!(entry.length == 0, "invalid frame metadata length");
        entry.length = f.length;
        ensure!(&entry == b, "index/frame mismatch");
        if let Some((max_rows, max_bytes)) = limits {
            bounded_expansion(
                &f.payload,
                b.rows,
                max_rows,
                max_bytes,
                native_mode(&self.header.codec)?,
            )?;
        }
        if let Some(begin) = begin {
            profile.frame_and_metadata_ns += begin.elapsed().as_nanos();
        }
        let begin = if PROFILE { Some(Instant::now()) } else { None };
        let batch = self
            .decoder
            .decode(Bytes::from(f.payload), native_mode(&self.header.codec)?)?;
        if let Some(begin) = begin {
            profile.decode_ns += begin.elapsed().as_nanos();
        }
        let begin = if PROFILE { Some(Instant::now()) } else { None };
        ensure!(
            info(&batch, &self.header, b.offset, b.length)? == *b,
            "decoded index mismatch"
        );
        if let Some(begin) = begin {
            profile.decoded_index_check_ns += begin.elapsed().as_nanos();
        }
        Ok(batch)
    }

    pub fn read_all(&mut self) -> Result<Vec<RecordBatch>> {
        (0..self.blocks.len()).map(|i| self.read_block(i)).collect()
    }

    /// Inclusive time range, preserving original row order, including late events.
    pub fn query(&mut self, symbol: &str, start: i64, end: i64) -> Result<QueryResult> {
        ensure!(start <= end, "reversed query range");
        let mut result = QueryResult {
            batches: Vec::new(),
            stats: QueryStats::default(),
        };
        for i in 0..self.blocks.len() {
            let b = &self.blocks[i];
            if b.max_time < start || b.min_time > end || !b.symbols.iter().any(|s| s == symbol) {
                continue;
            }
            result.stats.blocks_read += 1;
            result.stats.bytes_read += b.length;
            let batch = self.read_block(i)?;
            let (times, symbols) = fields(&batch, &self.header)?;
            let mask = BooleanArray::from(
                (0..batch.num_rows())
                    .map(|j| {
                        times.value(j) >= start
                            && times.value(j) <= end
                            && symbols.value(j) == symbol
                    })
                    .collect::<Vec<_>>(),
            );
            let selected = filter_record_batch(&batch, &mask)?;
            if selected.num_rows() > 0 {
                result.batches.push(selected);
            }
        }
        Ok(result)
    }
}
