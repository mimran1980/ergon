use l3_book::*;

const T: u64 = 1_720_000_000_000_000_000;

#[test]
fn encoded_length_matches_buffer_usage() -> Result<(), Box<dyn std::error::Error>> {
    // Use a large buffer; encoded_length() on the complete stage tells exact usage.
    let mut buf = vec![0u8; 4096];
    let bids = [(50i64, 1i64, [].as_slice())];
    let asks = [(60i64, 2i64, [].as_slice())];
    let actual = encode_book(&mut buf, &bids, &asks, b"BTCUSDT")?;
    assert!(actual > 0 && actual <= 512);
    Ok(())
}

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
            assert_eq!(oe.order_id(), *oid, "order {j}");
            assert_eq!(oe.quantity(), *qty);
            assert_eq!(oe.price(), *oprice);
        }
        assert!(og.next().is_none(), "no extra orders for level {i}");
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
fn verify_truncated_fails() {
    assert!(L3BookDecoder::verify(&[0u8; 4]).is_err());
}

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
fn dto_reencode_byte_identical() -> Result<(), Box<dyn std::error::Error>> {
    let o1 = [(1001u64, 5u64, 50800i64)]; let o2 = [(1003u64, 25u64, 50750i64)];
    let o3 = [(2001u64, 10u64, 50850i64)]; let o4 = [(2002u64, 20u64, 50900i64)];
    let bids = [(50800, 15, o1.as_slice()), (50750, 40, o2.as_slice())];
    let asks = [(50850, 20, o3.as_slice()), (50900, 30, o4.as_slice())];
    let symbol = b"BTCUSDT";

    let mut buf = vec![0u8; 4096];
    let actual = encode_book(&mut buf, &bids, &asks, symbol)?;

    #[derive(Debug, Clone)]
    struct O { oid: u64, qty: u64, price: i64 }
    #[derive(Debug, Clone)]
    struct L { price: i64, size: i64, orders: Vec<O> }

    let dec = L3BookDecoder::try_from(&buf[..])?;
    let mut db = Vec::new();
    let mut b = dec.into_bids()?;
    while let Some(e) = b.next().transpose()? { let price = e.price(); let size = e.size(); 
        let mut orders = Vec::new();
        let mut og = e.into_orders()?;
        while let Some(oe) = og.next() { orders.push(O { oid: oe.order_id(), qty: oe.quantity(), price: oe.price() }); }
        let _ = og.finish()?;
        db.push(L { price, size, orders });
    }
    let ab = b.finish()?;
    let mut da = Vec::new();
    let mut a = ab.into_asks()?;
    while let Some(e) = a.next().transpose()? { let price = e.price(); let size = e.size(); 
        let mut orders = Vec::new();
        let mut og = e.into_orders()?;
        while let Some(oe) = og.next() { orders.push(O { oid: oe.order_id(), qty: oe.quantity(), price: oe.price() }); }
        let _ = og.finish()?;
        da.push(L { price, size, orders });
    }

    let re_len = L3BookEncoder::compute_encoded_length_with_message_header(db.len(), da.len(), symbol.len());
    let mut re_buf = vec![0u8; re_len];
    let enc = L3BookEncoder::wrap_and_apply_header(&mut re_buf, 0)?;
    let enc = enc.fixed(&L3BookFixedFields { exchange_timestamp: T, sequence: 42 });
    let ab = enc.bids(db.len() as u16, |g| -> Result<(), sbe_rt::EncodeError> {
        for lvl in &db {
            g.add(|e| {
                e.price(lvl.price); e.size(lvl.size);
                e.orders(lvl.orders.len() as u16, |og| -> Result<(), sbe_rt::EncodeError> {
                    for o in &lvl.orders { og.add(|oe| -> Result<(), sbe_rt::EncodeError> { oe.order_id(o.oid); oe.quantity(o.qty); oe.price(o.price); Ok::<(), sbe_rt::EncodeError>(()) })?; }
                    Ok::<(), sbe_rt::EncodeError>(())
                })?;
                Ok::<(), sbe_rt::EncodeError>(())
            })?;
        }
        Ok::<(), sbe_rt::EncodeError>(())
    })?;
    let aa = ab.asks(da.len() as u16, |g| -> Result<(), sbe_rt::EncodeError> {
        for lvl in &da {
            g.add(|e| {
                e.price(lvl.price); e.size(lvl.size);
                e.orders(lvl.orders.len() as u16, |og| -> Result<(), sbe_rt::EncodeError> {
                    for o in &lvl.orders { og.add(|oe| -> Result<(), sbe_rt::EncodeError> { oe.order_id(o.oid); oe.quantity(o.qty); oe.price(o.price); Ok::<(), sbe_rt::EncodeError>(()) })?; }
                    Ok::<(), sbe_rt::EncodeError>(())
                })?;
                Ok::<(), sbe_rt::EncodeError>(())
            })?;
        }
        Ok::<(), sbe_rt::EncodeError>(())
    })?;
    assert_eq!(aa.symbol(symbol)?.as_bytes(), &buf[..]);
    Ok(())
}
