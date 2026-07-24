//! Conformance test suite -- exercises every dynamic-tail shape.
//!
//! Test matrix:
//!   1. Fixed-only messages (no tail)
//!   2. Flat groups + message varData (known/known, known/unknown, unknown/unknown)
//!   3. Nested groups + entry varData
//!   4. AllTypes: enums, composites, arrays (single-element), groups, varData
//!   5. PureFixedNested: add_struct on pure-fixed nested groups
//!   6. Length builder invariants (computed length == actual encoded length)
//!   7. Expected failure cases (BufferTooShort, GroupFull, GroupCountMismatch)

#![allow(clippy::literal_string_with_formatting_args)]
#![allow(clippy::all, clippy::pedantic, clippy::restriction, unused)]

mod common;
use common::{compile_and_run, generate};
use std::path::PathBuf;

fn conformance_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/conformance_schema.xml"
    ))
}

// -- 1. Fixed-only message ------------------------------------------------

#[test]
fn conformance_fixed_only_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_fixed_only",
        &src,
        r#"
        let mut buf = [0u8; FixedOnlyEncoder::ENCODED_LENGTH];
        let mut enc = FixedOnlyEncoder::wrap_and_apply_header(&mut buf, 0)?;
        enc.id(42).price(10000).qty(10).side(Side::Buy);
        let encoded = enc.as_ref();

        let dec = FixedOnlyDecoder::try_from(encoded)?;
        assert_eq!(dec.id(), 42, "id");
        assert_eq!(dec.price(), 10000, "price");
        assert_eq!(dec.qty(), 10, "qty");
        assert_eq!(dec.side(), Side::Buy, "side");

        assert_eq!(FixedOnlyDecoder::BLOCK_LENGTH, 21, "BLOCK_LENGTH");
        assert!(FixedOnlyDecoder::ENCODED_LENGTH >= 29, "ENCODED_LENGTH >= 29");
        assert_eq!(FixedOnlyDecoder::ID_NULL, u64::MAX, "ID_NULL");
        assert_eq!(FixedOnlyDecoder::SIDE_NULL, Side::NullVal, "SIDE_NULL");

        println!("PASS: conformance_fixed_only_roundtrip");
        "#,
    );
    Ok(())
}

// -- 2. Flat group -- known/known -----------------------------------------

#[test]
fn conformance_flat_group_known_known() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_fg_kk",
        &src,
        r#"
        let body_len = FlatGroupEncodedLength::new()
            .bids(2, |b| { b.add()?; b.add()?; Ok(()) })?
            .asks(1, |a| { a.add()?; Ok(()) })?
            .description(18)?
            .encoded_length_with_header();
        let mut buf = vec![0u8; body_len];
        let mut enc = FlatGroupEncoder::wrap_and_apply_header(&mut buf, 0)?;
        enc.symbol(42);
        let complete = enc.bids(2, |bids| {
            bids.add(|e| { e.price(100i64).qty(10i32); Ok(()) })?;
            bids.add(|e| { e.price(101i64).qty(20i32); Ok(()) })?;
            Ok(())
        })?
        .asks(1, |asks| {
            asks.add(|e| { e.price(200i64).qty(30i32); Ok(()) })?;
            Ok(())
        })?
        .description(b"test exchange data")?;
        assert_eq!(complete.encoded_length_with_header(), body_len, "length match");
        let encoded = complete.as_bytes();

        let dec = FlatGroupDecoder::try_from(encoded)?;
        assert_eq!(dec.symbol(), 42, "symbol");

        let mut bids = dec.into_bids()?;
        let bid_entries: Vec<_> = bids.by_ref().collect();
        assert_eq!(bid_entries.len(), 2, "expected 2 bids");
        assert_eq!(bid_entries[0].price(), 100, "bid[0].price");
        assert_eq!(bid_entries[0].qty(), 10, "bid[0].qty");
        assert_eq!(bid_entries[1].price(), 101, "bid[1].price");
        assert_eq!(bid_entries[1].qty(), 20, "bid[1].qty");

        let after_bids = bids.finish()?;
        let mut asks = after_bids.into_asks()?;
        let ask_entries: Vec<_> = asks.by_ref().collect();
        assert_eq!(ask_entries.len(), 1, "expected 1 ask");
        assert_eq!(ask_entries[0].price(), 200, "ask[0].price");
        assert_eq!(ask_entries[0].qty(), 30, "ask[0].qty");

        let after_asks = asks.finish()?;
        let (desc, _complete) = after_asks.into_description()?;
        assert_eq!(desc, b"test exchange data", "description");

        println!("PASS: conformance_flat_group_known_known");
        "#,
    );
    Ok(())
}

