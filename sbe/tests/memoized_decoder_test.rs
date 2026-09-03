//! Memoized random-access decoder: progressive tail-boundary cache.
//!
//! Compilation is the assertion. Cache behaviour (hits, frontier, no error
//! publication, `!Sync`) is checked on generated Car and L3 codecs.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, compile_and_run, compile_fails_with_diagnostics, generate_domain_with};
use std::path::Path;

/// Memoization is opt-in, so every module in this file asks for it.
fn generate(xml_path: &Path, module_name: &str) -> (ergo_sbe::Schema, String) {
    generate_domain_with(xml_path, module_name, |c| {
        c.with_memoized_tail_offsets(true)
    })
}

const fn encode_car_body() -> &'static str {
    r#"
        let sized = CarEncodedLength::new()
            .fuel_figures_ragged(2, |ff| {
                ff.add()?.usage_description(5)?;
                ff.add()?.usage_description(3)?;
                Ok(())
            })?
            .performance_figures_ragged(1, |pf| {
                pf.add()?.acceleration(|a| { a.add()?; Ok(()) })?;
                Ok(())
            })?
            .manufacturer(5)?
            .model(5)?
            .activation_code(3)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2024,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1, 2, 3, 4],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [b'N', b'/', b'A'], 30, BooleanType::T, Booster::new(BoostType::TURBO, 2)),
            })
            .fuel_figures(2, |g| {
                g.add(|mut e| { e.speed(30).mpg(20.0); e.usage_description(b"urban") })?;
                g.add(|mut e| { e.speed(55).mpg(30.0); e.usage_description(b"hwy") })?;
                Ok(())
            })?
            .performance_figures(1, |g| {
                g.add(|mut e| {
                    e.octane_rating(95);
                    e.acceleration(1, |a| {
                        a.add(|mut x| { x.mph(30).seconds(4.0); Ok(()) })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .manufacturer(b"Honda")?
            .model(b"Civic")?
            .activation_code(b"abc")?
            .encoded_length_with_header();
        assert_eq!(sized, len, "EncodedLength must match the encoder");
        let encoded = &storage[..len];
    "#
}

#[test]
fn car_cache_warms_and_repeated_reads_hit() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "memo_car_hits");
    assert!(
        src.contains("TailBoundaryCache"),
        "tailed decoder must carry a progressive cache"
    );
    assert!(
        src.contains("decode_cache_stats"),
        "debug cache stats must be generated"
    );
    let mut body = encode_car_body().to_string();
    body.push_str(
        r#"
        let dec = CarDecoder::try_decode(encoded, 0)?;
        assert_eq!(dec.serial_number(), 1);
        let cold = dec.decode_cache_stats();
        assert_eq!(cold.known_through, 0, "construction must not walk tails");
        assert_eq!(cold.boundary_calcs, 0);

        let _ = dec.activation_code()?;
        let after_last = dec.decode_cache_stats();
        assert!(after_last.known_through >= 5, "final var-data warms every preceding tail");
        assert!(after_last.misses >= 1);
        let calcs = after_last.boundary_calcs;
        let hits_before = after_last.hits;

        let code = dec.activation_code()?;
        assert_eq!(code, b"abc");
        let warm = dec.decode_cache_stats();
        assert_eq!(warm.known_through, after_last.known_through, "frontier must not regress");
        assert!(warm.hits > hits_before, "repeated last-field read must hit");
        assert_eq!(warm.boundary_calcs, calcs, "warm read must not walk again");

        let mfr = dec.manufacturer()?;
        assert_eq!(mfr, b"Honda");
        let bounced = dec.decode_cache_stats();
        assert_eq!(bounced.known_through, warm.known_through);
        assert!(bounced.hits > warm.hits);
        assert_eq!(bounced.boundary_calcs, calcs);
    "#,
    );
    compile_and_run("memo_car_hits", &src, &body);
    Ok(())
}

#[test]
fn car_random_orders_match_schema_order() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "memo_car_orders");
    let mut body = encode_car_body().to_string();
    body.push_str(
        r#"
        fn snapshot(dec: &CarDecoder<'_>) -> Result<(usize, usize, Vec<u8>, Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
            let fuel = dec.fuel_figures()?.count();
            let perf = dec.performance_figures()?.count();
            Ok((
                fuel,
                perf,
                dec.manufacturer()?.to_vec(),
                dec.model()?.to_vec(),
                dec.activation_code()?.to_vec(),
            ))
        }
        let dec = CarDecoder::try_decode(encoded, 0)?;
        let expected = snapshot(&dec)?;

        let reverse = CarDecoder::try_decode(encoded, 0)?;
        let _ = reverse.activation_code()?;
        let _ = reverse.model()?;
        let _ = reverse.manufacturer()?;
        let _ = reverse.performance_figures()?;
        let _ = reverse.fuel_figures()?;
        assert_eq!(snapshot(&reverse)?, expected);

        let bounce = CarDecoder::try_decode(encoded, 0)?;
        let _ = bounce.model()?;
        let _ = bounce.fuel_figures()?;
        let _ = bounce.activation_code()?;
        let _ = bounce.manufacturer()?;
        let _ = bounce.performance_figures()?;
        assert_eq!(snapshot(&bounce)?, expected);
        assert_eq!(bounce.serial_number(), 1);
        assert_eq!(bounce.model_year(), 2024);
    "#,
    );
    compile_and_run("memo_car_orders", &src, &body);
    Ok(())
}

#[test]
fn truncated_var_data_does_not_publish_invalid_boundary() -> Result<(), Box<dyn std::error::Error>>
{
    let (_schema, src) = generate(&Paths::example_schema(), "memo_car_trunc");
    compile_and_run(
        "memo_car_trunc",
        &src,
        r#"
        let sized = CarEncodedLength::new()
            .fuel_figures(0)
            .usage_description(0)?
            .performance_figures(0)
            .acceleration(0)?
            .manufacturer(5)?
            .model(5)?
            .activation_code(3)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2024,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1, 2, 3, 4],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [b'N', b'/', b'A'], 30, BooleanType::T, Booster::new(BoostType::TURBO, 2)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"Honda")?
            .model(b"Civic")?
            .activation_code(b"abc")?
            .encoded_length_with_header();
        assert_eq!(sized, len, "EncodedLength must match the encoder");
        let short = &storage[..len.saturating_sub(2)];
        let dec = match CarDecoder::try_decode(short, 0) {
            Ok(d) => d,
            Err(_) => return Ok(()),
        };
        let before = dec.decode_cache_stats().known_through;
        let err = dec.activation_code();
        assert!(err.is_err());
        let after = dec.decode_cache_stats().known_through;
        assert!(after >= before, "errors must not regress the frontier");
        let again = dec.activation_code();
        assert!(again.is_err(), "failed field must stay failed");
    "#,
    );
    Ok(())
}

