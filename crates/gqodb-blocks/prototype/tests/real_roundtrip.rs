use arrow_array::{
    ArrayRef, BooleanArray, Float64Array, Int64Array, LargeStringArray, RecordBatch,
};
use arrow_schema::{DataType, Field, Schema};
use bytes::Bytes;
use gqodb_blocks::real::{self, Mode};
use std::sync::Arc;

#[test]
fn nonnull_dictionary_runs_preserve_empty_unicode_and_transitions() {
    let strings: Vec<&str> = (0..1025)
        .map(|i| ["", "BTCUSDT", "β/日本", "x"][i / 257])
        .collect();
    let batch = RecordBatch::try_from_iter(vec![(
        "symbol",
        Arc::new(LargeStringArray::from(strings)) as ArrayRef,
    )])
    .unwrap();
    for mode in [Mode::Gqodb, Mode::GqodbBook] {
        let decoded = real::decode(Bytes::from(real::encode(&batch, mode).unwrap()), mode).unwrap();
        assert!(real::equal(&batch, &decoded));
    }
}

#[test]
fn schema_metadata_survives_all_formats() {
    let schema = Arc::new(Schema::new_with_metadata(
        vec![Field::new("price", DataType::Int64, false)],
        std::collections::HashMap::from([("price_scale".into(), "100000000".into())]),
    ));
    let batch =
        RecordBatch::try_new(schema, vec![Arc::new(Int64Array::from(vec![12345000000]))]).unwrap();
    for mode in Mode::ALL.into_iter().chain([Mode::GqodbBook]) {
        assert!(
            real::equal(
                &batch,
                &real::decode(Bytes::from(real::encode(&batch, mode).unwrap()), mode).unwrap()
            ),
            "{}",
            mode.name()
        );
    }
}

#[test]
fn trade_schema_preserves_int8_and_timestamp_timezone() {
    let batch = RecordBatch::try_from_iter(vec![
        (
            "side",
            Arc::new(arrow_array::Int8Array::from(vec![
                Some(i8::MIN),
                Some(i8::MAX),
                None,
            ])) as ArrayRef,
        ),
        (
            "dt",
            Arc::new(
                arrow_array::TimestampMillisecondArray::from(vec![
                    Some(-1),
                    Some(1788415200000),
                    None,
                ])
                .with_timezone("UTC"),
            ) as ArrayRef,
        ),
    ])
    .unwrap();
    for mode in Mode::ALL.into_iter().chain([Mode::GqodbBook]) {
        let decoded = real::decode(Bytes::from(real::encode(&batch, mode).unwrap()), mode).unwrap();
        assert!(real::equal(&batch, &decoded), "{}", mode.name());
    }
}

#[test]
fn mixed_real_schema_exact_and_corruption() {
    let schema = Arc::new(Schema::new(vec![
        Field::new("t", DataType::Int64, true),
        Field::new("p", DataType::Float64, true),
        Field::new("side", DataType::LargeUtf8, true),
        Field::new("maker", DataType::Boolean, true),
    ]));
    let values = [
        Some(1.12345),
        Some(-0.0),
        Some(f64::from_bits(0x7ff8000000000123)),
        None,
        Some(f64::INFINITY),
        Some(f64::MIN_POSITIVE),
    ];
    let arrays: Vec<ArrayRef> = vec![
        Arc::new(Int64Array::from(vec![
            Some(i64::MIN),
            Some(i64::MAX),
            None,
            Some(-1),
            Some(0),
            Some(1),
        ])),
        Arc::new(Float64Array::from(values.to_vec())),
        Arc::new(LargeStringArray::from(vec![
            Some("bid"),
            Some("ask"),
            None,
            Some(""),
            Some("Δ"),
            Some("bid"),
        ])),
        Arc::new(BooleanArray::from(vec![
            Some(true),
            None,
            Some(false),
            Some(false),
            Some(true),
            Some(false),
        ])),
    ];
    let batch = RecordBatch::try_new(schema, arrays).unwrap();
    for mode in Mode::ALL.into_iter().chain([Mode::GqodbBook]) {
        let bytes = real::encode(&batch, mode).unwrap();
        let decoded = real::decode(Bytes::from(bytes.clone()), mode).unwrap();
        assert!(real::equal(&batch, &decoded), "{}", mode.name());
        for i in 0..bytes.len() {
            let mut bad = bytes.clone();
            bad[i] ^= 1;
            assert!(real::decode(Bytes::from(bad), mode).is_err());
        }
        for n in [0, 1, 8, 12, bytes.len() - 1] {
            assert!(real::decode(Bytes::copy_from_slice(&bytes[..n]), mode).is_err());
        }
    }
}

#[test]
fn nonnull_fast_path_preserves_float_bits_and_integer_extrema() {
    for floats in [
        vec![0.0, 1.25, -12.5, 999.75],
        vec![
            -0.0,
            f64::from_bits(0x7ff8000000000123),
            f64::INFINITY,
            f64::MIN_POSITIVE,
        ],
    ] {
        let schema = Arc::new(Schema::new(vec![
            Field::new("t", DataType::Int64, true),
            Field::new("p", DataType::Float64, true),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(Int64Array::from(vec![i64::MIN, i64::MAX, 0, -1])),
                Arc::new(Float64Array::from(floats)),
            ],
        )
        .unwrap();
        for mode in Mode::ALL.into_iter().chain([Mode::GqodbBook]) {
            let encoded = real::encode(&batch, mode).unwrap();
            let decoded = real::decode(Bytes::from(encoded), mode).unwrap();
            assert!(real::equal(&batch, &decoded), "{}", mode.name());
        }
    }
}

#[test]
fn decimal_prefix_is_not_a_losslessness_proof() {
    let mut v = vec![Some(1.25); 2048];
    v[1536] = Some(f64::from_bits(1.25_f64.to_bits() + 1));
    let schema = Arc::new(Schema::new(vec![Field::new("p", DataType::Float64, true)]));
    let batch = RecordBatch::try_new(schema, vec![Arc::new(Float64Array::from(v))]).unwrap();
    for mode in Mode::ALL.into_iter().chain([Mode::GqodbBook]) {
        assert!(real::equal(
            &batch,
            &real::decode(Bytes::from(real::encode(&batch, mode).unwrap()), mode).unwrap()
        ));
    }
}