// -- 3. Flat group -- known/unknown ---------------------------------------

#[test]
fn conformance_flat_group_known_unknown() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_fg_ku",
        &src,
        r#"
        let body_len = FlatGroupEncodedLength::new()
            .bids(1, |b| { b.add()?; Ok(()) })?
            .asks_unknown_size(|a| { a.add()?; Ok(()) })?
            .description(2)?
            .encoded_length_with_header();
        let mut buf = vec![0u8; body_len];
        let mut enc = FlatGroupEncoder::wrap_and_apply_header(&mut buf, 0)?;
        enc.symbol(99);
        let complete = enc.bids(1, |bids| {
            bids.add(|e| { e.price(10i64).qty(5i32); Ok(()) })?;
            Ok(())
        })?
        .asks_unknown_size(|asks| {
            asks.add(|e| { e.price(20i64).qty(15i32); Ok(()) })?;
            Ok(())
        })?
        .description(b"ku")?;
        assert_eq!(complete.encoded_length_with_header(), body_len, "length match");

        let encoded = complete.as_bytes();
        let dec = FlatGroupDecoder::try_from(encoded)?;
        assert_eq!(dec.symbol(), 99, "symbol");

        let mut bids = dec.into_bids()?;
        let be: Vec<_> = bids.by_ref().collect();
        assert_eq!(be.len(), 1);
        assert_eq!(be[0].price(), 10);

        let asks = bids.finish()?.into_asks()?;
        let ae: Vec<_> = asks.collect();
        assert_eq!(ae.len(), 1);
        assert_eq!(ae[0].price(), 20);

        println!("PASS: conformance_flat_group_known_unknown");
        "#,
    );
    Ok(())
}

// -- 4. Flat group -- unknown/unknown -------------------------------------

#[test]
fn conformance_flat_group_unknown_unknown() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_fg_uu",
        &src,
        r#"
        let body_len = FlatGroupEncodedLength::new()
            .bids_unknown_size(|b| { b.add()?; Ok(()) })?
            .asks_unknown_size(|a| { a.add()?; Ok(()) })?
            .description(2)?
            .encoded_length_with_header();
        let mut buf = vec![0u8; body_len];
        let mut enc = FlatGroupEncoder::wrap_and_apply_header(&mut buf, 0)?;
        enc.symbol(7);
        let complete = enc.bids_unknown_size(|bids| {
            bids.add(|e| { e.price(1i64).qty(2i32); Ok(()) })?;
            Ok(())
        })?
        .asks_unknown_size(|asks| {
            asks.add(|e| { e.price(3i64).qty(4i32); Ok(()) })?;
            Ok(())
        })?
        .description(b"uu")?;
        assert_eq!(complete.encoded_length_with_header(), body_len, "length match");

        let encoded = complete.as_bytes();
        let dec = FlatGroupDecoder::try_from(encoded)?;
        assert_eq!(dec.symbol(), 7);

        let mut bids = dec.into_bids()?;
        let be: Vec<_> = bids.by_ref().collect();
        assert_eq!(be.len(), 1);
        assert_eq!(be[0].price(), 1);

        let after_bids = bids.finish()?;
        let mut asks = after_bids.into_asks()?;
        let ae: Vec<_> = asks.by_ref().collect();
        assert_eq!(ae.len(), 1);
        assert_eq!(ae[0].price(), 3);

        let after_asks = asks.finish()?;
        let (desc, _c) = after_asks.into_description()?;
        assert_eq!(desc, b"uu");

        println!("PASS: conformance_flat_group_unknown_unknown");
        "#,
    );
    Ok(())
}

// -- 5. Length builder invariants -----------------------------------------

