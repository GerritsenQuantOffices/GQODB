//! Experimental run reduction and byte-plane LZ4 for normalized book columns.
//! Preserves row order, every integer bit, and all repeated event metadata.
use crate::data::MAX_ROWS;
use anyhow::{Result, ensure};

const MAGIC: &[u8; 8] = b"GQOBOOK1";
const MAX_BYTES: usize = 128 * 1024 * 1024;

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn planes(values: &[i64]) -> (i64, u64, u8, Vec<u8>) {
    let base = values.iter().copied().min().unwrap_or(0);
    let mut scale = 0;
    let mut max = 0;
    for &v in values {
        let offset = v.wrapping_sub(base) as u64;
        max = max.max(offset);
        if scale != 1 {
            scale = gcd(scale, offset);
        }
    }
    let scale = scale.max(1);
    let width = (64 - (max / scale).leading_zeros()).div_ceil(8) as u8;
    let mut out = vec![0; values.len() * width as usize];
    for (i, &v) in values.iter().enumerate() {
        let offset = v.wrapping_sub(base) as u64;
        let q = if scale == 1 { offset } else { offset / scale };
        for byte in 0..width as usize {
            out[byte * values.len() + i] = (q >> (byte * 8)) as u8;
        }
    }
    (base, scale, width, out)
}

pub(crate) fn encode(values: &[Vec<i64>]) -> Result<Vec<u8>> {
    ensure!((1..=64).contains(&values.len()), "invalid columns");
    let rows = values[0].len();
    ensure!(
        rows <= MAX_ROWS && values.iter().all(|v| v.len() == rows),
        "invalid rows"
    );
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(&(rows as u32).to_le_bytes());
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for column in values {
        let runs =
            usize::from(!column.is_empty()) + column.windows(2).filter(|w| w[0] != w[1]).count();
        let rle = rows > 0 && runs <= rows / 4;
        let mut unique = Vec::new();
        let mut lengths = Vec::<u32>::new();
        if rle {
            for &v in column {
                if unique.last() == Some(&v) {
                    *lengths.last_mut().unwrap() += 1;
                } else {
                    unique.push(v);
                    lengths.push(1);
                }
            }
        }
        let input = if rle { &unique } else { column };
        let (base, scale, width, mut raw) = planes(input);
        if rle {
            // Separate count byte planes retain repetitive run-length patterns.
            for byte in 0..4 {
                for &n in &lengths {
                    raw.push((n >> (byte * 8)) as u8);
                }
            }
        }
        let compressed = lz4_flex::block::compress(&raw);
        let zipped = compressed.len() < raw.len();
        let stored = if zipped { &compressed } else { &raw };
        out.extend_from_slice(&[rle as u8, width, zipped as u8, 0]);
        out.extend_from_slice(&(input.len() as u32).to_le_bytes());
        out.extend_from_slice(&base.to_le_bytes());
        out.extend_from_slice(&scale.to_le_bytes());
        out.extend_from_slice(&(stored.len() as u32).to_le_bytes());
        out.extend_from_slice(stored);
    }
    ensure!(out.len() + 4 <= MAX_BYTES, "encoded book exceeds limit");
    out.extend_from_slice(&crc32fast::hash(&out).to_le_bytes());
    Ok(out)
}

fn restore_planes<const WIDTH: usize>(raw: &[u8], count: usize, base: u64, scale: u64) -> Vec<i64> {
    let planes: [&[u8]; WIDTH] = std::array::from_fn(|byte| &raw[byte * count..(byte + 1) * count]);
    let mut result = vec![0i64; count];
    for (i, value) in result.iter_mut().enumerate() {
        let mut q = 0u64;
        for (byte, plane) in planes.iter().enumerate() {
            q |= (plane[i] as u64) << (byte * 8);
        }
        *value = q.wrapping_mul(scale).wrapping_add(base) as i64;
    }
    result
}

