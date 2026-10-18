//! Structure-aware builder of `.gqodb.ob` segments from a stream of fuzz bytes.
//!
//! It re-implements the published container layout (see the store README,
//! "Container v1") with correct checksums at every layer, then, guided by the
//! fuzz input, injects semantic inconsistencies: index entries that lie about
//! offsets, lengths, rows, time ranges and symbols, frames with wrong magic,
//! payloads with a flipped byte (checksums repaired), codec mismatches, missing
//! or misplaced footers. When nothing was injected (`honest`), the reader MUST
//! accept the file and return exactly `batches`.
use crate::{arb, fixup};
use arbitrary::{Result, Unstructured};
use arrow_array::{Array, Int64Array, LargeStringArray, RecordBatch};
use gqodb_blocks::real::{self, Mode};
use serde_json::{Value, json};

pub struct Built {
    pub bytes: Vec<u8>,
    pub batches: Vec<RecordBatch>,
    pub honest: bool,
    pub book: bool,
}

pub fn frame(magic: &[u8; 8], meta: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut h = magic.to_vec();
    h.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    h.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    let mut hash = crc32fast::Hasher::new();
    hash.update(meta);
    hash.update(payload);
    h.extend_from_slice(&hash.finalize().to_le_bytes());
    h.extend_from_slice(&crc32fast::hash(&h).to_le_bytes());
    h.extend_from_slice(meta);
    h.extend_from_slice(payload);
    h
}

struct Ctx<'a, 'b> {
    u: &'a mut Unstructured<'b>,
    honest: bool,
}

impl Ctx<'_, '_> {
    /// True with probability ~1/`n`; any `true` marks the file as no longer honest.
    fn lie(&mut self, n: u32) -> Result<bool> {
        let l = self.u.ratio(1, n)?;
        self.honest &= !l;
        Ok(l)
    }
}

fn truth(batch: &RecordBatch, offset: u64, length: u64) -> Value {
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
    let mut symbols: Vec<&str> = s.iter().flatten().collect();
    symbols.sort_unstable();
    symbols.dedup();
    json!({
        "offset": offset, "length": length, "rows": batch.num_rows(),
        "min_time": t.values().iter().min(), "max_time": t.values().iter().max(),
        "symbols": symbols,
    })
}

fn perturb(c: &mut Ctx, entry: &mut Value) -> Result<()> {
    if c.lie(10)? {
        let key =
            ["offset", "length", "rows", "min_time", "max_time"][c.u.int_in_range(0..=4usize)?];
        let delta: i64 =
            [-1, 1, 24, i64::MAX, i64::MIN, 4_000_000_000][c.u.int_in_range(0..=5usize)?];
        let old = entry[key].as_i64().unwrap_or(0);
        entry[key] = json!(old.wrapping_add(delta));
    }
    if c.lie(14)? {
        entry["symbols"] = match c.u.int_in_range(0..=3)? {
            0 => json!([]),
            1 => json!(["b", "a"]),
            2 => json!(["a", "a"]),
            _ => json!(["ETH", "BTC", "x"]),
        };
    }
    Ok(())
}

pub fn build(u: &mut Unstructured) -> Result<Built> {
    let batches = arb::store_batches(u)?;
    let book = u.arbitrary::<bool>()?;
    let mut c = Ctx { u, honest: true };
    let mode = if book { Mode::GqodbBook } else { Mode::Gqodb };
    let codec = if book {
        "GQOREAL4/GQOBOOK1"
    } else {
        "GQOREAL3/GQOADP02"
    };

    // Header frame.
    let mut header = json!({
        "schema": serde_json::to_value(batches[0].schema().as_ref()).expect("schema json"),
        "time_column": "t", "symbol_column": "sym", "codec": codec,
    });
    if c.lie(20)? {
        match c.u.int_in_range(0..=3)? {
            0 => header["time_column"] = json!("sym"),
            1 => header["symbol_column"] = json!("t"),
            2 => header["codec"] = json!("GQOREAL3/GQOBOOK1"),
            _ => header["time_column"] = json!("missing"),
        }
    }
    let mut out = frame(b"GQOBHDR1", &serde_json::to_vec(&header).unwrap(), &[]);
    let first = out.len() as u64;

    // Block frames.
    let mut index = Vec::new();
    for batch in &batches {
        let offset = out.len() as u64;
        let mut payload = real::encode(batch, mode).expect("valid batch encodes");
        if c.lie(12)? {
            // Payload from the other native codec, or one flipped byte with repaired checksums.
            if c.u.arbitrary::<bool>()? {
                let other = if book { Mode::Gqodb } else { Mode::GqodbBook };
                payload = real::encode(batch, other).expect("encodes");
            } else if c.u.ratio(1, 4)? {
                // Tiny file, huge claimed string expansion (dictionary string x rows).
                let n = payload.len();
                payload.resize(n + 60_000, 0);
                let size = fixup::grow_real_meta(&mut payload, n, 60_000, 0);
                payload.truncate(size);
                fixup::real(&mut payload);
            } else {
                let i = c.u.int_in_range(0..=payload.len() - 1)?;
                payload[i] ^= c.u.int_in_range(1..=255u8)?;
                if c.u.ratio(7, 8)? {
                    fixup::real(&mut payload);
                }
            }
        }
        let length = 24 + payload.len() as u64; // metadata length is patched below
        let mut entry = truth(batch, offset, length);
        let mut meta_entry = entry.clone();
        meta_entry["length"] = json!(0);
        perturb(&mut c, &mut meta_entry)?;
        let mut meta = serde_json::to_vec(&meta_entry).unwrap();
        let length = 24 + meta.len() as u64 + payload.len() as u64;
        entry["length"] = json!(length);
        if c.lie(25)? {
            meta.clear();
        }
        let magic = if c.lie(30)? { b"GQOBBLK2" } else { b"GQOBBLK1" };
        let mut f = frame(magic, &meta, &payload);
        if c.lie(40)? {
            let keep = c.u.int_in_range(0..=f.len() - 1)?;
            f.truncate(keep);
        }
        out.extend_from_slice(&f);
        perturb(&mut c, &mut entry)?;
        index.push(entry);
    }
    if c.lie(25)? && !index.is_empty() {
        index.remove(c.u.int_in_range(0..=index.len() - 1)?);
    }
    if c.lie(30)? {
        index.push(index.first().cloned().unwrap_or(json!({})));
    }

    // Index + trailer.
    let index_offset = out.len() as u64;
    let body = serde_json::to_vec(&index).unwrap();
    let magic = if c.lie(30)? { b"GQOBIDX2" } else { b"GQOBIDX1" };
    if !c.lie(20)? {
        out.extend_from_slice(&frame(magic, &[], &body));
        let mut trailer = b"GQOBEND1".to_vec();
        let at = if c.lie(20)? {
            index_offset
                .wrapping_add(c.u.int_in_range(0..=64u64)?)
                .wrapping_sub(32)
        } else {
            index_offset
        };
        trailer.extend_from_slice(&at.to_le_bytes());
        trailer.extend_from_slice(&crc32fast::hash(&trailer).to_le_bytes());
        if !c.lie(30)? {
            out.extend_from_slice(&trailer);
        }
    }
    if c.lie(30)? {
        let cut =
            c.u.int_in_range(0..=out.len().saturating_sub(first as usize))?;
        out.truncate(out.len() - cut);
    }
    if c.lie(40)? {
        out.extend_from_slice(&[0xAB; 7]);
    }
    let honest = c.honest;
    Ok(Built {
        bytes: out,
        batches,
        honest,
        book,
    })
}
