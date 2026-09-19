use arrow_array::{Float64Array, RecordBatch};
use bytes::Bytes;
use gqodb_blocks::{
    batch::Options,
    real::{self, Decoder, Mode},
};
use std::sync::Arc;

#[test]
fn workers_preserve_order_bytes_and_errors() {
    let batches: Vec<_> = (0..7)
        .map(|i| {
            RecordBatch::try_from_iter(vec![(
                "price",
                Arc::new(Float64Array::from(vec![i as f64 * 1.25; 16384 + i]))
                    as arrow_array::ArrayRef,
            )])
            .unwrap()
        })
        .collect();
    let serial = Options::default().encode(&batches, Mode::Gqodb).unwrap();
    for workers in [1, 2, 4, 64] {
        let options = Options { workers };
        assert_eq!(serial, options.encode(&batches, Mode::Gqodb).unwrap());
        let mut blocks: Vec<_> = serial.iter().cloned().map(Bytes::from).collect();
        let decoded = options.decode(&blocks, Mode::Gqodb).unwrap();
        assert!(batches.iter().zip(&decoded).all(|(a, b)| real::equal(a, b)));
        blocks[3] = Bytes::from_static(b"broken");
        assert!(options.decode(&blocks, Mode::Gqodb).is_err());
        assert!(options.decode(&[], Mode::Gqodb).unwrap().is_empty());
    }
    assert!(
        Options { workers: 0 }
            .encode(&batches, Mode::Gqodb)
            .is_err()
    );
    assert!(Options { workers: 65 }.decode(&[], Mode::Gqodb).is_err());

    let mut decoder = Decoder::default();
    for (batch, block) in batches.iter().zip(serial) {
        assert!(real::equal(
            batch,
            &decoder.decode(Bytes::from(block), Mode::Gqodb).unwrap()
        ));
    }
    assert!(decoder.retained_bytes() > 0);
    assert!(
        decoder
            .decode(Bytes::from_static(b"broken"), Mode::Gqodb)
            .is_err()
    );
    let block = real::encode(&batches[0], Mode::Gqodb).unwrap();
    assert!(real::equal(
        &batches[0],
        &decoder.decode(Bytes::from(block), Mode::Gqodb).unwrap()
    ));
    decoder.release();
    assert_eq!(decoder.retained_bytes(), 0);
}
