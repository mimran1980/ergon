use l3_book::*;
const T: u64 = 1_720_000_000_000_000_000;

#[test]
fn fixed_fields_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncoder::compute_encoded_length_with_message_header(0, 0, 0);
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], b"")?;
    let dec = L3BookDecoder::try_from(&buf[..])?;
    assert_eq!(dec.exchange_timestamp(), T);
    assert_eq!(dec.sequence(), 42);
    Ok(())
}

#[test]
fn nested_orders_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let o1 = [(1001u64, 5u64, 50800i64), (1002, 10, 50801)];
    let o2 = [(1003u64, 25u64, 50750i64)];
    let o3 = [(2001u64, 10u64, 50850i64)];
    let bids = [(50800i64, 15i64, o1.as_slice()), (50750, 40, o2.as_slice())];
    let asks = [(50850i64, 20i64, o3.as_slice())];
    let mut buf = vec![0u8; 4096];
    let actual = encode_book(&mut buf, &bids, &asks, b"BTCUSDT")?;
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    let mut b = dec.into_bids()?;
    assert_eq!(b.len(), 2);
    for (i, (price, size, orders)) in bids.iter().enumerate() {
        let e = b.next().transpose()?.unwrap();
        assert_eq!(e.price(), *price);
        assert_eq!(e.size(), *size);
        let mut og = e.into_orders()?;
        for (j, (oid, qty, oprice)) in orders.iter().enumerate() {
            let oe = og.next().unwrap();
            assert_eq!(oe.order_id(), *oid);
            assert_eq!(oe.quantity(), *qty);
            assert_eq!(oe.price(), *oprice);
        }
        assert!(og.next().is_none());
        let _ = og.finish()?;
    }
    assert!(b.next().transpose()?.is_none());
    let ab = b.finish()?;
    let mut a = ab.into_asks()?;
    let e = a.next().transpose()?.unwrap();
    assert_eq!(e.price(), 50850);
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
    let len = L3BookEncoder::compute_encoded_length_with_message_header(0, 0, 7);
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], b"BTCUSDT")?;
    let d = format!("{}", L3BookDecoder::try_from(&buf[..])?);
    assert!(d.contains("1720000000000000000"));
    assert!(d.contains("sequence: 42"));
    Ok(())
}

#[test]
fn verify_ok() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncoder::compute_encoded_length_with_message_header(0, 0, 0);
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], b"")?;
    assert!(L3BookDecoder::verify(&buf[..]).is_ok());
    Ok(())
}

#[test]
fn verify_truncated_fails() { assert!(L3BookDecoder::verify(&[0u8; 4]).is_err()); }

#[test]
fn empty_groups() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncoder::compute_encoded_length_with_message_header(0, 0, 0);
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], b"")?;
    let dec = L3BookDecoder::try_from(&buf[..])?;
    let mut b = dec.into_bids()?;
    assert_eq!(b.len(), 0);
    assert!(b.next().transpose()?.is_none());
    Ok(())
}

#[test]
fn decoder_individual_field_accessors() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncoder::compute_encoded_length_with_message_header(0, 0, 0);
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], b"")?;
    let dec = L3BookDecoder::try_from(&buf[..])?;
    assert_eq!(dec.exchange_timestamp(), T);
    assert_eq!(dec.sequence(), 42);
    assert_eq!(dec.acting_version(), 0);
    let _ = dec.acting_block_length();
    Ok(())
}

#[test]
fn encoder_as_bytes_and_encoded_length() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 256];
    let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1 })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"XYZ")?;
    assert!(complete.encoded_length() > 0);
    assert!(complete.as_bytes().len() == complete.encoded_length());
    Ok(())
}

#[test]
fn max_value_fields_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 256];
    let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: u64::MAX, sequence: u64::MAX })
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
    let mut buf = vec![0u8; 256];
    let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: 0, sequence: 0 })
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
    let s: &[u8] = &[0u8; 2];
    assert!(L3BookDecoder::try_from(s).is_err());
    let s: &[u8] = &[0u8; 12];
    assert!(L3BookDecoder::try_from(s).is_err());
}

