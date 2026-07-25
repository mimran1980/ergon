//! L3 orderbook test — nested repeating groups with var-data.
#![allow(clippy::literal_string_with_formatting_args)]
#![allow(clippy::all, clippy::pedantic, clippy::restriction, unused)]

mod common;
use common::{compile_and_run, generate};
use std::path::PathBuf;

fn l3_schema() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/l3-orderbook-schema.xml"
    ))
}

#[test]
fn l3_schema_generates_and_compiles() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&l3_schema(), "l3book");
    assert!(src.contains("L3BookDecoder"), "missing L3BookDecoder");
    assert!(src.contains("BidsDecoder"), "missing bids group decoder");
    assert!(
        src.contains("BidsEntryDecoder"),
        "missing bids entry decoder"
    );
    assert!(
        src.contains("BidsOrdersDecoder"),
        "missing nested orders group decoder"
    );
    assert!(
        src.contains("BidsOrdersEntryDecoder"),
        "missing nested orders entry decoder"
    );
    assert!(src.contains("AsksDecoder"), "missing asks group decoder");
    assert!(
        src.contains("AsksOrdersDecoder"),
        "missing nested asks orders decoder"
    );
    assert!(
        src.contains("pub fn order_id"),
        "missing order_id var-data accessor"
    );
    assert!(src.contains("L3BookEncoder"), "missing L3BookEncoder");
    assert!(
        src.contains("L3BookEncodedLength"),
        "missing L3BookEncodedLength"
    );

    Ok(())
}

#[test]
fn l3_domain_objects_generated() -> Result<(), Box<dyn std::error::Error>> {
    let ir = ergo_sbe::parse_file(&l3_schema()).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let mut config = ergo_sbe::GenerationConfig::new("l3book");
    let config = config.enable_domain_objects();
    let g = ergo_sbe::Generator::new(config);
    let src = g
        .generate(&schema)
        .unwrap()
        .modules()
        .next()
        .unwrap()
        .source
        .clone();
    assert!(
        src.contains("pub struct L3BookDomain"),
        "missing L3BookDomain"
    );
    assert!(
        src.contains("pub struct L3BookBidsEntryDomain"),
        "missing bids entry domain"
    );
    assert!(
        src.contains("pub struct L3BookBidsEntryOrdersEntryDomain"),
        "missing nested orders entry domain"
    );
    assert!(
        src.contains("pub struct L3BookAsksEntryDomain"),
        "missing asks entry domain"
    );
    assert!(
        src.contains("impl<'a> From<L3BookDecoder<'a>> for L3BookDomain"),
        "missing From impl"
    );

    Ok(())
}

