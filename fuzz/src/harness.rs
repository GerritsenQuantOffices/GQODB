//! Bodies of the fuzz targets. A target is a thin `fuzz_target!` wrapper around one of
//! these functions, so the same checks can be replayed from tests and examples.
//!
//! Oracle policy: a PANIC, abort, hang or runaway allocation is always a finding.
//! In addition, every accepted input is re-checked against a second computation:
//! accepted data must re-encode and decode to the identical value, readers must
//! agree with a brute-force scan, and valid generated data must round-trip exactly.
use crate::{arb, scratch, segment};
use arbitrary::Unstructured;
use arrow_array::{Array, Int64Array, LargeStringArray, RecordBatch};
use bytes::Bytes;
use gqodb_blocks::{
    Codec, adaptive, batch as blocks_batch, block,
    real::{self, Decoder, Mode},
};
use gqodb_codec::Family;
use gqodb_store::{Reader, Writer};

/// Rows above which the (slow) re-encode check is skipped; the decode itself is always run.
const RECHECK_ROWS: usize = 4096;

fn same(a: &anyhow::Result<RecordBatch>, b: &anyhow::Result<RecordBatch>) -> bool {
    match (a, b) {
        (Ok(a), Ok(b)) => real::equal(a, b),
        (Err(_), Err(_)) => true,
        _ => false,
    }
}

/// KNOWN FINDING, OWNER DECISION PENDING (see docs/FUZZING.md, "Open finding"): `real::decode`,
/// `Reader::read_block`, `query` and `recover` allocate `rows x longest-dictionary-string` bytes for
/// a LargeUtf8 column before checking it against any bound, so a ~60 KB input can request tens of
/// GB. The checked layers (`gqodb-codec` preflight, `Reader::read_block_bounded`) refuse it.
/// Until the owner decides on a decoded-size cap for the unchecked layer, the targets that call the
/// unchecked API skip inputs whose estimate (same formula as the codec preflight) exceeds 128 MiB,
/// so that this one known finding does not mask others.
pub fn known_string_bomb(block: &[u8]) -> bool {
    const LIMIT: usize = 128 * 1024 * 1024;
    if block.len() < 32 || !(block.starts_with(b"GQOREAL3") || block.starts_with(b"GQOREAL4")) {
        return false;
    }
    let n = u32::from_le_bytes(block[8..12].try_into().unwrap()) as usize;
    let Some(meta) = block.get(12..12usize.saturating_add(n)) else {
        return false;
    };
    let payload = &block[12 + n..];
    let rows_at = if block.starts_with(b"GQOREAL3") {
        12
    } else {
        8
    };
    let Some(rows) = payload.get(rows_at..rows_at + 4) else {
        return false;
    };
    let rows = u32::from_le_bytes(rows.try_into().unwrap()) as usize;
    let Ok(layout) = serde_json::from_slice::<serde_json::Value>(meta) else {
        return false;
    };
    let widest: usize = layout["columns"]
        .as_array()
        .map(|cols| {
            cols.iter()
                .map(|c| {
                    c["dictionary"].as_array().map_or(0, |d| {
                        d.iter()
                            .filter_map(|s| s.as_str())
                            .map(str::len)
                            .max()
                            .unwrap_or(0)
                    })
                })
                .map(|w| w.saturating_mul(rows))
                .fold(0usize, usize::saturating_add)
        })
        .unwrap_or(0);
    widest > LIMIT
}

/// `known_string_bomb` over every block frame of a segment image (walks frames, trusts nothing).
fn segment_has_known_string_bomb(data: &[u8]) -> bool {
    let mut pos = 0usize;
    while data.len().saturating_sub(pos) >= 24 {
        let m = u32::from_le_bytes(data[pos + 8..pos + 12].try_into().unwrap()) as usize;
        let p = u32::from_le_bytes(data[pos + 12..pos + 16].try_into().unwrap()) as usize;
        let body = pos + 24;
        let Some(end) = body
            .checked_add(m)
            .and_then(|x| x.checked_add(p))
            .filter(|&e| e <= data.len())
        else {
            return false;
        };
        if &data[pos..pos + 8] == b"GQOBBLK1" && known_string_bomb(&data[body + m..end]) {
            return true;
        }
        pos = end;
    }
    false
}

