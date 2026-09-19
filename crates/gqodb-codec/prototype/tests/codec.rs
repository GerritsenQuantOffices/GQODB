#![forbid(unsafe_code)]
use arrow_array::{ArrayRef, Float64Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use bytes::Bytes;
use gqodb_blocks::real;
use gqodb_codec::*;
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "gqodb-codec-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self, p: &str) -> PathBuf {
        self.0.join(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn both_specialized_codecs_restore_exact_schema_metadata_nulls_and_values() {
    for family in [Family::Tick, Family::Orderbook] {
        let batch = fixture(family).unwrap();
        let encoded = encode(&batch, family).unwrap();
        let decoded = decode(Bytes::from(encoded), Some(family)).unwrap();
        assert!(real::equal(&batch, &decoded));
    }
}
#[test]
fn extra_float_columns_preserve_nan_payloads_and_negative_zero() {
    let b = fixture(Family::Tick).unwrap();
    let mut fields = b
        .schema()
        .fields()
        .iter()
        .map(|f| f.as_ref().clone())
        .collect::<Vec<_>>();
    fields.push(Field::new("extra", DataType::Float64, false));
    let mut cols = b.columns().to_vec();
    cols.push(Arc::new(Float64Array::from(vec![
        -0.0,
        f64::from_bits(0x7ff8_0000_0000_1234),
        f64::INFINITY,
    ])) as ArrayRef);
    let b = RecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(
            fields,
            b.schema().metadata().clone(),
        )),
        cols,
    )
    .unwrap();
    assert!(real::equal(
        &b,
        &decode(
            Bytes::from(encode(&b, Family::Tick).unwrap()),
            Some(Family::Tick)
        )
        .unwrap()
    ));
}
#[test]
fn filename_is_never_codec_identity() {
    let t = Temp::new();
    let p = t.path("misleading.gqodb.ob");
    write_block(&p, &fixture(Family::Tick).unwrap(), Family::Tick).unwrap();
    assert_eq!(inspect(&p, None).unwrap().format, Format::AdaptiveBlockV3);
    assert!(inspect(&p, Some(Family::Tick)).is_ok());
    assert!(inspect(&p, Some(Family::Orderbook)).is_err());
}
#[test]
fn segment_roundtrip_uses_actual_native_codec_and_footer() {
    let t = Temp::new();
    let b = fixture(Family::Orderbook).unwrap();
    let p = t.path("book.gqodb.ob");
    write_segment(&p, &[b.clone(), b.clone()]).unwrap();
    let report = inspect(&p, Some(Family::Orderbook)).unwrap();
    assert_eq!(report.blocks, 2);
    assert_eq!(report.rows, 6);
    let mut reader = gqodb_store::Reader::open(&p).unwrap();
    assert!(
        reader
            .read_all()
            .unwrap()
            .iter()
            .all(|actual| real::equal(actual, &b))
    );
    let mut bytes = fs::read(&p).unwrap();
    bytes.pop();
    fs::write(t.path("truncated"), bytes).unwrap();
    assert!(inspect(&t.path("truncated"), None).is_err());
}
#[test]
fn unknown_codec_schema_family_and_metadata_reject() {
    let b = fixture(Family::Tick).unwrap();
    let bytes = encode(&b, Family::Tick).unwrap();
    assert!(decode(Bytes::from(bytes.clone()), Some(Family::Orderbook)).is_err());
    let mut bad = bytes.clone();
    bad[..8].copy_from_slice(b"GQOREAL9");
    assert!(decode(Bytes::from(bad), None).is_err());
    let mut metadata = b.schema().metadata().clone();
    metadata.insert("gqodb.schema_version".into(), "999".into());
    let b = RecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(
            b.schema().fields().clone(),
            metadata,
        )),
        b.columns().to_vec(),
    )
    .unwrap();
    assert!(encode(&b, Family::Tick).is_err());
}
#[test]
fn every_truncated_block_and_corrupt_payload_fails() {
    let bytes = encode(&fixture(Family::Tick).unwrap(), Family::Tick).unwrap();
    for n in 0..bytes.len() {
        assert!(
            decode(Bytes::copy_from_slice(&bytes[..n]), None).is_err(),
            "length {n}"
        );
    }
    let mut corrupt = bytes;
    let last = corrupt.len() - 8;
    corrupt[last] ^= 1;
    assert!(decode(Bytes::from(corrupt), None).is_err());
}
#[test]
fn oversized_lengths_and_dimensions_fail_before_decoding() {
    let mut bytes = encode(&fixture(Family::Tick).unwrap(), Family::Tick).unwrap();
    bytes[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    fix_crc(&mut bytes);
    assert!(decode(Bytes::from(bytes), None).is_err());
    let mut bytes = encode(&fixture(Family::Tick).unwrap(), Family::Tick).unwrap();
    let meta = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    bytes[12 + meta + 12..12 + meta + 16].copy_from_slice(&u32::MAX.to_le_bytes());
    fix_crc(&mut bytes);
    assert!(decode(Bytes::from(bytes), None).is_err());
}
fn fix_crc(bytes: &mut [u8]) {
    let n = bytes.len() - 4;
    let crc = crc32fast::hash(&bytes[..n]);
    bytes[n..].copy_from_slice(&crc.to_le_bytes());
}
#[test]
fn legacy_blocks_are_inspectable_without_claiming_normalized_contract() {
    let t = Temp::new();
    let b = fixture(Family::Tick).unwrap();
    let legacy = RecordBatch::try_new(
        Arc::new(Schema::new(b.schema().fields().clone())),
        b.columns().to_vec(),
    )
    .unwrap();
    let bytes = real::encode(&legacy, real::Mode::Gqodb).unwrap();
    fs::write(t.path("legacy"), bytes).unwrap();
    let report = inspect(&t.path("legacy"), None).unwrap();
    assert!(!report.normalized_contract_valid);
    assert!(report.fully_decoded);
    assert!(inspect(&t.path("legacy"), Some(Family::Tick)).is_err());
}

#[test]
fn tick_segments_dispatch_using_stored_codec_even_with_ob_extension() {
    let t = Temp::new();
    let b = fixture(Family::Tick).unwrap();
    let path = t.path("tick-disguised.gqodb.ob");
    write_segment_with_family(&path, std::slice::from_ref(&b), Family::Tick).unwrap();
    let report = inspect(&path, Some(Family::Tick)).unwrap();
    assert_eq!(report.format, Format::NativeSegmentV1);
    assert_eq!(report.codec_profile, "gqodb-adaptive-lz4");
    assert!(inspect(&path, Some(Family::Orderbook)).is_err());
    let mut reader = gqodb_store::Reader::open(&path).unwrap();
    assert!(matches!(reader.codec(), real::Mode::Gqodb));
    assert!(real::equal(&reader.read_all().unwrap()[0], &b));
    let mut recovered = gqodb_store::Reader::recover(&path).unwrap();
    assert!(matches!(recovered.codec(), real::Mode::Gqodb));
    assert!(real::equal(&recovered.read_all().unwrap()[0], &b));
}

#[test]
fn dictionary_expansion_preflight_rejects_bomb_before_native_decode() {
    let mut bytes = encode(&fixture(Family::Tick).unwrap(), Family::Tick).unwrap();
    let meta_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let mut layout: serde_json::Value = serde_json::from_slice(&bytes[12..12 + meta_len]).unwrap();
    layout["columns"][5]["dictionary"] = serde_json::json!(["x".repeat(256)]);
    let meta = serde_json::to_vec(&layout).unwrap();
    let mut payload = bytes[12 + meta_len..bytes.len() - 4].to_vec();
    payload[12..16].copy_from_slice(&(1_048_576_u32).to_le_bytes());
    bytes = b"GQOREAL3".to_vec();
    bytes.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&meta);
    bytes.extend_from_slice(&payload);
    bytes.extend_from_slice(&[0; 4]);
    fix_crc(&mut bytes);
    let error = decode(Bytes::from(bytes), None).unwrap_err().to_string();
    assert!(error.contains("expansion"), "{error}");
}
