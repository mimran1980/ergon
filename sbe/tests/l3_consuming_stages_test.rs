//! Canonical bids/asks dual-group proof (DECISIONS.md §3, Tasks A/D) on the
//! L3 orderbook fixture.
//!
//! Runtime: decode `bids` then `asks` through the consuming stage API
//! (`into_bids` -> `finish` -> `into_asks` -> `finish` -> complete), reading
//! nested `orders` + `orderId` inside each level.
//!
//! Compile-fail: the NEW consuming API enforces wire order even while the
//! legacy `&self` surface still coexists — `into_asks` lives only on
//! `L3BookDecoderAfterBids`, and `finish` consumes the group decoder.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, compile_and_run, compile_fails, generate};

/// Full ordered decode of bids then asks (with nested orders + var-data) through
/// the consuming message-level stages.
#[test]
fn decode_l3_through_consuming_stages() {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "l3_stages_rt");
    compile_and_run(
        "l3_stages_rt",
        &src,
        r#"
        let mut buf = vec![0u8; 1024];
        let mut e = L3BookEncoder::wrap_and_apply_header(&mut buf, 0);
        e.timestamp(99);
        e.sequence(7);
        let c = e.bids(2, |g| {
            g.add(|lvl| {
                lvl.price(100);
                lvl.qty(10);
                lvl.orders(2, |o| {
                    o.add(|ord| { ord.order_qty(4); ord.order_id(b"ord-1").unwrap(); }).unwrap();
                    o.add(|ord| { ord.order_qty(6); ord.order_id(b"ord-2").unwrap(); }).unwrap();
                }).unwrap();
            }).unwrap();
            g.add(|lvl| {
                lvl.price(101);
                lvl.qty(5);
                lvl.orders(0, |_| {}).unwrap();
            }).unwrap();
        }).unwrap().asks(1, |g| {
            g.add(|lvl| {
                lvl.price(200);
                lvl.qty(20);
                lvl.orders(1, |o| {
                    o.add(|ord| { ord.order_qty(8); ord.order_id(b"ask-1").unwrap(); }).unwrap();
                }).unwrap();
            }).unwrap();
        }).unwrap();
        let encoded = c.as_bytes();
        let total_len = encoded.len();

        let dec = L3BookDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        assert_eq!(dec.timestamp(), 99);
        assert_eq!(dec.sequence(), 7);

        // bids: consume the message stage, iterate levels, read nested orders.
        let mut bids = dec.into_bids().unwrap();
        assert_eq!(bids.len(), 2);
        let mut level_prices = Vec::new();
        let mut level_qtys = Vec::new();
        let mut all_order_ids: Vec<Vec<Vec<u8>>> = Vec::new();
        while let Some(Ok(lvl)) = bids.next() {
            level_prices.push(lvl.price());
            level_qtys.push(lvl.qty());
            let mut ids: Vec<Vec<u8>> = Vec::new();
            for ord in lvl.orders().unwrap() {
                let ord = ord.unwrap();
                ids.push(ord.order_id().unwrap().to_vec());
            }
            all_order_ids.push(ids);
        }
        let after_bids = bids.finish().unwrap();
        assert_eq!(level_prices, vec![100i64, 101]);
        assert_eq!(level_qtys, vec![10i64, 5]);
        assert_eq!(all_order_ids, vec![vec![b"ord-1".to_vec(), b"ord-2".to_vec()], vec![]]);

        // asks: only reachable after bids finished.
        let mut asks = after_bids.into_asks().unwrap();
        assert_eq!(asks.len(), 1);
        let ask_level = asks.next().unwrap().unwrap();
        assert_eq!(ask_level.price(), 200);
        assert_eq!(ask_level.qty(), 20);
        let mut ask_order_qtys = Vec::new();
        for ord in ask_level.orders().unwrap() {
            ask_order_qtys.push(ord.unwrap().order_qty());
        }
        assert_eq!(ask_order_qtys, vec![8i64]);
        let done = asks.finish().unwrap();

        assert_eq!(done.encoded_length_with_header(), total_len);
        assert_eq!(done.as_bytes(), encoded);
    "#,
    );
}

/// Compile-fail: `into_asks` does not exist on the initial `L3BookDecoder`; it
/// is only on `L3BookDecoderAfterBids`. So decoding asks before bids cannot compile.
#[test]
fn cf_decode_asks_before_bids() {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "l3_cf_asks_before_bids");
    compile_fails(
        "l3_cf_asks_before_bids",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let mut e = L3BookEncoder::wrap_and_apply_header(&mut buf, 0);
        e.timestamp(1);
        e.sequence(1);
        let c = e.bids(0, |_| {}).unwrap().asks(0, |_| {}).unwrap();
        let dec = L3BookDecoder::wrap_and_apply_header(c.as_bytes(), 0).unwrap();
        let _ = dec.into_asks(); // ILLEGAL: no `into_asks` on the initial decoder
    "#,
    );
}

/// Compile-fail: `finish()` consumes the group decoder (which is non-Copy), so
/// the consumed decoder cannot be iterated afterwards.
#[test]
fn cf_finish_consumes_group_decoder() {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "l3_cf_finish_consumes");
    compile_fails(
        "l3_cf_finish_consumes",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let mut e = L3BookEncoder::wrap_and_apply_header(&mut buf, 0);
        e.timestamp(1);
        e.sequence(1);
        let c = e.bids(0, |_| {}).unwrap().asks(0, |_| {}).unwrap();
        let dec = L3BookDecoder::wrap_and_apply_header(c.as_bytes(), 0).unwrap();
        let mut bids = dec.into_bids().unwrap();
        let _after = bids.finish().unwrap(); // bids moved here
        let _ = bids.next();                  // ILLEGAL: use of moved value `bids`
    "#,
    );
}