pub fn block_decode(data: &[u8]) {
    if let Ok(cols) = block::decode(data) {
        cols.validate()
            .expect("accepted block must satisfy the schema contract");
        if cols.rows() <= RECHECK_ROWS {
            for layout in [
                block::Layout::Raw,
                block::Layout::Varint,
                block::Layout::Bitpack,
            ] {
                let again = block::encode(&cols, layout).expect("accepted data must re-encode");
                assert_eq!(
                    block::decode(&again).expect("re-decode"),
                    cols,
                    "{layout:?}"
                );
            }
        }
    }
    for codec in [
        Codec::ColumnLz4,
        Codec::DeltaVarintLz4,
        Codec::DeltaBitpackLz4,
    ] {
        let _ = codec.framing_bytes(Bytes::copy_from_slice(data));
    }
}

pub fn adaptive_decode(data: &[u8]) {
    if let Ok(cols) = adaptive::decode(data) {
        cols.validate()
            .expect("accepted block must satisfy the schema contract");
        if cols.rows() <= RECHECK_ROWS {
            for lz4 in [false, true] {
                let again = adaptive::encode(&cols, lz4).expect("accepted data must re-encode");
                assert_eq!(
                    adaptive::decode(&again).expect("re-decode"),
                    cols,
                    "lz4={lz4}"
                );
            }
        }
    }
    for codec in [Codec::AdaptiveSimd, Codec::AdaptiveSimdLz4] {
        let a = codec.decode(Bytes::copy_from_slice(data));
        assert_eq!(
            a.is_ok(),
            adaptive::decode(data).is_ok(),
            "facade and module disagree"
        );
        let _ = codec.framing_bytes(Bytes::copy_from_slice(data));
    }
}

thread_local! {
    static REUSED: std::cell::RefCell<Decoder> = std::cell::RefCell::new(Decoder::default());
}

/// Native real-block decoder (`GQOREAL3` tick or `GQOREAL4` book).
pub fn real_decode(mode: Mode, data: &[u8]) {
    if known_string_bomb(data) {
        return;
    }
    let input = Bytes::copy_from_slice(data);
    let fresh = real::decode(input.clone(), mode);
    // Differential: a Decoder whose scratch buffer is dirty from earlier inputs must agree.
    let reused = REUSED.with(|d| d.borrow_mut().decode(input.clone(), mode));
    assert!(
        same(&fresh, &reused),
        "reused scratch buffer changed the outcome"
    );
    if let Ok(batch) = &fresh
        && batch.num_rows() <= RECHECK_ROWS
        && let Ok(bytes) = real::encode(batch, mode)
    {
        let back = real::decode(Bytes::from(bytes), mode).expect("re-encoded block must decode");
        assert!(
            real::equal(batch, &back),
            "decode(encode(decode(x))) != decode(x)"
        );
    }
    // The other native mode must reject (magic differs) without panicking.
    let other = if matches!(mode, Mode::Gqodb) {
        Mode::GqodbBook
    } else {
        Mode::Gqodb
    };
    let _ = real::decode(input.clone(), other);
    // Multi-worker path must agree with the sequential one.
    if data.len().is_multiple_of(16) {
        let blocks = [input.clone(), input];
        let seq = blocks_batch::Options { workers: 1 }.decode(&blocks, mode);
        let par = blocks_batch::Options { workers: 2 }.decode(&blocks, mode);
        match (seq, par) {
            (Ok(a), Ok(b)) => assert!(a.iter().zip(&b).all(|(x, y)| real::equal(x, y))),
            (Err(_), Err(_)) => {}
            _ => panic!("sequential and parallel batch decode disagree"),
        }
    }
}

