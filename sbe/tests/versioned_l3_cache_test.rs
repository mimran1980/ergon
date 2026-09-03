//! Versioned nested L3 book: encode at version N, decode with any module.
//!
//! Codecs are generated from the latest schema (`versioned-l3-v3.xml`) with
//! [`ergo_sbe::GenerationConfig::with_encode_version`]. The encoder omits
//! nested groups and var-data above that version (zero bytes, not a
//! count-zero header). The decoder keeps the full tree, so encoder v2 +
//! decoder v1 and encoder v1 + decoder v2 both round-trip present fields.
//! Absent tails return `FieldNotInVersion`.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{
    Paths, compile_and_run, compile_and_run_modules, compile_and_run_two_modules,
    generate_domain_with,
};
use std::path::Path;

/// Memoization is opt-in; the cache assertions below need it on.
fn generate(xml_path: &Path, module_name: &str) -> (ergo_sbe::Schema, String) {
    generate_domain_with(xml_path, module_name, |c| {
        c.with_memoized_tail_offsets(true)
    })
}

#[test]
fn v3_decoder_reads_v0_buffer_in_any_order() -> Result<(), Box<dyn std::error::Error>> {
    let (_s0, src0) = generate(&Paths::versioned_l3_schema(0), "vl3_v0");
    let (_s3, src3) = generate(&Paths::versioned_l3_schema(3), "vl3_v3");
    compile_and_run_two_modules(
        "vl3_v0_into_v3",
        "vl3_v0",
        &src0,
        "vl3_v3",
        &src3,
        r#"
        use vl3_v3::sbe_rt;
        let sized = vl3_v0::L3BookEncodedLength::new()
            .bids_ragged(1, |b| {
                b.add()?.orders(|o| { o.add()?.order_id(2)?; Ok(()) })?;
                Ok(())
            })?
            .asks_ragged(0, |_| Ok(()))?
            .symbol(3)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = vl3_v0::L3BookEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&vl3_v0::L3BookFixedFields { timestamp: 11, sequence: 22 })
            .bids(1, |g| {
                g.add(|mut lvl| {
                    lvl.price(100).qty(10);
                    lvl.orders(1, |o| {
                        o.add(|mut ord| { ord.order_qty(4); ord.order_id(b"o1") })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .asks(0, |_| Ok(()))?
            .symbol(b"IBM")?
            .encoded_length_with_header();
        assert_eq!(sized, len, "EncodedLength must match the encoder");
        let encoded = &storage[..len];

        let dec = vl3_v3::L3BookDecoder::try_decode(encoded, 0)?;
        assert_eq!(dec.timestamp(), 11);
        assert_eq!(dec.sequence(), 22);
        assert!(dec.epoch().is_none(), "epoch is sinceVersion 1");
        assert!(dec.flags().is_none(), "flags is sinceVersion 3");
        match dec.source() {
            Err(sbe_rt::DecodeError::FieldNotInVersion { field, since_version, .. }) => {
                assert_eq!(field, "source");
                assert_eq!(since_version, 1);
            }
            other => panic!("expected FieldNotInVersion for source, got {other:?}"),
        }
        match dec.note() {
            Err(sbe_rt::DecodeError::FieldNotInVersion { .. }) => {}
            other => panic!("expected FieldNotInVersion for note, got {other:?}"),
        }

        let bids = dec.bids()?;
        let mut prices = Vec::new();
        for lvl in bids {
            let lvl = lvl?;
            prices.push(lvl.price());
            match lvl.venue() {
                Err(sbe_rt::DecodeError::FieldNotInVersion { .. }) => {}
                other => panic!("venue absent in v0, got {other:?}"),
            }
        }
        assert_eq!(prices, vec![100i64]);
        assert_eq!(dec.symbol()?, b"IBM");

        let reverse = vl3_v3::L3BookDecoder::try_decode(encoded, 0)?;
        assert_eq!(reverse.symbol()?, b"IBM");
        let asks = reverse.asks()?;
        assert!(asks.is_empty());
        let bids = reverse.bids()?;
        assert_eq!(bids.remaining_entries(), 1);
        assert_eq!(reverse.timestamp(), 11);
        let stats = reverse.decode_cache_stats();
        assert!(stats.known_through >= 2);
    "#,
    );
    Ok(())
}

#[test]
fn v3_roundtrip_dense_book() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, src) = generate(&Paths::versioned_l3_schema(3), "vl3_v3_dense");
    compile_and_run(
        "vl3_v3_dense",
        &src,
        r#"
        let sized = L3BookEncodedLength::new()
            .bids_ragged(1, |b| {
                b.add()?
                    .orders(|o| {
                        o.add()?
                            .allocations(|a| {
                                a.add()?.legs(|lg| { lg.add()?.leg_ref(2)?; Ok(()) })?;
                                Ok(())
                            })?
                            .order_id(2)?
                            .trader_id(2)?;
                        Ok(())
                    })?
                    .stats(|st| { st.add()?; Ok(()) })?
                    .venue(4)?;
                Ok(())
            })?
            .asks_ragged(0, |_| Ok(()))?
            .audit(1)?
            .symbol(3)?
            .source(3)?
            .checksum(2)?
            .note(1)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = L3BookEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&L3BookFixedFields {
                timestamp: 1,
                sequence: 2,
                epoch: 3,
                flags: 4,
            })
            .bids(1, |g| {
                g.add(|mut lvl| {
                    lvl.price(10).qty(1).participant(9);
                    let after_orders = lvl.orders(1, |o| {
                        o.add(|mut ord| {
                            ord.order_qty(8);
                            let after_alloc = ord.allocations(1, |a| {
                                a.add(|mut al| {
                                    al.alloc_qty(8);
                                    al.legs(1, |lg| {
                                        lg.add(|mut leg| {
                                            leg.leg_qty(8);
                                            leg.leg_ref(b"L1")
                                        })?;
                                        Ok(())
                                    })
                                })?;
                                Ok(())
                            })?;
                            after_alloc.order_id(b"id")?.trader_id(b"tr")
                        })?;
                        Ok(())
                    })?;
                    after_orders.stats(1, |s| {
                        s.add(|mut st| { st.fill_count(1).fill_qty(8); Ok(()) })?;
                        Ok(())
                    })?.venue(b"XNAS")
                })?;
                Ok(())
            })?
            .asks(0, |_| Ok(()))?
            .audit(1, |g| {
                g.add(|a| { a.ts(99).code(7); Ok(()) })?;
                Ok(())
            })?
            .symbol(b"IBM")?
            .source(b"src")?
            .checksum(b"ck")?
            .note(b"n")?
            .encoded_length_with_header();
        assert_eq!(sized, len, "EncodedLength must match the encoder");
        let encoded = &storage[..len];
        let dec = L3BookDecoder::try_decode(encoded, 0)?;
        assert_eq!(dec.timestamp(), 1);
        assert_eq!(dec.epoch(), Some(3));
        assert_eq!(dec.flags(), Some(4));
        assert_eq!(dec.symbol()?, b"IBM");
        assert_eq!(dec.note()?, b"n");
        assert_eq!(dec.source()?, b"src");
        assert_eq!(dec.checksum()?, b"ck");
        let bids = dec.bids()?;
        assert_eq!(bids.remaining_entries(), 1);
        let stats = dec.decode_cache_stats();
        assert!(stats.known_through >= 1);
        let again = dec.note()?;
        assert_eq!(again, b"n");
        let warm = dec.decode_cache_stats();
        assert!(warm.hits >= 1);
        assert_eq!(warm.known_through, stats.known_through.max(warm.known_through));
    "#,
    );
    Ok(())
}

#[test]
fn empty_groups_and_empty_var_data() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, src) = generate(&Paths::versioned_l3_schema(3), "vl3_empty");
    compile_and_run(
        "vl3_empty",
        &src,
        r#"
        let sized = L3BookEncodedLength::new()
            .bids_ragged(0, |_| Ok(()))?
            .asks_ragged(0, |_| Ok(()))?
            .audit(0)?
            .symbol(0)?
            .source(0)?
            .checksum(0)?
            .note(0)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = L3BookEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&L3BookFixedFields {
                timestamp: 0,
                sequence: 0,
                epoch: 0,
                flags: 0,
            })
            .bids(0, |_| Ok(()))?
            .asks(0, |_| Ok(()))?
            .audit(0, |_| Ok(()))?
            .symbol(b"")?
            .source(b"")?
            .checksum(b"")?
            .note(b"")?
            .encoded_length_with_header();
        assert_eq!(sized, len, "EncodedLength must match the encoder");
        let encoded = &storage[..len];
        let dec = L3BookDecoder::try_decode(encoded, 0)?;
        assert!(dec.bids()?.is_empty());
        assert!(dec.asks()?.is_empty());
        assert!(dec.audit()?.is_empty());
        assert_eq!(dec.symbol()?, b"");
        assert_eq!(dec.note()?, b"");
        let reverse = L3BookDecoder::try_decode(encoded, 0)?;
        assert_eq!(reverse.note()?, b"");
        assert!(reverse.bids()?.is_empty());
    "#,
    );
    Ok(())
}

