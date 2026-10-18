//! Recompute the checksum layers of the native formats after a mutation.
//! All functions are total: they never panic on short or inconsistent buffers.

const FRAME: usize = 24;

fn u32_at(buf: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(buf.get(at..at.checked_add(4)?)?.try_into().ok()?) as usize)
}

/// Last four bytes = CRC32 of everything before them (block, adaptive, real, parquet envelope).
pub fn tail_crc(buf: &mut [u8]) {
    if let Some(n) = buf.len().checked_sub(4) {
        let crc = crc32fast::hash(&buf[..n]);
        buf[n..].copy_from_slice(&crc.to_le_bytes());
    }
}

/// `GQOREAL3`/`GQOREAL4` block: inner native payload checksum, then the outer one.
pub fn real(buf: &mut [u8]) {
    if buf.len() >= 20 {
        let end = buf.len() - 4;
        if let Some(meta) = u32_at(buf, 8)
            && let Some(start) = 12usize.checked_add(meta)
            && start <= end
        {
            tail_crc(&mut buf[start..end]);
        }
    }
    tail_crc(buf);
}

/// Indexed segment: every frame (header CRC, body CRC, inner block CRCs) and the 20-byte trailer.
pub fn segment(buf: &mut [u8]) {
    let mut pos = 0usize;
    while buf.len().saturating_sub(pos) >= FRAME {
        let (Some(m), Some(p)) = (u32_at(buf, pos + 8), u32_at(buf, pos + 12)) else {
            break;
        };
        let body = pos + FRAME;
        let end = body.saturating_add(m).saturating_add(p);
        if end <= buf.len() {
            if &buf[pos..pos + 8] == b"GQOBBLK1" {
                real(&mut buf[body + m..end]);
            }
            let crc = crc32fast::hash(&buf[body..end]);
            buf[pos + 16..pos + 20].copy_from_slice(&crc.to_le_bytes());
        }
        let crc = crc32fast::hash(&buf[pos..pos + 20]);
        buf[pos + 20..pos + 24].copy_from_slice(&crc.to_le_bytes());
        if end > buf.len() {
            break;
        }
        pos = end;
    }
    if buf.len() >= 20 {
        let at = buf.len() - 20;
        let crc = crc32fast::hash(&buf[at..at + 16]);
        buf[at + 16..].copy_from_slice(&crc.to_le_bytes());
    }
}

/// Dispatch on the magic of the (possibly mutated) buffer.
pub fn auto(buf: &mut [u8]) {
    match buf.get(..8) {
        Some(b"GQOBHDR1") => segment(buf),
        Some(b"GQOREAL3" | b"GQOREAL4") => real(buf),
        Some(b"GQOBLK01" | b"GQOADP02") => tail_crc(buf),
        Some(m) if &m[..4] == b"PAR1" => tail_crc(buf),
        _ => {}
    }
}

/// Insert `extra` filler bytes into the JSON metadata of a `GQOREAL3`/`GQOREAL4` block and keep
/// the 4-byte metadata length consistent (checksums are repaired by the caller). The filler goes
/// inside the first dictionary string when the block has one, so the result is still valid JSON
/// whose decoded string expansion is far larger than the input. Returns the new size, or `size`
/// unchanged when the buffer is not such a block or has no room.
pub fn grow_real_meta(data: &mut [u8], size: usize, extra: usize, which: usize) -> usize {
    const MARKER: &[u8] = b"\"dictionary\":[\"";
    if size < 20 || !(data.starts_with(b"GQOREAL3") || data.starts_with(b"GQOREAL4")) {
        return size;
    }
    let Some(n) = u32_at(&data[..size], 8) else {
        return size;
    };
    let Some(meta_end) = 12usize
        .checked_add(n)
        .filter(|&e| e <= size.saturating_sub(4))
    else {
        return size;
    };
    let extra = extra.min(data.len() - size);
    if extra == 0 {
        return size;
    }
    let meta = &data[12..meta_end];
    let hits: Vec<usize> = (0..meta.len().saturating_sub(MARKER.len()))
        .filter(|&i| &meta[i..i + MARKER.len()] == MARKER)
        .collect();
    let at = match hits.get(which % hits.len().max(1)) {
        Some(&i) => 12 + i + MARKER.len(),
        None => 12 + which % (n + 1),
    };
    data.copy_within(at..size, at + extra);
    data[at..at + extra].fill(b'A');
    data[8..12].copy_from_slice(&((n + extra) as u32).to_le_bytes());
    size + extra
}
