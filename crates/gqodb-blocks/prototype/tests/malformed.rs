//! Regression tests for inputs found by the cargo-fuzz harness (see docs/FUZZING.md).
//! Each input must be refused with an error: no panic, no hang, no unbounded allocation.
use bytes::Bytes;
use gqodb_blocks::{
    Codec,
    real::{self, Mode},
};

fn with_crc(mut bytes: Vec<u8>) -> Vec<u8> {
    let crc = crc32fast::hash(&bytes);
    bytes.extend_from_slice(&crc.to_le_bytes());
    bytes
}

/// Fuzz target `real_tick_decode`. A 13-byte block whose CRC is valid but whose 4-byte
/// schema-length field overlaps the CRC: `end - 12` underflowed (panic in debug builds, a
/// wrapped huge bound followed by an out-of-range slice in release builds).
#[test]
fn real_block_shorter_than_its_own_schema_length_field_is_refused() {
    let mut input = b"GQOREAL3".to_vec();
    input.push(0x86);
    let input = with_crc(input);
    assert_eq!(input.len(), 13);
    for mode in [Mode::Gqodb, Mode::GqodbBook] {
        let mut input = input.clone();
        input[..8].copy_from_slice(if matches!(mode, Mode::Gqodb) {
            b"GQOREAL3"
        } else {
            b"GQOREAL4"
        });
        let len = input.len();
        let crc = crc32fast::hash(&input[..len - 4]);
        input[len - 4..].copy_from_slice(&crc.to_le_bytes());
        assert!(real::decode(Bytes::from(input), mode).is_err());
    }
    // Every length from 12 up to the smallest well-formed frame, same CRC-valid shape.
    for extra in 0..12 {
        let mut body = b"GQOREAL3".to_vec();
        body.extend(std::iter::repeat_n(0xff, extra));
        let input = with_crc(body);
        if input.len() >= 12 {
            assert!(real::decode(Bytes::from(input), Mode::Gqodb).is_err());
        }
    }
}

/// Fuzz target `parquet_decode`: the embedded `ARROW:schema` flatbuffer names a type the
/// `arrow-ipc` converter does not implement (or omits a required vector). Before the fix the
/// dependency panicked (`unimplemented!`, `unwrap`, `expect`) inside `ParquetRecordBatchReaderBuilder`.
/// The four files hit four distinct panic sites of arrow-ipc 59.3.0 `convert.rs` (lines 198, 332,
/// 354, 514). They are minimized synthetic fixtures, CRC envelope included.
#[test]
fn forged_embedded_arrow_schema_is_an_error_not_a_panic() {
    let inputs: [&[u8]; 4] = [
        include_bytes!("regressions/parquet_arrow_schema_panic_198.bin"),
        include_bytes!("regressions/parquet_arrow_schema_panic_332.bin"),
        include_bytes!("regressions/parquet_arrow_schema_panic_354.bin"),
        include_bytes!("regressions/parquet_arrow_schema_panic_514.bin"),
    ];
    for (i, input) in inputs.iter().enumerate() {
        let input = Bytes::copy_from_slice(input);
        for codec in Codec::ALL.into_iter().filter(|c| c.is_parquet()) {
            assert!(codec.decode(input.clone()).is_err(), "input {i} {codec:?}");
            assert!(
                codec.framing_bytes(input.clone()).is_err(),
                "input {i} {codec:?}"
            );
        }
        for mode in [
            Mode::ParquetLz4,
            Mode::ParquetDictionaryLz4,
            Mode::ParquetSnappy,
            Mode::ParquetBrotli,
            Mode::ParquetNormalizedLz4,
        ] {
            assert!(
                real::decode(input.clone(), mode).is_err(),
                "input {i} {}",
                mode.name()
            );
        }
    }
}

/// Run `f` on a worker thread; fail (instead of hanging the suite) if it takes 30 s, and return how
/// long it took so the hang tests can also demand a quick refusal.
fn within_30_seconds<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    let start = std::time::Instant::now();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    let out = rx
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect("decode did not finish within 30 s (hang regression)");
    // A guarded refusal takes milliseconds; the unguarded reader needs seconds to minutes.
    assert!(
        start.elapsed() < std::time::Duration::from_secs(4),
        "refusal took {:?}: the input still reaches the slow path",
        start.elapsed()
    );
    out
}

