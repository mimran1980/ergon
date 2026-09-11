//! The ordered lane: one callback per tail, in wire order.
//!
//! A façade over the staged stages, so this file proves the façade preserves
//! what the staged lane guarantees — single traversal, compile-time tail order,
//! and identical values — plus the `EntryInfo` metadata that is its own reason
//! to exist.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, compile_and_run, compile_fails_with_diagnostics, generate};

const ENCODE: &str = r#"
    let mut storage = [0u8; 512];
    let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
        .fixed(&CarFixedFields {
            serial_number: 7,
            model_year: 2020,
            available: BooleanType::T,
            code: Model::A,
            some_numbers: [1, 2, 3, 4],
            vehicle_code: *b"ABCDEF",
            extras: OptionalExtras::default(),
            engine: Engine::new(1500, 4, *b"123", 0i8, BooleanType::F, Booster::new(BoostType::NITROUS, 150)),
        })
        .fuel_figures(2, |g| {
            g.add(|mut e| { e.speed(30).mpg(1.0); e.usage_description(b"aa") })?;
            g.add(|mut e| { e.speed(60).mpg(2.0); e.usage_description(b"bbb") })?;
            Ok(())
        })?
        .performance_figures(1, |g| {
            g.add(|mut e| {
                e.octane_rating(95);
                e.acceleration(3, |a| {
                    for mph in [10u16, 20, 30] {
                        a.add(|x| { x.mph(mph).seconds(1.0); Ok(()) })?;
                    }
                    Ok(())
                })
            })?;
            Ok(())
        })?
        .manufacturer(b"Honda")?
        .model(b"Civic")?
        .activation_code(b"abc")?
        .encoded_length_with_header();
    let encoded = &storage[..len];
"#;

/// Every tail is visited once, in wire order, and `EntryInfo` reports the
/// position the wire declares.
#[test]
fn ordered_lane_walks_every_tail_with_entry_info() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ordered_walk");
    compile_and_run(
        "ordered_walk",
        &src,
        &format!(
            r#"
        {ENCODE}
        let mut fuel = Vec::new();
        let mut accel = Vec::new();
        let mut text = Vec::new();
        let done = CarDecoder::try_decode(encoded, 0)?
            .ordered()
            .fixed(|car| -> Result<(), sbe_rt::DecodeError> {{
                assert_eq!(car.serial_number(), 7);
                assert_eq!(car.model_year(), 2020);
                Ok(())
            }})?
            .fuel_figures(|e, info| -> Result<_, sbe_rt::DecodeError> {{
                assert_eq!(info.count, 2);
                assert_eq!(info.block_length, 6);
                assert_eq!(info.is_first(), info.index == 0);
                assert_eq!(info.is_last(), info.index == 1);
                assert_eq!(info.remaining(), 1 - info.index);
                fuel.push((info.index, e.speed()));
                e.into_usage_description().map(|(_u, done)| done)
            }})?
            .performance_figures(|e, info| -> Result<_, sbe_rt::DecodeError> {{
                assert_eq!((info.index, info.count), (0, 1));
                assert_eq!(e.octane_rating(), 95);
                // Nested tails still use the staged entry stages.
                let mut a = e.into_acceleration()?;
                for x in &mut a {{ accel.push(x.mph()); }}
                a.finish()
            }})?
            .manufacturer_as_str(|s| -> Result<(), sbe_rt::DecodeError> {{
                text.push(s.to_owned());
                Ok(())
            }})?
            .model(|b| -> Result<(), sbe_rt::DecodeError> {{
                text.push(String::from_utf8(b.to_vec()).unwrap());
                Ok(())
            }})?
            .activation_code(|b| -> Result<(), sbe_rt::DecodeError> {{
                text.push(String::from_utf8(b.to_vec()).unwrap());
                Ok(())
            }})?
            .done();

        assert_eq!(fuel, vec![(0usize, 30u16), (1, 60)]);
        assert_eq!(accel, vec![10u16, 20, 30]);
        assert_eq!(text, vec!["Honda".to_string(), "Civic".into(), "abc".into()]);
        // The terminal staged stage is handed back intact.
        assert_eq!(done.encoded_length_with_header(), encoded.len());
        assert_eq!(done.as_bytes_with_header(), encoded);
    "#
        ),
    );
    Ok(())
}