pub fn parquet_decode(data: &[u8]) {
    let input = Bytes::copy_from_slice(data);
    for codec in Codec::ALL.into_iter().filter(|c| c.is_parquet()) {
        let _ = codec.framing_bytes(input.clone());
        if let Ok(cols) = codec.decode(input.clone()) {
            cols.validate()
                .expect("accepted block must satisfy the schema contract");
            if cols.rows() <= RECHECK_ROWS {
                let again = codec.encode(&cols).expect("accepted data must re-encode");
                assert_eq!(
                    codec.decode(Bytes::from(again)).expect("re-decode"),
                    cols,
                    "{codec:?}"
                );
            }
        }
    }
    for mode in [
        Mode::ParquetLz4,
        Mode::ParquetDictionaryLz4,
        Mode::ParquetSnappy,
        Mode::ParquetBrotli,
        Mode::ParquetNormalizedLz4,
    ] {
        if let Ok(batch) = real::decode(input.clone(), mode)
            && batch.num_rows() <= RECHECK_ROWS
            && let Ok(bytes) = real::encode(&batch, mode)
        {
            let back =
                real::decode(Bytes::from(bytes), mode).expect("re-encoded block must decode");
            assert!(real::equal(&batch, &back), "{}", mode.name());
        }
    }
}

fn index_fields(reader: &Reader, batch: &RecordBatch) -> (Vec<i64>, Vec<String>) {
    let (t, s) = reader.index_columns();
    let t = batch
        .column_by_name(t)
        .unwrap()
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap();
    let s = batch
        .column_by_name(s)
        .unwrap()
        .as_any()
        .downcast_ref::<LargeStringArray>()
        .unwrap();
    (
        t.values().to_vec(),
        s.iter().map(|v| v.unwrap_or("").to_string()).collect(),
    )
}

/// Exercise every public read operation of an opened reader; returns decoded blocks, if all decode.
fn exercise(reader: &mut Reader, data: &[u8]) -> Option<Vec<RecordBatch>> {
    let _ = (reader.schema(), reader.codec(), reader.index_columns());
    let blocks = reader.blocks().to_vec();
    // The documented bounded path first: it must never allocate beyond its budget.
    for i in 0..blocks.len().min(6) {
        let _ = reader.read_block_bounded(i, gqodb_blocks::data::MAX_ROWS, 64 * 1024 * 1024);
    }
    if segment_has_known_string_bomb(data) {
        return None;
    }
    let _ = reader.read_block(blocks.len());
    let all: Option<Vec<RecordBatch>> = (0..blocks.len().min(12))
        .map(|i| reader.read_block(i).ok())
        .collect();
    let _ = reader.profile_read_all();
    let (lo, hi) = blocks.iter().fold((i64::MAX, i64::MIN), |(a, b), x| {
        (a.min(x.min_time), b.max(x.max_time))
    });
    let symbol = blocks
        .first()
        .and_then(|b| b.symbols.first())
        .cloned()
        .unwrap_or_default();
    let _ = reader.query("\u{0}absent", i64::MIN, i64::MAX);
    let _ = reader.query(&symbol, hi, lo);
    if let Ok(result) = reader.query(&symbol, lo, hi) {
        // Second computation: scan every block and filter by hand.
        if let (Some(all), true) = (&all, blocks.len() <= 12) {
            let mut want = Vec::new();
            for b in all {
                let (t, s) = index_fields(reader, b);
                want.extend(
                    t.iter()
                        .zip(&s)
                        .filter(|(t, s)| **t >= lo && **t <= hi && **s == symbol)
                        .map(|(t, _)| *t),
                );
            }
            let got: Vec<i64> = result
                .batches
                .iter()
                .flat_map(|b| index_fields(reader, b).0)
                .collect();
            assert_eq!(got, want, "indexed query disagrees with a full scan");
        }
    }
    all
}

pub fn store_reader(data: &[u8]) {
    let path = scratch::write("reader.gqodb.ob", data);
    if let Ok(mut reader) = Reader::open(&path) {
        exercise(&mut reader, data);
    }
}

