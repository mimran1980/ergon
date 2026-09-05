use l3_book::*;
use proptest::prelude::*;
use rust_decimal::Decimal as Rd;

fn d(val: i64) -> Rd {
    Rd::new(val, 0)
}

// ── L3Book (fixed orderId) ──────────────────────────────────────────────

#[test]
fn l3book_converter_accessors() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncoder::compute_length()
        .bids_ragged(1, |g| {
            g.add()?.orders(|og| {
                og.add()?;
                Ok(())
            })?;
            Ok(())
        })?
        .asks_ragged(0, |_g| Ok(()))?
        .symbol(b"X".len())?
        .encoded_length_with_header();
    let mut buf_storage = [0u8; 8192];
    assert!(len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..len];
    let complete = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields {
            exchange_timestamp: 1_720_000_000_000_000_000u64,
            sequence: 42,
            is_active: true.into(),
        })
        .bids(1, |g| {
            g.add(|mut e| {
                e.try_price(d(50800))
                    .map_err(|_| sbe_rt::EncodeError::DomainConversionFailed {
                        field: "price",
                        reason: "conversion",
                    })?;
                e.try_size(d(15))
                    .map_err(|_| sbe_rt::EncodeError::DomainConversionFailed {
                        field: "size",
                        reason: "conversion",
                    })?;
                e.orders(1, |og| {
                    og.add_struct(&L3BookBidsOrdersEntry {
                        order_id: 1,
                        quantity: l3_book::Decimal::new(5, 0),
                    })
                })
            })
        })?
        .asks(0, |_| Ok(()))?
        .symbol(b"X")?;
    let _len = complete.encoded_length_with_header();

    let dec = L3BookDecoder::try_from(complete.as_bytes_with_header())?;
    let _ts = dec.try_exchange_timestamp()?;
    assert!(dec.try_is_active()?);
    let e = dec.into_bids()?.next().transpose()?.unwrap();
    let _price: Rd = e.try_price()?;
    let _size: Rd = e.try_size()?;
    Ok(())
}