#[test]
fn l3_roundtrip_encode_decode() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&l3_schema(), "l3book");
    compile_and_run(
        "l3_roundtrip",
        &src,
        r#"
        let mut buf = vec![0u8; 4096];
        let mut book = L3BookEncoder::wrap_and_apply_header(&mut buf, 0);
        book.timestamp(12345u64);
        book.sequence(1u64);
        let after_bids = book.bids(2, |bids| {
            bids.add(|level| {
                level.price(50000i64).qty(10i64);
                level.orders(2, |orders| {
                    orders.add(|o| { o.order_qty(5i64); o.order_id(b"ORD-001").unwrap(); Ok(()) })?;
                    orders.add(|o| { o.order_qty(5i64); o.order_id(b"ORD-002").unwrap(); Ok(()) })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            bids.add(|level| {
                level.price(49999i64).qty(3i64);
                level.orders(1, |orders| {
                    orders.add(|o| { o.order_qty(3i64); o.order_id(b"ORD-003").unwrap(); Ok(()) })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        }).unwrap();
        let complete = after_bids.asks(1, |asks| {
            asks.add(|level| {
                level.price(50001i64).qty(8i64);
                level.orders(1, |orders| {
                    orders.add(|o| { o.order_qty(8i64); o.order_id(b"ORD-004").unwrap(); Ok(()) })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        }).unwrap();
        let encoded = complete.as_bytes();
        let decoder = L3BookDecoder::try_from(encoded).unwrap();
        assert_eq!(decoder.timestamp(), 12345, "timestamp");
        assert_eq!(decoder.sequence(), 1, "sequence");
        let mut bids = decoder.into_bids().unwrap();
        let bid_levels: Vec<_> = bids.by_ref().collect();
        assert_eq!(bid_levels.len(), 2, "expected 2 bid levels");
        let b0 = bid_levels[0].as_ref().unwrap();
        assert_eq!(b0.price(), 50000, "bid[0].price");
        assert_eq!(b0.qty(), 10, "bid[0].qty");
        let b0_orders = b0.orders().unwrap();
        let b0_order_entries: Vec<_> = b0_orders.collect();
        assert_eq!(b0_order_entries.len(), 2, "expected 2 orders in bid[0]");
        let o0 = b0_order_entries[0].as_ref().unwrap();
        assert_eq!(o0.order_qty(), 5, "bid[0].order[0].qty");
        assert_eq!(o0.order_id().unwrap(), b"ORD-001", "bid[0].order[0].id");
        let asks = bids.finish().unwrap().into_asks().unwrap();
        let ask_levels: Vec<_> = asks.collect();
        assert_eq!(ask_levels.len(), 1, "expected 1 ask level");
        let a0 = ask_levels[0].as_ref().unwrap();
        assert_eq!(a0.price(), 50001, "ask[0].price");
        assert_eq!(a0.qty(), 8, "ask[0].qty");
        println!("L3 orderbook roundtrip: PASSED");
        "#,
    );

    Ok(())
}

#[test]
fn l3_compute_encoded_length_positive() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&l3_schema(), "l3book");
    compile_and_run(
        "l3_len",
        &src,
        r#"
        let mut buf = vec![0u8; 4096];
        let mut book = L3BookEncoder::wrap_and_apply_header(&mut buf, 0);
        book.timestamp(0).sequence(0);
        let complete = book.bids(2, |bids| -> Result<(), sbe_rt::EncodeError> {
            bids.add(|l| { l.price(0).qty(0); l.orders(0, |_| Ok(()))?; Ok(()) })?;
            bids.add(|l| { l.price(0).qty(0); l.orders(0, |_| Ok(()))?; Ok(()) })?;
            Ok(())
        }).unwrap().asks(1, |asks| -> Result<(), sbe_rt::EncodeError> {
            asks.add(|l| { l.price(0).qty(0); l.orders(0, |_| Ok(()))?; Ok(()) })?;
            Ok(())
        }).unwrap();
        assert!(complete.encoded_length() > 0);
        println!("L3Book encode (2 bids, 1 ask): {} bytes", complete.encoded_length());
        "#,
    );

    Ok(())
}

#[test]
fn l3_roundtrip_3_orders_per_level() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&l3_schema(), "l3book");
    compile_and_run(
        "l3_three",
        &src,
        r#"
        let mut buf = vec![0u8; 8192];
        let mut book = L3BookEncoder::wrap_and_apply_header(&mut buf, 0);
        book.timestamp(999u64).sequence(7u64);
        let complete = book.bids(1, |bids| {
            bids.add(|level| {
                level.price(100i64).qty(30i64);
                level.orders(3, |orders| {
                    orders.add(|o| { o.order_qty(10i64); o.order_id(b"ID-A").unwrap(); Ok(()) })?;
                    orders.add(|o| { o.order_qty(10i64); o.order_id(b"ID-B").unwrap(); Ok(()) })?;
                    orders.add(|o| { o.order_qty(10i64); o.order_id(b"ID-C").unwrap(); Ok(()) })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        }).unwrap().asks(0, |_| Ok(())).unwrap();
        let encoded = complete.as_bytes();
        let dec = L3BookDecoder::try_from(encoded).unwrap();
        assert_eq!(dec.timestamp(), 999);
        let levels: Vec<_> = dec.into_bids().unwrap().collect();
        assert_eq!(levels.len(), 1);
        let l0 = levels[0].as_ref().unwrap();
        assert_eq!(l0.price(), 100);
        assert_eq!(l0.qty(), 30);
        let orders = l0.orders().unwrap();
        let order_entries: Vec<_> = orders.collect();
        assert_eq!(order_entries.len(), 3, "expected 3 orders");
        assert_eq!(order_entries[0].as_ref().unwrap().order_qty(), 10);
        assert_eq!(order_entries[0].as_ref().unwrap().order_id().unwrap(), b"ID-A");
        assert_eq!(order_entries[1].as_ref().unwrap().order_qty(), 10);
        assert_eq!(order_entries[1].as_ref().unwrap().order_id().unwrap(), b"ID-B");
        assert_eq!(order_entries[2].as_ref().unwrap().order_qty(), 10);
        assert_eq!(order_entries[2].as_ref().unwrap().order_id().unwrap(), b"ID-C");
        println!("3 orders per level: PASSED");
        "#,
    );

    Ok(())
}

#[test]
fn l3_roundtrip_12_orders_per_level() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&l3_schema(), "l3book");
    compile_and_run(
        "l3_twelve",
        &src,
        r#"
        let mut buf = vec![0u8; 16384];
        let mut book = L3BookEncoder::wrap_and_apply_header(&mut buf, 0);
        book.timestamp(555u64).sequence(42u64);
        let complete = book.bids(1, |bids| {
            bids.add(|level| {
                level.price(200i64).qty(120i64);
                level.orders(12, |orders| {
                    for i in 0..12u64 {
                        let id = format!("ORDER-{:02}", i);
                        orders.add(|o| {
                            o.order_qty(10i64);
                            o.order_id(id.as_bytes()).unwrap();
                            Ok(())
                        })?;
                    }
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        }).unwrap().asks(1, |asks| {
            asks.add(|level| {
                level.price(201i64).qty(120i64);
                level.orders(12, |orders| {
                    for i in 0..12u64 {
                        let id = format!("ASK-{:-3}", i);
                        orders.add(|o| {
                            o.order_qty(10i64);
                            o.order_id(id.as_bytes()).unwrap();
                            Ok(())
                        })?;
                    }
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        }).unwrap();
        let encoded = complete.as_bytes();
        let dec = L3BookDecoder::try_from(encoded).unwrap();
        assert_eq!(dec.timestamp(), 555);
        assert_eq!(dec.sequence(), 42);
        let mut bids = dec.into_bids().unwrap();
        let bid_levels: Vec<_> = bids.by_ref().collect();
        assert_eq!(bid_levels.len(), 1);
        let b0 = bid_levels[0].as_ref().unwrap();
        assert_eq!(b0.price(), 200);
        assert_eq!(b0.qty(), 120);
        let b0_orders = b0.orders().unwrap();
        let b0_entries: Vec<_> = b0_orders.collect();
        assert_eq!(b0_entries.len(), 12, "expected 12 bid orders");
        for (i, entry) in b0_entries.iter().enumerate() {
            let e = entry.as_ref().unwrap();
            assert_eq!(e.order_qty(), 10, "bid order {} qty", i);
            let expected = format!("ORDER-{:02}", i);
            assert_eq!(e.order_id().unwrap(), expected.as_bytes(), "bid order {} id", i);
        }
        let asks = bids.finish().unwrap().into_asks().unwrap();
        let ask_levels: Vec<_> = asks.collect();
        assert_eq!(ask_levels.len(), 1);
        let a0 = ask_levels[0].as_ref().unwrap();
        assert_eq!(a0.price(), 201);
        let a0_orders = a0.orders().unwrap();
        let a0_entries: Vec<_> = a0_orders.collect();
        assert_eq!(a0_entries.len(), 12, "expected 12 ask orders");
        for (i, entry) in a0_entries.iter().enumerate() {
            let e = entry.as_ref().unwrap();
            assert_eq!(e.order_qty(), 10, "ask order {} qty", i);
            let expected = format!("ASK-{:-3}", i);
            assert_eq!(e.order_id().unwrap(), expected.as_bytes(), "ask order {} id", i);
        }
        println!("12 orders per level (bids + asks): PASSED");
        "#,
    );

    Ok(())
}