pub fn store_recover(data: &[u8]) {
    if segment_has_known_string_bomb(data) {
        return;
    }
    let path = scratch::write("recover.gqodb.ob", data);
    let Ok(mut recovered) = Reader::recover(&path) else {
        return;
    };
    assert!(
        recovered.valid_bytes <= data.len() as u64,
        "valid prefix exceeds the file"
    );
    let blocks = recovered.blocks().to_vec();
    // recover() decoded and cross-checked every block, so reading each one again must succeed.
    for i in 0..blocks.len() {
        recovered
            .read_block(i)
            .expect("a recovered block must be readable");
    }
    // Idempotence: recovering exactly the verified prefix yields the same blocks.
    let prefix = scratch::write("prefix.gqodb.ob", &data[..recovered.valid_bytes as usize]);
    let again = Reader::recover(&prefix).expect("verified prefix must recover");
    assert_eq!(again.blocks(), &blocks[..], "recovery is not idempotent");
    assert_eq!(again.valid_bytes, recovered.valid_bytes);
    // A normally opened file must agree with recovery when both succeed and the index is truthful.
    if let Ok(mut opened) = Reader::open(&path)
        && opened.read_all().is_ok()
    {
        assert_eq!(
            opened.blocks(),
            &blocks[..],
            "open and recover disagree about the blocks"
        );
    }
}

pub fn store_structured(data: &[u8]) {
    let Ok(built) = segment::build(&mut Unstructured::new(data)) else {
        return;
    };
    let path = scratch::write("structured.gqodb.ob", &built.bytes);
    let opened = Reader::open(&path);
    if built.honest {
        let mut reader = opened.unwrap_or_else(|e| panic!("honest segment refused: {e:#}"));
        let all = exercise(&mut reader, &built.bytes).expect("honest segment must decode");
        assert_eq!(all.len(), built.batches.len().min(12));
        for (got, want) in all.iter().zip(&built.batches) {
            assert!(real::equal(got, want), "block round trip mismatch");
        }
        let rec = Reader::recover(&path)
            .unwrap_or_else(|e| panic!("honest segment not recoverable: {e:#}"));
        assert_eq!(rec.blocks(), reader.blocks());
    } else {
        if segment_has_known_string_bomb(&built.bytes) {
            return;
        }
        if let Ok(mut reader) = opened {
            exercise(&mut reader, &built.bytes);
        }
        if let Ok(mut rec) = Reader::recover(&path) {
            for i in 0..rec.blocks().len() {
                rec.read_block(i)
                    .expect("a recovered block must be readable");
            }
        }
    }
}

fn family_of(data: &[u8]) -> Option<Family> {
    match gqodb_codec::detect(data).ok()? {
        gqodb_codec::Format::AdaptiveBlockV3 => Some(Family::Tick),
        gqodb_codec::Format::BookBlockV4 => Some(Family::Orderbook),
        gqodb_codec::Format::NativeSegmentV1 => None,
    }
}

pub fn codec_facade(data: &[u8]) {
    let input = Bytes::copy_from_slice(data);
    for expected in [None, Some(Family::Tick), Some(Family::Orderbook)] {
        if let Ok(batch) = gqodb_codec::decode(input.clone(), expected) {
            let family = expected
                .or(family_of(data))
                .expect("decoded block has a family");
            if batch.num_rows() <= RECHECK_ROWS
                && let Ok(bytes) = gqodb_codec::encode(&batch, family)
            {
                let back =
                    gqodb_codec::decode(Bytes::from(bytes), Some(family)).expect("re-decode");
                assert!(
                    real::equal(&batch, &back),
                    "decode(encode(decode(x))) != decode(x)"
                );
            }
        }
    }
    let path = scratch::write("facade.bin", data);
    for expected in [None, Some(Family::Tick), Some(Family::Orderbook)] {
        if let Ok(report) = gqodb_codec::inspect(&path, expected) {
            assert_eq!(report.file_bytes, data.len() as u64);
            assert!(report.fully_decoded);
        }
    }
}