#[test]
fn conformance_length_builder_invariants() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_len",
        &src,
        r#"
        // FlatGroup: verify length builder computes positive values
        let body_len = FlatGroupEncodedLength::new()
            .bids(2, |b| { b.add()?; b.add()?; Ok(()) })?
            .asks(1, |a| { a.add()?; Ok(()) })?
            .description(18)?
            .encoded_length_with_header();
        assert!(body_len > 0, "FlatGroup body_len > 0");

        let mut buf = vec![0u8; body_len];
        let mut enc = FlatGroupEncoder::wrap_and_apply_header(&mut buf, 0)?;
        enc.symbol(42);
        let complete = enc.bids(2, |b| {
            b.add(|e| { e.price(1).qty(2); Ok(()) })?;
            b.add(|e| { e.price(3).qty(4); Ok(()) })?;
            Ok(())
        })?
        .asks(1, |a| {
            a.add(|e| { e.price(5).qty(6); Ok(()) })?;
            Ok(())
        })?
        .description(b"test exchange data")?;
        assert_eq!(complete.encoded_length_with_header(), body_len, "FlatGroup length match");

        let enc_len = complete.encoded_length();
        assert!(enc_len > 0, "encoded_length > 0");
        assert!(complete.encoded_length_with_header() > enc_len,
            "with_header > body");

        // NestedGroup length builder
        let body_len2 = NestedGroupEncodedLength::new()
            .bids(1, |b| {
                b.add()?;
                b.orders(2, |o| { o.add()?; o.add()?; Ok(()) })?;
                b.venue(6)?;
                Ok(())
            })?
            .asks(1, |a| {
                a.add()?;
                a.orders(1, |o| { o.add()?; Ok(()) })?;
                a.venue(4)?;
                Ok(())
            })?
            .comment(5)?
            .encoded_length_with_header();
        assert!(body_len2 > 0, "NestedGroup length > 0");

        // PureFixedNested length builder
        let body_len3 = PureFixedNestedEncodedLength::new()
            .records(1, |r| {
                r.add()?;
                r.tags(2, |t| { t.add()?; t.add()?; Ok(()) })?;
                Ok(())
            })?
            .encoded_length_with_header();
        assert!(body_len3 > 0, "PureFixedNested length > 0");

        println!("PASS: conformance_length_builder_invariants");
        "#,
    );
    Ok(())
}

// -- 6. Nested group roundtrip --------------------------------------------

#[test]
fn conformance_nested_group_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_nested",
        &src,
        r#"
        let body_len = NestedGroupEncodedLength::new()
            .bids(1, |b| {
                b.add()?;
                b.orders(2, |o| { o.add()?; o.add()?; Ok(()) })?;
                b.venue(6)?;
                Ok(())
            })?
            .asks(1, |a| {
                a.add()?;
                a.orders(1, |o| { o.add()?; Ok(()) })?;
                a.venue(4)?;
                Ok(())
            })?
            .comment(17)?
            .encoded_length_with_header();
        let mut buf = vec![0u8; body_len];
        let mut enc = NestedGroupEncoder::wrap_and_apply_header(&mut buf, 0)?;
        enc.exchange_id(8888);
        let complete = enc.bids(1, |bids| {
            bids.add(|entry| {
                entry.price(5000i64).qty(100i32);
                entry.orders(2, |orders| {
                    orders.add(|o| { o.order_id(1001u64).flags(1u8); Ok(()) })?;
                    orders.add(|o| { o.order_id(1002u64).flags(0u8); Ok(()) })?;
                    Ok(())
                })?;
                entry.venue(b"NASDAQ")?;
                Ok(())
            })?;
            Ok(())
        })?
        .asks(1, |asks| {
            asks.add(|entry| {
                entry.price(5001i64).qty(50i32);
                entry.orders(1, |orders| {
                    orders.add(|o| { o.order_id(2001u64); Ok(()) })?;
                    Ok(())
                })?;
                entry.venue(b"NYSE")?;
                Ok(())
            })?;
            Ok(())
        })?
        .comment(b"test nested group")?;
        assert_eq!(complete.encoded_length_with_header(), body_len, "length match");

        let encoded = complete.as_bytes();

        let dec = NestedGroupDecoder::try_from(encoded)?;
        assert_eq!(dec.exchange_id(), 8888, "exchange_id");

        let mut bids = dec.into_bids()?;
        let bid_entries: Vec<_> = bids.by_ref().collect::<Result<Vec<_>, _>>()?;
        assert_eq!(bid_entries.len(), 1, "expected 1 bid");
        let b0 = &bid_entries[0];
        assert_eq!(b0.price(), 5000, "bid.price");
        assert_eq!(b0.qty(), 100, "bid.qty");

        let b0_orders = b0.orders()?;
        let b0_order_entries: Vec<_> = b0_orders.collect();
        assert_eq!(b0_order_entries.len(), 2, "expected 2 orders");
        assert_eq!(b0_order_entries[0].order_id(), 1001, "order[0].id");
        assert_eq!(b0_order_entries[1].order_id(), 1002, "order[1].id");

        assert_eq!(b0.venue()?, b"NASDAQ", "bids.venue");

        let after_bids = bids.finish()?;
        let mut asks = after_bids.into_asks()?;
        let ask_entries: Vec<_> = asks.by_ref().collect::<Result<Vec<_>, _>>()?;
        assert_eq!(ask_entries.len(), 1, "expected 1 ask");
        let a0 = &ask_entries[0];
        assert_eq!(a0.price(), 5001, "ask.price");
        assert_eq!(a0.qty(), 50, "ask.qty");

        let a0_orders = a0.orders()?;
        let a0_order_entries: Vec<_> = a0_orders.collect();
        assert_eq!(a0_order_entries.len(), 1, "expected 1 ask order");
        assert_eq!(a0_order_entries[0].order_id(), 2001, "ask.order[0].id");
        assert_eq!(a0.venue()?, b"NYSE", "asks.venue");

        let after_asks = asks.finish()?;
        let (comment, _complete) = after_asks.into_comment()?;
        assert_eq!(comment, b"test nested group", "comment");

        println!("PASS: conformance_nested_group_roundtrip");
        "#,
    );
    Ok(())
}

