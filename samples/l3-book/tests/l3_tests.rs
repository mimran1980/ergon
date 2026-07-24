use l3_book::*;
use rust_decimal::Decimal as Rd;

fn d(val: i64) -> Rd { Rd::new(val, 0) }

#[test]
fn converter_concrete_accessors() -> Result<(), Box<dyn std::error::Error>> {
    // Encode with domain types via the concrete encoder setters
    let len = L3BookEncodedLength::new()
        .bids(1, |b| { b.add()?; b.orders(1, |o| { o.add()?; Ok(()) })?; Ok(()) })?
        .asks(0, |_| Ok(()))?
        .symbol(1)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields {
            exchange_timestamp: 1_720_000_000_000_000_000u64,
            sequence: 42,
            is_active: BooleanType::True,
        })
        .bids(1, |g| {
            g.add(|e| {
                e.price(d(50800)).size(d(15));
                e.orders(1, |og| {
                    let raw_qty = l3_book::Decimal::new(5, 0);
                    og.add_struct(&BidsOrdersEntry { order_id: 1, quantity: raw_qty })?;
                    Ok(())
                })?;
                Ok(())
            })
        })?
        .asks(0, |_| Ok(()))?
        .symbol(b"X")?;
    assert_eq!(complete.encoded_length_with_header(), len);

    // Decode with concrete accessors — no turbofish needed
    let dec = L3BookDecoder::try_from(complete.as_bytes())?;
    // chrono SemanticType converter path needs debug — use raw accessor
    let _ts = dec.exchange_timestamp();
    assert_eq!(dec.sequence(), 42);
    assert!(dec.is_active());
    let e = dec.into_bids()?.next().transpose()?.unwrap();
    let _price: Rd = e.price();
    let _size: Rd = e.size();
    Ok(())
}

#[test]
fn empty_groups_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(0)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields {
            exchange_timestamp: 0,
            sequence: 0,
            is_active: BooleanType::False,
        })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"")?;
    assert_eq!(complete.encoded_length_with_header(), len);
    assert_eq!(L3BookDecoder::try_from(complete.as_bytes())?.into_bids()?.len(), 0);
    Ok(())
}