#[test]
fn rewind_returns_initial_decoder() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncoder::compute_encoded_length_with_message_header(0, 0, 0);
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], b"")?;
    let dec = L3BookDecoder::try_from(&buf[..])?;
    let ts = dec.exchange_timestamp();
    assert_eq!(dec.rewind().exchange_timestamp(), ts);
    Ok(())
}

#[test]
fn skip_remaining_on_group() -> Result<(), Box<dyn std::error::Error>> {
    let o = [(1u64, 2u64, 50000i64), (3, 4, 50001), (5, 6, 50002)];
    let bids = [(50000i64, 10i64, o.as_slice())];
    let mut buf = vec![0u8; 4096];
    encode_book(&mut buf, &bids, &[], b"X")?;
    let dec = L3BookDecoder::try_from(&buf[..])?;
    let after = dec.into_bids()?.skip_remaining()?;
    assert_eq!(after.into_asks()?.len(), 0);
    Ok(())
}

#[test]
fn group_len_accessor() -> Result<(), Box<dyn std::error::Error>> {
    let o = [(1u64, 2u64, 3i64)];
    let bids = [(1i64, 2i64, o.as_slice()), (3, 4, o.as_slice()), (5, 6, o.as_slice())];
    let mut buf = vec![0u8; 4096];
    encode_book(&mut buf, &bids, &[], b"X")?;
    let dec = L3BookDecoder::try_from(&buf[..])?;
    assert_eq!(dec.into_bids()?.len(), 3);
    Ok(())
}

#[test]
fn complete_stage_as_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 256];
    let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 5 })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"HI")?;
    assert!(complete.as_bytes().len() >= 36);
    Ok(())
}

#[test]
fn two_encodes_different_data() -> Result<(), Box<dyn std::error::Error>> {
    let mut b1 = vec![0u8; 256]; let mut b2 = vec![0u8; 256];
    let c1 = L3BookEncoder::wrap_and_apply_header(&mut b1, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: 1000, sequence: 1 })
        .bids(0, |_| Ok(()))?
        .asks(0, |_| Ok(()))?
        .symbol(b"A")?;
    let c2 = L3BookEncoder::wrap_and_apply_header(&mut b2, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: 2000, sequence: 2 })
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
    let o = [(1u64, 2u64, -500i64)];
    let bids = [(-100i64, 5i64, o.as_slice())];
    let asks = [(-200i64, 3i64, o.as_slice())];
    let mut buf = vec![0u8; 4096];
    let actual = encode_book(&mut buf, &bids, &asks, b"OIL")?;
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    let e = dec.into_bids()?.next().transpose()?.unwrap();
    assert_eq!(e.price(), -100);
    let o = e.into_orders()?.next().unwrap();
    assert_eq!(o.price(), -500);
    Ok(())
}

#[test]
fn large_order_count() -> Result<(), Box<dyn std::error::Error>> {
    let orders: Vec<(u64, u64, i64)> = (0..50).map(|i| (i, i * 2, 50000 + i as i64)).collect();
    let bids = [(50000i64, 100i64, orders.as_slice())];
    let mut buf = vec![0u8; 8192];
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
    let mut buf = vec![0u8; 256];
    let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1 })
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
    let len = L3BookEncoder::compute_encoded_length_with_message_header(0, 0, sym.len());
    let mut buf = vec![0u8; len];
    encode_book(&mut buf, &[], &[], sym)?;
    let (got, _) = L3BookDecoder::try_from(&buf[..])?.into_bids()?.finish()?.into_asks()?.finish()?.into_symbol()?;
    assert_eq!(got, sym);
    Ok(())
}