pub fn codec_schema(data: &[u8]) {
    if let Ok(schema) = serde_json::from_slice::<arrow_schema::Schema>(data) {
        for family in [Family::Tick, Family::Orderbook] {
            let _ = gqodb_codec::validate_schema(&schema, family);
        }
    }
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = Family::parse(s);
    }
}

pub fn roundtrip_columns(data: &[u8]) {
    let Ok(cols) = arb::columns(&mut Unstructured::new(data)) else {
        return;
    };
    // The five Parquet variants cost ~100x the native codecs: run one (rotating by input hash)
    // per execution, and all of them for every eighth input length.
    let pick = data.iter().fold(0usize, |a, &b| {
        a.wrapping_mul(31).wrapping_add(usize::from(b))
    });
    let mut parquet_seen = 0;
    for codec in Codec::ALL {
        if codec.is_parquet() {
            parquet_seen += 1;
            if !data.len().is_multiple_of(8) && pick % 5 != parquet_seen - 1 {
                continue;
            }
        }
        let encoded = codec
            .encode(&cols)
            .unwrap_or_else(|e| panic!("{codec:?} refused valid data: {e:#}"));
        let back = codec
            .decode(Bytes::from(encoded.clone()))
            .unwrap_or_else(|e| panic!("{codec:?}: {e:#}"));
        assert_eq!(back, cols, "{codec:?} is not bit exact");
        let _ = codec.framing_bytes(Bytes::from(encoded));
    }
}

pub fn roundtrip_real(data: &[u8]) {
    let Ok(batch) = arb::batch(&mut Unstructured::new(data), 0) else {
        return;
    };
    let modes: &[Mode] = if batch.num_rows() == 0 {
        &[Mode::Gqodb, Mode::GqodbBook]
    } else {
        &[
            Mode::Gqodb,
            Mode::GqodbBook,
            Mode::ParquetLz4,
            Mode::ParquetDictionaryLz4,
            Mode::ParquetSnappy,
            Mode::ParquetBrotli,
            Mode::ParquetNormalizedLz4,
        ]
    };
    let pick = data.iter().fold(0usize, |a, &b| {
        a.wrapping_mul(31).wrapping_add(usize::from(b))
    });
    for (i, &mode) in modes.iter().enumerate() {
        // Parquet modes are ~100x slower than the native ones: rotate one per execution, all every 8th length.
        if i >= 2 && !data.len().is_multiple_of(8) && pick % 5 != i - 2 {
            continue;
        }
        let encoded = real::encode(&batch, mode)
            .unwrap_or_else(|e| panic!("{} refused valid data: {e:#}", mode.name()));
        let back = real::decode(Bytes::from(encoded.clone()), mode)
            .unwrap_or_else(|e| panic!("{}: {e:#}", mode.name()));
        assert!(
            real::equal(&batch, &back),
            "{} is not bit exact",
            mode.name()
        );
        let pair = [Bytes::from(encoded.clone()), Bytes::from(encoded)];
        let many = blocks_batch::Options { workers: 2 }
            .decode(&pair, mode)
            .expect("parallel decode");
        assert!(many.iter().all(|b| real::equal(&batch, b)));
    }
}

