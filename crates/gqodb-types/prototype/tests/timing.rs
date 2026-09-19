use gqodb_types::timing::{
    AssessedTime, ClockDomain, ClockReading, ClockState, EventTiming, SourceTime, SourceTimeKind,
    TimeQuality, TimingError, UtcInterval,
};

fn domain() -> ClockDomain {
    ClockDomain {
        session_id: [1; 16],
        epoch: 1,
    }
}
fn state() -> ClockState {
    ClockState {
        domain: domain(),
        id: 42,
        collection_started_mono_ns: 1_000_000_000,
        published_mono_ns: 1_100_000_000,
        valid_until_mono_ns: 6_000_000_000,
        synchronized: true,
        base_error_ns: 20_000,
        growth_allowance_ppb: Some(1000),
    }
}
fn reading() -> ClockReading {
    ClockReading {
        domain: domain(),
        utc_ns: 1_788_868_800_123_456_789,
        monotonic_ns: 2_000_000_001,
        read_span_ns: 50,
    }
}

#[test]
fn estimate_ages_from_collection_and_rounds_up_without_changing_raw_time() {
    let raw = reading();
    let assessed = AssessedTime::assess(raw, Some(&state())).unwrap();
    // 1.000000001 seconds * 1000 ppb -> 1001 ns, plus 20 us and 50 ns bracket.
    assert_eq!(assessed.error_estimate_ns(), Some(21_051));
    assert_eq!(assessed.reading(), raw);
    assert_eq!(assessed.clock_state_id(), Some(42));
    assert_eq!(assessed.quality(), TimeQuality::Estimated);
    assert_eq!(
        assessed.interval(),
        Some(UtcInterval {
            earliest_ns: raw.utc_ns - 21_051,
            latest_ns: raw.utc_ns + 21_051,
        })
    );
}

#[test]
fn evidence_must_exist_before_the_entire_utc_read_bracket() {
    let raw = reading();
    let mut report = state();
    report.published_mono_ns = raw.monotonic_ns - raw.read_span_ns as i64;
    assert_eq!(
        AssessedTime::assess(raw, Some(&report)).unwrap().quality(),
        TimeQuality::Estimated
    );
    report.published_mono_ns += 1;
    let future = AssessedTime::assess(raw, Some(&report)).unwrap();
    assert_eq!(future.quality(), TimeQuality::FutureEvidence);
    assert_eq!(future.clock_state_id(), None);
    assert_eq!(future.interval(), None);
    assert_eq!(future.reading(), raw);
    // Even perfect later evidence cannot qualify a previously received event.
    report.base_error_ns = 1;
    report.growth_allowance_ppb = Some(0);
    assert_eq!(
        AssessedTime::assess(raw, Some(&report)).unwrap().quality(),
        TimeQuality::FutureEvidence
    );
}

#[test]
fn failed_quality_remains_recordable_and_is_never_zero_uncertainty() {
    let raw = reading();
    let mut cases = vec![(None, TimeQuality::MissingEvidence)];
    let mut report = state();
    report.domain.epoch += 1;
    cases.push((Some(report), TimeQuality::DifferentDomain));
    report = state();
    report.domain.session_id = [2; 16];
    cases.push((Some(report), TimeQuality::DifferentDomain));
    report = state();
    report.valid_until_mono_ns = raw.monotonic_ns - 1;
    cases.push((Some(report), TimeQuality::ExpiredEvidence));
    report = state();
    report.synchronized = false;
    cases.push((Some(report), TimeQuality::Unsynchronized));
    report = state();
    report.growth_allowance_ppb = None;
    cases.push((Some(report), TimeQuality::MissingGrowthModel));
    report = state();
    report.collection_started_mono_ns = report.published_mono_ns + 1;
    cases.push((Some(report), TimeQuality::InvalidEvidence));
    report = state();
    report.base_error_ns = 0;
    cases.push((Some(report), TimeQuality::InvalidEvidence));
    for (report, expected) in cases {
        let assessed = AssessedTime::assess(raw, report.as_ref()).unwrap();
        assert_eq!(assessed.quality(), expected);
        assert_eq!(assessed.reading(), raw);
        assert_eq!(assessed.error_estimate_ns(), None);
        assert_eq!(assessed.interval(), None);
        let event = EventTiming::new(None, assessed, assessed, 1).unwrap();
        assert!(!event.available_by_estimated_utc(i64::MAX));
    }
}