#[test]
fn dto_reencode_byte_identical() -> Result<(), Box<dyn std::error::Error>> {
    let o1 = [(1001u64, 5u64, 50800i64)]; let o2 = [(1003u64, 25u64, 50750i64)];
    let o3 = [(2001u64, 10u64, 50850i64)]; let o4 = [(2002u64, 20u64, 50900i64)];
    let bids = [(50800, 15, o1.as_slice()), (50750, 40, o2.as_slice())];
    let asks = [(50850, 20, o3.as_slice()), (50900, 30, o4.as_slice())];
    let symbol = b"BTCUSDT";
    let mut buf = vec![0u8; 4096];
    let actual = encode_book(&mut buf, &bids, &asks, symbol)?;

    #[derive(Debug, Clone)] struct O { oid: u64, qty: u64, price: i64 }
    #[derive(Debug, Clone)] struct L { price: i64, size: i64, orders: Vec<O> }

    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    let mut db = Vec::new(); let mut b = dec.into_bids()?;
    while let Some(e) = b.next().transpose()? { let price = e.price(); let size = e.size();
        let mut orders = Vec::new(); let mut og = e.into_orders()?;
        while let Some(oe) = og.next() { orders.push(O { oid: oe.order_id(), qty: oe.quantity(), price: oe.price() }); }
        let _ = og.finish()?; db.push(L { price, size, orders });
    }
    let ab = b.finish()?; let mut da = Vec::new(); let mut a = ab.into_asks()?;
    while let Some(e) = a.next().transpose()? { let price = e.price(); let size = e.size();
        let mut orders = Vec::new(); let mut og = e.into_orders()?;
        while let Some(oe) = og.next() { orders.push(O { oid: oe.order_id(), qty: oe.quantity(), price: oe.price() }); }
        let _ = og.finish()?; da.push(L { price, size, orders });
    }

    let mut re_buf = vec![0u8; 4096];
    let ab = L3BookEncoder::wrap_and_apply_header(&mut re_buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 42 })
        .bids(db.len() as u16, |g| {
        for lvl in &db { g.add(|e| { e.price(lvl.price).size(lvl.size);
            e.orders(lvl.orders.len() as u16, |og| {
                for o in &lvl.orders { og.add(|oe| { oe.order_id(o.oid).quantity(o.qty).price(o.price); Ok(()) })?; }
                Ok(())
            })?; Ok(())
        })?; } Ok(())
    })?;
    let aa = ab.asks(da.len() as u16, |g| {
        for lvl in &da { g.add(|e| { e.price(lvl.price).size(lvl.size);
            e.orders(lvl.orders.len() as u16, |og| {
                for o in &lvl.orders { og.add(|oe| { oe.order_id(o.oid).quantity(o.qty).price(o.price); Ok(()) })?; }
                Ok(())
            })?; Ok(())
        })?; } Ok(())
    })?;
    assert_eq!(aa.symbol(symbol)?.as_bytes(), &buf[..actual]);
    Ok(())
}

// ── API combination tests ────────────────────────────────────────────

#[test]
fn explicit_count_with_add_per_field() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 4096];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1 })
        .bids(1, |g| {
            g.add(|e| { e.price(100).size(10); Ok(()) })
        })?
        .asks(0, |g| Ok(()))?
        .symbol(b"X")?
        .encoded_length();
    assert!(actual > 0);
    Ok(())
}

#[test]
fn unknown_size_with_add_per_field() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 4096];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1 })
        .bids_unknown_size(|g| {
            g.add(|e| { e.price(100).size(10); Ok(()) })
        })?
        .asks_unknown_size(|g| Ok(()))?
        .symbol(b"X")?
        .encoded_length();
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    assert_eq!(dec.into_bids()?.len(), 1);
    Ok(())
}

