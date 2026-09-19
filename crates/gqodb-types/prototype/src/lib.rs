//! Protocol-independent, exact L2 events. Times use one normalized local clock.
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
