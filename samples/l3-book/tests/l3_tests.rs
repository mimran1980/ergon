use l3_book::*;
fn d(m: i64, e: i8) -> Decimal { Decimal::new(m, e) }
const T: u64 = 1_720_000_000_000_000_000;

#[test]
fn fixed_fields_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(0)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let actual = encode_book(&mut buf, &[], &[], b"")?;
    assert_eq!(len, actual);
    let dec = L3BookDecoder::try_from(&buf[..])?;
    assert_eq!(dec.exchange_timestamp(), T);
    assert_eq!(dec.sequence(), 42);
    assert!(dec.is_active_bool());
    Ok(())
}

#[test]
fn nested_orders_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let o1 = [(1001u64, d(5, 0)), (1002, d(10, 0))];
    let o2 = [(1003u64, d(25, 0))];
    let o3 = [(2001u64, d(10, 0))];
    let bids = [(d(50800, 0), d(15, 0), o1.as_slice()), (d(50750, 0), d(40, 0), o2.as_slice())];
    let asks = [(d(50850, 0), d(20, 0), o3.as_slice())];
    let len = L3BookEncodedLength::new()
        .bids(bids.len() as u16, |b| {
            for (_, _, orders) in &bids {
                b.add()?;
                b.orders(orders.len() as u16, |o| { for _ in *orders { o.add()?; } Ok(()) })?;
            }
            Ok(())
        })?
        .asks(asks.len() as u16, |a| {
            for (_, _, orders) in &asks {
                a.add()?;
                a.orders(orders.len() as u16, |o| { for _ in *orders { o.add()?; } Ok(()) })?;
            }
            Ok(())
        })?
        .symbol(7)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let actual = encode_book(&mut buf, &bids, &asks, b"BTCUSDT")?;
    assert_eq!(len, actual);
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    let mut b = dec.into_bids()?;
    assert_eq!(b.len(), 2);
    for (price, size, orders) in &bids {
        let e = b.next().transpose()?.unwrap();
        assert_eq!(e.price_value(), *price);
        assert_eq!(e.size_value(), *size);
        let mut og = e.into_orders()?;
        for (oid, qty) in *orders {
            let oe = og.next().unwrap();
            assert_eq!(oe.order_id(), *oid);
            assert_eq!(oe.quantity_value(), *qty);
        }
        assert!(og.next().is_none());
        let _ = og.finish()?;
    }
    assert!(b.next().transpose()?.is_none());
    let ab = b.finish()?;
    let mut a = ab.into_asks()?;
    let e = a.next().transpose()?.unwrap();
    assert_eq!(e.price_value(), d(50850, 0));
    let mut og = e.into_orders()?;
    assert_eq!(og.next().unwrap().order_id(), 2001);
    assert!(og.next().is_none());
    let _ = og.finish()?;
    let aa = a.finish()?;
    let (sym, _) = aa.into_symbol()?;
    assert_eq!(sym, b"BTCUSDT");
    Ok(())
}

#[test]
fn display_contains_values() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(7)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], b"BTCUSDT")?;
    let d = format!("{}", L3BookDecoder::try_from(&buf[..])?);
    assert!(d.contains("1720000000000000000"));
    assert!(d.contains("sequence: 42"));
    Ok(())
}

#[test]
fn verify_ok() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(0)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], b"")?;
    assert!(L3BookDecoder::verify(&buf[..]).is_ok());
    Ok(())
}

#[test]
fn verify_truncated_fails() { assert!(L3BookDecoder::verify(&[0u8; 4]).is_err()); }

#[test]
fn empty_groups() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(0)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], b"")?;
    let dec = L3BookDecoder::try_from(&buf[..])?;
    assert_eq!(dec.into_bids()?.len(), 0);
    Ok(())
}

#[test]
fn decoder_individual_field_accessors() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(0)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], b"")?;
    let dec = L3BookDecoder::try_from(&buf[..])?;
    assert_eq!(dec.exchange_timestamp(), T);
    assert_eq!(dec.sequence(), 42);
    assert_eq!(dec.acting_version(), 0);
    Ok(())
}

#[test]
fn encoder_as_bytes_and_encoded_length() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(3)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1, is_active: BooleanType::True })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"XYZ")?;
    assert_eq!(complete.encoded_length_with_header(), len);
    assert_eq!(complete.as_bytes().len(), len);
    Ok(())
}

#[test]
fn max_value_fields_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(0)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: u64::MAX, sequence: u64::MAX, is_active: BooleanType::False })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"")?;
    let dec = L3BookDecoder::try_from(complete.as_bytes())?;
    assert_eq!(dec.exchange_timestamp(), u64::MAX);
    assert_eq!(dec.sequence(), u64::MAX);
    Ok(())
}