#[test]
fn explicit_count_with_add_struct_for_pure_fixed_nested() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 4096];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1 })
        .bids(1, |g| {
            g.add(|e| {
                e.price(100).size(10);
                e.orders(2, |og| {
                    og.add_struct(&BidsOrdersEntry { order_id: 1, quantity: 5, price: 100 })?;
                    og.add_struct(&BidsOrdersEntry { order_id: 2, quantity: 3, price: 101 })?;
                    Ok(())
                })?;
                Ok(())
            })
        })?
        .asks(0, |g| Ok(()))?
        .symbol(b"X")?
        .encoded_length();
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
    let mut buf = vec![0u8; 4096];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1 })
        .bids_unknown_size(|g| {
            for i in 0..3 {
                g.add(|e| {
                    e.price(100 + i * 10).size(10 + i);
                    e.orders_unknown_size(|og| {
                        og.add_struct(&BidsOrdersEntry { order_id: i as u64, quantity: 5, price: 100 + i * 10 })?;
                        Ok(())
                    })?;
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .asks_unknown_size(|g| Ok(()))?
        .symbol(b"X")?
        .encoded_length();
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    assert_eq!(dec.into_bids()?.len(), 3);
    Ok(())
}

#[test]
fn mixed_known_and_unknown_across_groups() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 4096];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1 })
        .bids(2, |g| {
            // explicit count for outer, _unknown_size for inner
            g.add(|e| {
                e.price(100).size(10);
                e.orders_unknown_size(|og| {
                    og.add_struct(&BidsOrdersEntry { order_id: 1, quantity: 1, price: 100 })?;
                    og.add_struct(&BidsOrdersEntry { order_id: 2, quantity: 2, price: 101 })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            g.add(|e| {
                e.price(200).size(20);
                e.orders(1, |og| {
                    og.add_struct(&BidsOrdersEntry { order_id: 3, quantity: 3, price: 200 })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        })?
        .asks_unknown_size(|g| Ok(()))?
        .symbol(b"XY")?
        .encoded_length();
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
    // Encode same data with add_struct and with per-field add
    let data = [(1u64, 2u64, 100i64), (3, 4, 101), (5, 6, 102)];

    let mut buf1 = vec![0u8; 4096];
    let len1 = L3BookEncoder::wrap_and_apply_header(&mut buf1, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1 })
        .bids(0, |g| Ok(()))?
        .asks(1, |g| {
            g.add(|e| {
                e.price(500).size(100);
                e.orders(3, |og| {
                    for (id, qty, price) in &data {
                        og.add_struct(&AsksOrdersEntry { order_id: *id, quantity: *qty, price: *price })?;
                    }
                    Ok(())
                })?;
                Ok(())
            })
        })?
        .symbol(b"T")?
        .encoded_length();

    let mut buf2 = vec![0u8; 4096];
    let len2 = L3BookEncoder::wrap_and_apply_header(&mut buf2, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1 })
        .bids(0, |g| Ok(()))?
        .asks(1, |g| {
            g.add(|e| {
                e.price(500).size(100);
                e.orders(3, |og| {
                    for (id, qty, price) in &data {
                        og.add(|oe| { oe.order_id(*id).quantity(*qty).price(*price); Ok(()) })?;
                    }
                    Ok(())
                })?;
                Ok(())
            })
        })?
        .symbol(b"T")?
        .encoded_length();

    assert_eq!(len1, len2);
    assert_eq!(&buf1[..len1], &buf2[..len2]);
    Ok(())
}

#[test]
fn zero_entries_with_unknown_size() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 4096];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1 })
        .bids_unknown_size(|g| Ok(()))?
        .asks_unknown_size(|g| Ok(()))?
        .symbol(b"")?
        .encoded_length();
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    assert_eq!(dec.into_bids()?.len(), 0);
    Ok(())
}

#[test]
fn many_entries_with_unknown_size() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 8192];
    let actual = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 1 })
        .bids_unknown_size(|g| {
            for i in 0..100 {
                g.add(|e| { e.price(i).size(i * 2); Ok(()) })?;
            }
            Ok(())
        })?
        .asks_unknown_size(|g| {
            for i in 0..50 {
                g.add(|e| { e.price(i + 1000).size(i * 3); Ok(()) })?;
            }
            Ok(())
        })?
        .symbol(b"MANY")?
        .encoded_length();
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    assert_eq!(dec.into_bids()?.len(), 100);
    Ok(())
}
