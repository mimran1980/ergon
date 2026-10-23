//! The `NewOrder` frame (`schemas/bench-new-order.xml`, shaped like
//! ergon-stack's `trading.xml`) and the arms timed against it.
//!
//! Shared by `benches/encode_bench.rs`, `benches/decode_bench.rs`,
//! `benches/latency_distribution.rs` and `tests/fairness_policy_test.rs`, so
//! the benchmarks time exactly the bodies the test proves equal.
//!
//! | Arm | Work |
//! |---|---|
//! | [`encode_fixed`] | header, `fixed(&NewOrderFixedFields)`, `asset` setter |
//! | [`encode_from`] | header, `encode_from(&Order)` through `NewOrderSource` |
//! | [`read_mapped`] | `side()`, `price()`, `qty()` into the application types |
//! | [`read_raw`] | `raw_side()`, `price_value()`, `qty_value()` mantissas |

#![allow(clippy::all, clippy::pedantic, clippy::restriction)]

use crate::new_order::{
    Decimal9, NewOrderEncoder, NewOrderFixedFields, NewOrderSource, Side, sbe_rt::EncodeError,
};
use crate::new_order_mapped::{NewOrderDecoder, sbe_rt::MappingError};

/// The application side `new_order_mapped` maps the schema's `Side` to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppSide {
    /// `Side::Buy`.
    Buy,
    /// `Side::Sell`.
    Sell,
}

/// The application price `new_order_mapped` maps `Decimal9` to (the
/// constant exponent is not on the wire).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppPrice {
    /// Units of 10^-9.
    pub mantissa: i64,
}

/// An order as the engine holds it: a flag for the side and plain
/// mantissas, as ergon-stack's engine does before it encodes.
#[derive(Clone, Copy, Debug)]
pub struct Order<'a> {
    /// Wall-clock send time, ns.
    pub ts: u64,
    /// The market-data tick it answers, ns.
    pub tick_ts: u64,
    /// Engine-assigned id.
    pub order_id: u64,
    /// `true` buys, `false` sells.
    pub buy: bool,
    /// Price mantissa, 10^-9.
    pub price: i64,
    /// Quantity mantissa, 10^-9.
    pub qty: i64,
    /// Asset name, the one var-data field.
    pub asset: &'a [u8],
}

/// A buy order.
pub const BUY: Order<'static> = Order {
    ts: 1_761_177_600_123_456_789,
    tick_ts: 1_761_177_600_123_400_000,
    order_id: 42,
    buy: true,
    price: 67_123_450_000_000,
    qty: 1_500_000,
    asset: b"BTCUSDT",
};

/// A sell order, so both enum values are covered.
pub const SELL: Order<'static> = Order {
    ts: 1_761_177_600_223_456_789,
    tick_ts: 1_761_177_600_223_400_000,
    order_id: 43,
    buy: false,
    price: 67_120_000_000_000,
    qty: 2_000_000,
    asset: b"BTCUSDT",
};

/// Header 8 + fixed block 41 + length prefix 4 + `"BTCUSDT"`.
pub const FRAME_LEN: usize = 8 + 41 + 4 + 7;

impl NewOrderSource for Order<'_> {
    #[inline]
    fn ts(&self) -> u64 {
        self.ts
    }
    #[inline]
    fn tick_ts(&self) -> u64 {
        self.tick_ts
    }
    #[inline]
    fn order_id(&self) -> u64 {
        self.order_id
    }
    #[inline]
    fn side(&self) -> Side {
        if self.buy { Side::Buy } else { Side::Sell }
    }
    #[inline]
    fn price(&self) -> Decimal9 {
        Decimal9::new(self.price)
    }
    #[inline]
    fn qty(&self) -> Decimal9 {
        Decimal9::new(self.qty)
    }
    #[inline]
    fn asset(&self) -> &[u8] {
        self.asset
    }
}

/// Encode `order` at offset 0 as ergon-stack's engine does: header,
/// `fixed()`, then the `asset` setter. Returns the frame length.
#[inline]
pub fn encode_fixed(buf: &mut [u8], order: &Order<'_>) -> Result<usize, EncodeError> {
    Ok(NewOrderEncoder::wrap_and_apply_header(buf, 0)
        .fixed(&NewOrderFixedFields {
            ts: order.ts,
            tick_ts: order.tick_ts,
            order_id: order.order_id,
            side: if order.buy { Side::Buy } else { Side::Sell },
            price: Decimal9::new(order.price),
            qty: Decimal9::new(order.qty),
        })
        .asset(order.asset)?
        .encoded_length_with_header())
}

/// Encode `order` at offset 0 through its `NewOrderSource` getters:
/// header, then `encode_from`. Returns the frame length.
#[inline]
pub fn encode_from(buf: &mut [u8], order: &Order<'_>) -> Result<usize, EncodeError> {
    Ok(NewOrderEncoder::wrap_and_apply_header(buf, 0)
        .encode_from(order)?
        .encoded_length_with_header())
}

/// Side, price and quantity through the mapped accessors.
#[inline]
pub fn read_mapped(
    decoder: &NewOrderDecoder<'_>,
) -> Result<(AppSide, AppPrice, AppPrice), MappingError> {
    Ok((decoder.side()?, decoder.price()?, decoder.qty()?))
}

/// Side, price and quantity as the wire holds them: the enum's `u8` and the
/// two mantissas.
#[inline]
pub fn read_raw(decoder: &NewOrderDecoder<'_>) -> (u8, i64, i64) {
    (
        decoder.raw_side(),
        decoder.price_value().mantissa(),
        decoder.qty_value().mantissa(),
    )
}