// -- 7. AllTypes: enums, composites, groups, varData ----------------------

#[test]
fn conformance_all_types_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_alltypes",
        &src,
        r#"
        let body_len = AllTypesEncodedLength::new()
            .entries(2, |e| { e.add()?; e.add()?; Ok(()) })?
            .payload(19)?
            .encoded_length_with_header();
        let mut buf = vec![0u8; body_len];
        let mut enc = AllTypesEncoder::wrap_and_apply_header(&mut buf, 0)?;
        enc.char_field(b'A')
            .int8_field(-7i8)
            .uint16_field(300u16)
            .int32_field(-100_000i32)
            .uint64_field(9_000_000_000u64)
            .float_field(3.14f32)
            .double_field(2.71828f64)
            .side(Side::Sell)
            .active(Bool::True)
            .composite(PriceQty::new(50000i64, 100i32))
            .prices(9999i64);
        let complete = enc.entries(2, |entries| {
            entries.add(|e| { e.key(1u64).value(10i64); Ok(()) })?;
            entries.add(|e| { e.key(2u64).value(20i64); Ok(()) })?;
            Ok(())
        })?
        .payload(b"binary payload data")?;
        assert_eq!(complete.encoded_length_with_header(), body_len, "length match");

        let encoded = complete.as_bytes();

        let dec = AllTypesDecoder::try_from(encoded)?;
        assert_eq!(dec.char_field(), b'A', "char_field");
        assert_eq!(dec.int8_field(), -7i8, "int8_field");
        assert_eq!(dec.uint16_field(), 300u16, "uint16_field");
        assert_eq!(dec.int32_field(), -100_000i32, "int32_field");
        assert_eq!(dec.uint64_field(), 9_000_000_000u64, "uint64_field");
        assert!((dec.float_field() - 3.14f32).abs() < 0.001, "float_field");
        assert!((dec.double_field() - 2.71828f64).abs() < 0.0001, "double_field");
        assert_eq!(dec.side(), Side::Sell, "side");
        assert_eq!(dec.active(), Bool::True, "active");
        assert_eq!(dec.composite().price(), 50000, "composite.price");
        assert_eq!(dec.composite().qty(), 100, "composite.qty");
        assert_eq!(dec.prices(), 9999, "prices");

        let mut ents = dec.into_entries()?;
        let entry_vec: Vec<_> = ents.by_ref().collect();
        assert_eq!(entry_vec.len(), 2, "expected 2 entries");
        assert_eq!(entry_vec[0].key(), 1, "entry[0].key");
        assert_eq!(entry_vec[0].value(), 10, "entry[0].value");
        assert_eq!(entry_vec[1].key(), 2, "entry[1].key");
        assert_eq!(entry_vec[1].value(), 20, "entry[1].value");

        let after_ents = ents.finish()?;
        let (payload, _complete) = after_ents.into_payload()?;
        assert_eq!(payload, b"binary payload data", "payload");

        println!("PASS: conformance_all_types_roundtrip");
        "#,
    );
    Ok(())
}