/// Empty groups invoke the callback zero times and still reach the next tail.
#[test]
fn ordered_lane_empty_group_invokes_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ordered_empty");
    compile_and_run(
        "ordered_empty",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1, model_year: 0, available: BooleanType::F,
                code: Model::NullVal, some_numbers: [0u32; 4], vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"")?
            .model(b"")?
            .activation_code(b"")?
            .encoded_length_with_header();
        let encoded = &storage[..len];

        let mut calls = 0usize;
        let done = CarDecoder::try_decode(encoded, 0)?
            .ordered()
            .fuel_figures(|e, _| -> Result<_, sbe_rt::DecodeError> {
                calls += 1;
                e.into_usage_description().map(|(_u, d)| d)
            })?
            .performance_figures(|e, _| -> Result<_, sbe_rt::DecodeError> {
                calls += 1;
                e.into_acceleration()?.finish()
            })?
            .manufacturer(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })?
            .model(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })?
            .activation_code(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })?
            .done();
        assert_eq!(calls, 0, "empty groups invoke the callback zero times");
        assert_eq!(done.encoded_length_with_header(), encoded.len());
    "#,
    );
    Ok(())
}

/// The ordered lane decodes the same values the staged lane does.
#[test]
fn ordered_lane_matches_staged_lane() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ordered_parity");
    compile_and_run(
        "ordered_parity",
        &src,
        &format!(
            r#"
        {ENCODE}
        let mut staged = Vec::new();
        let (mfr_s, _) = CarDecoder::try_decode(encoded, 0)?
            .into_fuel_figures(|e| -> Result<_, sbe_rt::DecodeError> {{
                staged.push(e.speed());
                e.into_usage_description().map(|(_u, d)| d)
            }})?
            .into_performance_figures(|e| -> Result<_, sbe_rt::DecodeError> {{
                staged.push(e.octane_rating() as u16);
                e.into_acceleration()?.finish()
            }})?
            .into_manufacturer_as_str()?;

        let mut ord = Vec::new();
        let mut mfr_o = String::new();
        CarDecoder::try_decode(encoded, 0)?
            .ordered()
            .fuel_figures(|e, _| -> Result<_, sbe_rt::DecodeError> {{
                ord.push(e.speed());
                e.into_usage_description().map(|(_u, d)| d)
            }})?
            .performance_figures(|e, _| -> Result<_, sbe_rt::DecodeError> {{
                ord.push(e.octane_rating() as u16);
                e.into_acceleration()?.finish()
            }})?
            .manufacturer_as_str(|s| -> Result<(), sbe_rt::DecodeError> {{
                mfr_o.push_str(s);
                Ok(())
            }})?;

        assert_eq!(staged, ord, "both lanes decode identical values");
        assert_eq!(mfr_s, mfr_o);
    "#
        ),
    );
    Ok(())
}

/// Compile-fail: tail order is a type, here too — `manufacturer` is not on the
/// stage before the groups have been consumed.
#[test]
fn cf_ordered_lane_enforces_tail_order() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "cf_ordered_order");
    compile_fails_with_diagnostics(
        "cf_ordered_order",
        &src,
        r#"
        let buf = [0u8; 64];
        let dec = CarDecoder::wrap(&buf, 0, 0, 0);
        // ILLEGAL: manufacturer comes after both groups.
        let _ = dec.ordered().manufacturer(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) });
    "#,
        &["no method named `manufacturer`"],
    );
    Ok(())
}

/// A callback error consumes the lane and returns no continuation.
#[test]
fn ordered_lane_callback_error_propagates() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ordered_cb_err");
    compile_and_run(
        "ordered_cb_err",
        &src,
        &format!(
            r#"
        {ENCODE}
        #[derive(Debug)]
        enum E {{ Decode(sbe_rt::DecodeError), Mine }}
        impl From<sbe_rt::DecodeError> for E {{
            fn from(e: sbe_rt::DecodeError) -> Self {{ E::Decode(e) }}
        }}
        let r = CarDecoder::try_decode(encoded, 0)?
            .ordered()
            .fuel_figures(|_e, info| -> Result<_, E> {{
                if info.index == 1 {{ return Err(E::Mine); }}
                _e.into_usage_description().map(|(_u, d)| d).map_err(E::from)
            }});
        assert!(matches!(r, Err(E::Mine)), "callback error must propagate");
    "#
        ),
    );
    Ok(())
}
