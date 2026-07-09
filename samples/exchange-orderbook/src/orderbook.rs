//! Local orderbook built from exchange SBE messages.
//!
//! Uses newtype wrappers with custom `Ord` so the ordering is encoded in the
//! type, not the container. `BidLevel` sorts highest price first; `AskLevel`
//! sorts lowest price first. Prices are transparent `Decimal` values.

use rust_decimal::Decimal;
use std::cmp::Ordering;
use std::collections::BTreeSet;

/// A bid level — highest price first.
#[derive(Debug, Clone, Copy)]
pub struct BidLevel {
    pub price: Decimal,
    #[allow(dead_code)]
    pub size: Decimal,
}

impl PartialEq for BidLevel {
    fn eq(&self, other: &Self) -> bool {
        self.price == other.price
    }
}
impl Eq for BidLevel {}
impl PartialOrd for BidLevel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for BidLevel {
    fn cmp(&self, other: &Self) -> Ordering {
        // Highest price = best bid = smallest in BTreeSet ordering
        other.price.cmp(&self.price)
    }
}

/// An ask level — lowest price first.
#[derive(Debug, Clone, Copy)]
pub struct AskLevel {
    pub price: Decimal,
    #[allow(dead_code)]
    pub size: Decimal,
}

impl PartialEq for AskLevel {
    fn eq(&self, other: &Self) -> bool {
        self.price == other.price
    }
}
impl Eq for AskLevel {}
impl PartialOrd for AskLevel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for AskLevel {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lowest price = best ask = smallest in BTreeSet ordering
        self.price.cmp(&other.price)
    }
}

/// A single exchange's orderbook.
#[derive(Debug, Clone)]
pub struct LocalBook {
    pub symbol: String,
    pub bids: BTreeSet<BidLevel>,
    pub asks: BTreeSet<AskLevel>,
    pub price_exponent: i8,
    pub size_exponent: i8,
}

impl LocalBook {
    pub fn new(symbol: impl Into<String>, price_exponent: i8, size_exponent: i8) -> Self {
        Self {
            symbol: symbol.into(),
            bids: BTreeSet::new(),
            asks: BTreeSet::new(),
            price_exponent,
            size_exponent,
        }
    }

    pub fn price_dec(&self, mantissa: i64) -> Decimal {
        Decimal::from_i128_with_scale(mantissa as i128, (-self.price_exponent) as u32)
    }

    pub fn size_dec(&self, mantissa: i64) -> Decimal {
        Decimal::from_i128_with_scale(mantissa as i128, (-self.size_exponent) as u32)
    }

    pub fn apply_snapshot(
        &mut self,
        bids: impl IntoIterator<Item = (i64, i64)>,
        asks: impl IntoIterator<Item = (i64, i64)>,
    ) {
        self.bids.clear();
        for (price, size) in bids {
            let p = self.price_dec(price);
            let s = self.size_dec(size);
            if s > Decimal::ZERO {
                self.bids.insert(BidLevel { price: p, size: s });
            }
        }
        self.asks.clear();
        for (price, size) in asks {
            let p = self.price_dec(price);
            let s = self.size_dec(size);
            if s > Decimal::ZERO {
                self.asks.insert(AskLevel { price: p, size: s });
            }
        }
    }

    #[allow(dead_code)]
    pub fn top_bids(&self, n: usize) -> Vec<BidLevel> {
        self.bids.iter().take(n).copied().collect()
    }

    #[allow(dead_code)]
    pub fn top_asks(&self, n: usize) -> Vec<AskLevel> {
        self.asks.iter().take(n).copied().collect()
    }

    pub fn best_bid(&self) -> Option<Decimal> {
        // BTreeSet is sorted lowest-first per our Ord; first is best bid
        self.bids.first().map(|l| l.price)
    }

    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.first().map(|l| l.price)
    }

    /// Apply a snapshot using Decimal prices directly (for JSON-based feeds).
    pub fn apply_snapshot_dec(
        &mut self,
        bids: impl IntoIterator<Item = (Decimal, Decimal)>,
        asks: impl IntoIterator<Item = (Decimal, Decimal)>,
    ) {
        self.bids.clear();
        for (p, s) in bids {
            if s > Decimal::ZERO {
                self.bids.insert(BidLevel { price: p, size: s });
            }
        }
        self.asks.clear();
        for (p, s) in asks {
            if s > Decimal::ZERO {
                self.asks.insert(AskLevel { price: p, size: s });
            }
        }
    }
}