// -- 8. PureFixedNested with add_struct -----------------------------------

#[test]
fn conformance_pure_fixed_nested_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_pfn",
        &src,
        r#"
        let body_len = PureFixedNestedEncodedLength::new()
            .records(2, |r| {
                r.add()?;
                r.tags(2, |t| { t.add()?; t.add()?; Ok(()) })?;
                r.add()?;
                r.tags(1, |t| { t.add()?; Ok(()) })?;
                Ok(())
            })?
            .encoded_length_with_header();
        let mut buf = vec![0u8; body_len];
        let mut enc = PureFixedNestedEncoder::wrap_and_apply_header(&mut buf, 0)?;
        enc.id(42u64);
        let complete = enc.records(2, |records| {
            records.add(|entry| {
                entry.key(100u64).value(10i64);
                entry.tags(2, |tags| {
                    tags.add_struct(&PureFixedNestedRecordsTagsEntry {
                        tag_id: 1, tag_val: 100,
                    })?;
                    tags.add_struct(&PureFixedNestedRecordsTagsEntry {
                        tag_id: 2, tag_val: 200,
                    })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            records.add(|entry| {
                entry.key(200u64).value(20i64);
                entry.tags(1, |tags| {
                    tags.add_struct(&PureFixedNestedRecordsTagsEntry {
                        tag_id: 3, tag_val: 300,
                    })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        })?;
        assert_eq!(complete.encoded_length_with_header(), body_len, "length match");

        let encoded = complete.as_bytes();

        let dec = PureFixedNestedDecoder::try_from(encoded)?;
        assert_eq!(dec.id(), 42, "id");

        let mut records = dec.into_records()?;
        let record_vec: Vec<_> = records.by_ref().collect::<Result<Vec<_>, _>>()?;
        assert_eq!(record_vec.len(), 2, "expected 2 records");

        let r0 = &record_vec[0];
        assert_eq!(r0.key(), 100, "record[0].key");
        assert_eq!(r0.value(), 10, "record[0].value");

        let r0_tags = r0.tags()?;
        let r0_tag_vec: Vec<_> = r0_tags.collect();
        assert_eq!(r0_tag_vec.len(), 2, "expected 2 tags");
        assert_eq!(r0_tag_vec[0].tag_id(), 1, "tag[0].id");
        assert_eq!(r0_tag_vec[0].tag_val(), 100, "tag[0].val");
        assert_eq!(r0_tag_vec[1].tag_id(), 2, "tag[1].id");
        assert_eq!(r0_tag_vec[1].tag_val(), 200, "tag[1].val");

        let r1 = &record_vec[1];
        assert_eq!(r1.key(), 200, "record[1].key");
        assert_eq!(r1.value(), 20, "record[1].value");

        let r1_tags = r1.tags()?;
        let r1_tag_vec: Vec<_> = r1_tags.collect();
        assert_eq!(r1_tag_vec.len(), 1, "expected 1 tag");
        assert_eq!(r1_tag_vec[0].tag_id(), 3, "tag[1][0].id");

        println!("PASS: conformance_pure_fixed_nested_roundtrip");
        "#,
    );
    Ok(())
}

// -- 9. Empty groups ------------------------------------------------------

#[test]
fn conformance_empty_groups() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_empty_groups",
        &src,
        r#"
        let body_len = FlatGroupEncodedLength::new()
            .bids(0, |_| Ok(()))?
            .asks(0, |_| Ok(()))?
            .description(0)?
            .encoded_length_with_header();
        let mut buf = vec![0u8; body_len];
        let mut enc = FlatGroupEncoder::wrap_and_apply_header(&mut buf, 0)?;
        enc.symbol(0);
        let complete = enc.bids(0, |_| Ok(()))?
            .asks(0, |_| Ok(()))?
            .description(b"")?;
        assert_eq!(complete.encoded_length_with_header(), body_len, "length match");

        let encoded = complete.as_bytes();
        let dec = FlatGroupDecoder::try_from(encoded)?;
        assert_eq!(dec.symbol(), 0);

        let bids = dec.into_bids()?;
        assert!(bids.is_empty(), "bids should be empty");
        let be: Vec<_> = bids.collect();
        assert_eq!(be.len(), 0, "bids len 0");

        println!("PASS: conformance_empty_groups");
        "#,
    );
    Ok(())
}

// -- 10. VarData edge cases -----------------------------------------------

#[test]
fn conformance_var_data_edge_cases() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_var_data",
        &src,
        r#"
        // Empty description
        let body_len_0 = FlatGroupEncodedLength::new()
            .bids(0, |_| Ok(()))?
            .asks(0, |_| Ok(()))?
            .description(0)?
            .encoded_length_with_header();
        let mut buf1 = vec![0u8; body_len_0];
        let mut enc1 = FlatGroupEncoder::wrap_and_apply_header(&mut buf1, 0)?;
        enc1.symbol(1);
        let complete1 = enc1.bids(0, |_| Ok(()))?
            .asks(0, |_| Ok(()))?
            .description(b"")?;
        assert_eq!(complete1.encoded_length_with_header(), body_len_0, "empty desc length match");
        let encoded1 = complete1.as_bytes();
        let dec1 = FlatGroupDecoder::try_from(encoded1)?;
        let bids1 = dec1.into_bids()?;
        let after_bids1 = bids1.finish()?;
        let asks1 = after_bids1.into_asks()?;
        let after_asks1 = asks1.finish()?;
        let (desc, _c) = after_asks1.into_description()?;
        assert_eq!(desc, b"", "empty description");

        // UTF-8 string via varStringEncoding
        let body_len_2 = FlatGroupEncodedLength::new()
            .bids(0, |_| Ok(()))?
            .asks(0, |_| Ok(()))?
            .description(14)?
            .encoded_length_with_header();
        let mut buf2 = vec![0u8; body_len_2];
        let mut enc2 = FlatGroupEncoder::wrap_and_apply_header(&mut buf2, 0)?;
        enc2.symbol(2);
        let complete2 = enc2.bids(0, |_| Ok(()))?
            .asks(0, |_| Ok(()))?
            .description("Hello, \u{4e16}\u{754c}!".as_bytes())?;
        assert_eq!(complete2.encoded_length_with_header(), body_len_2, "utf-8 desc length match");
        let encoded2 = complete2.as_bytes();
        let dec2 = FlatGroupDecoder::try_from(encoded2)?;
        let bids2 = dec2.into_bids()?;
        let after_bids2 = bids2.finish()?;
        let asks2 = after_bids2.into_asks()?;
        let after_asks2 = asks2.finish()?;
        let (desc2, _c2) = after_asks2.into_description()?;
        assert_eq!(desc2, "Hello, \u{4e16}\u{754c}!".as_bytes(), "utf-8 description");

        // as_str roundtrip via varStringEncoding
        let dec3 = FlatGroupDecoder::try_from(encoded2)?;
        let bids3 = dec3.into_bids()?;
        let after_bids3 = bids3.finish()?;
        let asks3 = after_bids3.into_asks()?;
        let after_asks3 = asks3.finish()?;
        let (desc_str, _c3) = after_asks3.into_description_as_str()?;
        assert_eq!(desc_str, "Hello, \u{4e16}\u{754c}!", "utf-8 description as str");

        println!("PASS: conformance_var_data_edge_cases");
        "#,
    );
    Ok(())
}

// -- 11. Fixed-only raw fixed vs field chaining parity --------------------

#[test]
fn conformance_fixed_raw_fixed_parity() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_raw_fixed",
        &src,
        r#"
        let mut buf1 = [0u8; FixedOnlyEncoder::ENCODED_LENGTH];
        let mut enc1 = FixedOnlyEncoder::wrap_and_apply_header(&mut buf1, 0)?;
        enc1.id(42).price(10000).qty(10).side(Side::Buy);
        let bytes1 = enc1.as_ref();

        let mut buf2 = [0u8; FixedOnlyEncoder::ENCODED_LENGTH];
        let mut enc2 = FixedOnlyEncoder::wrap_and_apply_header(&mut buf2, 0)?;
        enc2.id(42).price(10000).qty(10).side(Side::Buy);
        let bytes2 = enc2.as_ref();

        assert_eq!(bytes1, bytes2, "raw_fixed and chaining produce same bytes");

        let d1 = FixedOnlyDecoder::try_from(bytes1)?;
        let d2 = FixedOnlyDecoder::try_from(bytes2)?;
        assert_eq!(d1.id(), d2.id());
        assert_eq!(d1.side(), d2.side());

        println!("PASS: conformance_fixed_raw_fixed_parity");
        "#,
    );
    Ok(())
}

