//! Shared timing for ticks, books and other events. No clock reads or host I/O.
//!
//! Inputs are descriptions, not authenticated evidence. The caller must obtain
//! them from its trusted clock provider. Validated outputs only implement
//! Serialize: importing them requires reconstructing and checking their inputs.
use serde::{Deserialize, Serialize};

/// A unique capture-session identifier, bound to one host and boot in its
/// persisted manifest. Never reuse it across capture sessions or machines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockDomain {
    pub session_id: [u8; 16],
    pub epoch: u64,
}

impl ClockDomain {
    fn valid(self) -> bool {
        self.session_id != [0; 16] && self.epoch != 0
    }
}

/// Monotonic time is read at the END of the bracket surrounding the UTC read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockReading {
    pub domain: ClockDomain,
    pub utc_ns: i64,
    pub monotonic_ns: i64,
    pub read_span_ns: u64,
}

impl ClockReading {
    fn valid(self) -> bool {
        self.domain.valid()
            && self.monotonic_ns >= 0
            && self.read_span_ns <= self.monotonic_ns as u64
    }
}

/// One immutable, persisted provider observation. IDs are unique in a domain.
///
/// The base estimate applies at collection start. The growth allowance MUST
/// cover uncertainty, collection duration and possible slew changes. It is a
/// caller-qualified model, not automatically chrony's reported `Skew`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockState {
    pub domain: ClockDomain,
    pub id: u64,
    pub collection_started_mono_ns: i64,
    pub published_mono_ns: i64,
    pub valid_until_mono_ns: i64,
    pub synchronized: bool,
    pub base_error_ns: u64,
    pub growth_allowance_ppb: Option<u64>,
}

impl ClockState {
    fn valid(self) -> bool {
        self.domain.valid()
            && self.id != 0
            && self.collection_started_mono_ns >= 0
            && self.published_mono_ns >= self.collection_started_mono_ns
            && self.valid_until_mono_ns > self.published_mono_ns
            && self.base_error_ns > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeQuality {
    Estimated,
    MissingEvidence,
    InvalidEvidence,
    DifferentDomain,
    FutureEvidence,
    ExpiredEvidence,
    Unsynchronized,
    MissingGrowthModel,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct UtcInterval {
    pub earliest_ns: i64,
    pub latest_ns: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingError {
    InvalidReading,
    InvalidSourceResolution,
    InvalidSequence,
    DifferentSession,
    MonotonicRegression,
    EpochRegression,
    UnmarkedWallclockRegression,
}

impl std::fmt::Display for TimingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidReading => "invalid clock reading or domain",
            Self::InvalidSourceResolution => "source resolution must be positive",
            Self::InvalidSequence => "capture sequence must be positive",
            Self::DifferentSession => "receive and availability use different sessions",
            Self::MonotonicRegression => "availability precedes receipt",
            Self::EpochRegression => "availability clock epoch regresses",
            Self::UnmarkedWallclockRegression => "wallclock regression requires a new epoch",
        })
    }
}
impl std::error::Error for TimingError {}

/// UTC stays unmodified even when evidence fails. An absent interval is never
/// equivalent to a zero error estimate. This is an estimated, not physical bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AssessedTime {
    reading: ClockReading,
    clock_state_id: Option<u64>,
    quality: TimeQuality,
    error_estimate_ns: Option<u64>,
    interval: Option<UtcInterval>,
}