pub(crate) fn decode(input: &[u8], scratch: &mut Vec<u8>) -> Result<Vec<Vec<i64>>> {
    ensure!(
        (20..=MAX_BYTES).contains(&input.len()),
        "invalid book length"
    );
    ensure!(&input[..8] == MAGIC, "invalid book magic");
    let end = input.len() - 4;
    ensure!(
        crc32fast::hash(&input[..end]) == u32::from_le_bytes(input[end..].try_into()?),
        "book checksum"
    );
    let u32_at = |p: usize| u32::from_le_bytes(input[p..p + 4].try_into().unwrap()) as usize;
    let rows = u32_at(8);
    let cols = u32_at(12);
    ensure!(
        rows <= MAX_ROWS && (1..=64).contains(&cols),
        "invalid book dimensions"
    );
    let mut result = Vec::with_capacity(cols);
    let mut pos = 16;
    for _ in 0..cols {
        ensure!(end - pos >= 28, "truncated book entry");
        let rle = input[pos];
        let width = input[pos + 1] as usize;
        let zipped = input[pos + 2];
        ensure!(
            rle <= 1 && width <= 8 && zipped <= 1 && input[pos + 3] == 0,
            "invalid book flags"
        );
        let count = u32_at(pos + 4);
        let base = u64::from_le_bytes(input[pos + 8..pos + 16].try_into()?);
        let scale = u64::from_le_bytes(input[pos + 16..pos + 24].try_into()?);
        let stored = u32_at(pos + 24);
        ensure!(scale > 0 && count <= rows, "invalid count/scale");
        ensure!(
            if rle == 0 {
                count == rows
            } else {
                count > 0 && rows > 0
            },
            "invalid run count"
        );
        pos += 28;
        ensure!(stored <= end - pos, "truncated book payload");
        let expanded = count * (width + if rle == 1 { 4 } else { 0 });
        let payload = &input[pos..pos + stored];
        let raw = if zipped == 1 {
            scratch.resize(expanded, 0);
            ensure!(
                lz4_flex::block::decompress_into(payload, scratch)? == expanded,
                "book expanded length"
            );
            &scratch[..]
        } else {
            payload
        };
        ensure!(raw.len() == expanded, "invalid raw length");
        let decoded = match width {
            0 => vec![base as i64; count],
            1 => restore_planes::<1>(raw, count, base, scale),
            2 => restore_planes::<2>(raw, count, base, scale),
            3 => restore_planes::<3>(raw, count, base, scale),
            4 => restore_planes::<4>(raw, count, base, scale),
            5 => restore_planes::<5>(raw, count, base, scale),
            6 => restore_planes::<6>(raw, count, base, scale),
            7 => restore_planes::<7>(raw, count, base, scale),
            8 => restore_planes::<8>(raw, count, base, scale),
            _ => unreachable!("width validated before allocation"),
        };
        let column = if rle == 1 {
            let mut column = Vec::with_capacity(rows);
            for (i, value) in decoded.into_iter().enumerate() {
                let mut n = 0u32;
                for byte in 0..4 {
                    n |= (raw[(width + byte) * count + i] as u32) << (byte * 8);
                }
                ensure!(
                    n > 0 && n as usize <= rows - column.len(),
                    "invalid run length"
                );
                column.resize(column.len() + n as usize, value);
            }
            column
        } else {
            decoded
        };
        ensure!(column.len() == rows, "book row count mismatch");
        result.push(column);
        pos += stored;
    }
    ensure!(pos == end, "trailing book bytes");
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fix_crc(bytes: &mut [u8]) {
        let end = bytes.len() - 4;
        let crc = crc32fast::hash(&bytes[..end]);
        bytes[end..].copy_from_slice(&crc.to_le_bytes());
    }
    #[test]
    fn exact_extremes_runs_empty_and_high_entropy() {
        let mut seed = 7u64;
        let random = (0..4097)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                seed as i64
            })
            .collect();
        for values in [
            vec![vec![]],
            vec![vec![i64::MIN, i64::MAX, -1, 0]],
            vec![
                vec![i64::MIN; 4097],
                (0..4097).map(|i| i / 50).collect(),
                random,
            ],
        ] {
            let bytes = encode(&values).unwrap();
            assert_eq!(decode(&bytes, &mut Vec::new()).unwrap(), values);
        }
    }
    #[test]
    fn every_byte_width_and_run_boundary_roundtrips() {
        for width in 1..=8 {
            for rows in [1, 3, 4, 127, 128, 129, 513] {
                let high = if width == 8 {
                    i64::MAX
                } else {
                    (1i64 << (width * 8)) - 1
                };
                let values = vec![
                    (0..rows).map(|i| [0, 1, high][i % 3]).collect(),
                    (0..rows)
                        .map(|i| if i < rows / 2 { i64::MIN } else { i64::MAX })
                        .collect(),
                ];
                assert_eq!(
                    decode(&encode(&values).unwrap(), &mut Vec::new()).unwrap(),
                    values
                );
            }
        }
    }
    #[test]
    fn truncation_corruption_and_forged_structure_fail() {
        let bytes = encode(&[vec![42; 32]]).unwrap();
        for n in 0..bytes.len() {
            assert!(decode(&bytes[..n], &mut Vec::new()).is_err());
        }
        for i in 0..bytes.len() {
            let mut bad = bytes.clone();
            bad[i] ^= 1;
            assert!(decode(&bad, &mut Vec::new()).is_err());
        }
        for (offset, value) in [
            (16, 2),
            (17, 9),
            (18, 2),
            (19, 1),
            (20, 0),
            (32, 0),
            (40, 255),
        ] {
            let mut bad = bytes.clone();
            bad[offset] = value;
            fix_crc(&mut bad);
            assert!(decode(&bad, &mut Vec::new()).is_err(), "offset {offset}");
        }
        // Uncompressed one-run constant: count planes begin at byte 44.
        let mut bad = bytes.clone();
        assert_eq!(bad[18], 0);
        bad[44] = 33;
        fix_crc(&mut bad);
        assert!(decode(&bad, &mut Vec::new()).is_err());
    }
}
