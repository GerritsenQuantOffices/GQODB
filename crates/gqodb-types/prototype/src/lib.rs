//! Protocol-independent, exact L2 order-book events and clock-evidence timing types.
//!
//! This is a 0.x preview crate: no stability promise, breaking changes are possible
//! (see `docs/API_STABILITY.md` in the repository). Prices and quantities are
//! integers with explicit decimal scales in the [`Envelope`]; nothing is rounded.
//! All times in the book types are `i64` nanoseconds on one normalized local clock.
//!
//! The [`timing`] module holds the separate clock-evidence types: it describes how
//! well a timestamp is known and never reads a clock itself.
//!
//! # Example
//!
//! Build a small full-depth snapshot event. A price of `10_050` with
//! `price_decimals = 2` is 100.50; the integer is what is stored.
//!
//! ```
//! use gqodb_types::{
//!     BookEvent, BookId, DepthCoverage, Envelope, EventKind, Level, Side,
//! };
//!
//! let event = BookEvent {
//!     envelope: Envelope {
//!         book: BookId { venue: 1, instrument: 7, stream: 1 },
//!         epoch: 1,
//!         ordinal: 0,
//!         transport_sequence: None,
//!         application_sequence: None,
//!         source_time: None,
//!         received_at: 1_000,
//!         available_at: 1_050,
//!         price_decimals: 2,
//!         quantity_decimals: 0,
//!     },
//!     event: EventKind::Snapshot {
//!         levels: vec![Level { side: Side::Bid, price: 10_050, quantity: 3 }],
//!         coverage: DepthCoverage::FULL,
//!     },
//! };
//! assert_eq!(event.clone(), event);
//! assert!(event.envelope.available_at >= event.envelope.received_at);
//! ```
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

pub mod timing;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookId {
    pub venue: u32,
    pub instrument: u32,
    pub stream: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Level {
    pub side: Side,
    pub price: i64,
    pub quantity: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Coverage {
    Full,
    Top { depth: usize },
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DepthCoverage {
    pub bids: Coverage,
    pub asks: Coverage,
}
impl DepthCoverage {
    pub const FULL: Self = Self {
        bids: Coverage::Full,
        asks: Coverage::Full,
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    Set { level: Level },
    Delete { side: Side, price: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventKind {
    Snapshot {
        levels: Vec<Level>,
        coverage: DepthCoverage,
    },
    Delta {
        changes: Vec<Change>,
        coverage: DepthCoverage,
    },
    Gap,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub book: BookId,
    pub epoch: u64,
    /// Contiguous normalized commit ordinal within this book epoch.
    pub ordinal: u64,
    /// Transport metadata, NOT the per-book ordinal.
    pub transport_sequence: Option<u64>,
    pub application_sequence: Option<u64>,
    pub source_time: Option<i64>,
    pub received_at: i64,
    /// After validation and all dependencies, on the same clock as received_at.
    pub available_at: i64,
    pub price_decimals: u8,
    pub quantity_decimals: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookEvent {
    pub envelope: Envelope,
    pub event: EventKind,
}