fn parquet_outcomes(input: &'static [u8]) -> Vec<(String, bool)> {
    let input = Bytes::from_static(input);
    let mut out = Vec::new();
    for codec in Codec::ALL.into_iter().filter(|c| c.is_parquet()) {
        out.push((format!("{codec:?}"), codec.decode(input.clone()).is_err()));
        // `framing_bytes` reads only the footer, so it may legitimately succeed on these inputs.
    }
    for mode in [
        Mode::ParquetLz4,
        Mode::ParquetDictionaryLz4,
        Mode::ParquetSnappy,
        Mode::ParquetBrotli,
        Mode::ParquetNormalizedLz4,
    ] {
        out.push((
            mode.name().to_string(),
            real::decode(input.clone(), mode).is_err(),
        ));
    }
    out
}

/// Fuzz target `parquet_decode` (timeout). A page header carries an unknown field holding a list of
/// zero-byte (boolean) elements with a huge declared size; the `parquet` crate skips it one
/// element at a time (nested lists multiply), so a 2 KB input kept a core busy for over 10
/// minutes. The page-header walk refuses it up front.
#[test]
fn page_header_with_unbounded_list_is_refused_quickly() {
    let outcomes = within_30_seconds(|| {
        parquet_outcomes(include_bytes!(
            "regressions/parquet_page_header_list_hang.bin"
        ))
    });
    assert!(outcomes.iter().all(|(_, refused)| *refused), "{outcomes:?}");
}

/// Fuzz target `parquet_decode` (timeout, found on the first fix's code). The page header declares a
/// binary field longer than its column chunk; the `parquet` reader hands the header parser the rest
/// of the FILE, skips that field into the next chunk's bytes and then meets a boolean-list bomb there.
/// The page-header walk therefore has to run past the chunk end exactly as the reader does.
#[test]
fn page_header_bomb_behind_an_overlong_binary_field_is_refused_quickly() {
    let outcomes = within_30_seconds(|| {
        parquet_outcomes(include_bytes!(
            "regressions/parquet_page_header_list_past_chunk_end.bin"
        ))
    });
    assert!(outcomes.iter().all(|(_, refused)| *refused), "{outcomes:?}");
}

/// Fuzz target `parquet_decode` (timeout, found on the page-header fix's code). The same boolean-list
/// bomb sits in the FILE FOOTER (`FileMetaData` unknown field), which `parquet` skips one element at
/// a time while the footer is parsed, before any page header exists. 833 bytes, minimized.
#[test]
fn footer_with_unbounded_list_is_refused_quickly() {
    let outcomes = within_30_seconds(|| {
        parquet_outcomes(include_bytes!("regressions/parquet_footer_list_hang.bin"))
    });
    assert!(outcomes.iter().all(|(_, refused)| *refused), "{outcomes:?}");
}

/// Fuzz target `parquet_decode` (crash). A forged DELTA_BINARY_PACKED header declares zero values
/// but carries a first value: `parquet` 59.3.0 `encodings/decoding.rs:750` computes
/// `values_left -= 1` on zero (panic with overflow checks, wrapped counter otherwise).
#[test]
fn forged_delta_header_is_an_error_not_a_panic() {
    let outcomes = within_30_seconds(|| {
        parquet_outcomes(include_bytes!(
            "regressions/parquet_delta_header_underflow.bin"
        ))
    });
    assert!(outcomes.iter().all(|(_, refused)| *refused), "{outcomes:?}");
}

/// Fuzz target `parquet_decode` (crash). Forged bit widths make `parquet` 59.3.0 `util/bit_util.rs:868`
/// shift by more than the integer width (panic with overflow checks, silent wrap otherwise).
#[test]
fn forged_bit_width_is_an_error_not_a_panic() {
    let outcomes = within_30_seconds(|| {
        parquet_outcomes(include_bytes!(
            "regressions/parquet_bit_unpack_shift_overflow.bin"
        ))
    });
    assert!(outcomes.iter().all(|(_, refused)| *refused), "{outcomes:?}");
}

/// Fuzz target `parquet_decode` (crash, not overflow-check dependent). Forged definition levels make
/// `arrow-buffer` 59.3.1 `bit_chunk_iterator.rs:224` assert "offset + len out of bounds" while
/// `parquet` appends a packed boolean range. This panics in release builds too.
#[test]
fn forged_definition_levels_are_an_error_not_a_panic() {
    let outcomes = within_30_seconds(|| {
        parquet_outcomes(include_bytes!(
            "regressions/parquet_definition_levels_bounds.bin"
        ))
    });
    assert!(outcomes.iter().all(|(_, refused)| *refused), "{outcomes:?}");
}