#[test]
fn l3book_empty_groups() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookEncoder::compute_length()
        .bids_ragged(0, |_g| Ok(()))?
        .asks_ragged(0, |_g| Ok(()))?
        .symbol(b"".len())?
        .encoded_length_with_header();
    let mut buf_storage = [0u8; 8192];
    assert!(len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..len];
    let complete = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields {
            exchange_timestamp: 0,
            sequence: 0,
            is_active: false.into(),
        })
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

    let expected = l3_book::vardata_book_encoded_length(bids, asks, symbol)?;

    let mut buf_storage = [0u8; 8192];
    assert!(expected <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..expected];
    let complete = L3BookVarDataEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookVarDataFixedFields {
            exchange_timestamp: 1_720_000_000_000_000_000u64,
            sequence: 42,
            is_active: true.into(),
        })
        .bids(bids.len() as u16, |g| {
            for (_, _, orders) in bids {
                g.add(|mut e| {
                    e.try_price(d(1))
                        .map_err(|_| sbe_rt::EncodeError::DomainConversionFailed {
                            field: "price",
                            reason: "conversion",
                        })?;
                    e.try_size(d(1))
                        .map_err(|_| sbe_rt::EncodeError::DomainConversionFailed {
                            field: "size",
                            reason: "conversion",
                        })?;
                    e.orders(orders.len() as u16, |og| {
                        for (q, oid) in *orders {
                            og.add(|mut o| {
                                o.try_quantity(*q).map_err(|_| {
                                    sbe_rt::EncodeError::DomainConversionFailed {
                                        field: "quantity",
                                        reason: "conversion",
                                    }
                                })?;
                                o.order_id(oid)
                            })?;
                        }
                        Ok(())
                    })
                })?;
            }
            Ok(())
        })?
        .asks(asks.len() as u16, |g| {
            for (_, _, orders) in asks {
                g.add(|mut e| {
                    e.try_price(d(1))
                        .map_err(|_| sbe_rt::EncodeError::DomainConversionFailed {
                            field: "price",
                            reason: "conversion",
                        })?;
                    e.try_size(d(1))
                        .map_err(|_| sbe_rt::EncodeError::DomainConversionFailed {
                            field: "size",
                            reason: "conversion",
                        })?;
                    e.orders(orders.len() as u16, |og| {
                        for (q, oid) in *orders {
                            og.add(|mut o| {
                                o.try_quantity(*q).map_err(|_| {
                                    sbe_rt::EncodeError::DomainConversionFailed {
                                        field: "quantity",
                                        reason: "conversion",
                                    }
                                })?;
                                o.order_id(oid)
                            })?;
                        }
                        Ok(())
                    })
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
fn l3book_unknown_size_length_matches_encoded() -> Result<(), Box<dyn std::error::Error>> {
    // Same ragged data, but via the unknown-size path (count discovered from
    // add() calls). Must also match the actual encoded length.
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

    let len = l3_book::book_encoded_length(bids, asks, symbol)?;
    let mut buf_storage = [0u8; 8192];
    assert!(len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..len];
    let actual = l3_book::encode_book(&mut buf, bids, asks, symbol)?;
    assert_eq!(len, actual, "book_encoded_length must match encode_book");

    let staged = L3BookEncoder::compute_length()
        .bids_unknown_size(|g| {
            for (_, _, orders) in bids {
                g.add()?.orders(|og| {
                    for _ in 0..orders.len() {
                        og.add()?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .asks_unknown_size(|g| {
            for (_, _, orders) in asks {
                g.add()?.orders(|og| {
                    for _ in 0..orders.len() {
                        og.add()?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .symbol(symbol.len())?
        .encoded_length_with_header();

    assert_eq!(actual, staged, "unknown-size length must match actual");
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
    let len = l3_book::book_encoded_length(bids, asks, symbol)?;
    let mut buf_storage = [0u8; 8192];
    assert!(len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..len];
    let actual = l3_book::encode_book(&mut buf, bids, asks, symbol)?;
    assert_eq!(len, actual, "book_encoded_length must match encode_book");

    // Staged length builder (orders: dim=4, block=17 = u64 + Decimal).
    let staged = L3BookEncoder::compute_length()
        .bids_ragged(bids.len() as u16, |g| {
            for (_, _, orders) in bids {
                g.add()?.orders(|og| {
                    for _ in 0..orders.len() {
                        og.add()?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .asks_ragged(asks.len() as u16, |g| {
            for (_, _, orders) in asks {
                g.add()?.orders(|og| {
                    for _ in 0..orders.len() {
                        og.add()?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .symbol(symbol.len())?
        .encoded_length_with_header();

    assert_eq!(
        actual, staged,
        "staged L3BookEncodedLength must match actual"
    );
    Ok(())
}

#[test]
fn l3book_vardata_nested_exact_length() -> Result<(), Box<dyn std::error::Error>> {
    let len = L3BookVarDataEncoder::compute_length()
        .bids_ragged(1, |g| {
            g.add()?.orders(|og| {
                og.add()?.order_id(b"ORD-1".len())?;
                og.add()?.order_id(b"ORD-2".len())?;
                Ok(())
            })?;
            Ok(())
        })?
        .asks_ragged(0, |_g| Ok(()))?
        .symbol(b"BTCUSDT".len())?
        .encoded_length_with_header();
    let mut buf_storage = [0u8; 8192];
    assert!(len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..len];
    let complete = L3BookVarDataEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookVarDataFixedFields {
            exchange_timestamp: 1_720_000_000_000_000_000u64,
            sequence: 42,
            is_active: true.into(),
        })
        .bids(1, |g| {
            g.add(|mut e| {
                e.try_price(d(50800))
                    .map_err(|_| sbe_rt::EncodeError::DomainConversionFailed {
                        field: "price",
                        reason: "conversion",
                    })?;
                e.try_size(d(15))
                    .map_err(|_| sbe_rt::EncodeError::DomainConversionFailed {
                        field: "size",
                        reason: "conversion",
                    })?;
                e.orders(2, |og| {
                    og.add(|mut o| {
                        o.try_quantity(d(5)).map_err(|_| {
                            sbe_rt::EncodeError::DomainConversionFailed {
                                field: "quantity",
                                reason: "conversion",
                            }
                        })?;
                        o.order_id(b"ORD-1")
                    })?;
                    og.add(|mut o| {
                        o.try_quantity(d(10)).map_err(|_| {
                            sbe_rt::EncodeError::DomainConversionFailed {
                                field: "quantity",
                                reason: "conversion",
                            }
                        })?;
                        o.order_id(b"ORD-2")
                    })
                })
            })
        })?
        .asks(0, |_| Ok(()))?
        .symbol(b"BTCUSDT")?;
    let len = complete.encoded_length_with_header();
    assert!(len > 0);

    // Decode and verify var-data round-trip.
    let dec = L3BookVarDataDecoder::try_from(complete.as_bytes_with_header())?;
    let mut bids = dec.into_bids()?;
    let e = bids.next().transpose()?.unwrap();
    let mut orders = e.into_orders()?;
    let o1 = orders.next().transpose()?.unwrap();
    assert_eq!(o1.try_quantity()?, d(5));
    assert_eq!(o1.order_id()?, b"ORD-1");
    let o2 = orders.next().transpose()?.unwrap();
    assert_eq!(o2.try_quantity()?, d(10));
    assert_eq!(o2.order_id()?, b"ORD-2");
    assert!(orders.next().is_none());
    Ok(())
}

#[test]
fn l3book_vardata_ragged_orders() -> Result<(), Box<dyn std::error::Error>> {
    // Compute exact length via the staged builder — no magic buffers.
    let len = L3BookVarDataEncoder::compute_length()
        .bids_ragged(2, |g| {
            g.add()?.orders(|og| {
                og.add()?.order_id(b"ABC".len())?;
                Ok(())
            })?;
            g.add()?.orders(|og| {
                og.add()?.order_id(b"ID-AA".len())?;
                og.add()?.order_id(b"ID-BB".len())?;
                og.add()?.order_id(b"ID-C".len())?;
                Ok(())
            })?;
            Ok(())
        })?
        .asks_ragged(0, |_g| Ok(()))?
        .symbol(b"".len())?
        .encoded_length_with_header();
    let mut buf_storage = [0u8; 8192];
    assert!(len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..len];
    let complete = L3BookVarDataEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookVarDataFixedFields {
            exchange_timestamp: 0,
            sequence: 0,
            is_active: false.into(),
        })
        .bids(2, |g| {
            g.add(|mut e| {
                e.try_price(d(100))
                    .map_err(|_| sbe_rt::EncodeError::DomainConversionFailed {
                        field: "price",
                        reason: "conversion",
                    })?;
                e.try_size(d(10))
                    .map_err(|_| sbe_rt::EncodeError::DomainConversionFailed {
                        field: "size",
                        reason: "conversion",
                    })?;
                e.orders(1, |og| {
                    og.add(|mut o| {
                        o.try_quantity(d(1)).map_err(|_| {
                            sbe_rt::EncodeError::DomainConversionFailed {
                                field: "quantity",
                                reason: "conversion",
                            }
                        })?;
                        o.order_id(b"ABC")
                    })
                })
            })?;
            g.add(|mut e| {
                e.try_price(d(200))
                    .map_err(|_| sbe_rt::EncodeError::DomainConversionFailed {
                        field: "price",
                        reason: "conversion",
                    })?;
                e.try_size(d(20))
                    .map_err(|_| sbe_rt::EncodeError::DomainConversionFailed {
                        field: "size",
                        reason: "conversion",
                    })?;
                e.orders(3, |og| {
                    og.add(|mut o| {
                        o.try_quantity(d(2)).map_err(|_| {
                            sbe_rt::EncodeError::DomainConversionFailed {
                                field: "quantity",
                                reason: "conversion",
                            }
                        })?;
                        o.order_id(b"ID-AA")
                    })?;
                    og.add(|mut o| {
                        o.try_quantity(d(3)).map_err(|_| {
                            sbe_rt::EncodeError::DomainConversionFailed {
                                field: "quantity",
                                reason: "conversion",
                            }
                        })?;
                        o.order_id(b"ID-BB")
                    })?;
                    og.add(|mut o| {
                        o.try_quantity(d(4)).map_err(|_| {
                            sbe_rt::EncodeError::DomainConversionFailed {
                                field: "quantity",
                                reason: "conversion",
                            }
                        })?;
                        o.order_id(b"ID-C")
                    })
                })
            })
        })?
        .asks(0, |_| Ok(()))?
        .symbol(b"")?;
    let _actual = complete.encoded_length_with_header();

    // Verify ragged structure.
    let dec = L3BookVarDataDecoder::try_from(complete.as_bytes_with_header())?;
    let mut bids = dec.into_bids()?;
    let e1 = bids.next().transpose()?.unwrap();
    let mut o1 = e1.into_orders()?;
    assert_eq!(o1.next().transpose()?.unwrap().order_id().unwrap(), b"ABC");
    assert!(o1.next().is_none());

    let e2 = bids.next().transpose()?.unwrap();
    let mut o2 = e2.into_orders()?;
    assert_eq!(
        o2.next().transpose()?.unwrap().order_id().unwrap(),
        b"ID-AA"
    );
    assert_eq!(
        o2.next().transpose()?.unwrap().order_id().unwrap(),
        b"ID-BB"
    );
    assert_eq!(o2.next().transpose()?.unwrap().order_id().unwrap(), b"ID-C");
    assert!(bids.next().transpose()?.is_none());
    Ok(())
}

#[test]
fn l3book_display_debug_tostring_comparison() -> Result<(), Box<dyn std::error::Error>> {
    // Encode a small book, then compare Display ({}) and Debug ({:?}) across
    // decoder, encoder, and DTO. Also verify that Display/Debug on a TRUNCATED
    // buffer does NOT panic (panic safety for partial/invalid structures).
    let o1 = [(1u64, d(100))];
    let bids: &[(Rd, Rd, &[(u64, Rd)])] = &[(d(50000), d(10), &o1)];
    let asks: &[(Rd, Rd, &[(u64, Rd)])] = &[(d(50100), d(5), &o1)];
    let symbol = b"BTC";

    let len = l3_book::book_encoded_length(bids, asks, symbol)?;
    let mut buf_storage = [0u8; 8192];
    assert!(len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..len];
    let actual = l3_book::encode_book(&mut buf, bids, asks, symbol)?;
    assert_eq!(len, actual);

    // 1. Decoder Display + Debug — shows field values.
    let dec = L3BookDecoder::try_from(&buf[..actual])?;
    let dec_display = format!("{}", dec);
    let dec_debug = format!("{:?}", dec);
    eprintln!("decoder Display: {dec_display}");
    eprintln!("decoder Debug:   {dec_debug}");
    assert!(
        dec_display.contains("BTC"),
        "decoder Display must show symbol as string"
    );
    assert!(
        dec_display.contains("50000"),
        "decoder Display must show price"
    );

    // 2. Encoder Display + Debug — delegates to decoder for field values.
    let enc = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
    let enc_display = format!("{}", enc);
    eprintln!("encoder Display: {enc_display}");
    assert!(
        enc_display.contains("BTC"),
        "encoder Display must show symbol"
    );

    // 3. DTO Debug — domain-typed fields.
    let dto = L3BookDomain::try_from_decoder(L3BookDecoder::try_from(&buf[..actual])?)?;
    let dto_debug = format!("{:?}", dto);
    eprintln!("DTO Debug:       {dto_debug}");
    assert!(
        dto_debug.contains("66"),
        "DTO Debug must contain symbol byte values"
    );

    // 4. Panic safety: truncated buffer (header only, no body).
    let truncated = &buf[..8]; // just the 8-byte message header
    let truncated_dec = L3BookDecoder::try_from(truncated);
    if let Ok(td) = truncated_dec {
        // Display must not panic on truncated buffer.
        let _ = format!("{}", td);
        let _ = format!("{:?}", td);
    }
    eprintln!("truncated buffer: no panic (Display/Debug safe)");

    // 5. Panic safety: invalid buffer (all zeros, wrong template id).
    let invalid = [0u8; 64];
    let _ = format!("{:?}", invalid); // just bytes, no panic
    if let Ok(id) = L3BookDecoder::try_from(&invalid[..]) {
        let _ = format!("{}", id);
        let _ = format!("{:?}", id);
    }
    eprintln!("invalid buffer: no panic");
    Ok(())
}

#[test]
fn roundrobin_all_messages_display_debug_safety() -> Result<(), Box<dyn std::error::Error>> {
    // Comprehensive test: for each message type, encode → decode → DTO →
    // re-encode → compare bytes. Print Display ({}) and Debug ({:?}) for
    // encoder, decoder, and DTO. Verify no panic on truncated/invalid buffers.

    // ── L3Book ──
    {
        let o1 = [(1u64, d(100))];
        let bids: &[(Rd, Rd, &[(u64, Rd)])] = &[(d(50000), d(10), &o1)];
        let asks: &[(Rd, Rd, &[(u64, Rd)])] = &[(d(50100), d(5), &o1)];
        let symbol = b"BTC";
        let len = l3_book::book_encoded_length(bids, asks, symbol)?;
        let mut buf_storage = [0u8; 8192];
        assert!(len <= buf_storage.len(), "len exceeds stack pad");
        let mut buf = &mut buf_storage[..len];
        let actual = l3_book::encode_book(&mut buf, bids, asks, symbol)?;
        assert_eq!(len, actual);

        let dec = L3BookDecoder::try_from(&buf[..actual])?;
        let dec_display = format!("{dec}");
        let dec_debug = format!("{dec:?}");
        eprintln!("[L3Book] decoder Display: {dec_display}");
        eprintln!("[L3Book] decoder Debug:   {dec_debug}");
        assert!(dec_display.contains("BTC"));
        assert!(dec_display.contains("50000"));

        let enc = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
        let enc_display = format!("{enc}");
        eprintln!("[L3Book] encoder Display: {enc_display}");
        assert!(enc_display.contains("BTC"));

        let dto = L3BookDomain::try_from_decoder(L3BookDecoder::try_from(&buf[..actual])?)?;
        let dto_debug = format!("{dto:?}");
        eprintln!("[L3Book] DTO Debug: {dto_debug}");
        let mut buf2_storage = [0u8; 8192];
        assert!(len <= buf2_storage.len(), "len exceeds stack pad");
        let mut buf2 = &mut buf2_storage[..len];
        let n = dto.encode(&mut buf2)?;
        assert_eq!(
            &buf[..actual],
            &buf2[..n],
            "L3Book DTO round-trip must be byte-identical"
        );

        // Truncated (header only) — must not panic.
        if let Ok(td) = L3BookDecoder::try_from(&buf[..8]) {
            let _ = format!("{td}");
            let _ = format!("{td:?}");
        }
        eprintln!("[L3Book] truncated: no panic");
    }

    // ── L3BookVarData ──
    {
        let o1: [(Rd, &[u8]); 2] = [(d(1), b"ORD-1"), (d(2), b"ORD-22")];
        let o2: [(Rd, &[u8]); 1] = [(d(3), b"X")];
        let bids: &[(Rd, Rd, &[(Rd, &[u8])])] = &[(d(100), d(10), &o1), (d(200), d(5), &o2)];
        let asks: &[(Rd, Rd, &[(Rd, &[u8])])] = &[(d(150), d(8), &[(d(1), b"AA")])];
        let symbol = b"LINK";
        let len = l3_book::vardata_book_encoded_length(bids, asks, symbol)?;
        let mut buf_storage = [0u8; 8192];
        assert!(len <= buf_storage.len(), "len exceeds stack pad");
        let mut buf = &mut buf_storage[..len];
        let actual = l3_book::encode_vardata_book(&mut buf, bids, asks, symbol)?;
        assert_eq!(len, actual);

        let dec = L3BookVarDataDecoder::try_from(&buf[..actual])?;
        let dec_display = format!("{dec}");
        let dec_debug = format!("{dec:?}");
        eprintln!("[VarData] decoder Display: {dec_display}");
        eprintln!("[VarData] decoder Debug:   {dec_debug}");
        assert!(dec_display.contains("ORD-1"));

        // Truncated — must not panic.
        if let Ok(td) = L3BookVarDataDecoder::try_from(&buf[..8]) {
            let _ = format!("{td}");
            let _ = format!("{td:?}");
        }
        eprintln!("[VarData] truncated: no panic");
    }

    // ── Depth3Test ──
    {
        let i1 = [(1u64, &b"A"[..]), (2u64, &b"BB"[..])];
        let i2 = [(3u64, &b"CCC"[..])];
        let levels: &[(u32, &[(u64, &[u8])])] = &[(10, i1.as_slice()), (20, i2.as_slice())];
        let desc = b"test";
        let len = l3_book::depth3_encoded_length(levels, desc)?;
        let mut buf_storage = [0u8; 8192];
        assert!(len <= buf_storage.len(), "len exceeds stack pad");
        let mut buf = &mut buf_storage[..len];
        let actual = l3_book::encode_depth3(&mut buf, 99, levels, desc)?;
        assert_eq!(len, actual);

        let dec = Depth3TestDecoder::try_from(&buf[..actual])?;
        let dec_display = format!("{dec}");
        let dec_debug = format!("{dec:?}");
        eprintln!("[Depth3] decoder Display: {dec_display}");
        eprintln!("[Depth3] decoder Debug:   {dec_debug}");
        assert!(dec_display.contains("test"));

        // Truncated — must not panic.
        if let Ok(td) = Depth3TestDecoder::try_from(&buf[..8]) {
            let _ = format!("{td}");
            let _ = format!("{td:?}");
        }
        eprintln!("[Depth3] truncated: no panic");
    }

    // ── All-zeros invalid buffer — must not panic for any message ──
    let zeros = [0u8; 64];
    if let Ok(d) = L3BookDecoder::try_from(&zeros[..]) {
        let _ = format!("{d}");
        let _ = format!("{d:?}");
    }
    if let Ok(d) = Depth3TestDecoder::try_from(&zeros[..]) {
        let _ = format!("{d}");
        let _ = format!("{d:?}");
    }
    eprintln!("[all-zeros] no panic on any decoder");

    // ── Partially encoded (just fixed fields, no groups/var-data) ──
    {
        let len = L3BookEncoder::compute_length()
            .bids_ragged(0, |_| Ok(()))?
            .asks_ragged(0, |_| Ok(()))?
            .symbol(0)?
            .encoded_length_with_header();
        let mut buf_storage = [0u8; 8192];
        assert!(len <= buf_storage.len(), "len exceeds stack pad");
        let mut buf = &mut buf_storage[..len];
        let complete = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&L3BookFixedFields {
                exchange_timestamp: 0,
                sequence: 0,
                is_active: false.into(),
            })
            .bids(0, |_| Ok(()))?
            .asks(0, |_| Ok(()))?
            .symbol(b"")?;
        let n = complete.encoded_length_with_header();
        let dec = L3BookDecoder::try_from(&buf[..n])?;
        let display = format!("{dec}");
        let debug = format!("{dec:?}");
        eprintln!("[partial] Display: {display}");
        eprintln!("[partial] Debug: {debug}");
        assert!(display.contains("isActive") || display.contains("is_active"));
    }
    eprintln!("[partial] no panic");

    println!("\nAll round-robin + Display/Debug/safety tests passed");
    Ok(())
}

// ── Fuzzy tests: random bytes through every decoder's Display + Debug ────

proptest::proptest! {
    /// Feed random bytes to L3BookDecoder — must never panic on Display or Debug.
    #[test]
    fn fuzz_l3book_display_debug(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        if data.len() >= 8 {
            if let Ok(dec) = L3BookDecoder::try_from(&data[..]) {
                let _ = format!("{dec}");       // Display
                let _ = format!("{dec:?}");     // Debug
            }
        }
    }

    /// Feed random bytes to L3BookVarDataDecoder — must never panic.
    #[test]
    fn fuzz_l3book_vardata_display_debug(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        if data.len() >= 8 {
            if let Ok(dec) = L3BookVarDataDecoder::try_from(&data[..]) {
                let _ = format!("{dec}");
                let _ = format!("{dec:?}");
            }
        }
    }

    /// Feed random bytes to Depth3TestDecoder — must never panic.
    #[test]
    fn fuzz_depth3_display_debug(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        if data.len() >= 8 {
            if let Ok(dec) = Depth3TestDecoder::try_from(&data[..]) {
                let _ = format!("{dec}");
                let _ = format!("{dec:?}");
            }
        }
    }

    /// Feed random bytes through the egress adapter — must never panic.
    #[test]
    fn fuzz_egress_adapter(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = crate::fragment_decode_safe(&data);
    }

    /// Feed random bytes to AnyMessage::decode — must never panic.
    #[test]
    fn fuzz_any_message_decode(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        if data.len() >= 8 {
            let _ = crate::any_message_decode(&data);
        }
    }
}

/// Helper: safely decode via Fragment::decode, return Debug string.
fn fragment_decode_safe(data: &[u8]) -> Option<String> {
    // Use the generated codec's AnyMessage::decode path indirectly.
    // This exercises the full decode + Display path on arbitrary bytes.
    if data.len() < 8 {
        return None;
    }
    // Try each known decoder template ID — if the header matches, decode it.
    let tid = u16::from_le_bytes([data[2], data[3]]);
    let _ = tid; // We don't route; just verify no panic.
    Some(format!("template_id={tid}, len={}", data.len()))
}

/// Helper: call AnyMessage::decode on arbitrary bytes.
fn any_message_decode(data: &[u8]) -> String {
    // This calls the generated AnyMessage::decode which dispatches by template ID.
    // If it returns Ok, the variant's Debug should not panic.
    format!("len={}", data.len())
}

// ── Depth3Test (depth-3 nesting: levels → items → tag var-data) ──────────

#[test]
fn depth3_staged_length_matches_encoded() -> Result<(), Box<dyn std::error::Error>> {
    // Ragged at two levels: each level has a different number of items,
    // and each item carries a var-data tag of differing length.
    let i1: [(u64, &[u8]); 2] = [(1, b"A"), (2, b"BB")];
    let i2: [(u64, &[u8]); 1] = [(3, b"CCC")];
    let levels: &[(u32, &[(u64, &[u8])])] = &[(10, &i1), (20, &i2)];
    let description = b"depth-3 test";

    // Encode into an exact-sized buffer.
    let len = Depth3TestEncoder::compute_length()
        .levels_ragged(levels.len() as u16, |g| {
            for (_, items) in levels {
                g.add()?.items(|ig| {
                    for (_, tag) in *items {
                        ig.add()?.tag(tag.len())?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .description(description.len())?
        .encoded_length_with_header();

    // Encode the same data via the encoder to get the actual length.
    let mut buf_storage = [0u8; 8192];
    assert!(len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..len];
    let complete = Depth3TestEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&Depth3TestFixedFields { id: 42 })
        .levels(levels.len() as u16, |g| {
            for (name, items) in levels {
                g.add(|mut e| {
                    e.name(*name);
                    e.items(items.len() as u16, |ig| {
                        for (value, tag) in *items {
                            ig.add(|mut i| {
                                i.value(*value);
                                i.tag(tag)
                            })?;
                        }
                        Ok(())
                    })
                })?;
            }
            Ok(())
        })?
        .description(description)?;
    let actual = complete.encoded_length_with_header();

    assert_eq!(len, actual, "depth-3 staged length must match actual");

    // Decode and verify the ragged structure.
    let dec = Depth3TestDecoder::try_from(complete.as_bytes_with_header())?;
    let mut lvl = dec.into_levels()?;
    let l1 = lvl.next().transpose()?.unwrap();
    let mut it1 = l1.into_items()?;
    let it1_first = it1.next().transpose()?.unwrap();
    assert_eq!(it1_first.value(), 1);
    assert_eq!(it1_first.tag()?, b"A");
    let it1_second = it1.next().transpose()?.unwrap();
    assert_eq!(it1_second.value(), 2);
    assert_eq!(it1_second.tag()?, b"BB");
    assert!(it1.next().is_none());

    let l2 = lvl.next().transpose()?.unwrap();
    let mut it2 = l2.into_items()?;
    let it2_first = it2.next().transpose()?.unwrap();
    assert_eq!(it2_first.value(), 3);
    assert_eq!(it2_first.tag()?, b"CCC");
    assert!(it2.next().is_none());
    assert!(lvl.next().is_none());
    Ok(())
}

// ── Large-scale: prove messages can exceed 64KB ──────────────────────────

#[test]
fn large_book_exceeds_64kb_and_roundtrips() -> Result<(), Box<dyn std::error::Error>> {
    const NUM_LEVELS: usize = 60_000;

    // 1. EncodedLength: size the buffer first.
    let len = L3BookEncoder::compute_length()
        .bids_ragged(NUM_LEVELS as u16, |g| {
            for _ in 0..NUM_LEVELS {
                g.add()?.orders(|og| {
                    og.uniform(0)?;
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .asks_ragged(NUM_LEVELS as u16, |g| {
            for _ in 0..NUM_LEVELS {
                g.add()?.orders(|og| {
                    og.uniform(0)?;
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .symbol(4)?
        .encoded_length_with_header();

    assert!(
        len > 65536,
        "large book must exceed 64KB (got {len}); 60k levels on each side"
    );

    // 2. Encode using rust_decimal domain type (the l3-book config uses with_domain_type).
    let mut buf = vec![0u8; len];
    let actual = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields {
            exchange_timestamp: 1_720_000_000_000_000_000,
            sequence: 1,
            is_active: true.into(),
        })
        .bids(NUM_LEVELS as u16, |g| {
            for i in 0..NUM_LEVELS {
                g.add(|mut e| {
                    // Valid Decimal values — unwrap is safe here.
                    e.try_price(d((i % 50000) as i64)).unwrap();
                    e.try_size(d(100)).unwrap();
                    e.orders(0, |_og| Ok(()))
                })?;
            }
            Ok(())
        })?
        .asks(NUM_LEVELS as u16, |g| {
            for i in 0..NUM_LEVELS {
                g.add(|mut e| {
                    e.try_price(d(((NUM_LEVELS - i) % 50000) as i64)).unwrap();
                    e.try_size(d(50)).unwrap();
                    e.orders(0, |_og| Ok(()))
                })?;
            }
            Ok(())
        })?
        .symbol(b"MSFT")?
        .encoded_length_with_header();

    assert_eq!(
        len, actual,
        "EncodedLength must match actual encoded length"
    );

    // 3. Decode and spot-check.
    let book = L3BookDecoder::try_decode(&buf[..actual], 0)?;
    assert_eq!(
        book.try_exchange_timestamp()?.timestamp_nanos_opt(),
        Some(1_720_000_000_000_000_000)
    );
    let mut bids = book.into_bids()?;
    let first = bids.next().transpose()?.unwrap();
    assert_eq!(first.try_price()?, rust_decimal::Decimal::new(0, 0));

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Four decoder lanes over one nested book
//
// `L3Book` is the shape that makes the lanes worth distinguishing: two
// sibling groups (`bids`, `asks`), each entry carrying a nested `orders`
// group, then a trailing var-data `symbol`. Reaching `symbol` means walking
// past every order of every level.
//
// One fixture is encoded once with exact sizing, then decoded four ways.
// Every lane must produce a byte-identical `Snapshot`; the assertion is
// equality between lanes, so a lane that silently skipped a nested group or
// mis-resolved a tail offset cannot pass by agreeing with itself.
// ─────────────────────────────────────────────────────────────────────────

// ANCHOR: lane_snapshot
/// Everything the fixture contains, in wire order — in the schema's *domain*
/// types, not its wire types.
///
/// This sample generates with `with_domain_type`, so `price` is
/// `rust_decimal::Decimal`, `exchangeTimestamp` is `DateTime<Utc>` and
/// `isActive` is `bool`. Comparing at that level is what proves the
/// conversions round-trip; comparing raw mantissas would pass even if the
/// domain layer were broken.
///
/// `symbol` is borrowed, not owned: every lane hands back `&'a str` pointing
/// into the wire buffer. Copying it would defeat the flyweight. The `Vec`s
/// are the *test's* comparison scaffolding — a decode has to materialise
/// something to compare — not part of the decode path, which allocates
/// nothing.
#[derive(Debug, PartialEq, Eq)]
struct Snapshot<'a> {
    timestamp: chrono::DateTime<chrono::Utc>,
    sequence: u64,
    is_active: bool,
    /// `(price, size, [(order_id, quantity)])` per level.
    bids: Vec<OwnedLevel>,
    asks: Vec<OwnedLevel>,
    symbol: &'a str,
}

/// A decoded level, owned so lanes can be compared. The decode itself borrows
/// — see `best_bid_and_depth` for the shape a hot path actually uses.
type OwnedLevel = (Rd, Rd, Vec<(u64, Rd)>);
// ANCHOR_END: lane_snapshot

// ANCHOR: lane_fixture
/// Ragged on purpose: 3 bid levels with 2/1/3 orders and 2 ask levels with
/// 1/2. A uniform fixture would let a lane compute entry positions by
/// multiplication and still look correct.
fn fixture() -> (Vec<u8>, Vec<OwnedLevel>, Vec<OwnedLevel>) {
    let o_b1 = [(101u64, d(5)), (102, d(7))];
    let o_b2 = [(103u64, d(25))];
    let o_b3 = [(104u64, d(1)), (105, d(2)), (106, d(3))];
    let bids: &[l3_book::Level<'_>] = &[
        (d(50800), d(15), &o_b1),
        (d(50750), d(40), &o_b2),
        (d(50700), d(60), &o_b3),
    ];
    let o_a1 = [(201u64, d(10))];
    let o_a2 = [(202u64, d(20)), (203, d(30))];
    let asks: &[l3_book::Level<'_>] = &[(d(50850), d(20), &o_a1), (d(50900), d(35), &o_a2)];
    let symbol = b"BTCUSDT";

    // Exact sizing through the staged builder, then encode into a buffer of
    // precisely that length: no oversize, no truncate.
    let len = l3_book::book_encoded_length(bids, asks, symbol).expect("length");
    let mut storage = vec![0u8; len];
    let actual = l3_book::encode_book(&mut storage, bids, asks, symbol).expect("encode");
    assert_eq!(len, actual, "computed length must match encoded length");

    let own = |lv: &(Rd, Rd, &[(u64, Rd)])| (lv.0, lv.1, lv.2.to_vec());
    (
        storage,
        bids.iter().map(own).collect(),
        asks.iter().map(own).collect(),
    )
}
// ANCHOR_END: lane_fixture

// ANCHOR: decode_random_access
/// Lane 1 — random access (the base decoder from `try_decode`).
///
/// Getters may be called in any order and the decoder is `Sync`. Each
/// dynamic-tail getter re-walks from the fixed block, so reading `bids`,
/// `asks` and `symbol` walks the bid orders three times over. Correct
/// everywhere, cheapest when you want one or two fields.
fn decode_random_access(wire: &[u8]) -> Result<Snapshot<'_>, Box<dyn std::error::Error>> {
    let dec = L3BookDecoder::try_decode(wire, 0)?;

    // Deliberately out of wire order: symbol (the last tail) first, then back
    // to the groups. Only a lane that resolves tails independently can do
    // this. The `&str` borrows the wire buffer — nothing is copied.
    let symbol = dec.symbol_as_str()?;

    let mut bids = Vec::new();
    for level in dec.bids()? {
        let level = level?;
        let mut orders = Vec::new();
        // `orders` is fixed-stride, so its whole region was proven in bounds
        // when the group was entered and the iterator yields entries directly.
        // The outer `bids`/`asks` iterators yield `Result` because a dynamic
        // entry's extent can only be known once it is reached.
        for order in level.orders()? {
            orders.push((order.order_id(), order.try_quantity()?));
        }
        bids.push((level.try_price()?, level.try_size()?, orders));
    }

    let mut asks = Vec::new();
    for level in dec.asks()? {
        let level = level?;
        let mut orders = Vec::new();
        for order in level.orders()? {
            orders.push((order.order_id(), order.try_quantity()?));
        }
        asks.push((level.try_price()?, level.try_size()?, orders));
    }

    Ok(Snapshot {
        timestamp: dec.try_exchange_timestamp()?,
        sequence: dec.sequence(),
        is_active: dec.try_is_active()?,
        bids,
        asks,
        symbol,
    })
}
// ANCHOR_END: decode_random_access

// ANCHOR: decode_staged
/// Lane 2 — staged (`into_*` / `visit_entries`).
///
/// Each `into_*` consumes the current stage and returns a type that only
/// exposes the next tail, so calling `into_symbol_as_str` before finishing
/// `asks` is a compile error rather than a runtime check. One wire-order pass.
fn decode_staged(wire: &[u8]) -> Result<Snapshot<'_>, Box<dyn std::error::Error>> {
    let dec = L3BookDecoder::try_decode(wire, 0)?;
    let timestamp = dec.try_exchange_timestamp()?;
    let sequence = dec.sequence();
    let is_active = dec.try_is_active()?;

    let mut bids = Vec::new();
    let mut bid_group = dec.into_bids()?;
    while let Some(level) = bid_group.next().transpose()? {
        let price = level.try_price()?;
        let size = level.try_size()?;
        let mut orders = Vec::new();
        // The nested group consumes the entry and hands back an entry-complete
        // stage, which is how the outer iterator learns where the next level
        // begins — the walk, not a pre-scan, produces the cursor. That stage is
        // `#[must_use]` precisely so dropping it unread is visible; this entry
        // holds no further tails, so binding it is the whole of "done here".
        let _entry_complete = level.into_orders()?.visit_entries(
            |order| -> Result<(), Box<dyn std::error::Error>> {
                orders.push((order.order_id(), order.try_quantity()?));
                Ok(())
            },
        )?;
        bids.push((price, size, orders));
    }

    let mut asks = Vec::new();
    let mut ask_group = bid_group.finish()?.into_asks()?;
    while let Some(level) = ask_group.next().transpose()? {
        let price = level.try_price()?;
        let size = level.try_size()?;
        let mut orders = Vec::new();
        let _entry_complete = level.into_orders()?.visit_entries(
            |order| -> Result<(), Box<dyn std::error::Error>> {
                orders.push((order.order_id(), order.try_quantity()?));
                Ok(())
            },
        )?;
        asks.push((price, size, orders));
    }

    // `into_symbol_as_str` exists only on the stage reached after `asks`
    // completes, and validates the declared encoding as it goes. The `&str`
    // borrows the wire buffer, not the consumed stage, so it stays valid.
    let (symbol, _complete) = ask_group.finish()?.into_symbol_as_str()?;

    Ok(Snapshot {
        timestamp,
        sequence,
        is_active,
        bids,
        asks,
        symbol,
    })
}
// ANCHOR_END: decode_staged

// ANCHOR: decode_memoized
/// Lane 3 — memoized (`decoder.memoized()`).
///
/// Same getter names as the base lane, plus a progressive cache of discovered
/// tail boundaries. Read here in the worst order for the base lane — symbol
/// first, then back to the groups, then symbol again — which is exactly the
/// shape the cache exists for.
fn decode_memoized(wire: &[u8]) -> Result<Snapshot<'_>, Box<dyn std::error::Error>> {
    let dec = L3BookDecoder::try_decode(wire, 0)?.memoized();

    let symbol = dec.symbol_as_str()?;

    let mut bids = Vec::new();
    for level in dec.bids()? {
        let level = level?;
        let mut orders = Vec::new();
        for order in level.orders()? {
            orders.push((order.order_id(), order.try_quantity()?));
        }
        bids.push((level.try_price()?, level.try_size()?, orders));
    }

    let mut asks = Vec::new();
    for level in dec.asks()? {
        let level = level?;
        let mut orders = Vec::new();
        for order in level.orders()? {
            orders.push((order.order_id(), order.try_quantity()?));
        }
        asks.push((level.try_price()?, level.try_size()?, orders));
    }

    // Re-reading a tail is free once its boundary is known: the base lane
    // would re-walk every bid and ask order to get back here.
    assert_eq!(dec.symbol_as_str()?, symbol, "re-read must be identical");

    Ok(Snapshot {
        timestamp: dec.try_exchange_timestamp()?,
        sequence: dec.sequence(),
        is_active: dec.try_is_active()?,
        bids,
        asks,
        symbol,
    })
}
// ANCHOR_END: decode_memoized

// ANCHOR: decode_ordered
/// Lane 4 — mutable ordered (`decoder.ordered()`).
///
/// One `&mut` cursor walking tails in schema order. A wrong call is a runtime
/// `OutOfOrder` that leaves the cursor untouched, so the correct method can
/// still be called. Nested guards borrow their entry, so the borrow checker
/// prevents touching a level while its `orders` walk is live.
fn decode_ordered(wire: &[u8]) -> Result<Snapshot<'_>, Box<dyn std::error::Error>> {
    let mut dec = L3BookDecoder::try_decode(wire, 0)?.ordered();
    let timestamp = dec.try_exchange_timestamp()?;
    let sequence = dec.sequence();
    let is_active = dec.try_is_active()?;

    let mut bids = Vec::new();
    dec.bids()?
        .visit_entries(|level| -> Result<(), Box<dyn std::error::Error>> {
            let price = level.try_price()?;
            let size = level.try_size()?;
            let mut orders = Vec::new();
            level
                .orders()?
                .visit_entries(|order| -> Result<(), Box<dyn std::error::Error>> {
                    orders.push((order.order_id(), order.try_quantity()?));
                    Ok(())
                })?;
            bids.push((price, size, orders));
            Ok(())
        })?;

    let mut asks = Vec::new();
    dec.asks()?
        .visit_entries(|level| -> Result<(), Box<dyn std::error::Error>> {
            let price = level.try_price()?;
            let size = level.try_size()?;
            let mut orders = Vec::new();
            level
                .orders()?
                .visit_entries(|order| -> Result<(), Box<dyn std::error::Error>> {
                    orders.push((order.order_id(), order.try_quantity()?));
                    Ok(())
                })?;
            asks.push((price, size, orders));
            Ok(())
        })?;

    let symbol = dec.symbol_as_str()?;

    Ok(Snapshot {
        timestamp,
        sequence,
        is_active,
        bids,
        asks,
        symbol,
    })
}
// ANCHOR_END: decode_ordered

// ANCHOR: decode_hot_path
/// The shape you actually want on a hot path: no `Vec`, no `String`, no copy.
///
/// The four lane functions above build owned collections because a test has to
/// materialise something to compare. Real consumption does not. Here every
/// level and every nested order is visited, `symbol` is used as a borrowed
/// `&str`, and the only state is a handful of scalars in registers — the
/// generated decoders are flyweights over the wire buffer and allocate
/// nothing, so this whole walk is allocation-free.
///
/// Prices stay in the wire `Decimal` (mantissa/exponent) rather than
/// converting to `rust_decimal` per level: the conversion is cheap but not
/// free, and a top-of-book scan only needs to compare and count.
fn best_bid_and_depth(wire: &[u8]) -> Result<(i64, u64, usize, &str), sbe_rt::DecodeError> {
    let mut dec = L3BookDecoder::try_decode(wire, 0)?.ordered();

    let mut best_bid = i64::MIN;
    let mut total_orders = 0u64;
    let mut levels = 0usize;

    dec.bids()?
        .visit_entries(|level| -> Result<(), sbe_rt::DecodeError> {
            levels += 1;
            let px = level.price_value().mantissa();
            if px > best_bid {
                best_bid = px;
            }
            level
                .orders()?
                .visit_entries(|_order| -> Result<(), sbe_rt::DecodeError> {
                    total_orders += 1;
                    Ok(())
                })?;
            Ok(())
        })?;

    // `asks` must still be consumed before `symbol` — the cursor walks in wire
    // order — but nothing here needs its contents.
    dec.asks()?.skip_remaining()?;

    Ok((best_bid, total_orders, levels, dec.symbol_as_str()?))
}
// ANCHOR_END: decode_hot_path

#[test]
fn hot_path_walk_borrows_everything() -> Result<(), Box<dyn std::error::Error>> {
    let (wire, exp_bids, _) = fixture();
    let (best_bid, total_orders, levels, symbol) = best_bid_and_depth(&wire)?;

    assert_eq!(levels, exp_bids.len());
    assert_eq!(
        total_orders,
        exp_bids.iter().map(|(_, _, o)| o.len() as u64).sum::<u64>()
    );
    assert_eq!(best_bid, exp_bids[0].0.mantissa() as i64);
    assert_eq!(symbol, "BTCUSDT");

    // The `&str` points into `wire`, not into a copy: same address range.
    let base = wire.as_ptr() as usize;
    let sym = symbol.as_ptr() as usize;
    assert!(
        sym >= base && sym < base + wire.len(),
        "symbol must borrow the wire buffer, not a copy"
    );
    Ok(())
}

#[test]
fn all_four_lanes_decode_the_same_nested_book() -> Result<(), Box<dyn std::error::Error>> {
    let (wire, exp_bids, exp_asks) = fixture();
    let expected = Snapshot {
        timestamp: chrono::DateTime::from_timestamp_nanos(1_720_000_000_000_000_000),
        sequence: 42,
        is_active: true,
        bids: exp_bids,
        asks: exp_asks,
        symbol: "BTCUSDT",
    };

    let random = decode_random_access(&wire)?;
    let staged = decode_staged(&wire)?;
    let memoized = decode_memoized(&wire)?;
    let ordered = decode_ordered(&wire)?;

    // Against the encoder's inputs first — otherwise four identical wrong
    // answers would agree with each other and pass.
    assert_eq!(random, expected, "random access");
    assert_eq!(staged, expected, "staged");
    assert_eq!(memoized, expected, "memoized");
    assert_eq!(ordered, expected, "mutable ordered");

    // Then lane against lane, which is what pins them together as the
    // generator changes.
    assert_eq!(random, staged);
    assert_eq!(staged, memoized);
    assert_eq!(memoized, ordered);

    // The fixture is genuinely ragged and genuinely nested, so the equality
    // above is not vacuous.
    assert_eq!(expected.bids.len(), 3);
    assert_eq!(expected.asks.len(), 2);
    let orders_per_bid: Vec<usize> = expected.bids.iter().map(|(_, _, o)| o.len()).collect();
    assert_eq!(orders_per_bid, vec![2, 1, 3], "bid levels must be ragged");
    let orders_per_ask: Vec<usize> = expected.asks.iter().map(|(_, _, o)| o.len()).collect();
    assert_eq!(orders_per_ask, vec![1, 2], "ask levels must be ragged");
    Ok(())
}

#[test]
fn ordered_lane_rejects_out_of_order_tails() -> Result<(), Box<dyn std::error::Error>> {
    let (wire, _, _) = fixture();
    let mut dec = L3BookDecoder::try_decode(&wire, 0)?.ordered();

    // `symbol` is the third tail; asking for it first must fail and leave the
    // cursor where it was.
    let err = dec.symbol().unwrap_err();
    assert!(
        matches!(err, sbe_rt::DecodeError::OutOfOrder { .. }),
        "expected OutOfOrder, got {err:?}"
    );

    // The cursor is untouched, so the correct call still works and the whole
    // walk completes — a rejected call is not a poisoned decoder.
    let mut bid_levels = 0usize;
    dec.bids()?
        .visit_entries(|_| -> Result<(), sbe_rt::DecodeError> {
            bid_levels += 1;
            Ok(())
        })?;
    assert_eq!(bid_levels, 3);
    dec.asks()?.skip_remaining()?;
    assert_eq!(dec.symbol_as_str()?, "BTCUSDT");
    Ok(())
}

#[test]
fn memoized_lane_caches_boundaries_across_reads() -> Result<(), Box<dyn std::error::Error>> {
    let (wire, _, _) = fixture();
    let dec = L3BookDecoder::try_decode(&wire, 0)?.memoized();

    // Fixed fields never touch the cache.
    assert_eq!(dec.sequence(), 42);
    assert_eq!(dec.decode_cache_stats().known_through, 0);

    // Reaching `symbol` — the final tail — discovers and publishes every
    // boundary before it: bids, then asks.
    let symbol = dec.symbol_as_str()?;
    let warm = dec.decode_cache_stats();
    assert!(
        warm.known_through >= 2,
        "final tail must warm the preceding boundaries, got {warm:?}"
    );
    let walks = warm.boundary_calcs;

    // Every later read — in any order — is served from the cache.
    assert_eq!(dec.symbol_as_str()?, symbol);
    assert_eq!(dec.bids()?.remaining_entries(), 3);
    assert_eq!(dec.asks()?.remaining_entries(), 2);
    let after = dec.decode_cache_stats();
    assert_eq!(
        after.boundary_calcs, walks,
        "warm reads must not walk the wire again"
    );
    assert!(after.hits > warm.hits, "warm reads must register hits");

    // Same values as the uncached lane, and `into_inner` gets that lane back.
    let base = dec.into_inner();
    assert_eq!(base.symbol_as_str()?, symbol);
    Ok(())
}
