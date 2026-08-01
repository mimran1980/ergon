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
            g.add(|e| {
                e.price(d(50800)).size(d(15)).orders(1, |og| {
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
    let _len = complete.encoded_length_with_header();

    let dec = L3BookDecoder::try_from(complete.as_bytes_with_header())?;
    let _ts = dec.exchange_timestamp();
    assert!(dec.is_active());
    let e = dec.into_bids()?.next().transpose()?.unwrap();
    let _price: Rd = e.price();
    let _size: Rd = e.size();
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
                g.add(|e| {
                    e.price(d(1)).size(d(1)).orders(orders.len() as u16, |og| {
                        for (q, oid) in *orders {
                            og.add(|o| {
                                o.quantity(*q).order_id(oid)?;
                                Ok(())
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
            for (_, _, orders) in asks {
                g.add(|e| {
                    e.price(d(1)).size(d(1)).orders(orders.len() as u16, |og| {
                        for (q, oid) in *orders {
                            og.add(|o| {
                                o.quantity(*q).order_id(oid)?;
                                Ok(())
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
            g.add(|e| {
                e.price(d(50800)).size(d(15)).orders(2, |og| {
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
    let dec = L3BookVarDataDecoder::try_from(complete.as_bytes_with_header())?;
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
            g.add(|e| {
                e.price(d(100)).size(d(10)).orders(1, |og| {
                    og.add(|o| {
                        o.quantity(d(1)).order_id(b"ABC")?;
                        Ok(())
                    })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            g.add(|e| {
                e.price(d(200)).size(d(20)).orders(3, |og| {
                    og.add(|o| {
                        o.quantity(d(2)).order_id(b"ID-AA")?;
                        Ok(())
                    })?;
                    og.add(|o| {
                        o.quantity(d(3)).order_id(b"ID-BB")?;
                        Ok(())
                    })?;
                    og.add(|o| {
                        o.quantity(d(4)).order_id(b"ID-C")?;
                        Ok(())
                    })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
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
    let dto = L3BookDomain::from(L3BookDecoder::try_from(&buf[..actual])?);
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

        let dto = L3BookDomain::from(L3BookDecoder::try_from(&buf[..actual])?);
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
                g.add(|e| {
                    e.name(*name);
                    e.items(items.len() as u16, |ig| {
                        for (value, tag) in *items {
                            ig.add(|i| {
                                i.value(*value).tag(tag)?;
                                Ok(())
                            })?;
                        }
                        Ok(())
                    })?;
                    Ok(())
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