/// The page-header walk and the panic containment must not refuse anything the encoders write.
#[test]
fn every_parquet_codec_still_round_trips_after_the_guards() {
    use gqodb_blocks::data::{Kind, Regime, generate};
    for codec in Codec::ALL.into_iter().filter(|c| c.is_parquet()) {
        for kind in Kind::ALL {
            for (regime, rows) in [
                (Regime::Quiet, 0),
                (Regime::Volatile, 1),
                (Regime::Volatile, 70_000),
            ] {
                let data = generate(kind, regime, rows, 11).unwrap();
                let bytes = codec.encode(&data).unwrap();
                assert_eq!(
                    codec.decode(Bytes::from(bytes)).unwrap(),
                    data,
                    "{codec:?} {kind:?} {rows}"
                );
            }
        }
    }
}

/// Fuzz target `parquet_decode` (re-encode check). A Parquet file with two columns of the same name but
/// different types decodes to a valid Arrow batch; encoding it again keyed the per-column encoding by
/// name, so the last column's DELTA_BINARY_PACKED was applied to a Float64 column and
/// `parquet` 59.3.0 `encodings/encoding/mod.rs:531` panicked. The encoder now returns an error.
#[test]
fn duplicate_column_names_with_different_types_are_an_error_not_a_panic() {
    use arrow_array::{ArrayRef, Float64Array, Int64Array, RecordBatch};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;
    let schema = Arc::new(Schema::new(vec![
        Field::new("a", DataType::Float64, false),
        Field::new("a", DataType::Int64, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Float64Array::from(vec![1.5, 2.5])) as ArrayRef,
            Arc::new(Int64Array::from(vec![1, 2])) as ArrayRef,
        ],
    )
    .unwrap();
    for mode in [
        Mode::ParquetLz4,
        Mode::ParquetDictionaryLz4,
        Mode::ParquetSnappy,
        Mode::ParquetBrotli,
    ] {
        assert!(real::encode(&batch, mode).is_err(), "{}", mode.name());
    }
    // Same name, same encoding: still written, as before.
    let same = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("a", DataType::Int64, false),
            Field::new("a", DataType::Int64, false),
        ])),
        vec![
            Arc::new(Int64Array::from(vec![1, 2])) as ArrayRef,
            Arc::new(Int64Array::from(vec![3, 4])) as ArrayRef,
        ],
    )
    .unwrap();
    let bytes = real::encode(&same, Mode::ParquetLz4).unwrap();
    assert!(real::decode(Bytes::from(bytes), Mode::ParquetLz4).is_ok());
}

/// OPEN FINDING, owner decision pending (docs/FUZZING.md, "Open finding"). A ~60 KB block whose
/// dictionary string is 60 000 bytes long and whose column holds 1 048 576 rows of it makes
/// `real::decode` ask for rows x string length = ~60 GB before checking any bound; the checked layers
/// (`gqodb-codec`, `Reader::read_block_bounded`) refuse the same bytes. Ignored because, until a
/// decoded-size cap exists, running it aborts the process. If you run it, run it under a memory
/// limit: `ulimit -v 4000000; cargo test -p gqodb-blocks --test malformed -- --ignored`.
#[test]
#[ignore = "open finding: unchecked real::decode allocates rows x longest dictionary string"]
fn unchecked_decode_must_not_allocate_rows_times_string_length() {
    use arrow_array::{ArrayRef, Int64Array, LargeStringArray, RecordBatch};
    use gqodb_blocks::data::MAX_ROWS;
    use std::sync::Arc;
    let batch = RecordBatch::try_from_iter(vec![
        (
            "t",
            Arc::new(Int64Array::from(vec![5; MAX_ROWS])) as ArrayRef,
        ),
        (
            "sym",
            Arc::new(LargeStringArray::from(vec!["BTC"; MAX_ROWS])) as ArrayRef,
        ),
    ])
    .unwrap();
    let block = real::encode(&batch, Mode::Gqodb).unwrap();
    let meta_len = u32::from_le_bytes(block[8..12].try_into().unwrap()) as usize;
    let meta = String::from_utf8(block[12..12 + meta_len].to_vec()).unwrap();
    let meta = meta.replace("\"BTC\"", &format!("\"{}\"", "A".repeat(60_000)));
    let mut forged = block[..8].to_vec();
    forged.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    forged.extend_from_slice(meta.as_bytes());
    forged.extend_from_slice(&block[12 + meta_len..block.len() - 4]);
    forged = with_crc(forged);
    assert!(real::decode(Bytes::from(forged), Mode::Gqodb).is_err());
}
