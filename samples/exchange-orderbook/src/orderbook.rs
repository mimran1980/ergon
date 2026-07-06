//! Local orderbook built from exchange SBE messages.
//!
//! Maintains separate bid/ask books as BTreeMaps with `rust_decimal::Decimal`
//! prices and sizes. Exponents from the SBE message are applied on ingest.

use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// A single exchange's orderbook.
#[derive(Debug, Clone)]
pub struct LocalBook {
    /// Instrument symbol, e.g. "BTCUSDT".
    pub symbol: String,
    /// Bids: price → size, descending (best bid first via `rev()`).
    pub bids: BTreeMap<Decimal, Decimal>,
    /// Asks: price → size, ascending (best ask first).
    pub asks: BTreeMap<Decimal, Decimal>,
    /// Price mantissa exponent: actual_price = mantissa × 10^exponent.
    price_exponent: i8,
    /// Size mantissa exponent.
    size_exponent: i8,
}

impl LocalBook {
    pub fn new(symbol: impl Into<String>, price_exponent: i8, size_exponent: i8) -> Self {
        Self {
            symbol: symbol.into(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            price_exponent,
            size_exponent,
        }
    }

    /// Convert a mantissa to a Decimal using the stored price exponent.
    pub fn price_dec(&self, mantissa: i64) -> Decimal {
        Decimal::from_i128_with_scale(mantissa as i128, (-self.price_exponent) as u32)
    }

    /// Convert a mantissa to a Decimal using the stored size exponent.
    pub fn size_dec(&self, mantissa: i64) -> Decimal {
        Decimal::from_i128_with_scale(mantissa as i128, (-self.size_exponent) as u32)
    }

    /// Apply a full depth snapshot, replacing the current book.
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
                self.bids.insert(p, s);
            }
        }
        self.asks.clear();
        for (price, size) in asks {
            let p = self.price_dec(price);
            let s = self.size_dec(size);
            if s > Decimal::ZERO {
                self.asks.insert(p, s);
            }
        }
    }

    /// Top N bid levels, best first.
    pub fn top_bids(&self, n: usize) -> Vec<(Decimal, Decimal)> {
        self.bids
            .iter()
            .rev()
            .take(n)
            .map(|(p, s)| (*p, *s))
            .collect()
    }

    /// Top N ask levels, best first.
    pub fn top_asks(&self, n: usize) -> Vec<(Decimal, Decimal)> {
        self.asks.iter().take(n).map(|(p, s)| (*p, *s)).collect()
    }

    /// Best bid price, if any.
    pub fn best_bid(&self) -> Option<Decimal> {
        self.bids.keys().last().copied()
    }

    /// Best ask price, if any.
    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.keys().next().copied()
    }
}
