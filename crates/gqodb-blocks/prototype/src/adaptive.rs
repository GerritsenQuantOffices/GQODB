//! B02: lossless column predictors plus safe Rust SIMD bitpacking.
use crate::data::{Columns, Kind, MAX_ROWS};
use anyhow::{Result, ensure};
use bitpacking::{BitPacker, BitPacker4x};

const MAGIC: &[u8; 8] = b"GQOADP02";
const ENTRY: usize = 44;
const HEADER: usize = 16;
const MAX_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
enum Predictor {
    Absolute = 0,
    Delta = 1,
    Linear = 2,
    Reference = 3,
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let next = a % b;
        a = b;
        b = next;
    }
    a
}

fn residuals(
    input: &[i64],
    reference: &[i64],
    predictor: Predictor,
    origin: i64,
    step: i64,
) -> Vec<i64> {
    match predictor {
        Predictor::Absolute => input.to_vec(),
        Predictor::Delta => {
            let mut result = Vec::with_capacity(input.len());
            if !input.is_empty() {
                result.push(0);
            }
            result.extend(input.windows(2).map(|v| v[1].wrapping_sub(v[0])));
            result
        }
        Predictor::Linear => input
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                v.wrapping_sub(origin)
                    .wrapping_sub(step.wrapping_mul(i as i64))
            })
            .collect(),
        Predictor::Reference => input
            .iter()
            .zip(reference)
            .map(|(&v, &r)| v.wrapping_sub(r))
            .collect(),
    }
}

fn parameters(values: &[i64]) -> (i64, u64, u32) {
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
    (base, scale, 64 - (max / scale).leading_zeros())
}

fn select(data: &[Vec<i64>], column: usize, previous: &[usize]) -> (Predictor, u8, i64, i64) {
    let values = &data[column];
    let n = values.len().min(1024);
    let sample = &values[..n];
    let origin = values.first().copied().unwrap_or(0);
    let step = values
        .get(1)
        .copied()
        .unwrap_or(origin)
        .wrapping_sub(origin);
    let mut selected = (Predictor::Absolute, u8::MAX, origin, step);
    let mut best = parameters(sample).2;
    for predictor in [Predictor::Delta, Predictor::Linear] {
        let test = residuals(sample, &[], predictor, origin, step);
        let score = parameters(&test).2 + matches!(predictor, Predictor::Delta) as u32;
        if score < best {
            best = score;
            selected.0 = predictor;
        }
    }
    for &reference in previous {
        let test = residuals(
            sample,
            &data[reference][..n],
            Predictor::Reference,
            origin,
            step,
        );
        let score = parameters(&test).2;
        if score < best {
            best = score;
            selected.0 = Predictor::Reference;
            selected.1 = reference as u8;
        }
    }
    selected
}

fn pack(values: &[i64], base: i64, scale: u64) -> Vec<u8> {
    let packer = BitPacker4x::new();
    let mut out = Vec::with_capacity(values.len() * 4);
    for group in values.chunks(128) {
        let mut quotients = [0_u64; 128];
        let mut integers = [0_u32; 128];
        let mut max = 0_u64;
        for (i, &v) in group.iter().enumerate() {
            let offset = v.wrapping_sub(base) as u64;
            let q = if scale == 1 { offset } else { offset / scale };
            quotients[i] = q;
            integers[i] = q as u32;
            max |= q;
        }
        if max > u32::MAX as u64 {
            out.push(255);
            for q in &quotients[..group.len()] {
                out.extend_from_slice(&q.to_le_bytes());
            }
        } else {
            let width = packer.num_bits(&integers);
            out.push(width);
            if width != 0 {
                let start = out.len();
                out.resize(start + width as usize * 16, 0);
                packer.compress(&integers, &mut out[start..], width);
            }
        }
    }
    out
}

#[cfg(test)]
fn unpack(input: &[u8], rows: usize, base: i64, scale: u64) -> Result<Vec<i64>> {
    unpack_map(input, rows, base, scale, |v| v)
}

