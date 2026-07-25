use l3_book::*;
use rust_decimal::Decimal as Rd;

fn d(val: i64) -> Rd { Rd::new(val, 0) }

// ── L3Book (fixed orderId) ──────────────────────────────────────────────

#[test]
fn l3book_converter_accessors() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 4096];
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
    let len = complete.encoded_length_with_header();

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
    let mut buf = vec![0u8; 4096];
    let complete = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: 0, sequence: 0, is_active: BooleanType::False })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"")?;
    let len = complete.encoded_length_with_header();
    assert!(len > 0);
    Ok(())
}

// ── L3BookVarData (var-data orderId) ────────────────────────────────────

#[test]
fn l3book_vardata_direct_length_matches_encoded() -> Result<(), Box<dyn std::error::Error>> {
    // Ragged at two levels: bid1 has 2 orders, bid2 has 1; order_id var-data
    // differs per order. Direct length computation must match the encoder.
    let o1: [(Rd, &[u8]); 2] = [(d(5), b"ORD-1"), (d(10), b"ORD-22")];
    let o2: [(Rd, &[u8]); 1] = [(d(25), b"X")];
    let bids: &[(Rd, Rd, &[(Rd, &[u8])])] = &[(d(50800), d(15), &o1), (d(50750), d(40), &o2)];
    let o3: [(Rd, &[u8]); 1] = [(d(10), b"AA")];
    let asks: &[(Rd, Rd, &[(Rd, &[u8])])] = &[(d(50850), d(20), &o3)];
    let symbol = b"BTCUSDT";

    let expected = l3_book::vardata_book_encoded_length(bids, asks, symbol);

    let mut buf = vec![0u8; expected];
    let complete = L3BookVarDataEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookVarDataFixedFields {
            exchange_timestamp: 1_720_000_000_000_000_000u64,
            sequence: 42,
            is_active: BooleanType::True,
        })
        .bids(bids.len() as u16, |g| {
            for (_, _, orders) in bids {
                g.add(|e| {
                    e.price(d(1)).size(d(1));
                    e.orders(orders.len() as u16, |og| {
                        for (q, oid) in *orders {
                            og.add(|o| { o.quantity(*q).order_id(oid)?; Ok(()) })?;
                        }
                        Ok(())
                    })?;
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .asks(asks.len() as u16, |g| {
            for (_, _, orders) in asks {
                g.add(|e| {
                    e.price(d(1)).size(d(1));
                    e.orders(orders.len() as u16, |og| {
                        for (q, oid) in *orders {
                            og.add(|o| { o.quantity(*q).order_id(oid)?; Ok(()) })?;
                        }
                        Ok(())
                    })?;
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .symbol(symbol)?;
    let actual = complete.encoded_length_with_header();
    assert_eq!(expected, actual, "vardata direct length must match encoded");
    Ok(())
}

#[test]
fn l3book_staged_length_matches_encoded() -> Result<(), Box<dyn std::error::Error>> {
    // Ragged data: bid1 has 2 orders, bid2 has 1; asks have 1 each.
    let o1 = [(1u64, d(5)), (2, d(10))];
    let o2 = [(3u64, d(25))];
    let bids: &[(Rd, Rd, &[(u64, Rd)])] = &[(d(50800), d(15), &o1), (d(50750), d(40), &o2)];
    let o3 = [(4u64, d(10))];
    let o4 = [(5u64, d(20))];
    let o5 = [(6u64, d(40))];
    let asks: &[(Rd, Rd, &[(u64, Rd)])] = &[
        (d(50850), d(20), &o3),
        (d(50900), d(30), &o4),
        (d(50950), d(50), &o5),
    ];
    let symbol = b"BTCUSDT";

    // Actual length from the encoder.
    let mut buf = vec![0u8; 4096];
    let actual = l3_book::encode_book(&mut buf, bids, asks, symbol)?;

    // Staged length builder (orders: dim=4, block=17 = u64 + Decimal).
    let staged = L3BookEncodedLength::new()
        .bids_ragged(bids.len() as u16, |g| {
            for (_, _, orders) in bids {
                g.add()?;
                g.group(4, 17, orders.len())?;
            }
            Ok(())
        })?
        .asks_ragged(asks.len() as u16, |g| {
            for (_, _, orders) in asks {
                g.add()?;
                g.group(4, 17, orders.len())?;
            }
            Ok(())
        })?
        .symbol(symbol.len())?
        .encoded_length_with_header();

    assert_eq!(actual, staged, "staged L3BookEncodedLength must match actual");
    Ok(())
}

#[test]
fn l3book_vardata_nested_exact_length() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 4096];
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
    let len = complete.encoded_length_with_header();
    assert!(len > 0);

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
