//! Experimental lossless block formats and benchmark controls, not a database.
//!
//! This is a 0.x preview crate: no stability promise, and the `real` block formats
//! have no compatibility guarantee beyond what the tests pin. Much of this crate is
//! benchmark and feasibility code (see `docs/API_STABILITY.md` in the repository).
//!
//! The entry point for Arrow data is [`real::encode`] / [`real::decode`]: a round
//! trip either restores the batch exactly (schema, metadata, values, nulls) or
//! returns an error.
//!
//! # Example
//!
//! ```
//! use arrow_array::{ArrayRef, Int64Array, LargeStringArray, RecordBatch};
//! use bytes::Bytes;
//! use gqodb_blocks::real::{self, Mode};
//! use std::sync::Arc;
//!
//! let batch = RecordBatch::try_from_iter(vec![
//!     ("price", Arc::new(Int64Array::from(vec![Some(10_050), None, Some(10_051)])) as ArrayRef),
//!     ("symbol", Arc::new(LargeStringArray::from(vec!["SYNTH", "SYNTH", "SYNTH"])) as ArrayRef),
//! ])
//! .unwrap();
//!
//! let bytes = real::encode(&batch, Mode::GqodbBook).unwrap();
//! assert_eq!(&bytes[..8], b"GQOREAL4"); // the header, not a file extension, names the format
//! let decoded = real::decode(Bytes::from(bytes), Mode::GqodbBook).unwrap();
//! assert!(real::equal(&batch, &decoded)); // exact, including the null
//! ```
//!
//! The fixed eight-column integer fixtures used by the benchmarks go through
//! [`Codec`]:
//!
//! ```
//! use bytes::Bytes;
//! use gqodb_blocks::{
//!     Codec,
//!     data::{generate, Kind, Regime},
//! };
//!
//! let columns = generate(Kind::Trade, Regime::Quiet, 256, 7).unwrap();
//! let bytes = Codec::AdaptiveSimdLz4.encode(&columns).unwrap();
//! let decoded = Codec::AdaptiveSimdLz4.decode(Bytes::from(bytes)).unwrap();
//! assert_eq!(columns, decoded);
//! ```
#![forbid(unsafe_code)]

pub mod adaptive;
pub mod batch;
pub mod block;
mod book;
pub mod data;
pub mod market;
pub mod parquet;
pub mod real;

use anyhow::Result;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Codec {
    ColumnLz4,
    DeltaVarintLz4,
    DeltaBitpackLz4,
    AdaptiveSimd,
    AdaptiveSimdLz4,
    ParquetPlainLz4,
    ParquetDeltaLz4,
    ParquetDictionaryLz4,
    ParquetDeltaSnappy,
    ParquetDeltaBrotli,
}

impl Codec {
    pub const ALL: [Self; 10] = [
        Self::ColumnLz4,
        Self::DeltaVarintLz4,
        Self::DeltaBitpackLz4,
        Self::AdaptiveSimd,
        Self::AdaptiveSimdLz4,
        Self::ParquetPlainLz4,
        Self::ParquetDeltaLz4,
        Self::ParquetDictionaryLz4,
        Self::ParquetDeltaSnappy,
        Self::ParquetDeltaBrotli,
    ];

    fn parquet_mode(self) -> Option<parquet::ParquetMode> {
        use parquet::ParquetMode as P;
        match self {
            Self::ParquetPlainLz4 => Some(P::PlainLz4),
            Self::ParquetDeltaLz4 => Some(P::DeltaLz4),
            Self::ParquetDictionaryLz4 => Some(P::DictionaryLz4),
            Self::ParquetDeltaSnappy => Some(P::DeltaSnappy),
            Self::ParquetDeltaBrotli => Some(P::DeltaBrotli),
            _ => None,
        }
    }

    pub fn is_parquet(self) -> bool {
        self.parquet_mode().is_some()
    }

    pub fn encode(self, data: &data::Columns) -> Result<Vec<u8>> {
        if matches!(self, Self::AdaptiveSimd | Self::AdaptiveSimdLz4) {
            return adaptive::encode(data, matches!(self, Self::AdaptiveSimdLz4));
        }
        if let Some(mode) = self.parquet_mode() {
            return parquet::encode(data, mode);
        }
        let layout = match self {
            Self::ColumnLz4 => block::Layout::Raw,
            Self::DeltaVarintLz4 => block::Layout::Varint,
            _ => block::Layout::Bitpack,
        };
        block::encode(data, layout)
    }

    pub fn decode(self, input: Bytes) -> Result<data::Columns> {
        if matches!(self, Self::AdaptiveSimd | Self::AdaptiveSimdLz4) {
            return adaptive::decode(&input);
        }
        if self.is_parquet() {
            parquet::decode(input)
        } else {
            block::decode(&input)
        }
    }

    pub fn framing_bytes(self, input: Bytes) -> Result<usize> {
        if matches!(self, Self::AdaptiveSimd | Self::AdaptiveSimdLz4) {
            return Ok(adaptive::framing_bytes());
        }
        if self.is_parquet() {
            parquet::framing_bytes(input)
        } else {
            Ok(block::framing_bytes())
        }
    }
}