fn unpack_map(
    input: &[u8],
    rows: usize,
    base: i64,
    scale: u64,
    mut restore: impl FnMut(i64) -> i64,
) -> Result<Vec<i64>> {
    ensure!(scale != 0, "zero scale");
    let packer = BitPacker4x::new();
    let mut result = Vec::with_capacity(rows);
    let mut pos = 0;
    while result.len() < rows {
        ensure!(pos < input.len(), "missing packed width");
        let width = input[pos];
        pos += 1;
        let count = (rows - result.len()).min(128);
        if width == 255 {
            ensure!(pos + count * 8 <= input.len(), "truncated u64 group");
            for v in input[pos..pos + count * 8].chunks_exact(8) {
                let q = u64::from_le_bytes(v.try_into().unwrap());
                result.push(restore(
                    q.wrapping_mul(scale).wrapping_add(base as u64) as i64
                ));
            }
            pos += count * 8;
        } else {
            ensure!(width <= 32, "invalid SIMD width");
            let bytes = width as usize * 16;
            ensure!(pos + bytes <= input.len(), "truncated SIMD group");
            let mut integers = [0_u32; 128];
            if width != 0 {
                packer.decompress(&input[pos..pos + bytes], &mut integers, width);
            }
            ensure!(
                integers[count..].iter().all(|&v| v == 0),
                "nonzero group padding"
            );
            result.extend(integers[..count].iter().map(|&q| {
                restore((q as u64).wrapping_mul(scale).wrapping_add(base as u64) as i64)
            }));
            pos += bytes;
        }
    }
    ensure!(pos == input.len(), "trailing packed bytes");
    Ok(result)
}

pub fn encode(data: &Columns, use_lz4: bool) -> Result<Vec<u8>> {
    data.validate()?;
    encode_values(&data.values, use_lz4, data.kind as u8)
}

pub(crate) fn encode_values(values: &[Vec<i64>], use_lz4: bool, kind: u8) -> Result<Vec<u8>> {
    let cols = values.len();
    ensure!((1..=64).contains(&cols), "invalid columns");
    let rows = values[0].len();
    ensure!(
        rows <= MAX_ROWS && values.iter().all(|v| v.len() == rows),
        "invalid rows"
    );
    let mut order: Vec<usize> = (0..cols).collect();
    if cols >= 2 {
        order.swap(0, 1);
    }
    let mut out = vec![0; HEADER + ENTRY * cols];
    out[..8].copy_from_slice(MAGIC);
    out[8] = use_lz4 as u8;
    out[9] = kind;
    out[10..12].copy_from_slice(&(cols as u16).to_le_bytes());
    out[12..16].copy_from_slice(&(rows as u32).to_le_bytes());
    for (index, &column) in order.iter().enumerate() {
        let (predictor, reference, origin, step) = select(values, column, &order[..index]);
        let ref_values: &[i64] = if reference == u8::MAX {
            &[]
        } else {
            &values[reference as usize]
        };
        let residuals = residuals(&values[column], ref_values, predictor, origin, step);
        let (base, scale, _) = parameters(&residuals);
        let packed = pack(&residuals, base, scale);
        let compressed = if use_lz4 {
            lz4_flex::block::compress(&packed)
        } else {
            Vec::new()
        };
        let is_compressed = use_lz4 && compressed.len() < packed.len();
        let stored = if is_compressed { &compressed } else { &packed };
        let d = HEADER + index * ENTRY;
        out[d] = column as u8;
        out[d + 1] = predictor as u8;
        out[d + 2] = reference;
        out[d + 3] = is_compressed as u8;
        out[d + 4..d + 12].copy_from_slice(&origin.to_le_bytes());
        out[d + 12..d + 20].copy_from_slice(&step.to_le_bytes());
        out[d + 20..d + 28].copy_from_slice(&base.to_le_bytes());
        out[d + 28..d + 36].copy_from_slice(&scale.to_le_bytes());
        out[d + 36..d + 40].copy_from_slice(&(packed.len() as u32).to_le_bytes());
        out[d + 40..d + 44].copy_from_slice(&(stored.len() as u32).to_le_bytes());
        out.extend_from_slice(stored);
    }
    out.extend_from_slice(&crc32fast::hash(&out).to_le_bytes());
    Ok(out)
}

pub fn decode(input: &[u8]) -> Result<Columns> {
    let (kind, values) = decode_values(input)?;
    let result = Columns {
        kind: Kind::from_id(kind)?,
        values,
    };
    result.validate()?;
    Ok(result)
}

pub(crate) fn decode_values(input: &[u8]) -> Result<(u8, Vec<Vec<i64>>)> {
    decode_values_with_scratch(input, &mut Vec::new())
}

