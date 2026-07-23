//! L3 order book codecs — generated from schemas/l3-book.xml.

#[allow(dead_code, unused_imports, unused_variables, clippy::all)]
mod l3_codec {
    include!(concat!(env!("OUT_DIR"), "/l3_codec.rs"));
}
pub use l3_codec::*;

const TS: u64 = 1_720_000_000_000_000_000;

pub fn encode_book(
    buf: &mut [u8],
    bids: &[(i64, i64, &[(u64, u64, i64)])],
    asks: &[(i64, i64, &[(u64, u64, i64)])],
    symbol: &[u8],
) -> Result<usize, sbe_rt::EncodeError> {
    let after_bids = L3BookEncoder::wrap_and_apply_header(buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: TS, sequence: 42 })
        .bids_unknown_size(|g| -> Result<(), sbe_rt::EncodeError> {
        for (price, size, orders) in bids {
            g.add(|e| {
                e.price(*price).size(*size);
                e.orders(orders.len() as u16, |og| -> Result<(), sbe_rt::EncodeError> {
                    for (oid, qty, o_price) in *orders {
                        og.add(|oe| -> Result<(), sbe_rt::EncodeError> { oe.order_id(*oid).quantity(*qty).price(*o_price); Ok(()) })?;
                    }
                    Ok(())
                })?;
                Ok(())
            })?;
        }
        Ok(())
    })?;

    let after_asks = after_bids.asks_unknown_size(|g| -> Result<(), sbe_rt::EncodeError> {
        for (price, size, orders) in asks {
            g.add(|e| {
                e.price(*price).size(*size);
                e.orders(orders.len() as u16, |og| -> Result<(), sbe_rt::EncodeError> {
                    for (oid, qty, o_price) in *orders {
                        og.add(|oe| -> Result<(), sbe_rt::EncodeError> { oe.order_id(*oid).quantity(*qty).price(*o_price); Ok(()) })?;
                    }
                    Ok(())
                })?;
                Ok(())
            })?;
        }
        Ok(())
    })?;

    let complete = after_asks.symbol(symbol)?;
    Ok(complete.encoded_length())
}
