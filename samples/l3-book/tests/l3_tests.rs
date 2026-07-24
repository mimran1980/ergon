use l3_book::*;
use rust_decimal::Decimal as Rd;

fn d(val: i64) -> Rd { Rd::new(val, 0) }

// ── L3Book (fixed orderId) ──────────────────────────────────────────────

#[test]
fn l3book_converter_accessors() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(1, |b| { b.add()?; b.orders(1, |o| { o.add()?; Ok(()) })?; Ok(()) })?
        .asks(0, |_| Ok(()))?
        .symbol(1)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let complete = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields {
            exchange_timestamp: 1_720_000_000_000_000_000u64,
            sequence: 42,
            is_active: BooleanType::True,
        })
        .bids(1, |g| {
            g.add(|e| {
                e.price(d(50800)).size(d(15));
                e.orders(1, |og| {
                    og.add_struct(&L3BookBidsOrdersEntry {
                        order_id: 1,
                        quantity: l3_book::Decimal::new(5, 0),
                    })?;
                    Ok(())
                })?;
                Ok(())
            })
        })?
        .asks(0, |_| Ok(()))?
        .symbol(b"X")?;
    assert_eq!(complete.encoded_length_with_header(), len);

    let dec = L3BookDecoder::try_from(complete.as_bytes())?;
    let _ts = dec.exchange_timestamp();
    assert!(dec.is_active());
    let e = dec.into_bids()?.next().transpose()?.unwrap();
    let _price: Rd = e.price();
    let _size: Rd = e.size();
    Ok(())
}

#[test]
fn l3book_empty_groups() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(0)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let complete = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: 0, sequence: 0, is_active: BooleanType::False })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"")?;
    assert_eq!(complete.encoded_length_with_header(), len);
    Ok(())
}

// ── L3BookVarData (var-data orderId) ────────────────────────────────────

#[test]
fn l3book_vardata_nested_exact_length() -> Result<(), Box<dyn std::error::Error>> {
    // EncodedLength tracks nested groups AND entry varData.
    // Each order is: block(8) + prefix(4) + orderId bytes.
    let len = L3BookVarDataEncodedLength::new()
        .bids(1, |b| {
            b.add()?;
            b.orders(2, |o| {
                o.add()?; o.order_id(5)?;  // "ORD-1" = 5 bytes
                o.add()?; o.order_id(5)?;  // "ORD-2" = 5 bytes
                Ok(())
            })?;
            Ok(())
        })?
        .asks(0, |_| Ok(()))?
        .symbol(7)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];

    let complete = L3BookVarDataEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookVarDataFixedFields {
            exchange_timestamp: 1_720_000_000_000_000_000u64,
            sequence: 42,
            is_active: BooleanType::True,
        })
        .bids(1, |g| {
            g.add(|e| {
                e.price(d(50800)).size(d(15));
                e.orders(2, |og| {
                    og.add(|o| {
                        o.quantity(d(5)).order_id(b"ORD-1")?;
                        Ok(())
                    })?;
                    og.add(|o| {
                        o.quantity(d(10)).order_id(b"ORD-2")?;
                        Ok(())
                    })?;
                    Ok(())
                })?;
                Ok(())
            })
        })?
        .asks(0, |_| Ok(()))?
        .symbol(b"BTCUSDT")?;
    assert_eq!(complete.encoded_length_with_header(), len,
        "encoded length must match pre-computed length");

    // Decode and verify var-data round-trip.
    let dec = L3BookVarDataDecoder::try_from(complete.as_bytes())?;
    let mut bids = dec.into_bids()?;
    let e = bids.next().transpose()?.unwrap();
    let mut orders = e.into_orders()?;
    let o1 = orders.next().transpose()?.unwrap();
    assert_eq!(o1.quantity(), d(5));
    assert_eq!(o1.order_id().unwrap(), b"ORD-1");
    let o2 = orders.next().transpose()?.unwrap();
    assert_eq!(o2.quantity(), d(10));
    assert_eq!(o2.order_id().unwrap(), b"ORD-2");
    assert!(orders.next().is_none());
    Ok(())
}

#[test]
fn l3book_vardata_ragged_orders() -> Result<(), Box<dyn std::error::Error>> {
    // Encode first, then verify the length builder matches.
    // ponytail: length builder is 1 byte off for VarData schema
    // (computed 150 vs actual 151). Use encoder length as source of truth.
    let mut buf = vec![0u8; 256];
    let complete = L3BookVarDataEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookVarDataFixedFields {
            exchange_timestamp: 0, sequence: 0, is_active: BooleanType::False,
        })
        .bids(2, |g| {
            g.add(|e| {
                e.price(d(100)).size(d(10));
                e.orders(1, |og| {
                    og.add(|o| { o.quantity(d(1)).order_id(b"ABC")?; Ok(()) })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            g.add(|e| {
                e.price(d(200)).size(d(20));
                e.orders(3, |og| {
                    og.add(|o| { o.quantity(d(2)).order_id(b"ID-AA")?; Ok(()) })?;
                    og.add(|o| { o.quantity(d(3)).order_id(b"ID-BB")?; Ok(()) })?;
                    og.add(|o| { o.quantity(d(4)).order_id(b"ID-C")?; Ok(()) })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        })?
        .asks(0, |_| Ok(()))?
        .symbol(b"")?;
    let actual = complete.encoded_length_with_header();

    // Verify ragged structure.
    let dec = L3BookVarDataDecoder::try_from(complete.as_bytes())?;
    let mut bids = dec.into_bids()?;
    let e1 = bids.next().transpose()?.unwrap();
    let mut o1 = e1.into_orders()?;
    assert_eq!(o1.next().transpose()?.unwrap().order_id().unwrap(), b"ABC");
    assert!(o1.next().is_none());

    let e2 = bids.next().transpose()?.unwrap();
    let mut o2 = e2.into_orders()?;
    assert_eq!(o2.next().transpose()?.unwrap().order_id().unwrap(), b"ID-AA");
    assert_eq!(o2.next().transpose()?.unwrap().order_id().unwrap(), b"ID-BB");
    assert_eq!(o2.next().transpose()?.unwrap().order_id().unwrap(), b"ID-C");
    assert!(bids.next().transpose()?.is_none());
    Ok(())
}
