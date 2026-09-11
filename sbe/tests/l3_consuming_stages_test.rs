//! Canonical bids/asks dual-group proof on the L3 orderbook fixture.
//!
//! Runtime: decode `bids` then `asks` through the consuming stage API
//! (`into_bids` → `into_asks` → complete), reading nested `orders` + `orderId`
//! inside each level.
//!
//! Compile-fail: the consuming API enforces wire order — `into_asks` lives only
//! on `L3BookDecoderAfterBids` (and the bids iterator), and `into_bids`
//! consumes the decoder.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, compile_and_run, compile_fails_with_diagnostics, generate};

/// Full ordered decode of bids then asks (with nested orders + var-data) through
/// the consuming message-level stages.
#[test]
fn decode_l3_through_consuming_stages() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "l3_stages_rt");
    compile_and_run(
        "l3_stages_rt",
        &src,
        r#"
        let mut buf = [0u8; 1024];
        let c = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
        .fixed(&L3BookFixedFields { timestamp: 99, sequence: 7 })
        .bids(2, |g| {
            g.add(|mut lvl| {
                lvl.price(100);
                lvl.qty(10);
                lvl.orders(2, |o| {
                    o.add(|mut ord| { ord.order_qty(4); ord.order_id(b"ord-1") }).unwrap();
                    o.add(|mut ord| { ord.order_qty(6); ord.order_id(b"ord-2") })
                })
            }).unwrap();
            g.add(|mut lvl| {
                lvl.price(101);
                lvl.qty(5);
                lvl.orders(0, |_| Ok(()))
            })
        }).unwrap().asks(1, |g| {
            g.add(|mut lvl| {
                lvl.price(200);
                lvl.qty(20);
                lvl.orders(1, |o| {
                    o.add(|mut ord| {
                        ord.order_qty(8);
                        ord.order_id(b"ask-1")
                    })
                })
            })
        }).unwrap();
        let encoded = c.as_bytes_with_header();
        // as_bytes_with_header() is the explicit header-inclusive view.
        assert_eq!(c.as_bytes_with_header(), encoded);
        let total_len = encoded.len();

        let dec = L3BookDecoder::try_decode(encoded, 0).unwrap();
        assert_eq!(dec.timestamp(), 99);
        assert_eq!(dec.sequence(), 7);

        // bids: consume the message stage, iterate levels, read nested orders.
        let mut level_prices = Vec::new();
        let mut level_qtys = Vec::new();
        let mut all_order_ids: Vec<Vec<Vec<u8>>> = Vec::new();
        let mut ask_prices = Vec::new();
        let mut ask_order_qtys = Vec::new();
        let done = dec
            .into_bids(|lvl| -> Result<_, sbe_rt::DecodeError> {
                level_prices.push(lvl.price());
                level_qtys.push(lvl.qty());
                let mut ids: Vec<Vec<u8>> = Vec::new();
                let complete = lvl.into_orders(|ord| {
                    let (id, done) = ord.into_order_id()?;
                    ids.push(id.to_vec());
                    Ok(done)
                })?;
                all_order_ids.push(ids);
                Ok(complete)
            })
            .unwrap()
            .into_asks(|lvl| -> Result<_, sbe_rt::DecodeError> {
                ask_prices.push(lvl.price());
                assert_eq!(lvl.qty(), 20);
                lvl.into_orders(|ord| {
                    ask_order_qtys.push(ord.order_qty());
                    ord.into_order_id().map(|(_id, done)| done)
                })
            })
            .unwrap();
        assert_eq!(level_prices, vec![100i64, 101]);
        assert_eq!(level_qtys, vec![10i64, 5]);
        assert_eq!(all_order_ids, vec![vec![b"ord-1".to_vec(), b"ord-2".to_vec()], vec![]]);
        assert_eq!(ask_prices, vec![200i64]);
        assert_eq!(ask_order_qtys, vec![8i64]);

        assert_eq!(done.encoded_length_with_header(), total_len);
        assert_eq!(done.as_bytes_with_header(), encoded);
    "#,
    );

    Ok(())
}

