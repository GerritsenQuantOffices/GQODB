//! Shared body of the `types_parse` target and its seeds (`gqodb-types`).
//!
//! First input byte: even = parse the rest as JSON into one of the public types and
//! check that a successful parse survives a serialise/parse round trip; odd = build
//! clock readings/states from the remaining bytes (biased towards the arithmetic
//! edges) and check the invariants of `AssessedTime::assess` and `EventTiming::new`.
use arbitrary::{Result as ArbResult, Unstructured};
use gqodb_types::{
    BookEvent, BookId, Change, Coverage, DepthCoverage, Envelope, EventKind, Level, Side,
    timing::{
        AssessedTime, ClockDomain, ClockReading, ClockState, EventTiming, SourceTime,
        SourceTimeKind, TimeQuality,
    },
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub const JSON_TYPES: u8 = 11;

fn reparse<T: DeserializeOwned + Serialize + PartialEq + std::fmt::Debug>(body: &[u8]) {
    if let Ok(v) = serde_json::from_slice::<T>(body) {
        let again = serde_json::to_vec(&v).expect("serialise");
        let back: T = serde_json::from_slice(&again).expect("re-parse of own output");
        assert_eq!(v, back, "serde round trip changed the value");
    }
}

#[derive(Deserialize)]
struct Pair {
    reading: ClockReading,
    state: Option<ClockState>,
}

fn json(index: u8, body: &[u8]) {
    match index % JSON_TYPES {
        0 => reparse::<BookEvent>(body),
        1 => reparse::<Envelope>(body),
        2 => reparse::<EventKind>(body),
        3 => reparse::<Change>(body),
        4 => reparse::<Level>(body),
        5 => reparse::<Coverage>(body),
        6 => reparse::<DepthCoverage>(body),
        7 => reparse::<ClockReading>(body),
        8 => reparse::<ClockState>(body),
        9 => reparse::<SourceTime>(body),
        _ => {
            if let Ok(p) = serde_json::from_slice::<Pair>(body) {
                check_assess(p.reading, p.state.as_ref());
            }
        }
    }
}

fn num_i64(u: &mut Unstructured) -> ArbResult<i64> {
    Ok(match u.int_in_range(0..=4)? {
        0 => [0, 1, -1, i64::MAX, i64::MIN, i64::MAX - 1][u.int_in_range(0..=5usize)?],
        1 => i64::from(u.int_in_range(0..=1000u16)?),
        2 => i64::MAX - i64::from(u.int_in_range(0..=1000u16)?),
        _ => u.arbitrary()?,
    })
}

fn num_u64(u: &mut Unstructured) -> ArbResult<u64> {
    Ok(match u.int_in_range(0..=4)? {
        0 => [0, 1, u64::MAX, u64::MAX - 1, i64::MAX as u64][u.int_in_range(0..=4usize)?],
        1 => u64::from(u.int_in_range(0..=1000u16)?),
        2 => u64::MAX - u64::from(u.int_in_range(0..=1000u16)?),
        _ => u.arbitrary()?,
    })
}

fn domain(u: &mut Unstructured) -> ArbResult<ClockDomain> {
    Ok(ClockDomain {
        session_id: [[0; 16], [1; 16], [2; 16]][u.int_in_range(0..=2usize)?],
        epoch: [0, 1, 1, 2, 3][u.int_in_range(0..=4usize)?],
    })
}

fn reading(u: &mut Unstructured, d: Option<ClockDomain>) -> ArbResult<ClockReading> {
    Ok(ClockReading {
        domain: match d {
            Some(d) => d,
            None => domain(u)?,
        },
        utc_ns: num_i64(u)?,
        monotonic_ns: num_i64(u)?,
        read_span_ns: num_u64(u)?,
    })
}

fn state(u: &mut Unstructured, d: ClockDomain) -> ArbResult<ClockState> {
    Ok(ClockState {
        domain: if u.ratio(7, 8)? { d } else { domain(u)? },
        id: num_u64(u)?,
        collection_started_mono_ns: num_i64(u)?,
        published_mono_ns: num_i64(u)?,
        valid_until_mono_ns: num_i64(u)?,
        synchronized: u.arbitrary()?,
        base_error_ns: num_u64(u)?,
        growth_allowance_ppb: if u.arbitrary()? {
            Some(num_u64(u)?)
        } else {
            None
        },
    })
}

fn check_assess(r: ClockReading, s: Option<&ClockState>) -> Option<AssessedTime> {
    let a = AssessedTime::assess(r, s).ok()?;
    assert_eq!(a.reading(), r);
    match a.quality() {
        TimeQuality::Estimated => {
            let (i, e) = (
                a.interval().expect("interval"),
                a.error_estimate_ns().expect("error"),
            );
            let s = s.expect("estimate needs a state");
            assert!(s.synchronized && s.domain == r.domain && s.growth_allowance_ppb.is_some());
            assert_eq!(a.clock_state_id(), Some(s.id));
            assert!(e >= s.base_error_ns && e >= r.read_span_ns);
            assert!(i.earliest_ns <= r.utc_ns && r.utc_ns <= i.latest_ns);
            assert_eq!(
                i128::from(i.latest_ns) - i128::from(i.earliest_ns),
                2 * i128::from(e),
                "interval must be symmetric around the UTC reading"
            );
        }
        _ => assert!(a.interval().is_none() && a.error_estimate_ns().is_none()),
    }
    Some(a)
}

fn structured(data: &[u8]) -> ArbResult<()> {
    let mut u = Unstructured::new(data);
    let d = domain(&mut u)?;
    let r1 = reading(&mut u, Some(d))?;
    let d2 = if u.ratio(7, 8)? { d } else { domain(&mut u)? };
    let r2 = reading(&mut u, Some(d2))?;
    let s1 = if u.arbitrary()? {
        Some(state(&mut u, d)?)
    } else {
        None
    };
    let s2 = if u.arbitrary()? {
        Some(state(&mut u, r2.domain)?)
    } else {
        None
    };
    let received = check_assess(r1, s1.as_ref());
    let available = check_assess(r2, s2.as_ref());
    let broker = if u.arbitrary()? {
        Some(SourceTime {
            utc_ns: num_i64(&mut u)?,
            resolution_ns: num_u64(&mut u)?,
            kind: [
                SourceTimeKind::ExchangeEvent,
                SourceTimeKind::BrokerSend,
                SourceTimeKind::Other,
            ][u.int_in_range(0..=2usize)?],
        })
    } else {
        None
    };
    let seq = num_u64(&mut u)?;
    if let (Some(rc), Some(av)) = (received, available)
        && let Ok(t) = EventTiming::new(broker, rc, av, seq)
    {
        assert!(seq != 0);
        assert_eq!(
            rc.reading().domain.session_id,
            av.reading().domain.session_id
        );
        assert!(av.reading().monotonic_ns >= rc.reading().monotonic_ns);
        assert!(av.reading().domain.epoch >= rc.reading().domain.epoch);
        let _ = t.available_by_estimated_utc(num_i64(&mut u)?);
        serde_json::to_vec(&t).expect("serialise timing");
    }
    Ok(())
}

pub fn run(data: &[u8]) {
    let Some((&sel, rest)) = data.split_first() else {
        return;
    };
    if sel % 2 == 0 {
        json(sel / 2, rest);
    } else {
        let _ = structured(rest);
    }
}

pub fn seeds() -> Vec<(String, Vec<u8>)> {
    let level = Level {
        side: Side::Bid,
        price: 100,
        quantity: 5,
    };
    let cov = DepthCoverage {
        bids: Coverage::Top { depth: 10 },
        asks: Coverage::Full,
    };
    let env = Envelope {
        book: BookId {
            venue: 1,
            instrument: 2,
            stream: 3,
        },
        epoch: 1,
        ordinal: 7,
        transport_sequence: Some(9),
        application_sequence: None,
        source_time: Some(-5),
        received_at: 10,
        available_at: 12,
        price_decimals: 2,
        quantity_decimals: 0,
    };
    let domain = ClockDomain {
        session_id: [1; 16],
        epoch: 1,
    };
    let reading = ClockReading {
        domain,
        utc_ns: 1_000_000,
        monotonic_ns: 500,
        read_span_ns: 10,
    };
    let state = ClockState {
        domain,
        id: 1,
        collection_started_mono_ns: 0,
        published_mono_ns: 100,
        valid_until_mono_ns: 10_000,
        synchronized: true,
        base_error_ns: 50,
        growth_allowance_ppb: Some(200),
    };
    let ev = BookEvent {
        envelope: env.clone(),
        event: EventKind::Delta {
            changes: vec![
                Change::Set { level },
                Change::Delete {
                    side: Side::Ask,
                    price: 101,
                },
            ],
            coverage: cov,
        },
    };
    let snapshot = EventKind::Snapshot {
        levels: vec![level],
        coverage: DepthCoverage::FULL,
    };
    let source = SourceTime {
        utc_ns: 5,
        resolution_ns: 1000,
        kind: SourceTimeKind::ExchangeEvent,
    };
    let pair = serde_json::json!({"reading": reading, "state": state});
    let docs: Vec<Vec<u8>> = vec![
        serde_json::to_vec(&ev).unwrap(),
        serde_json::to_vec(&env).unwrap(),
        serde_json::to_vec(&snapshot).unwrap(),
        serde_json::to_vec(&Change::Set { level }).unwrap(),
        serde_json::to_vec(&level).unwrap(),
        serde_json::to_vec(&Coverage::Top { depth: 3 }).unwrap(),
        serde_json::to_vec(&cov).unwrap(),
        serde_json::to_vec(&reading).unwrap(),
        serde_json::to_vec(&state).unwrap(),
        serde_json::to_vec(&source).unwrap(),
        serde_json::to_vec(&pair).unwrap(),
    ];
    assert_eq!(docs.len(), usize::from(JSON_TYPES));
    let mut out: Vec<(String, Vec<u8>)> = docs
        .into_iter()
        .enumerate()
        .map(|(i, d)| {
            let mut v = vec![(i as u8) * 2];
            v.extend(d);
            (format!("json-{i}"), v)
        })
        .collect();
    for s in 1..=4u64 {
        let mut v = vec![1u8];
        v.extend(crate::seeds::prng_bytes(s, 200));
        out.push((format!("struct-{s}"), v));
    }
    out
}
