//! Deterministic integer fixtures. L2 fixtures are records, not exchange-valid books.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_ROWS: usize = 1_048_576;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Quote = 1,
    Trade = 2,
    L2 = 3,
}

impl Kind {
    pub const ALL: [Self; 3] = [Self::Quote, Self::Trade, Self::L2];

    pub fn from_id(id: u8) -> Result<Self> {
        match id {
            1 => Ok(Self::Quote),
            2 => Ok(Self::Trade),
            3 => Ok(Self::L2),
            _ => anyhow::bail!("unknown schema ID: {id}"),
        }
    }

    pub fn names(self) -> [&'static str; 8] {
        let tail = match self {
            Self::Quote => ["bid", "ask", "bid_quantity", "ask_quantity"],
            Self::Trade => ["trade_id", "price", "quantity", "side"],
            Self::L2 => ["price", "quantity", "side", "action"],
        };
        [
            "exchange_ns",
            "receive_ns",
            "ingest_sequence",
            "source_sequence",
            tail[0],
            tail[1],
            tail[2],
            tail[3],
        ]
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Regime {
    Quiet,
    Volatile,
}

impl Regime {
    pub const ALL: [Self; 2] = [Self::Quiet, Self::Volatile];
}

/// Schema v1 fixes synthetic venue/instrument/session labels and decimal scales.
/// These blocks are fixtures, not an arbitrary real-world ingestion format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Columns {
    pub kind: Kind,
    pub values: Vec<Vec<i64>>,
}

impl Columns {
    pub fn rows(&self) -> usize {
        self.values.first().map_or(0, Vec::len)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(self.values.len() == 8, "schema requires eight columns");
        ensure!(self.rows() <= MAX_ROWS, "too many rows");
        ensure!(
            self.values.iter().all(|v| v.len() == self.rows()),
            "ragged columns"
        );
        Ok(())
    }

    pub fn raw_bytes(&self) -> usize {
        self.rows() * self.values.len() * 8
    }

    pub fn digest(&self) -> String {
        let mut hash = Sha256::new();
        hash.update([self.kind as u8]);
        hash.update((self.rows() as u64).to_le_bytes());
        for col in &self.values {
            for value in col {
                hash.update(value.to_le_bytes());
            }
        }
        format!("{:x}", hash.finalize())
    }
}

/// SplitMix64; fixed algorithm makes datasets reproducible across platforms.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn draw(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

pub fn generate(kind: Kind, regime: Regime, rows: usize, seed: u64) -> Result<Columns> {
    ensure!(rows <= MAX_ROWS, "too many rows");
    let mut rng = Rng::new(seed);
    let mut result = Columns {
        kind,
        values: (0..8).map(|_| Vec::with_capacity(rows)).collect(),
    };
    let mut receive = 1_780_000_000_000_000_000_i64;
    let mut price = 6_000_000_i64;
    for row in 0..rows {
        let volatile = matches!(regime, Regime::Volatile);
        let step = if volatile {
            (rng.draw() % 2_000_001) as i64
        } else {
            1_000_000
        };
        receive += step;
        let late = if volatile && row % 97 == 0 {
            10_000_000
        } else {
            0
        };
        let exchange = receive
            - 100_000
            - late
            - if volatile {
                (rng.draw() % 200_000) as i64
            } else {
                0
            };
        let exchange = if row > 0 && row % 13 == 0 {
            result.values[0][row - 1]
        } else {
            exchange
        };
        price += if volatile {
            (rng.draw() % 2001) as i64 - 1000
        } else {
            (rng.draw() % 3) as i64 - 1
        };
        let qty = if volatile {
            (rng.draw() % 1_000_000) as i64
        } else {
            (rng.draw() % 8 + 1) as i64 * 100
        };
        let side = (rng.draw() & 1) as i64;
        let source_seq = if volatile { row % 32_768 } else { row } as i64;
        let tail = match kind {
            Kind::Quote => [
                price,
                price
                    + if volatile {
                        (rng.draw() % 50 + 1) as i64
                    } else {
                        2
                    },
                qty,
                qty + (rng.draw() % 8) as i64,
            ],
            Kind::Trade => [row as i64 + 5_000_000, price, qty, side],
            Kind::L2 => [
                price + (rng.draw() % if volatile { 2000 } else { 20 }) as i64,
                if row % 11 == 0 { 0 } else { qty },
                side,
                if row % 11 == 0 { 2 } else { 1 },
            ],
        };
        let values = [
            exchange, receive, row as i64, source_seq, tail[0], tail[1], tail[2], tail[3],
        ];
        for (col, value) in result.values.iter_mut().zip(values) {
            col.push(value);
        }
    }
    result.validate()?;
    Ok(result)
}