#[test]
fn each_snapshot_encodes_and_later_decoders_read_earlier_wire()
-> Result<(), Box<dyn std::error::Error>> {
    let mut modules = Vec::new();
    let mut sources = Vec::new();
    for version in 0u16..=3 {
        let name = format!("vl3_v{version}");
        let (_schema, src) = generate_domain_with(&Paths::versioned_l3_schema(3), &name, |c| {
            c.with_memoized_tail_offsets(true)
                .with_encode_version(version)
        });
        sources.push(src);
        modules.push(name);
    }
    let pairs: Vec<(&str, &str)> = modules
        .iter()
        .map(String::as_str)
        .zip(sources.iter().map(String::as_str))
        .collect();
    compile_and_run_modules(
        "vl3_version_matrix",
        &pairs,
        r#"
        use vl3_v0::sbe_rt as rt0;
        use vl3_v1::sbe_rt as rt1;
        use vl3_v2::sbe_rt as rt2;
        use vl3_v3::sbe_rt as rt3;

        macro_rules! absent {
            ($rt:ident, $expr:expr, $name:expr, $since:expr, $wire:expr) => {
                match $expr {
                    Err($rt::DecodeError::FieldNotInVersion {
                        field,
                        since_version,
                        wire_version,
                    }) => {
                        assert_eq!(field, $name);
                        assert_eq!(since_version, $since);
                        assert_eq!(wire_version, $wire);
                    }
                    Err(other) => panic!(
                        "expected FieldNotInVersion for {} at wire {}, got {other:?}",
                        $name, $wire
                    ),
                    Ok(_) => panic!(
                        "expected FieldNotInVersion for {} at wire {}, got Ok",
                        $name, $wire
                    ),
                }
            };
        }

        fn encode_v0() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let sized = vl3_v0::L3BookEncodedLength::new()
                .bids_ragged(1, |b| {
                    b.add()?.orders(|o| { o.add()?.order_id(2)?; Ok(()) })?;
                    Ok(())
                })?
                .asks_ragged(0, |_| Ok(()))?
                .symbol(3)?
                .encoded_length_with_header();
            let mut buf = vec![0u8; sized];
            let len = vl3_v0::L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
                .fixed(&vl3_v0::L3BookFixedFields {
                    timestamp: 11,
                    sequence: 22,
                })
                .bids(1, |g| {
                    g.add(|mut lvl| {
                        lvl.price(100).qty(10);
                        lvl.orders(1, |o| {
                            o.add(|mut ord| {
                                ord.order_qty(4);
                                ord.order_id(b"o1")
                            })?;
                            Ok(())
                        })
                    })?;
                    Ok(())
                })?
                .asks(0, |_| Ok(()))?
                .symbol(b"IBM")?
                .encoded_length_with_header();
            assert_eq!(sized, len, "EncodedLength must match the encoder");
            buf.truncate(len);
            Ok(buf)
        }

        fn encode_v1() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let sized = vl3_v1::L3BookEncodedLength::new()
                .bids_ragged(1, |b| {
                    b.add()?
                        .orders(|o| { o.add()?.order_id(2)?.trader_id(2)?; Ok(()) })?
                        .venue(4)?;
                    Ok(())
                })?
                .asks_ragged(0, |_| Ok(()))?
                .symbol(3)?
                .source(3)?
                .encoded_length_with_header();
            let mut buf = vec![0u8; sized];
            let len = vl3_v1::L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
                .fixed(&vl3_v1::L3BookFixedFields {
                    timestamp: 11,
                    sequence: 22,
                    epoch: 33,
                })
                .bids(1, |g| {
                    g.add(|mut lvl| {
                        lvl.price(100).qty(10).participant(9);
                        lvl.orders(1, |o| {
                            o.add(|mut ord| {
                                ord.order_qty(4);
                                ord.order_id(b"o1")?.trader_id(b"tr")
                            })?;
                            Ok(())
                        })?
                        .venue(b"XNAS")
                    })?;
                    Ok(())
                })?
                .asks(0, |_| Ok(()))?
                .symbol(b"IBM")?
                .source(b"src")?
                .encoded_length_with_header();
            assert_eq!(sized, len, "EncodedLength must match the encoder");
            buf.truncate(len);
            Ok(buf)
        }

        fn encode_v2() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let sized = vl3_v2::L3BookEncodedLength::new()
                .bids_ragged(1, |b| {
                    b.add()?
                        .orders(|o| {
                            o.add()?
                                .allocations(|a| { a.add()?; Ok(()) })?
                                .order_id(2)?
                                .trader_id(2)?;
                            Ok(())
                        })?
                        .stats(|st| { st.add()?; Ok(()) })?
                        .venue(4)?;
                    Ok(())
                })?
                .asks_ragged(0, |_| Ok(()))?
                .audit(1)?
                .symbol(3)?
                .source(3)?
                .checksum(2)?
                .encoded_length_with_header();
            let mut buf = vec![0u8; sized];
            let len = vl3_v2::L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
                .fixed(&vl3_v2::L3BookFixedFields {
                    timestamp: 11,
                    sequence: 22,
                    epoch: 33,
                })
                .bids(1, |g| {
                    g.add(|mut lvl| {
                        lvl.price(100).qty(10).participant(9);
                        let after_orders = lvl.orders(1, |o| {
                            o.add(|mut ord| {
                                ord.order_qty(4);
                                let after_alloc = ord.allocations(1, |a| {
                                    a.add(|al| {
                                        al.alloc_qty(8);
                                        Ok(())
                                    })?;
                                    Ok(())
                                })?;
                                after_alloc.order_id(b"o1")?.trader_id(b"tr")
                            })?;
                            Ok(())
                        })?;
                        after_orders
                            .stats(1, |s| {
                                s.add(|st| {
                                    st.fill_count(1).fill_qty(8);
                                    Ok(())
                                })?;
                                Ok(())
                            })?
                            .venue(b"XNAS")
                    })?;
                    Ok(())
                })?
                .asks(0, |_| Ok(()))?
                .audit(1, |g| {
                    g.add(|a| {
                        a.ts(99).code(7);
                        Ok(())
                    })?;
                    Ok(())
                })?
                .symbol(b"IBM")?
                .source(b"src")?
                .checksum(b"ck")?
                .encoded_length_with_header();
            assert_eq!(sized, len, "EncodedLength must match the encoder");
            buf.truncate(len);
            Ok(buf)
        }

        fn encode_v3() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let sized = vl3_v3::L3BookEncodedLength::new()
                .bids_ragged(1, |b| {
                    b.add()?
                        .orders(|o| {
                            o.add()?
                                .allocations(|a| {
                                    a.add()?.legs(|lg| { lg.add()?.leg_ref(2)?; Ok(()) })?;
                                    Ok(())
                                })?
                                .order_id(2)?
                                .trader_id(2)?;
                            Ok(())
                        })?
                        .stats(|st| { st.add()?; Ok(()) })?
                        .venue(4)?;
                    Ok(())
                })?
                .asks_ragged(0, |_| Ok(()))?
                .audit(1)?
                .symbol(3)?
                .source(3)?
                .checksum(2)?
                .note(1)?
                .encoded_length_with_header();
            let mut buf = vec![0u8; sized];
            let len = vl3_v3::L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
                .fixed(&vl3_v3::L3BookFixedFields {
                    timestamp: 11,
                    sequence: 22,
                    epoch: 33,
                    flags: 44,
                })
                .bids(1, |g| {
                    g.add(|mut lvl| {
                        lvl.price(100).qty(10).participant(9);
                        let after_orders = lvl.orders(1, |o| {
                            o.add(|mut ord| {
                                ord.order_qty(4);
                                let after_alloc = ord.allocations(1, |a| {
                                    a.add(|mut al| {
                                        al.alloc_qty(8);
                                        al.legs(1, |lg| {
                                            lg.add(|mut leg| {
                                                leg.leg_qty(8);
                                                leg.leg_ref(b"L1")
                                            })?;
                                            Ok(())
                                        })
                                    })?;
                                    Ok(())
                                })?;
                                after_alloc.order_id(b"o1")?.trader_id(b"tr")
                            })?;
                            Ok(())
                        })?;
                        after_orders
                            .stats(1, |s| {
                                s.add(|st| {
                                    st.fill_count(1).fill_qty(8);
                                    Ok(())
                                })?;
                                Ok(())
                            })?
                            .venue(b"XNAS")
                    })?;
                    Ok(())
                })?
                .asks(0, |_| Ok(()))?
                .audit(1, |g| {
                    g.add(|a| {
                        a.ts(99).code(7);
                        Ok(())
                    })?;
                    Ok(())
                })?
                .symbol(b"IBM")?
                .source(b"src")?
                .checksum(b"ck")?
                .note(b"n")?
                .encoded_length_with_header();
            assert_eq!(sized, len, "EncodedLength must match the encoder");
            buf.truncate(len);
            Ok(buf)
        }

        macro_rules! check_full {
            ($mod:ident, $rt:ident, $encoded:expr, $wire:expr) => {{
                let dec = $mod::L3BookDecoder::try_decode($encoded, 0)?;
                assert_eq!(dec.acting_version(), $wire);
                assert_eq!(dec.timestamp(), 11);
                assert_eq!(dec.sequence(), 22);
                assert_eq!(dec.symbol()?, b"IBM");
                if $wire >= 1 {
                    assert_eq!(dec.epoch(), Some(33));
                    assert_eq!(dec.source()?, b"src");
                } else {
                    assert!(dec.epoch().is_none());
                    absent!($rt, dec.source(), "source", 1, $wire);
                }
                if $wire >= 2 {
                    assert_eq!(dec.checksum()?, b"ck");
                    let audit = dec.audit()?;
                    assert_eq!(audit.remaining_entries(), 1);
                    for row in audit {
                        assert_eq!(row.ts(), 99);
                        assert_eq!(row.code(), 7);
                    }
                } else {
                    absent!($rt, dec.checksum(), "checksum", 2, $wire);
                    absent!($rt, dec.audit(), "audit", 2, $wire);
                }
                if $wire >= 3 {
                    assert_eq!(dec.flags(), Some(44));
                    assert_eq!(dec.note()?, b"n");
                } else {
                    assert!(dec.flags().is_none());
                    absent!($rt, dec.note(), "note", 3, $wire);
                }
                let bids = dec.bids()?;
                assert_eq!(bids.remaining_entries(), 1);
                for lvl in bids {
                    let lvl = lvl?;
                    assert_eq!(lvl.price(), 100);
                    assert_eq!(lvl.qty(), 10);
                    if $wire >= 1 {
                        assert_eq!(lvl.participant(), Some(9));
                        assert_eq!(lvl.venue()?, b"XNAS");
                    } else {
                        assert!(lvl.participant().is_none());
                        absent!($rt, lvl.venue(), "venue", 1, $wire);
                    }
                    if $wire >= 2 {
                        let stats = lvl.stats()?;
                        assert_eq!(stats.remaining_entries(), 1);
                        for st in stats {
                            assert_eq!(st.fill_count(), 1);
                            assert_eq!(st.fill_qty(), 8);
                        }
                    } else {
                        absent!($rt, lvl.stats(), "stats", 2, $wire);
                    }
                    let orders = lvl.orders()?;
                    for ord in orders {
                        let ord = ord?;
                        assert_eq!(ord.order_qty(), 4);
                        assert_eq!(ord.order_id()?, b"o1");
                        if $wire >= 1 {
                            assert_eq!(ord.trader_id()?, b"tr");
                        } else {
                            absent!($rt, ord.trader_id(), "trader_id", 1, $wire);
                        }
                        if $wire >= 2 {
                            let allocs = ord.allocations()?;
                            assert_eq!(allocs.remaining_entries(), 1);
                            for al in allocs {
                                let al = al?;
                                assert_eq!(al.alloc_qty(), 8);
                                if $wire >= 3 {
                                    let legs = al.legs()?;
                                    assert_eq!(legs.remaining_entries(), 1);
                                    for leg in legs {
                                        let leg = leg?;
                                        assert_eq!(leg.leg_qty(), 8);
                                        assert_eq!(leg.leg_ref()?, b"L1");
                                    }
                                } else {
                                    absent!($rt, al.legs(), "legs", 3, $wire);
                                }
                            }
                        } else {
                            absent!($rt, ord.allocations(), "allocations", 2, $wire);
                        }
                    }
                }
                assert!(dec.asks()?.is_empty());
                let reverse = $mod::L3BookDecoder::try_decode($encoded, 0)?;
                assert_eq!(reverse.symbol()?, b"IBM");
                assert_eq!(reverse.timestamp(), 11);
                if $wire >= 3 {
                    assert_eq!(reverse.note()?, b"n");
                }
            }};
        }

        assert_eq!(vl3_v0::L3BookEncoder::SCHEMA_VERSION, 0);
        assert_eq!(vl3_v1::L3BookEncoder::SCHEMA_VERSION, 1);
        assert_eq!(vl3_v2::L3BookEncoder::SCHEMA_VERSION, 2);
        assert_eq!(vl3_v3::L3BookEncoder::SCHEMA_VERSION, 3);
        assert_eq!(vl3_v0::L3BookDecoder::SCHEMA_VERSION, 3);
        assert_eq!(vl3_v1::L3BookDecoder::SCHEMA_VERSION, 3);
        assert_eq!(vl3_v2::L3BookDecoder::SCHEMA_VERSION, 3);
        assert_eq!(vl3_v3::L3BookDecoder::SCHEMA_VERSION, 3);

        let v0 = encode_v0()?;
        let v1 = encode_v1()?;
        let v2 = encode_v2()?;
        let v3 = encode_v3()?;

        for (wire, encoded) in [
            (0u16, v0.as_slice()),
            (1, v1.as_slice()),
            (2, v2.as_slice()),
            (3, v3.as_slice()),
        ] {
            check_full!(vl3_v0, rt0, encoded, wire);
            check_full!(vl3_v1, rt1, encoded, wire);
            check_full!(vl3_v2, rt2, encoded, wire);
            check_full!(vl3_v3, rt3, encoded, wire);
        }
    "#,
    );
    Ok(())
}