#[test]
fn zero_value_fields_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(0)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: 0, sequence: 0, is_active: BooleanType::False })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"")?;
    let dec = L3BookDecoder::try_from(complete.as_bytes())?;
    assert_eq!(dec.exchange_timestamp(), 0);
    assert_eq!(dec.sequence(), 0);
    Ok(())
}

#[test]
fn try_from_short_buffer() {
    assert!(L3BookDecoder::try_from(&[0u8; 2][..]).is_err());
    assert!(L3BookDecoder::try_from(&[0u8; 12][..]).is_err());
}

#[test]
fn rewind_returns_initial_decoder() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(0)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], b"")?;
    let dec = L3BookDecoder::try_from(&buf[..])?;
    let ts = dec.exchange_timestamp();
    assert_eq!(dec.rewind().exchange_timestamp(), ts);
    Ok(())
}

#[test]
fn skip_remaining_on_group() -> Result<(), Box<dyn std::error::Error>> {
    let o = [(1u64, d(2, 0)), (3, d(4, 0)), (5, d(6, 0))];
    let bids = [(d(50000, 0), d(10, 0), o.as_slice())];
    let len = L3BookEncodedLength::new()
        .bids(bids.len() as u16, |b| {
            b.add()?; b.orders(o.len() as u16, |og| { for _ in &o { og.add()?; } Ok(()) })?;
            Ok(())
        })?
        .asks(0, |_| Ok(()))?
        .symbol(1)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &bids, &[], b"X")?;
    let dec = L3BookDecoder::try_from(&buf[..])?;
    let after = dec.into_bids()?.skip_remaining()?;
    assert_eq!(after.into_asks()?.len(), 0);
    Ok(())
}

#[test]
fn group_len_accessor() -> Result<(), Box<dyn std::error::Error>> {
    let o = [(1u64, d(3, 0))];
    let bids = [(d(1, 0), d(2, 0), o.as_slice()), (d(3, 0), d(4, 0), o.as_slice()), (d(5, 0), d(6, 0), o.as_slice())];
    let len = L3BookEncodedLength::new()
        .bids(bids.len() as u16, |b| {
            for _ in &bids { b.add()?; b.orders(o.len() as u16, |og| { for _ in &o { og.add()?; } Ok(()) })?; }
            Ok(())
        })?
        .asks(0, |_| Ok(()))?
        .symbol(1)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &bids, &[], b"X")?;
    let dec = L3BookDecoder::try_from(&buf[..])?;
    assert_eq!(dec.into_bids()?.len(), 3);
    Ok(())
}

#[test]
fn complete_stage_as_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(2)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 5, is_active: BooleanType::True })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"HI")?;
    assert_eq!(complete.as_bytes().len(), len);
    Ok(())
}

#[test]
fn two_encodes_different_data() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(1)?
        .encoded_length_with_header();
    let mut b1 = vec![0u8; len]; let mut b2 = vec![0u8; len];
    let c1 = L3BookEncoder::wrap_and_apply_header(&mut b1, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: 1000, sequence: 1, is_active: BooleanType::False })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"A")?;
    let c2 = L3BookEncoder::wrap_and_apply_header(&mut b2, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: 2000, sequence: 2, is_active: BooleanType::True })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"B")?;
    assert_ne!(c1.as_bytes(), c2.as_bytes());
    assert_eq!(L3BookDecoder::try_from(c1.as_bytes())?.exchange_timestamp(), 1000);
    assert_eq!(L3BookDecoder::try_from(c2.as_bytes())?.exchange_timestamp(), 2000);
    Ok(())
}

#[test]
fn negative_prices_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let o = [(1u64, d(2, 0))];
    let bids = [(d(-100, 0), d(5, 0), o.as_slice())];
    let asks = [(d(-200, 0), d(3, 0), o.as_slice())];
    let len = L3BookEncodedLength::new()
        .bids(bids.len() as u16, |b| {
            b.add()?; b.orders(o.len() as u16, |og| { og.add()?; Ok(()) })?;
            Ok(())
        })?
        .asks(asks.len() as u16, |a| {
            a.add()?; a.orders(o.len() as u16, |og| { og.add()?; Ok(()) })?;
            Ok(())
        })?
        .symbol(3)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let actual = encode_book(&mut buf, &bids, &asks, b"OIL")?;
    assert_eq!(len, actual);
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    let e = dec.into_bids()?.next().transpose()?.unwrap();
    assert_eq!(e.price_value(), d(-100, 0));
    Ok(())
}