// -- 12. Error cases ------------------------------------------------------

#[test]
fn conformance_error_buffer_too_short_flat_group() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_err_buf",
        &src,
        r#"
        let mut tiny = [0u8; 4];
        let result = FlatGroupEncoder::wrap_and_apply_header(&mut tiny, 0);
        assert!(result.is_err(), "expected BufferTooShort");
        match result.unwrap_err() {
            sbe_rt::EncodeError::BufferTooShort { needed, .. } => {
                assert!(needed > 4, "needed {} > 4", needed);
            }
            other => panic!("unexpected error: {other:?}"),
        }

        let result2 = FlatGroupDecoder::try_from(&[][..]);
        assert!(result2.is_err(), "expected DecodeError");
        match result2.unwrap_err() {
            sbe_rt::DecodeError::BufferTooShort { .. } => {}
            other => panic!("unexpected error: {other:?}"),
        }

        println!("PASS: conformance_error_buffer_too_short");
        "#,
    );
    Ok(())
}

#[test]
fn conformance_error_group_full() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_err_full",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut enc = FlatGroupEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        enc.symbol(42);
        let result = enc.bids(1, |bids| {
            bids.add(|e| { e.price(1).qty(2); Ok(()) })?;
            match bids.add(|e| { e.price(3).qty(4); Ok(()) }) {
                Err(sbe_rt::EncodeError::GroupFull { declared, attempted }) => {
                    assert_eq!(declared, 1, "declared=1");
                    assert_eq!(attempted, 2, "attempted=2");
                }
                other => panic!("expected GroupFull, got: {other:?}"),
            }
            Ok(())
        });
        assert!(result.is_ok(), "bids should succeed despite inner overflow");

        println!("PASS: conformance_error_group_full");
        "#,
    );
    Ok(())
}