pub(crate) fn decode_values_with_scratch(
    input: &[u8],
    scratch: &mut Vec<u8>,
) -> Result<(u8, Vec<Vec<i64>>)> {
    ensure!(
        input.len() >= HEADER + 4 && input.len() <= MAX_BYTES,
        "invalid adaptive block size"
    );
    ensure!(
        &input[..8] == MAGIC && input[8] <= 1,
        "invalid adaptive magic/flags"
    );
    let end = input.len() - 4;
    ensure!(
        crc32fast::hash(&input[..end]) == u32::from_le_bytes(input[end..].try_into().unwrap()),
        "adaptive checksum mismatch"
    );
    let kind = input[9];
    let cols = u16::from_le_bytes(input[10..12].try_into().unwrap()) as usize;
    let rows = u32::from_le_bytes(input[12..16].try_into().unwrap()) as usize;
    ensure!(
        (1..=64).contains(&cols) && rows <= MAX_ROWS && HEADER + ENTRY * cols <= end,
        "invalid dimensions"
    );
    let mut values = vec![Vec::new(); cols];
    let mut seen = vec![false; cols];
    let mut pos = HEADER + ENTRY * cols;
    for index in 0..cols {
        let d = HEADER + index * ENTRY;
        let column = input[d] as usize;
        ensure!(
            column < cols && !seen[column],
            "invalid or duplicate column"
        );
        let predictor = match input[d + 1] {
            0 => Predictor::Absolute,
            1 => Predictor::Delta,
            2 => Predictor::Linear,
            3 => Predictor::Reference,
            _ => anyhow::bail!("unknown predictor"),
        };
        let reference = input[d + 2] as usize;
        if matches!(predictor, Predictor::Reference) {
            ensure!(
                reference < cols && seen[reference],
                "invalid/forward reference"
            );
        } else {
            ensure!(reference == u8::MAX as usize, "unexpected reference");
        }
        let compressed = input[d + 3];
        ensure!(compressed <= input[8], "invalid compression flag");
        let read_i64 = |p| i64::from_le_bytes(input[p..p + 8].try_into().unwrap());
        let origin = read_i64(d + 4);
        let step = read_i64(d + 12);
        let base = read_i64(d + 20);
        let scale = u64::from_le_bytes(input[d + 28..d + 36].try_into().unwrap());
        let expanded = u32::from_le_bytes(input[d + 36..d + 40].try_into().unwrap()) as usize;
        let stored = u32::from_le_bytes(input[d + 40..d + 44].try_into().unwrap()) as usize;
        ensure!(
            expanded <= rows.div_ceil(128) * 1025 && stored <= end - pos,
            "invalid column lengths"
        );
        let payload = &input[pos..pos + stored];
        let packed = if compressed == 1 {
            scratch.resize(expanded, 0);
            let written = lz4_flex::block::decompress_into(payload, scratch)?;
            ensure!(written == expanded, "expanded length mismatch");
            &scratch[..]
        } else {
            payload
        };
        ensure!(packed.len() == expanded, "expanded length mismatch");
        let restored = match predictor {
            Predictor::Absolute => unpack_map(packed, rows, base, scale, |v| v)?,
            Predictor::Delta => {
                let mut previous = origin;
                let restored = unpack_map(packed, rows, base, scale, |v| {
                    previous = previous.wrapping_add(v);
                    previous
                })?;
                ensure!(
                    restored.first().is_none_or(|&v| v == origin),
                    "invalid initial delta"
                );
                restored
            }
            Predictor::Linear => {
                let mut predicted = origin;
                unpack_map(packed, rows, base, scale, |v| {
                    let result = v.wrapping_add(predicted);
                    predicted = predicted.wrapping_add(step);
                    result
                })?
            }
            Predictor::Reference => {
                let mut refs = values[reference].iter();
                unpack_map(packed, rows, base, scale, |v| {
                    v.wrapping_add(*refs.next().unwrap())
                })?
            }
        };
        values[column] = restored;
        seen[column] = true;
        pos += stored;
    }
    ensure!(pos == end, "trailing adaptive bytes");
    Ok((kind, values))
}

pub fn framing_bytes() -> usize {
    HEADER + ENTRY * 8 + 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_simd_groups_fail_before_library_call() {
        for width in 33..255 {
            assert!(unpack(&[width], 1, 0, 1).is_err());
        }
        assert!(unpack(&[32, 0], 128, 0, 1).is_err());
        assert!(unpack(&[255, 0], 1, 0, 1).is_err());
        assert!(unpack(&[0], 1, 0, 0).is_err());
        assert!(unpack(&[0, 0], 1, 0, 1).is_err());
    }
}
