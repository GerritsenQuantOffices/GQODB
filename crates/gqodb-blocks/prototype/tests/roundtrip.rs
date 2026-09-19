use bytes::Bytes;
use gqodb_blocks::{
    Codec, adaptive, block,
    data::{Columns, Kind, Regime, Rng, generate},
};

#[test]
fn all_schemas_regimes_and_group_boundaries() {
    for kind in Kind::ALL {
        for regime in Regime::ALL {
            for rows in [0, 1, 2, 127, 128, 129, 130, 257, 4096] {
                let input = generate(kind, regime, rows, 42).unwrap();
                for codec in Codec::ALL {
                    let encoded = codec.encode(&input).unwrap();
                    assert_eq!(
                        input,
                        codec.decode(Bytes::from(encoded)).unwrap(),
                        "{codec:?}, {kind:?}, {regime:?}, {rows}"
                    );
                }
            }
        }
    }
}

#[test]
fn extrema_and_random_bitpatterns_are_lossless() {
    let mut rng = Rng::new(777);
    let mut v = vec![i64::MIN, i64::MAX, 0, -1, 1, i64::MIN, i64::MIN, i64::MAX];
    v.extend((0..2048).map(|_| rng.draw() as i64));
    let input = Columns {
        kind: Kind::Quote,
        values: vec![v; 8],
    };
    for codec in Codec::ALL {
        assert_eq!(
            input,
            codec
                .decode(Bytes::from(codec.encode(&input).unwrap()))
                .unwrap(),
            "{codec:?}"
        );
    }
}

#[test]
fn constant_and_repeated_timestamps_survive() {
    let input = Columns {
        kind: Kind::L2,
        values: vec![vec![77; 1000]; 8],
    };
    for codec in Codec::ALL {
        assert_eq!(
            input,
            codec
                .decode(Bytes::from(codec.encode(&input).unwrap()))
                .unwrap()
        );
    }
}

#[test]
fn every_truncation_and_corruption_is_rejected() {
    let input = generate(Kind::Trade, Regime::Volatile, 20, 19).unwrap();
    for codec in Codec::ALL {
        let encoded = codec.encode(&input).unwrap();
        for end in 0..encoded.len() {
            assert!(
                codec
                    .decode(Bytes::copy_from_slice(&encoded[..end]))
                    .is_err(),
                "{codec:?} truncation {end}"
            );
        }
        for pos in 0..encoded.len() {
            let mut bad = encoded.clone();
            bad[pos] ^= 1;
            assert!(
                codec.decode(Bytes::from(bad)).is_err(),
                "{codec:?} corruption {pos}"
            );
        }
    }
}

fn rechecksum(bytes: &mut [u8]) {
    let n = bytes.len() - 4;
    let crc = crc32fast::hash(&bytes[..n]);
    bytes[n..].copy_from_slice(&crc.to_le_bytes());
}

#[test]
fn invalid_dimensions_and_lengths_fail_even_with_valid_crc() {
    let data = generate(Kind::Quote, Regime::Quiet, 10, 1).unwrap();
    let good = block::encode(&data, block::Layout::Varint).unwrap();
    for (pos, replacement) in [
        (8, vec![99]),
        (9, vec![99]),
        (10, 9_u16.to_le_bytes().to_vec()),
        (12, u32::MAX.to_le_bytes().to_vec()),
        (16, u32::MAX.to_le_bytes().to_vec()),
        (20, u32::MAX.to_le_bytes().to_vec()),
    ] {
        let mut bad = good.clone();
        bad[pos..pos + replacement.len()].copy_from_slice(&replacement);
        rechecksum(&mut bad);
        assert!(block::decode(&bad).is_err());
    }
    let mut trailing = good.clone();
    trailing.insert(trailing.len() - 4, 0);
    rechecksum(&mut trailing);
    assert!(block::decode(&trailing).is_err());
}

#[test]
fn ragged_inputs_fail_and_generator_is_stable() {
    let mut data = generate(Kind::L2, Regime::Volatile, 100, 9).unwrap();
    assert_eq!(data, generate(Kind::L2, Regime::Volatile, 100, 9).unwrap());
    assert_ne!(data, generate(Kind::L2, Regime::Volatile, 100, 10).unwrap());
    data.values[2].pop();
    for codec in Codec::ALL {
        assert!(codec.encode(&data).is_err());
    }
}

#[test]
fn adaptive_prediction_can_fail_after_sampling_without_losing_information() {
    let mut data = generate(Kind::Quote, Regime::Quiet, 4096, 11).unwrap();
    let mut rng = Rng::new(9101);
    for column in &mut data.values {
        for value in &mut column[1024..] {
            *value = rng.draw() as i64;
        }
        column[2048] = i64::MIN;
        column[2049] = i64::MAX;
    }
    for codec in [Codec::AdaptiveSimd, Codec::AdaptiveSimdLz4] {
        assert_eq!(
            data,
            codec
                .decode(Bytes::from(codec.encode(&data).unwrap()))
                .unwrap()
        );
    }
}

#[test]
fn adaptive_rejects_forged_dimensions_predictors_and_references() {
    let data = generate(Kind::Quote, Regime::Quiet, 100, 11).unwrap();
    let good = adaptive::encode(&data, true).unwrap();
    for (pos, replacement) in [
        (8, vec![2]),
        (9, vec![99]),
        (10, 9_u16.to_le_bytes().to_vec()),
        (12, u32::MAX.to_le_bytes().to_vec()),
        (16, vec![8]),
        (17, vec![99]),
        (18, vec![0]),
        (19, vec![2]),
        (44, 0_u64.to_le_bytes().to_vec()),
        (52, u32::MAX.to_le_bytes().to_vec()),
        (56, u32::MAX.to_le_bytes().to_vec()),
    ] {
        let mut bad = good.clone();
        bad[pos..pos + replacement.len()].copy_from_slice(&replacement);
        rechecksum(&mut bad);
        assert!(adaptive::decode(&bad).is_err(), "offset {pos}");
    }
    let mut bad = good.clone();
    bad[17] = 3;
    bad[18] = 1; // First physical column refers to itself before decoding.
    rechecksum(&mut bad);
    assert!(adaptive::decode(&bad).is_err());
    let mut bad = good.clone();
    bad[60] = bad[16]; // Duplicate physical column in second directory entry.
    rechecksum(&mut bad);
    assert!(adaptive::decode(&bad).is_err());
}