#[test]
fn large_order_count() -> Result<(), Box<dyn std::error::Error>> {
    let orders: Vec<(u64, Decimal)> = (0..50).map(|i| (i as u64, d(i * 2, 0))).collect();
    let bids = [(d(50000, 0), d(100, 0), orders.as_slice())];
    let len = L3BookEncodedLength::new()
        .bids(bids.len() as u16, |b| {
            b.add()?;
            b.orders(orders.len() as u16, |o| { for _ in &orders { o.add()?; } Ok(()) })?;
            Ok(())
        })?
        .asks(0, |_| Ok(()))?
        .symbol(1)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &bids, &[], b"X")?;
    let dec = L3BookDecoder::try_from(&buf[..])?;
    let e = dec.into_bids()?.next().transpose()?.unwrap();
    let mut og = e.into_orders()?;
    let mut c = 0;
    while let Some(_) = og.next() { c += 1; }
    assert_eq!(c, 50);
    Ok(())
}

#[test]
fn empty_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(0)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1, is_active: BooleanType::True })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"")?;
    let dec = L3BookDecoder::try_from(complete.as_bytes())?;
    let (sym, _) = dec.into_bids()?.finish()?.into_asks()?.finish()?.into_symbol()?;
    assert!(sym.is_empty());
    Ok(())
}

#[test]
fn long_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let sym = b"ETHBTC-PERP-2024Q3";
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(sym.len())?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], sym)?;
    let (got, _) = L3BookDecoder::try_from(&buf[..])?.into_bids()?.finish()?.into_asks()?.finish()?.into_symbol()?;
    assert_eq!(got, sym);
    Ok(())
}

#[test]
fn explicit_count_with_add_per_field() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(1, |b| { b.add()?; Ok(()) })?
        .asks(0, |_| Ok(()))?
        .symbol(1)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1, is_active: BooleanType::False })
        .bids(1, |g| { g.add(|e| { e.price(d(100, 0)).size(d(10, 0)); Ok(()) }) })?
        .asks(0, |g| Ok(()))?
        .symbol(b"X")?
        .encoded_length_with_header();
    assert_eq!(len, actual);
    Ok(())
}

#[test]
fn unknown_size_with_add_per_field() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids_unknown_size(|b| { b.add()?; Ok(()) })?
        .asks_unknown_size(|_| Ok(()))?
        .symbol(1)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1, is_active: BooleanType::False })
        .bids_unknown_size(|g| { g.add(|e| { e.price(d(100, 0)).size(d(10, 0)); Ok(()) }) })?
        .asks_unknown_size(|g| Ok(()))?
        .symbol(b"X")?
        .encoded_length_with_header();
    assert_eq!(len, actual);
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    assert_eq!(dec.into_bids()?.len(), 1);
    Ok(())
}

#[test]
fn explicit_count_with_add_struct() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(1, |b| { b.add()?; b.orders(2, |o| { o.add()?; o.add()?; Ok(()) })?; Ok(()) })?
        .asks(0, |_| Ok(()))?
        .symbol(1)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1, is_active: BooleanType::False })
        .bids(1, |g| {
            g.add(|e| {
                e.price(d(100, 0))
                    .size(d(10, 0))
                    .orders(2, |og| {
                        og.add_struct(&BidsOrdersEntry { order_id: 1, quantity: d(5, 0) })?;
                        og.add_struct(&BidsOrdersEntry { order_id: 2, quantity: d(3, 0) })?;
                        Ok(())
                    })?;
                Ok(())
            })
        })?
        .asks(0, |g| Ok(()))?
        .symbol(b"X")?
        .encoded_length_with_header();
    assert_eq!(len, actual);
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    let e = dec.into_bids()?.next().transpose()?.unwrap();
    let mut og = e.into_orders()?;
    assert_eq!(og.next().unwrap().order_id(), 1);
    assert_eq!(og.next().unwrap().order_id(), 2);
    assert!(og.next().is_none());
    Ok(())
}