#[test]
fn availability_uses_its_own_assessment_and_upper_interval_endpoint() {
    let report = state();
    let received = AssessedTime::assess(reading(), Some(&report)).unwrap();
    let mut ready_raw = reading();
    ready_raw.monotonic_ns += 1000;
    ready_raw.utc_ns += 1000;
    let ready = AssessedTime::assess(ready_raw, Some(&report)).unwrap();
    let broker = SourceTime {
        utc_ns: ready_raw.utc_ns + 5_000_000, // Unsynchronized source can be ahead.
        resolution_ns: 1_000_000,
        kind: SourceTimeKind::BrokerSend,
    };
    let event = EventTiming::new(Some(broker), received, ready, 7).unwrap();
    assert_eq!(event.broker_time(), Some(broker));
    assert_eq!(event.capture_sequence(), 7);
    assert!(!event.available_by_estimated_utc(ready_raw.utc_ns));
    let endpoint = ready.interval().unwrap().latest_ns;
    assert!(!event.available_by_estimated_utc(endpoint - 1));
    assert!(event.available_by_estimated_utc(endpoint));
    ready_raw.monotonic_ns = report.valid_until_mono_ns;
    assert_eq!(
        AssessedTime::assess(ready_raw, Some(&report))
            .unwrap()
            .quality(),
        TimeQuality::Estimated
    );
    ready_raw.monotonic_ns += 1;
    let expired = AssessedTime::assess(ready_raw, Some(&report)).unwrap();
    let event = EventTiming::new(Some(broker), received, expired, 8).unwrap();
    assert_eq!(event.received().quality(), TimeQuality::Estimated);
    assert_eq!(event.available().quality(), TimeQuality::ExpiredEvidence);
    assert!(!event.available_by_estimated_utc(i64::MAX));
}

#[test]
fn epoch_changes_preserve_clock_steps_but_session_and_order_must_match() {
    let received = AssessedTime::assess(reading(), None).unwrap();
    let mut next = reading();
    next.monotonic_ns += 100;
    next.utc_ns -= 1000;
    let ready = AssessedTime::assess(next, None).unwrap();
    assert_eq!(
        EventTiming::new(None, received, ready, 1),
        Err(TimingError::UnmarkedWallclockRegression)
    );
    next.domain.epoch += 1;
    let ready = AssessedTime::assess(next, Some(&state())).unwrap();
    let event = EventTiming::new(None, received, ready, 1).unwrap();
    assert_eq!(event.available().reading().utc_ns, next.utc_ns);
    assert_eq!(event.available().quality(), TimeQuality::DifferentDomain);
    assert_eq!(
        EventTiming::new(None, ready, received, 1),
        Err(TimingError::MonotonicRegression)
    );
    next.domain.session_id = [2; 16];
    assert_eq!(
        EventTiming::new(None, received, AssessedTime::assess(next, None).unwrap(), 1),
        Err(TimingError::DifferentSession)
    );
}

#[test]
fn arithmetic_extremes_cannot_wrap_or_clip_into_a_trusted_interval() {
    for utc_ns in [i64::MIN, i64::MAX] {
        let raw = ClockReading {
            utc_ns,
            ..reading()
        };
        let assessed = AssessedTime::assess(raw, Some(&state())).unwrap();
        assert_eq!(assessed.quality(), TimeQuality::ArithmeticOverflow);
        assert_eq!(assessed.interval(), None);
        assert_eq!(assessed.error_estimate_ns(), None);
        assert_eq!(assessed.reading(), raw);
    }
    let mut report = state();
    report.valid_until_mono_ns = i64::MAX;
    report.base_error_ns = u64::MAX;
    report.growth_allowance_ppb = Some(u64::MAX);
    let raw = ClockReading {
        monotonic_ns: i64::MAX,
        ..reading()
    };
    let assessed = AssessedTime::assess(raw, Some(&report)).unwrap();
    assert_eq!(assessed.quality(), TimeQuality::ArithmeticOverflow);
    assert_eq!(assessed.interval(), None);
}

#[test]
fn malformed_capture_metadata_is_rejected_without_fabricating_source_time() {
    for raw in [
        ClockReading {
            monotonic_ns: -1,
            ..reading()
        },
        ClockReading {
            read_span_ns: u64::MAX,
            ..reading()
        },
        ClockReading {
            domain: ClockDomain {
                session_id: [0; 16],
                epoch: 1,
            },
            ..reading()
        },
    ] {
        assert_eq!(
            AssessedTime::assess(raw, None),
            Err(TimingError::InvalidReading)
        );
    }
    let received = AssessedTime::assess(reading(), None).unwrap();
    assert_eq!(
        EventTiming::new(None, received, received, 0),
        Err(TimingError::InvalidSequence)
    );
    assert_eq!(
        EventTiming::new(
            Some(SourceTime {
                utc_ns: 0,
                resolution_ns: 0,
                kind: SourceTimeKind::Other
            }),
            received,
            received,
            1
        ),
        Err(TimingError::InvalidSourceResolution)
    );
    let event = EventTiming::new(None, received, received, 1).unwrap();
    assert_eq!(event.broker_time(), None);
}