impl AssessedTime {
    pub fn assess(reading: ClockReading, state: Option<&ClockState>) -> Result<Self, TimingError> {
        if !reading.valid() {
            return Err(TimingError::InvalidReading);
        }
        let mut result = Self {
            reading,
            clock_state_id: None,
            quality: TimeQuality::MissingEvidence,
            error_estimate_ns: None,
            interval: None,
        };
        let Some(state) = state else {
            return Ok(result);
        };
        if !state.valid() {
            result.quality = TimeQuality::InvalidEvidence;
            return Ok(result);
        }
        if state.domain != reading.domain {
            result.quality = TimeQuality::DifferentDomain;
            return Ok(result);
        }
        // A publication during the read bracket might postdate the UTC read.
        // Only a state already available before the bracket can qualify it.
        let bracket_start = reading.monotonic_ns - reading.read_span_ns as i64;
        if state.published_mono_ns > bracket_start {
            result.quality = TimeQuality::FutureEvidence;
            return Ok(result);
        }
        result.clock_state_id = Some(state.id);
        if reading.monotonic_ns > state.valid_until_mono_ns {
            result.quality = TimeQuality::ExpiredEvidence;
            return Ok(result);
        }
        if !state.synchronized {
            result.quality = TimeQuality::Unsynchronized;
            return Ok(result);
        }
        let Some(growth) = state.growth_allowance_ppb else {
            result.quality = TimeQuality::MissingGrowthModel;
            return Ok(result);
        };
        let age = (reading.monotonic_ns - state.collection_started_mono_ns) as u128;
        let aging = (age * u128::from(growth)).div_ceil(1_000_000_000);
        let total = u128::from(state.base_error_ns) + aging + u128::from(reading.read_span_ns);
        // Calculate in i128; an interval wider than the stored i64 UTC domain
        // cannot be presented as a valid bound by clipping its endpoints.
        let error = u64::try_from(total).ok();
        let interval = error.and_then(|error| {
            Some(UtcInterval {
                earliest_ns: i64::try_from(i128::from(reading.utc_ns) - i128::from(error)).ok()?,
                latest_ns: i64::try_from(i128::from(reading.utc_ns) + i128::from(error)).ok()?,
            })
        });
        if interval.is_none() {
            result.quality = TimeQuality::ArithmeticOverflow;
            return Ok(result);
        }
        result.quality = TimeQuality::Estimated;
        result.error_estimate_ns = error;
        result.interval = interval;
        Ok(result)
    }

    pub fn reading(&self) -> ClockReading {
        self.reading
    }
    pub fn clock_state_id(&self) -> Option<u64> {
        self.clock_state_id
    }
    pub fn quality(&self) -> TimeQuality {
        self.quality
    }
    pub fn error_estimate_ns(&self) -> Option<u64> {
        self.error_estimate_ns
    }
    pub fn interval(&self) -> Option<UtcInterval> {
        self.interval
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTimeKind {
    ExchangeEvent,
    BrokerSend,
    Other,
}

/// Only construct when the source contract permits normalization to UTC.
/// Preserve the original representation and source identity in raw/metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceTime {
    pub utc_ns: i64,
    pub resolution_ns: u64,
    pub kind: SourceTimeKind,
}

/// Shared companion metadata for any tick, book or news payload. Independent
/// receive/availability assessments prevent reusing an expired receive estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EventTiming {
    schema_version: u32,
    broker_time: Option<SourceTime>,
    received: AssessedTime,
    available: AssessedTime,
    capture_sequence: u64,
}

impl EventTiming {
    pub fn new(
        broker_time: Option<SourceTime>,
        received: AssessedTime,
        available: AssessedTime,
        capture_sequence: u64,
    ) -> Result<Self, TimingError> {
        if broker_time.is_some_and(|source| source.resolution_ns == 0) {
            return Err(TimingError::InvalidSourceResolution);
        }
        if capture_sequence == 0 {
            return Err(TimingError::InvalidSequence);
        }
        if received.reading.domain.session_id != available.reading.domain.session_id {
            return Err(TimingError::DifferentSession);
        }
        if available.reading.monotonic_ns < received.reading.monotonic_ns {
            return Err(TimingError::MonotonicRegression);
        }
        if available.reading.domain.epoch < received.reading.domain.epoch {
            return Err(TimingError::EpochRegression);
        }
        if available.reading.domain.epoch == received.reading.domain.epoch
            && available.reading.utc_ns < received.reading.utc_ns
        {
            return Err(TimingError::UnmarkedWallclockRegression);
        }
        Ok(Self {
            schema_version: 1,
            broker_time,
            received,
            available,
            capture_sequence,
        })
    }

    pub fn broker_time(&self) -> Option<SourceTime> {
        self.broker_time
    }
    pub fn received(&self) -> AssessedTime {
        self.received
    }
    pub fn available(&self) -> AssessedTime {
        self.available
    }
    pub fn capture_sequence(&self) -> u64 {
        self.capture_sequence
    }

    /// Conservative filter under the stated estimate assumptions. Stream order
    /// and dependency checks remain the replay driver's responsibility.
    pub fn available_by_estimated_utc(&self, cutoff_ns: i64) -> bool {
        self.available
            .interval
            .is_some_and(|interval| interval.latest_ns <= cutoff_ns)
    }
}