#[test]
fn unknown_size_outer_with_add_struct_nested() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids_unknown_size(|b| { for _ in 0..3 { b.add()?; b.orders(1, |o| { o.add()?; Ok(()) })?; } Ok(()) })?
        .asks_unknown_size(|_| Ok(()))?
        .symbol(1)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1, is_active: BooleanType::False })
        .bids_unknown_size(|g| {
            for i in 0..3 {
                g.add(|e| {
                    e.price(d(100 + i * 10, 0))
                        .size(d(10 + i, 0))
                        .orders_unknown_size(|og| {
                            og.add_struct(&BidsOrdersEntry { order_id: i as u64, quantity: d(5, 0) })?;
                            Ok(())
                        })?;
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .asks_unknown_size(|g| Ok(()))?
        .symbol(b"X")?
        .encoded_length_with_header();
    assert_eq!(len, actual);
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    assert_eq!(dec.into_bids()?.len(), 3);
    Ok(())
}

#[test]
fn mixed_known_and_unknown_across_groups() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids(2, |b| {
            b.add()?; b.orders_unknown_size(|o| { o.add()?; o.add()?; Ok(()) })?;
            b.add()?; b.orders(1, |o| { o.add()?; Ok(()) })?;
            Ok(())
        })?
        .asks_unknown_size(|_| Ok(()))?
        .symbol(2)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1, is_active: BooleanType::False })
        .bids(2, |g| {
            g.add(|e| {
                e.price(d(100, 0)).size(d(10, 0));
                e.orders_unknown_size(|og| {
                    og.add_struct(&BidsOrdersEntry { order_id: 1, quantity: d(1, 0) })?;
                    og.add_struct(&BidsOrdersEntry { order_id: 2, quantity: d(2, 0) })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            g.add(|e| {
                e.price(d(200, 0))
                    .size(d(20, 0))
                    .orders(1, |og| { og.add_struct(&BidsOrdersEntry { order_id: 3, quantity: d(3, 0) })?; Ok(()) })?;
                Ok(())
            })?;
            Ok(())
        })?
        .asks_unknown_size(|g| Ok(()))?
        .symbol(b"XY")?
        .encoded_length_with_header();
    assert_eq!(len, actual);
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    let mut b = dec.into_bids()?;
    assert_eq!(b.len(), 2);
    let e1 = b.next().transpose()?.unwrap();
    let mut o1 = e1.into_orders()?;
    assert_eq!(o1.next().unwrap().order_id(), 1);
    assert_eq!(o1.next().unwrap().order_id(), 2);
    assert!(o1.next().is_none());
    Ok(())
}

#[test]
fn add_struct_vs_add_per_field_produce_identical_output() -> Result<(), Box<dyn std::error::Error>> {
    let data = [(1u64, d(2, 0)), (3, d(4, 0)), (5, d(6, 0))];
    let len = L3BookEncodedLength::new()
        .bids(0, |_| Ok(()))?
        .asks(1, |a| { a.add()?; a.orders(data.len() as u16, |o| { for _ in &data { o.add()?; } Ok(()) })?; Ok(()) })?
        .symbol(1)?
        .encoded_length_with_header();
    let mut buf1 = vec![0u8; len];
    let len1 = L3BookEncoder::wrap_and_apply_header(&mut buf1, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1, is_active: BooleanType::False })
        .bids(0, |g| Ok(()))?
        .asks(1, |g| {
            g.add(|e| {
                e.price(d(500, 0))
                    .size(d(100, 0))
                    .orders(data.len() as u16, |og| {
                        for (id, qty) in &data {
                            og.add_struct(&AsksOrdersEntry { order_id: *id, quantity: *qty })?;
                        }
                        Ok(())
                    })?;
                Ok(())
            })
        })?
        .symbol(b"T")?
        .encoded_length_with_header();
    let mut buf2 = vec![0u8; len];
    let len2 = L3BookEncoder::wrap_and_apply_header(&mut buf2, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1, is_active: BooleanType::False })
        .bids(0, |g| Ok(()))?
        .asks(1, |g| {
            g.add(|e| {
                e.price(d(500, 0))
                    .size(d(100, 0))
                    .orders(data.len() as u16, |og| {
                        for (id, qty) in &data {
                            og.add(|oe| { oe.order_id(*id).quantity(*qty); Ok(()) })?;
                        }
                        Ok(())
                    })?;
                Ok(())
            })
        })?
        .symbol(b"T")?
        .encoded_length_with_header();
    assert_eq!(len1, len2);
    assert_eq!(len1, len);
    assert_eq!(&buf1[..len1], &buf2[..len2]);
    Ok(())
}

#[test]
fn zero_entries_with_unknown_size() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids_unknown_size(|_| Ok(()))?
        .asks_unknown_size(|_| Ok(()))?
        .symbol(0)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1, is_active: BooleanType::False })
        .bids_unknown_size(|g| Ok(()))?
        .asks_unknown_size(|g| Ok(()))?
        .symbol(b"")?
        .encoded_length_with_header();
    assert_eq!(len, actual);
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    assert_eq!(dec.into_bids()?.len(), 0);
    Ok(())
}

#[test]
fn many_entries_with_unknown_size() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncodedLength::new()
        .bids_unknown_size(|b| { for _ in 0..100 { b.add()?; } Ok(()) })?
        .asks_unknown_size(|a| { for _ in 0..50 { a.add()?; } Ok(()) })?
        .symbol(4)?
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1, is_active: BooleanType::False })
        .bids_unknown_size(|g| {
            for i in 0..100 { g.add(|e| { e.price(d(i, 0)).size(d(i * 2, 0)); Ok(()) })?; }
            Ok(())
        })?
        .asks_unknown_size(|g| {
            for i in 0..50 { g.add(|e| { e.price(d(i + 1000, 0)).size(d(i * 3, 0)); Ok(()) })?; }
            Ok(())
        })?
        .symbol(b"MANY")?
        .encoded_length_with_header();
    assert_eq!(len, actual);
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    assert_eq!(dec.into_bids()?.len(), 100);
    Ok(())
}