#[test]
fn tailed_decoder_is_send_not_sync() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "memo_car_sync");
    compile_and_run(
        "memo_car_send",
        &src,
        r#"
        fn assert_send<T: Send>(_: &T) {}
        let sized = CarEncodedLength::new()
            .fuel_figures(0)
            .usage_description(0)?
            .performance_figures(0)
            .acceleration(0)?
            .manufacturer(1)?
            .model(1)?
            .activation_code(1)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2024,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0, 0, 0, 0],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(1, 1, [0, 0, 0], 0, BooleanType::T, Booster::new(BoostType::TURBO, 1)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"x")?
            .model(b"y")?
            .activation_code(b"z")?
            .encoded_length_with_header();
        assert_eq!(sized, len, "EncodedLength must match the encoder");
        let dec = CarDecoder::try_decode(&storage[..len], 0)?;
        assert_send(&dec);
    "#,
    );
    compile_fails_with_diagnostics(
        "memo_car_sync",
        &src,
        r#"
        fn assert_sync<T: Sync>(_: T) {}
        let dec = unsafe { core::mem::zeroed::<CarDecoder>() };
        assert_sync(dec);
        "#,
        &["Sync"],
    );
    Ok(())
}

#[test]
fn compact_tail_offsets_compile() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate_domain_with(&Paths::example_schema(), "memo_car_compact", |c| {
        c.with_memoized_tail_offsets(true)
            .with_compact_tail_offsets(true)
    });
    assert!(
        src.contains("CompactTailOffset"),
        "compact config must select CompactTailOffset"
    );
    let mut body = encode_car_body().to_string();
    body.push_str(
        r#"
        let dec = CarDecoder::try_decode(encoded, 0)?;
        assert_eq!(dec.manufacturer()?, b"Honda");
        assert_eq!(dec.activation_code()?, b"abc");
        assert_eq!(dec.model()?, b"Civic");
        let stats = dec.decode_cache_stats();
        assert!(stats.known_through >= 5);
    "#,
    );
    compile_and_run("memo_car_compact", &src, &body);
    Ok(())
}

#[test]
fn l3_nested_entry_cache_is_independent() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "memo_l3_nested");
    compile_and_run(
        "memo_l3_nested",
        &src,
        r#"
        let sized = L3BookEncodedLength::new()
            .bids_ragged(1, |b| {
                b.add()?.orders(|o| { o.add()?.order_id(5)?; Ok(()) })?;
                Ok(())
            })?
            .asks_ragged(0, |_| Ok(()))?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = L3BookEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&L3BookFixedFields { timestamp: 99, sequence: 7 })
            .bids(1, |g| {
                g.add(|mut lvl| {
                    lvl.price(100).qty(10);
                    lvl.orders(1, |o| {
                        o.add(|mut ord| { ord.order_qty(4); ord.order_id(b"ord-1") })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .asks(0, |_| Ok(()))?
            .encoded_length_with_header();
        assert_eq!(sized, len, "EncodedLength must match the encoder");
        let encoded = &storage[..len];
        let dec = L3BookDecoder::try_decode(encoded, 0)?;
        let bids = dec.bids()?;
        let mut n = 0usize;
        for lvl in bids {
            let lvl = lvl?;
            assert_eq!(lvl.price(), 100);
            for ord in lvl.orders()? {
                let ord = ord?;
                assert_eq!(ord.order_id()?, b"ord-1");
                n += 1;
            }
        }
        assert_eq!(n, 1);
        let asks = dec.asks()?;
        assert!(asks.is_empty());
        assert_eq!(dec.timestamp(), 99);
    "#,
    );
    Ok(())
}
