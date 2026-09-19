//! Bounded experimental block framing and two lossless delta transforms.
use crate::data::{Columns, Kind, MAX_ROWS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

const MAGIC: &[u8; 8] = b"GQOBLK01";
const HEADER: usize = 16;
const DIRECTORY_ENTRY: usize = 8;
const MAX_BLOCK_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout {
    Raw = 0,
    Varint = 1,
    Bitpack = 2,
}

fn zigzag(delta: i64) -> u64 {
    ((delta << 1) ^ (delta >> 63)) as u64
}
fn unzigzag(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}

fn deltas(values: &[i64]) -> Vec<u64> {
    values
        .windows(2)
        .map(|v| zigzag(v[1].wrapping_sub(v[0])))
        .collect()
}

fn transform(values: &[i64], layout: Layout) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 8);
    if layout == Layout::Raw {
        for v in values {
            out.extend_from_slice(&v.to_le_bytes());
        }
        return out;
    }
    let Some(first) = values.first() else {
        return out;
    };
    out.extend_from_slice(&first.to_le_bytes());
    let encoded = deltas(values);
    if layout == Layout::Varint {
        for mut v in encoded {
            while v >= 128 {
                out.push((v as u8 & 127) | 128);
                v >>= 7;
            }
            out.push(v as u8);
        }
    } else {
        for group in encoded.chunks(128) {
            let width = 64 - group.iter().fold(0_u64, |a, v| a | v).leading_zeros();
            out.push(width as u8);
            let mut pending = 0_u128;
            let mut bits = 0;
            for &v in group {
                pending |= (v as u128) << bits;
                bits += width;
                while bits >= 8 {
                    out.push(pending as u8);
                    pending >>= 8;
                    bits -= 8;
                }
            }
            if bits > 0 {
                out.push(pending as u8);
            }
        }
    }
    out
}

fn restore(input: &[u8], rows: usize, layout: Layout) -> Result<Vec<i64>> {
    if layout == Layout::Raw {
        ensure!(input.len() == rows * 8, "invalid raw column length");
        return Ok(input
            .chunks_exact(8)
            .map(|v| i64::from_le_bytes(v.try_into().unwrap()))
            .collect());
    }
    if rows == 0 {
        ensure!(input.is_empty(), "nonempty zero-row column");
        return Ok(vec![]);
    }
    ensure!(input.len() >= 8, "missing first value");
    let mut out = Vec::with_capacity(rows);
    out.push(i64::from_le_bytes(input[..8].try_into().unwrap()));
    let mut pos = 8;
    if layout == Layout::Varint {
        for _ in 1..rows {
            let mut v = 0_u64;
            let mut done = false;
            for byte_index in 0..10 {
                ensure!(pos < input.len(), "truncated varint");
                let byte = input[pos];
                pos += 1;
                ensure!(byte_index != 9 || byte <= 1, "varint overflow");
                v |= ((byte & 127) as u64) << (byte_index * 7);
                if byte & 128 == 0 {
                    done = true;
                    break;
                }
            }
            ensure!(done, "unterminated varint");
            out.push(out.last().unwrap().wrapping_add(unzigzag(v)));
        }
    } else {
        while out.len() < rows {
            ensure!(pos < input.len(), "missing bit width");
            let width = input[pos] as u32;
            pos += 1;
            ensure!(width <= 64, "invalid bit width");
            let count = (rows - out.len()).min(128);
            let nbytes = (count * width as usize).div_ceil(8);
            ensure!(pos + nbytes <= input.len(), "truncated packed group");
            let end = pos + nbytes;
            let mut pending = 0_u128;
            let mut bits = 0;
            let mask = (1_u128 << width) - 1;
            for _ in 0..count {
                while bits < width {
                    pending |= (input[pos] as u128) << bits;
                    pos += 1;
                    bits += 8;
                }
                let v = (pending & mask) as u64;
                pending >>= width;
                bits -= width;
                out.push(out.last().unwrap().wrapping_add(unzigzag(v)));
            }
            ensure!(pos == end && pending == 0, "noncanonical packed padding");
        }
    }
    ensure!(pos == input.len(), "trailing transformed bytes");
    Ok(out)
}