pub fn roundtrip_codec(data: &[u8]) {
    let mut u = Unstructured::new(data);
    let Ok((orderbook, quote)) =
        (|| Ok::<_, arbitrary::Error>((u.arbitrary::<bool>()?, u.arbitrary::<bool>()?)))()
    else {
        return;
    };
    let family = if orderbook {
        Family::Orderbook
    } else {
        Family::Tick
    };
    let Ok(batch) = arb::normalized(&mut u, orderbook, quote) else {
        return;
    };
    let encoded = gqodb_codec::encode(&batch, family)
        .unwrap_or_else(|e| panic!("valid batch refused: {e:#}"));
    for expected in [None, Some(family)] {
        let back = gqodb_codec::decode(Bytes::from(encoded.clone()), expected).expect("decode");
        assert!(
            real::equal(&batch, &back),
            "codec round trip is not bit exact"
        );
    }
    let path = scratch::fresh("codec.gqodb.ob");
    gqodb_codec::write_segment_with_family(&path, &[batch.clone(), batch.clone()], family)
        .unwrap_or_else(|e| panic!("valid segment refused: {e:#}"));
    let report = gqodb_codec::inspect(&path, Some(family))
        .unwrap_or_else(|e| panic!("own segment refused: {e:#}"));
    assert_eq!((report.blocks, report.rows), (2, 2 * batch.num_rows()));
    assert!(report.normalized_contract_valid);
    let mut reader = Reader::open(&path).expect("open own segment");
    for got in reader.read_all().expect("read own segment") {
        assert!(real::equal(&got, &batch));
    }
    let _ = std::fs::remove_file(&path);
}

pub fn roundtrip_store(data: &[u8]) {
    let mut u = Unstructured::new(data);
    let Ok(batches) = arb::store_batches(&mut u) else {
        return;
    };
    let Ok(book) = u.arbitrary::<bool>() else {
        return;
    };
    let mode = if book { Mode::GqodbBook } else { Mode::Gqodb };
    let path = scratch::fresh("store.gqodb.ob");
    let mut writer =
        Writer::create_with_codec(&path, batches[0].schema(), "t", "sym", mode).expect("create");
    let mut ends = Vec::new();
    for b in &batches {
        writer.append(b).expect("append");
        ends.push(std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0));
    }
    writer.finish().expect("finish");
    let mut reader = Reader::open(&path).unwrap_or_else(|e| panic!("own segment refused: {e:#}"));
    let all = reader.read_all().expect("read_all");
    assert!(
        all.iter().zip(&batches).all(|(a, b)| real::equal(a, b)),
        "segment round trip is not bit exact"
    );
    // Indexed range query against a brute-force scan of the input batches.
    let Ok(sym) = u.int_in_range(0..=arb::SYMBOLS.len()) else {
        return;
    };
    let symbol = arb::SYMBOLS.get(sym).copied().unwrap_or("absent");
    let (a, b): (i64, i64) = (
        u.arbitrary().unwrap_or(i64::MIN),
        u.arbitrary().unwrap_or(i64::MAX),
    );
    let (lo, hi) = (a.min(b), a.max(b));
    let result = reader.query(symbol, lo, hi).expect("query");
    let mut want = Vec::new();
    for batch in &batches {
        let t = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        let s = batch
            .column(1)
            .as_any()
            .downcast_ref::<LargeStringArray>()
            .unwrap();
        let idx = batch
            .column(2)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        for j in 0..batch.num_rows() {
            if t.value(j) >= lo && t.value(j) <= hi && s.value(j) == symbol {
                want.push(idx.value(j));
            }
        }
    }
    let got: Vec<i64> = result
        .batches
        .iter()
        .flat_map(|b| {
            b.column(2)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap()
                .values()
                .to_vec()
        })
        .collect();
    assert_eq!(got, want, "query result differs from brute force");
    // Prefix recovery: cut the file anywhere after the header; exactly the complete blocks survive.
    let bytes = std::fs::read(&path).expect("read back");
    let first = reader.blocks()[0].offset;
    let cut = u
        .int_in_range(first as usize..=bytes.len())
        .unwrap_or(bytes.len());
    let cut_path = scratch::fresh("cut.gqodb.ob");
    std::fs::write(&cut_path, &bytes[..cut]).expect("write cut");
    let rec = Reader::recover(&cut_path)
        .unwrap_or_else(|e| panic!("cut at {cut} not recoverable: {e:#}"));
    let complete = ends.iter().filter(|&&e| e as usize <= cut).count();
    assert_eq!(
        rec.blocks(),
        &reader.blocks()[..complete],
        "prefix recovery returned the wrong blocks"
    );
    let _ = (std::fs::remove_file(&path), std::fs::remove_file(&cut_path));
}
