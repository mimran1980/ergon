//! L3 order book codecs — generated from schemas/l3-book.xml.

#[allow(dead_code, unused_imports, unused_variables, clippy::all)]
mod l3_codec {
    include!(concat!(env!("OUT_DIR"), "/l3_codec.rs"));
}
pub use l3_codec::*;

const TS: u64 = 1_720_000_000_000_000_000;

/// Encode an L3 order book into `buf`. Returns the header-inclusive encoded length.
///
/// Uses known-size group construction — counts are known from slice lengths,
/// so no back-patching is needed.
pub fn encode_book(
    buf: &mut [u8],
    bids: &[(Decimal, Decimal, &[(u64, Decimal)])],
    asks: &[(Decimal, Decimal, &[(u64, Decimal)])],
    symbol: &[u8],
) -> Result<usize, sbe_rt::EncodeError> {
    let complete = L3BookEncoder::wrap_and_apply_header(buf, 0)?
        .fixed(&L3BookFixedFields {
            exchange_timestamp: TS,
            sequence: 42,
            is_active: BooleanType::True,
        })
        .bids(bids.len() as u16, |g| {
            for (price, size, orders) in bids {
                g.add(|e| {
                    e.price(*price).size(*size);
                    e.orders(orders.len() as u16, |og| {
                        for (oid, qty) in *orders {
                            og.add_struct(&BidsOrdersEntry {
                                order_id: *oid,
                                quantity: *qty,
                            })?;
                        }
                        Ok(())
                    })?;
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .asks(asks.len() as u16, |g| {
            for (price, size, orders) in asks {
                g.add(|e| {
                    e.price(*price).size(*size);
                    e.orders(orders.len() as u16, |og| {
                        for (oid, qty) in *orders {
                            og.add_struct(&AsksOrdersEntry {
                                order_id: *oid,
                                quantity: *qty,
                            })?;
                        }
                        Ok(())
                    })?;
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .symbol(symbol)?;
    Ok(complete.encoded_length_with_header())
}