/// Nested dynamic `into_*` iterators: bids → orders → orderId, then asks.
#[test]
fn decode_l3_through_into_entries() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "l3_visit_rt");
    compile_and_run(
        "l3_visit_rt",
        &src,
        r#"
        let mut storage = [0u8; 512];
        let len = L3BookEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&L3BookFixedFields { timestamp: 99, sequence: 7 })
            .bids(2, |g| {
                g.add(|mut lvl| {
                    lvl.price(100).qty(10);
                    lvl.orders(2, |o| {
                        o.add(|mut ord| {
                            ord.order_qty(4);
                            ord.order_id(b"ord-1")
                        })?;
                        o.add(|mut ord| {
                            ord.order_qty(6);
                            ord.order_id(b"ord-2")
                        })?;
                        Ok(())
                    })
                })?;
                g.add(|mut lvl| {
                    lvl.price(101).qty(5);
                    lvl.orders(0, |_| Ok(()))
                })?;
                Ok(())
            })?
            .asks(1, |g| {
                g.add(|mut lvl| {
                    lvl.price(200).qty(20);
                    lvl.orders(1, |o| {
                        o.add(|mut ord| {
                            ord.order_qty(8);
                            ord.order_id(b"ask-1")
                        })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        let encoded = &storage[..len];

        let dec = L3BookDecoder::try_decode(encoded, 0)?;
        assert_eq!(dec.timestamp(), 99);
        assert_eq!(dec.sequence(), 7);

        let mut level_prices = Vec::new();
        let mut all_order_ids: Vec<Vec<Vec<u8>>> = Vec::new();
        let mut ask_prices = Vec::new();
        let done = dec
            .into_bids(|lvl| -> Result<_, sbe_rt::DecodeError> {
                level_prices.push(lvl.price());
                let mut ids = Vec::new();
                let complete = lvl.into_orders(|ord| {
                    let (id, done) = ord.into_order_id()?;
                    ids.push(id.to_vec());
                    Ok(done)
                })?;
                all_order_ids.push(ids);
                Ok(complete)
            })?
            .into_asks(|lvl| -> Result<_, sbe_rt::DecodeError> {
                ask_prices.push(lvl.price());
                lvl.into_orders(|ord| ord.into_order_id().map(|(_id, done)| done))
            })?;
        assert_eq!(level_prices, vec![100i64, 101]);
        assert_eq!(all_order_ids, vec![vec![b"ord-1".to_vec(), b"ord-2".to_vec()], vec![]]);
        assert_eq!(ask_prices, vec![200i64]);
        assert_eq!(done.encoded_length_with_header(), len);
        assert_eq!(done.as_bytes_with_header(), encoded);
    "#,
    );
    Ok(())
}

/// Compile-fail: `into_asks` does not exist on the initial `L3BookDecoder`; it
/// is only on `L3BookDecoderAfterBids`. So decoding asks before bids cannot compile.
#[test]
fn cf_decode_asks_before_bids() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "l3_cf_asks_before_bids");
    compile_fails_with_diagnostics(
        "l3_cf_asks_before_bids",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let c = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
        .fixed(&L3BookFixedFields { timestamp: 1, sequence: 1 })
        .bids(0, |_| Ok(())).unwrap().asks(0, |_| Ok(())).unwrap();
        let dec = L3BookDecoder::try_decode(c.as_bytes_with_header(), 0).unwrap();
        let _ = dec.into_asks(); // ILLEGAL: no `into_asks` on the initial decoder
    "#,
        &["no method named `into_asks`"],
    );

    Ok(())
}

/// Compile-fail: `into_bids` consumes the decoder, so the original value cannot
/// be used afterwards.
#[test]
fn cf_finish_consumes_group_decoder() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "l3_cf_finish_consumes");
    compile_fails_with_diagnostics(
        "l3_cf_finish_consumes",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let c = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
        .fixed(&L3BookFixedFields { timestamp: 1, sequence: 1 })
        .bids(0, |_| Ok(())).unwrap().asks(0, |_| Ok(())).unwrap();
        let dec = L3BookDecoder::try_decode(c.as_bytes_with_header(), 0).unwrap();
        let _after = dec.into_bids(|lvl| lvl.into_orders(
            |ord| ord.into_order_id().map(|(_id, done)| done),
        )).unwrap();
        let _ = dec.timestamp(); // ILLEGAL: use of moved value `dec`
    "#,
        &["borrow of moved value: `dec`"],
    );

    Ok(())
}