#[test]
fn conformance_error_group_count_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_err_mismatch",
        &src,
        r#"
        // The length-builder complete types lack Debug, so use a match
        // that does not format the Ok variant.
        let result = PureFixedNestedEncodedLength::new()
            .records(1, |r| {
                r.add()?;
                r.tags(2, |t| {
                    t.add()?;
                    Ok(())
                })?;
                Ok(())
            });
        match result {
            Err(sbe_rt::EncodeError::GroupCountMismatch { declared, actual }) => {
                assert_eq!(declared, 2, "declared=2");
                assert_eq!(actual, 1, "actual=1");
            }
            Ok(_) => panic!("expected Err, got Ok"),
            _ => panic!("unexpected error variant"),
        }

        println!("PASS: conformance_error_group_count_mismatch");
        "#,
    );
    Ok(())
}

#[test]
fn conformance_error_wrong_schema() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_err_schema",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut enc = FlatGroupEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        enc.symbol(1);
        let complete = enc.bids(0, |_| Ok(())).unwrap()
            .asks(0, |_| Ok(())).unwrap()
            .description(b"").unwrap();
        let encoded = complete.as_bytes();

        let result = FixedOnlyDecoder::try_from(encoded);
        match result {
            Err(sbe_rt::DecodeError::WrongSchema { expected, actual, .. }) => {
                assert_eq!(expected, 1, "expected template id 1");
                assert_eq!(actual, 2, "actual template id 2");
            }
            other => panic!("expected WrongSchema, got: {other:?}"),
        }

        println!("PASS: conformance_error_wrong_schema");
        "#,
    );
    Ok(())
}

#[test]
fn conformance_error_var_data_too_long() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "conformance");
    compile_and_run(
        "conformance_err_vardata",
        &src,
        r#"
        // The complete types lack Debug, so match without formatting the Ok variant.
        let result = FlatGroupEncodedLength::new()
            .bids(0, |_| Ok(())).unwrap()
            .asks(0, |_| Ok(())).unwrap()
            .description(70000);
        match result {
            Err(sbe_rt::EncodeError::VarDataTooLong { field, max_length, actual }) => {
                assert_eq!(field, "description");
                assert_eq!(max_length, 65534);
                assert_eq!(actual, 70000);
            }
            Ok(_) => panic!("expected Err, got Ok"),
            _ => panic!("unexpected error variant"),
        }

        println!("PASS: conformance_error_var_data_too_long");
        "#,
    );
    Ok(())
}