#[test]
fn encode_version_beyond_schema_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_sbe::{GenerateError, GenerationConfig, Generator, Schema, parse_file};
    let ir = parse_file(&Paths::versioned_l3_schema(3))?;
    let schema = Schema::from_ir(ir);
    let err = Generator::new(GenerationConfig::new("too_new").with_encode_version(99))
        .generate(&schema)
        .unwrap_err();
    match err {
        GenerateError::InvalidConfiguration { option, .. } => {
            assert_eq!(option, "encode_version");
        }
        other => panic!("expected InvalidConfiguration, got {other:?}"),
    }
    Ok(())
}

#[test]
fn snapshot_encoder_matches_encode_version_filter() -> Result<(), Box<dyn std::error::Error>> {
    let (_s_snap, snap) = generate(&Paths::versioned_l3_schema(1), "snap_v1");
    let (_s_filt, filt) = generate_domain_with(&Paths::versioned_l3_schema(3), "filt_v1", |c| {
        c.with_memoized_tail_offsets(true).with_encode_version(1)
    });
    compile_and_run_two_modules(
        "vl3_snapshot_vs_encode_version",
        "snap_v1",
        &snap,
        "filt_v1",
        &filt,
        r#"
        fn encode_snap() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let sized = snap_v1::L3BookEncodedLength::new()
                .bids_ragged(1, |b| {{
                    b.add()?
                        .orders(|o| {{ o.add()?.order_id(2)?.trader_id(2)?; Ok(()) }})?
                        .venue(4)?;
                    Ok(())
                }})?
                .asks_ragged(0, |_| Ok(()))?
                .symbol(3)?
                .source(3)?
                .encoded_length_with_header();
            let mut buf = vec![0u8; sized];
            let len = snap_v1::L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
                .fixed(&snap_v1::L3BookFixedFields {
                    timestamp: 11,
                    sequence: 22,
                    epoch: 33,
                })
                .bids(1, |g| {
                    g.add(|mut lvl| {
                        lvl.price(100).qty(10).participant(9);
                        lvl.orders(1, |o| {
                            o.add(|mut ord| {
                                ord.order_qty(4);
                                ord.order_id(b"o1")?.trader_id(b"tr")
                            })?;
                            Ok(())
                        })?
                        .venue(b"XNAS")
                    })?;
                    Ok(())
                })?
                .asks(0, |_| Ok(()))?
                .symbol(b"IBM")?
                .source(b"src")?
                .encoded_length_with_header();
            assert_eq!(sized, len, "EncodedLength must match the encoder");
            buf.truncate(len);
            Ok(buf)
        }
        fn encode_filt() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let sized = filt_v1::L3BookEncodedLength::new()
                .bids_ragged(1, |b| {{
                    b.add()?
                        .orders(|o| {{ o.add()?.order_id(2)?.trader_id(2)?; Ok(()) }})?
                        .venue(4)?;
                    Ok(())
                }})?
                .asks_ragged(0, |_| Ok(()))?
                .symbol(3)?
                .source(3)?
                .encoded_length_with_header();
            let mut buf = vec![0u8; sized];
            let len = filt_v1::L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
                .fixed(&filt_v1::L3BookFixedFields {
                    timestamp: 11,
                    sequence: 22,
                    epoch: 33,
                })
                .bids(1, |g| {
                    g.add(|mut lvl| {
                        lvl.price(100).qty(10).participant(9);
                        lvl.orders(1, |o| {
                            o.add(|mut ord| {
                                ord.order_qty(4);
                                ord.order_id(b"o1")?.trader_id(b"tr")
                            })?;
                            Ok(())
                        })?
                        .venue(b"XNAS")
                    })?;
                    Ok(())
                })?
                .asks(0, |_| Ok(()))?
                .symbol(b"IBM")?
                .source(b"src")?
                .encoded_length_with_header();
            assert_eq!(sized, len, "EncodedLength must match the encoder");
            buf.truncate(len);
            Ok(buf)
        }
        let a = encode_snap()?;
        let b = encode_filt()?;
        assert_eq!(a.len(), b.len());
        assert_eq!(a, b);
        let la = a.len();
        assert_eq!(snap_v1::L3BookEncoder::SCHEMA_VERSION, 1);
        assert_eq!(filt_v1::L3BookEncoder::SCHEMA_VERSION, 1);
        let dec = filt_v1::L3BookDecoder::try_decode(&a[..la], 0)?;
        assert_eq!(dec.acting_version(), 1);
        assert!(dec.flags().is_none());
        match dec.checksum() {
            Err(filt_v1::sbe_rt::DecodeError::FieldNotInVersion { field, .. }) => {
                assert_eq!(field, "checksum");
            }
            other => panic!("expected absent checksum on v1 wire, got {other:?}"),
        }
    "#,
    );
    Ok(())
}