pub fn encode(data: &Columns, layout: Layout) -> Result<Vec<u8>> {
    data.validate()?;
    let mut payload = Vec::new();
    let mut directory = Vec::new();
    for values in &data.values {
        let encoded = transform(values, layout);
        let compressed = lz4_flex::block::compress(&encoded);
        directory.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        payload.extend_from_slice(&compressed);
    }
    let mut out = Vec::with_capacity(HEADER + directory.len() + payload.len() + 4);
    out.extend_from_slice(MAGIC);
    out.push(layout as u8);
    out.push(data.kind as u8);
    out.extend_from_slice(&(data.values.len() as u16).to_le_bytes());
    out.extend_from_slice(&(data.rows() as u32).to_le_bytes());
    out.extend(directory);
    out.extend(payload);
    out.extend_from_slice(&crc32fast::hash(&out).to_le_bytes());
    Ok(out)
}

pub fn decode(input: &[u8]) -> Result<Columns> {
    ensure!(
        input.len() >= HEADER + 4 && input.len() <= MAX_BLOCK_BYTES,
        "invalid block size"
    );
    ensure!(&input[..8] == MAGIC, "invalid magic/version");
    let checksum_at = input.len() - 4;
    let expected = u32::from_le_bytes(input[checksum_at..].try_into().unwrap());
    ensure!(
        crc32fast::hash(&input[..checksum_at]) == expected,
        "block checksum mismatch"
    );
    let layout = match input[8] {
        0 => Layout::Raw,
        1 => Layout::Varint,
        2 => Layout::Bitpack,
        _ => anyhow::bail!("unknown layout"),
    };
    let kind = Kind::from_id(input[9])?;
    let cols = u16::from_le_bytes(input[10..12].try_into().unwrap()) as usize;
    let rows = u32::from_le_bytes(input[12..16].try_into().unwrap()) as usize;
    ensure!(cols == 8 && rows <= MAX_ROWS, "invalid dimensions");
    let mut pos = HEADER + cols * DIRECTORY_ENTRY;
    ensure!(pos <= checksum_at, "truncated directory");
    let mut values = Vec::with_capacity(cols);
    for c in 0..cols {
        let d = HEADER + c * DIRECTORY_ENTRY;
        let expanded = u32::from_le_bytes(input[d..d + 4].try_into().unwrap()) as usize;
        let compressed = u32::from_le_bytes(input[d + 4..d + 8].try_into().unwrap()) as usize;
        // Varint worst case: 8-byte initial value plus ten bytes per subsequent delta.
        ensure!(
            expanded <= rows.saturating_mul(10),
            "expanded column exceeds bound"
        );
        ensure!(
            compressed <= checksum_at - pos,
            "truncated compressed column"
        );
        let raw = lz4_flex::block::decompress(&input[pos..pos + compressed], expanded)?;
        ensure!(raw.len() == expanded, "decompressed size mismatch");
        values.push(restore(&raw, rows, layout)?);
        pos += compressed;
    }
    ensure!(pos == checksum_at, "trailing block bytes");
    let result = Columns { kind, values };
    result.validate()?;
    Ok(result)
}

pub fn framing_bytes() -> usize {
    HEADER + 8 * DIRECTORY_ENTRY + 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_transforms() {
        assert!(restore(&[0; 7], 1, Layout::Varint).is_err());
        let mut malformed = vec![0; 8];
        malformed.extend([255; 10]);
        assert!(restore(&malformed, 2, Layout::Varint).is_err());
        let mut malformed = vec![0; 8];
        malformed.push(65);
        assert!(restore(&malformed, 2, Layout::Bitpack).is_err());
        assert!(restore(&[1], 0, Layout::Raw).is_err());
    }
}