/// Entry-level consuming stages (Task D): a bid level's nested `orders` (and each
/// order's `orderId` var-data) are read through consuming entry stages, in wire
/// order. The level entry is consumed by `into_orders`; each order entry by
/// `into_order_id`.
#[test]
fn decode_l3_entry_consuming_stages() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "l3_entry_stages_rt");
    compile_and_run(
        "l3_entry_stages_rt",
        &src,
        r#"
        let mut buf = [0u8; 1024];
        let c = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
        .fixed(&L3BookFixedFields { timestamp: 5, sequence: 3 })
        .bids(2, |g| {
            g.add(|mut lvl| {
                lvl.price(100);
                lvl.qty(10);
                lvl.orders(2, |o| {
                    o.add(|mut ord| { ord.order_qty(4); ord.order_id(b"ord-1") }).unwrap();
                    o.add(|mut ord| { ord.order_qty(6); ord.order_id(b"ord-2") })
                })
            }).unwrap();
            g.add(|mut lvl| {
                lvl.price(101);
                lvl.qty(5);
                lvl.orders(0, |_| Ok(()))
            })
        }).unwrap().asks(0, |_| Ok(())).unwrap();
        let encoded = c.as_bytes_with_header();
        assert_eq!(c.as_bytes_with_header(), encoded);

        let dec = L3BookDecoder::try_decode(encoded, 0).unwrap();
        let mut order_ids = Vec::new();
        let mut prices = Vec::new();
        let done = dec
            .into_bids(|lvl| -> Result<_, sbe_rt::DecodeError> {
                prices.push(lvl.price());
                lvl.into_orders(|ord| {
                    let (id, done) = ord.into_order_id()?;
                    order_ids.push(id.to_vec());
                    Ok(done)
                })
            })
            .unwrap()
            // asks is empty, so the closure runs zero times.
            .into_asks(|lvl| -> Result<_, sbe_rt::DecodeError> {
                lvl.into_orders(|ord| ord.into_order_id().map(|(_id, done)| done))
            })
            .unwrap();
        assert_eq!(prices, vec![100i64, 101]);
        assert_eq!(order_ids, vec![b"ord-1".to_vec(), b"ord-2".to_vec()]);
        assert_eq!(done.encoded_length_with_header(), encoded.len());
        assert_eq!(done.as_bytes_with_header(), encoded);
    "#,
    );

    Ok(())
}

/// Compile-fail: `into_orders()` consumes the (non-Copy) entry decoder, so the
/// consumed level cannot be read afterwards.
#[test]
fn cf_entry_consumed_by_into_orders() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "l3_cf_entry_consumed");
    compile_fails_with_diagnostics(
        "l3_cf_entry_consumed",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let c = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
        .fixed(&L3BookFixedFields { timestamp: 1, sequence: 1 })
        .bids(1, |g| {
            g.add(|mut lvl| { lvl.price(1); lvl.qty(1); lvl.orders(0, |_| Ok(())) })
        }).unwrap().asks(0, |_| Ok(())).unwrap();
        let dec = L3BookDecoder::try_decode(c.as_bytes_with_header(), 0).unwrap();
        let _ = dec.into_bids(|lvl| -> Result<_, sbe_rt::DecodeError> {
            let complete = lvl.into_orders(
                |ord| ord.into_order_id().map(|(_id, done)| done),
            )?;
            let _p = lvl.price(); // ILLEGAL: use of moved value `lvl`
            Ok(complete)
        });
    "#,
        &["borrow of moved value: `lvl`"],
    );

    Ok(())
}
