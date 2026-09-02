//! Concrete consuming decoder tail stages.
//!
//! These tests prove the new sequential decoder API end-to-end:
//!
//! ```text
//! CarDecoder --into_fuel_figures()--> FuelFiguresDecoder --finish()-->
//! CarDecoderAfterFuelFigures --into_performance_figures()--> ... -->
//! CarDecoderAfterManufacturer --into_model()--> CarDecoderAfterModel
//! --into_activation_code()--> CarDecoderComplete
//! ```
//!
//! Wire order is enforced by consumption: each `into_*`/`finish`/`skip_remaining`
//! takes `self`, so a later tail component is unreachable until the current one
//! is consumed. Random-access `&self` accessors remain; these tests exercise
//! only the consuming path.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{
    Paths, compile_and_run, compile_fails_with_diagnostics, generate, generate_domain_with,
};
use ergo_sbe::{ConversionSelector, GenerationProfile};

const MODULE_FULL: &str = "ordered_stages_full";
const MODULE_FINISH: &str = "ordered_stages_finish";
const MODULE_EMPTY: &str = "ordered_stages_empty";

/// Encode a car with every tail shape (group with var-data entries, group with
/// nested-group entries, three message-level var-data fields), then decode it
/// through the consuming stage API and assert every value matches. This is a
/// full wire-order round trip through the new decoder stages.
#[test]
fn decode_car_through_consuming_stages() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE_FULL);
    compile_and_run(
        MODULE_FULL,
        &src,
        r#"
        let mut buf = [0u8; 4096];
        // Unset fields keep the zero wire image the pre-`fixed()` form produced.
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            });
        let car = car.fuel_figures(3, |g| -> Result<(), sbe_rt::EncodeError> {
            g.add(|mut e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban Cycle") })?;
            g.add(|mut e| { e.speed(55).mpg(49.0); e.usage_description(b"Combined Cycle") })?;
            g.add(|mut e| { e.speed(75).mpg(40.0); e.usage_description(b"Highway Cycle") })?;
            Ok(())
        })?;
        let car = car.performance_figures(2, |g| -> Result<(), sbe_rt::EncodeError> {
            g.add(|mut e| { e.octane_rating(95); e.acceleration(0, |_| Ok(())) })?;
            g.add(|mut e| { e.octane_rating(99); e.acceleration(0, |_| Ok(())) })
        })?;
        let car = car.manufacturer(b"Honda")?;
        let car = car.model(b"Civic VTi")?;
        let complete = car.activation_code(b"abcdef")?;
        assert!(complete.encoded_length_with_header() > 0);
        let encoded = complete.as_bytes_with_header();
        let total_len = encoded.len();

        let dec = CarDecoder::try_decode(encoded, 0).unwrap();
        assert_eq!(dec.serial_number(), 1234);
        assert_eq!(dec.model_year(), 2013);

        // First group: consume the message stage, iterate, then finish().
        let mut fuel = dec.into_fuel_figures().unwrap();
        assert_eq!(fuel.remaining(), 3);
        let mut rows = Vec::new();
        while let Some(Ok(e)) = fuel.next() {
            rows.push((e.speed(), e.mpg(), e.usage_description().unwrap().to_vec()));
        }
        let after_fuel = fuel.finish().unwrap();
        assert_eq!(rows, vec![
            (30, 35.9_f32, b"Urban Cycle".to_vec()),
            (55, 49.0_f32, b"Combined Cycle".to_vec()),
            (75, 40.0_f32, b"Highway Cycle".to_vec()),
        ]);

        // Second group (entries carry a nested group dimension header even at 0).
        let mut perf = after_fuel.into_performance_figures().unwrap();
        assert_eq!(perf.remaining(), 2);
        let mut octanes = Vec::new();
        while let Some(Ok(e)) = perf.next() {
            octanes.push(e.octane_rating());
        }
        let after_perf = perf.finish().unwrap();
        assert_eq!(octanes, vec![95u8, 99u8]);

        // Message-level var-data: each into_* returns (bytes, next stage).
        let (mfr, after_mfr) = after_perf.into_manufacturer().unwrap();
        assert_eq!(mfr, b"Honda");
        let (model, after_model) = after_mfr.into_model().unwrap();
        assert_eq!(model, b"Civic VTi");
        let (code, done) = after_model.into_activation_code().unwrap();
        assert_eq!(code, b"abcdef");

        // Terminal stage extent helpers.
        assert_eq!(done.encoded_length_with_header(), total_len);
        assert_eq!(done.as_bytes_with_header(), encoded);
    "#,
    );

    Ok(())
}

/// `finish()` must scan past UNREAD entries (not just fully-read ones): read one
/// fuel figure, then finish() — the remaining two (with their var-data tails)
/// must be skipped so the next stage lands at the right offset.
#[test]
fn finish_skips_unread_entries() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE_FINISH);
    compile_and_run(
        MODULE_FINISH,
        &src,
        r#"
        let mut buf = [0u8; 4096];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 7,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            });
        let car = car.fuel_figures(3, |g| -> Result<(), sbe_rt::EncodeError> {
            g.add(|mut e| { e.speed(10).mpg(1.0); e.usage_description(b"aaa") })?;
            g.add(|mut e| { e.speed(20).mpg(2.0); e.usage_description(b"bbbb") })?;
            g.add(|mut e| { e.speed(30).mpg(3.0); e.usage_description(b"ccccc") })
        })?;
        let car = car.performance_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?;
        let car = car.manufacturer(b"M")?;
        let car = car.model(b"N")?;
        let complete = car.activation_code(b"P")?;
        assert!(complete.encoded_length_with_header() > 0);
        let encoded = complete.as_bytes_with_header();

        let dec = CarDecoder::try_decode(encoded, 0).unwrap();
        let mut fuel = dec.into_fuel_figures().unwrap();
        let first = fuel.next().unwrap().unwrap();
        assert_eq!(first.speed(), 10);
        let after_fuel = fuel.skip_remaining().unwrap();

        // We must still land at performance_figures, then the var-data, correctly.
        let mut perf = after_fuel.into_performance_figures().unwrap();
        assert_eq!(perf.remaining(), 0);
        let after_perf = perf.finish().unwrap();
        let (mfr, after_mfr) = after_perf.into_manufacturer().unwrap();
        assert_eq!(mfr, b"M");
        let (model, after_model) = after_mfr.into_model().unwrap();
        assert_eq!(model, b"N");
        let (code, done) = after_model.into_activation_code().unwrap();
        assert_eq!(code, b"P");
        assert_eq!(done.encoded_length_with_header(), encoded.len());
    "#,
    );

    Ok(())
}

/// All three `into_*_as_str()` return `&'a str` tied to the buffer, not the
/// stage — prove all three `&str` coexist after all three calls complete.
#[test]
fn multiple_var_data_strings_coexist() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "coexist_strings");
    compile_and_run(
        "coexist_strings",
        &src,
        r#"
        let mut buf = [0u8; 4096];
        let complete = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?
            .performance_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?
            .manufacturer(b"Honda")?
            .model(b"Civic VTi")?
            .activation_code(b"abcdef")?;
        let encoded = complete.as_bytes_with_header();

        let decoder = CarDecoder::try_decode(encoded, 0)?;
        let after_fuel = decoder.into_fuel_figures()?.finish()?;
        let after_perf = after_fuel.into_performance_figures()?.finish()?;

        // All three into_*_as_str() calls complete before any assert.
        let (mfr, decoder) = after_perf.into_manufacturer_as_str()?;
        let (model, decoder) = decoder.into_model_as_str()?;
        let (code, _done) = decoder.into_activation_code_as_str()?;
        // Prove all three &str coexist — each borrows 'a from the original wire buffer.
        assert_eq!((mfr, model, code), ("Honda", "Civic VTi", "abcdef"));
    "#,
    );

    Ok(())
}

/// The optional-crate var-data accessors are generated only when the
/// *generator* has the feature — and when it does, they must compile and run.
/// They used to be emitted unconditionally behind `#[cfg(feature = "…")]`,
/// which resolved against the consumer crate's own feature set and so never
/// matched: dead code in every generated module, and no test could reach it.
#[test]
#[cfg(feature = "compact_str")]
fn optional_crate_var_data_accessors_run() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "opt_crate_vd");
    assert!(
        src.contains("into_manufacturer_as_compact_str"),
        "compact_str feature on: accessor must be generated"
    );
    compile_and_run(
        "opt_crate_vd",
        &src,
        r#"
        let len = CarEncoder::compute_length()
            .fuel_figures(0)
            .finish_empty()?
            .performance_figures(0)
            .finish_empty()?
            .manufacturer(5)?
            .model(9)?
            .activation_code(6)?
            .encoded_length_with_header();
        let mut buf = vec![0u8; len];
        let encoded = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?
            .performance_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?
            .manufacturer(b"Honda")?
            .model(b"Civic VTi")?
            .activation_code(b"abcdef")?
            .as_bytes_with_header()
            .to_vec();

        let after_perf = CarDecoder::try_decode(&encoded, 0)?
            .into_fuel_figures()?
            .finish()?
            .into_performance_figures()?
            .finish()?;
        let (mfr, next) = after_perf.into_manufacturer_as_compact_str()?;
        let (model, next) = next.into_model_as_smol_str()?;
        let (code, _done) = next.into_activation_code_as_bytes()?;
        assert_eq!(mfr, "Honda");
        assert_eq!(model, "Civic VTi");
        assert_eq!(&code[..], b"abcdef");
    "#,
    );
    Ok(())
}

/// Empty groups and empty var-data still traverse through the same stages.
#[test]
fn empty_tail_components_traverse_stages() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE_EMPTY);
    compile_and_run(
        MODULE_EMPTY,
        &src,
        r#"
        let mut buf = [0u8; 4096];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            });
        let car = car.fuel_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?;
        let car = car.performance_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?;
        let car = car.manufacturer(b"")?;
        let car = car.model(b"")?;
        let complete = car.activation_code(b"")?;
        assert!(complete.encoded_length_with_header() > 0);
        let encoded = complete.as_bytes_with_header();

        let dec = CarDecoder::try_decode(encoded, 0).unwrap();
        let fuel = dec.into_fuel_figures().unwrap();
        assert!(fuel.is_empty());
        let after_fuel = fuel.finish().unwrap();
        let perf = after_fuel.into_performance_figures().unwrap();
        assert!(perf.is_empty());
        let after_perf = perf.finish().unwrap();
        let (mfr, after_mfr) = after_perf.into_manufacturer().unwrap();
        assert!(mfr.is_empty());
        let (model, after_model) = after_mfr.into_model().unwrap();
        assert!(model.is_empty());
        let (code, done) = after_model.into_activation_code().unwrap();
        assert!(code.is_empty());
        assert_eq!(done.encoded_length_with_header(), encoded.len());
    "#,
    );

    Ok(())
}

/// `remaining_entries()` / `is_empty()` are O(1) observers of the wire count,
/// including after partial iterator consumption.
#[test]
fn remaining_entries_and_is_empty_before_and_after_partial_consumption()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "remaining_entries_partial");
    compile_and_run(
        "remaining_entries_partial",
        &src,
        r#"
        let mut storage = [0u8; 512];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(3, |g| {
                g.add(|mut e| {
                    e.speed(10).mpg(1.0);
                    e.usage_description(b"aaa")
                })?;
                g.add(|mut e| {
                    e.speed(20).mpg(2.0);
                    e.usage_description(b"bbbb")
                })?;
                g.add(|mut e| {
                    e.speed(30).mpg(3.0);
                    e.usage_description(b"ccccc")
                })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let encoded = &storage[..len];

        let dec = CarDecoder::try_decode(encoded, 0)?;
        let mut fuel = dec.into_fuel_figures()?;
        assert_eq!(fuel.remaining_entries(), 3);
        assert_eq!(fuel.remaining(), 3);
        assert!(!fuel.is_empty());
        let first = fuel.next().unwrap()?;
        assert_eq!(first.speed(), 10);
        assert_eq!(fuel.remaining_entries(), 2);
        assert!(!fuel.is_empty());
        let _ = fuel.next().unwrap()?;
        let _ = fuel.next().unwrap()?;
        assert_eq!(fuel.remaining_entries(), 0);
        assert!(fuel.is_empty());
        let _ = fuel.finish()?;
    "#,
    );
    Ok(())
}

/// Ordered `visit_entries` walks dynamic fuel figures in one pass without
/// calling `encoded_length()` first, then continues through the rest of the
/// message.
#[test]
fn visit_entries_dynamic_fuel_figures() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "visit_fuel");
    compile_and_run(
        "visit_fuel",
        &src,
        r#"
        let mut storage = [0u8; 512];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(3, |g| {
                g.add(|mut e| {
                    e.speed(30).mpg(35.9);
                    e.usage_description(b"Urban Cycle")
                })?;
                g.add(|mut e| {
                    e.speed(55).mpg(49.0);
                    e.usage_description(b"Combined Cycle")
                })?;
                g.add(|mut e| {
                    e.speed(75).mpg(40.0);
                    e.usage_description(b"Highway Cycle")
                })?;
                Ok(())
            })?
            .performance_figures(2, |g| {
                g.add(|mut e| {
                    e.octane_rating(95);
                    e.acceleration(0, |_| Ok(()))
                })?;
                g.add(|mut e| {
                    e.octane_rating(99);
                    e.acceleration(0, |_| Ok(()))
                })?;
                Ok(())
            })?
            .manufacturer(b"Honda")?
            .model(b"Civic VTi")?
            .activation_code(b"abcdef")?
            .encoded_length_with_header();
        let encoded = &storage[..len];

        let car = CarDecoder::try_decode(encoded, 0)?;
        let figures = car.into_fuel_figures()?;
        assert_eq!(figures.remaining_entries(), 3);
        assert!(!figures.is_empty());
        let mut rows = Vec::new();
        let car = figures.visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
            let speed = entry.speed();
            let mpg = entry.mpg();
            let (usage, complete) = entry.into_usage_description()?;
            rows.push((speed, mpg, usage.to_vec()));
            Ok(complete)
        })?;
        assert_eq!(rows, vec![
            (30, 35.9_f32, b"Urban Cycle".to_vec()),
            (55, 49.0_f32, b"Combined Cycle".to_vec()),
            (75, 40.0_f32, b"Highway Cycle".to_vec()),
        ]);

        let mut octanes = Vec::new();
        let (mfr, car) = car
            .into_performance_figures()?
            .visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
                octanes.push(entry.octane_rating());
                entry
                    .into_acceleration()?
                    .visit_entries(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })
            })?
            .into_manufacturer()?;
        let (model, car) = car.into_model()?;
        let (code, done) = car.into_activation_code()?;
        assert_eq!(octanes, vec![95u8, 99u8]);
        assert_eq!((mfr, model, code), (&b"Honda"[..], &b"Civic VTi"[..], &b"abcdef"[..]));
        assert_eq!(done.encoded_length_with_header(), len);
    "#,
    );
    Ok(())
}

/// Empty groups invoke `visit_entries` zero times and return the next stage.
#[test]
fn visit_entries_empty_groups() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "visit_empty");
    compile_and_run(
        "visit_empty",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
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

        let car = CarDecoder::try_decode(encoded, 0)?;
        let figures = car.into_fuel_figures()?;
        assert!(figures.is_empty());
        assert_eq!(figures.remaining_entries(), 0);
        let mut visited = 0usize;
        let (mfr, car) = figures
            .visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
                visited += 1;
                entry.into_usage_description().map(|(_, c)| c)
            })?
            .into_performance_figures()?
            .visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
                visited += 1;
                entry
                    .into_acceleration()?
                    .visit_entries(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })
            })?
            .into_manufacturer()?;
        let (model, car) = car.into_model()?;
        let (code, done) = car.into_activation_code()?;
        assert_eq!(visited, 0);
        assert!(mfr.is_empty());
        assert!(model.is_empty());
        assert!(code.is_empty());
        assert_eq!(done.encoded_length_with_header(), len);
    "#,
    );
    Ok(())
}

/// A callback error consumes the ordered stage and returns no continuation.
#[test]
fn visit_entries_callback_error_consumes_stage() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "visit_cb_err");
    compile_and_run(
        "visit_cb_err",
        &src,
        r#"
        #[derive(Debug)]
        struct Boom;
        impl From<sbe_rt::DecodeError> for Boom {
            fn from(_: sbe_rt::DecodeError) -> Self { Boom }
        }
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(2, |g| {
                g.add(|mut e| {
                    e.speed(10).mpg(1.0);
                    e.usage_description(b"aa")
                })?;
                g.add(|mut e| {
                    e.speed(20).mpg(2.0);
                    e.usage_description(b"bb")
                })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let car = CarDecoder::try_decode(encoded, 0)?;
        let figures = car.into_fuel_figures()?;
        let err = figures.visit_entries(|entry| -> Result<FuelFiguresEntryDecoderComplete<'_>, Boom> {
            let (_usage, complete) = entry.into_usage_description().map_err(Boom::from)?;
            let _ = complete;
            Err(Boom)
        });
        assert!(err.is_err());
    "#,
    );
    Ok(())
}

/// Truncated dynamic tails fail `visit_entries` with `BufferTooShort`.
#[test]
fn visit_entries_malformed_truncated_entry() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "visit_trunc");
    compile_and_run(
        "visit_trunc",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(1, |g| {
                g.add(|mut e| {
                    e.speed(10).mpg(1.0);
                    e.usage_description(b"abcdef")
                })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let header = MessageHeader(read_bytes::<8>(encoded, 0));
        // Header + acting block + fuel dimension + entry fixed fields, cut
        // before the usage-description payload so visit_entries fails on the
        // dynamic tail rather than on a later message-level field.
        let fuel_start = 8 + header.block_length() as usize;
        let truncated = &encoded[..fuel_start + 4 + 6 + 1];
        let car = CarDecoder::wrap(
            truncated,
            0,
            header.block_length() as usize,
            header.version(),
        );
        let figures = car.into_fuel_figures()?;
        let err = figures.visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
            entry.into_usage_description().map(|(_, c)| c)
        });
        match err {
            Err(sbe_rt::DecodeError::BufferTooShort { .. }) => {}
            Err(e) => panic!("expected BufferTooShort, got {e:?}"),
            Ok(_) => panic!("expected BufferTooShort, visit_entries succeeded"),
        }
    "#,
    );
    Ok(())
}

/// Returning a completion that belongs to a different entry panics.
#[test]
fn visit_entries_wrong_entry_completion_panics() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "visit_wrong_complete");
    compile_and_run(
        "visit_wrong_complete",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(2, |g| {
                g.add(|mut e| {
                    e.speed(10).mpg(1.0);
                    e.usage_description(b"aa")
                })?;
                g.add(|mut e| {
                    e.speed(20).mpg(2.0);
                    e.usage_description(b"bb")
                })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let car = CarDecoder::try_decode(encoded, 0)?;
        let figures = car.into_fuel_figures()?;
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut stolen = None;
            figures.visit_entries(|entry| -> Result<FuelFiguresEntryDecoderComplete<'_>, sbe_rt::DecodeError> {
                let (_usage, complete) = entry.into_usage_description()?;
                match stolen.take() {
                    None => {
                        stolen = Some(unsafe { core::ptr::read(&complete) });
                        Ok(complete)
                    }
                    Some(prev) => Ok(prev),
                }
            }).unwrap();
        }));
        assert!(panicked.is_err());
        let msg = panicked.unwrap_err();
        let msg = msg.downcast_ref::<&str>().copied()
            .or_else(|| msg.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("");
        assert!(
            msg.contains("does not belong to the supplied entry"),
            "panic diagnostic was {msg:?}"
        );
    "#,
    );
    Ok(())
}

/// `visit_entries` is only on attached group decoders.
#[test]
fn cf_visit_entries_not_on_detached() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "cf_visit_detached");
    compile_fails_with_diagnostics(
        "cf_visit_detached",
        &src,
        r#"
        let buf = [0u8; 16];
        let g = FuelFiguresDecoder::wrap(&buf, 0, 0).unwrap();
        let _ = g.visit_entries(|entry| entry.into_usage_description().map(|(_, c)| c));
    "#,
        &["no method named `visit_entries`"],
    );
    Ok(())
}

/// Lean and Full profiles both emit the ordered interface and it runs.
#[test]
fn visit_entries_lean_and_full_profiles() -> Result<(), Box<dyn std::error::Error>> {
    for (module, profile) in [
        ("visit_lean", GenerationProfile::Lean),
        ("visit_full", GenerationProfile::Full),
    ] {
        let (_schema, src) =
            generate_domain_with(&Paths::example_schema(), module, |c| c.profile(profile));
        assert!(
            src.contains("fn remaining_entries"),
            "{module} must emit remaining_entries"
        );
        assert!(
            src.contains("fn visit_entries"),
            "{module} must emit visit_entries"
        );
        compile_and_run(
            module,
            &src,
            r#"
            let mut storage = [0u8; 256];
            let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
                .fixed(&CarFixedFields {
                    serial_number: 1,
                    model_year: 0,
                    available: BooleanType::F,
                    code: Model::NullVal,
                    some_numbers: [0u32; 4],
                    vehicle_code: [0u8; 6],
                    extras: OptionalExtras::default(),
                    engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
                })
                .fuel_figures(1, |g| {
                    g.add(|mut e| {
                        e.speed(10).mpg(1.0);
                        e.usage_description(b"aa")
                    })?;
                    Ok(())
                })?
                .performance_figures(0, |_| Ok(()))?
                .manufacturer(b"M")?
                .model(b"N")?
                .activation_code(b"P")?
                .encoded_length_with_header();
            let encoded = &storage[..len];
            let car = CarDecoder::try_decode(encoded, 0)?;
            let mut speeds = Vec::new();
            let (mfr, car) = car
                .into_fuel_figures()?
                .visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
                    speeds.push(entry.speed());
                    entry.into_usage_description().map(|(_, c)| c)
                })?
                .into_performance_figures()?
                .visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
                    entry
                        .into_acceleration()?
                        .visit_entries(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })
                })?
                .into_manufacturer()?;
            let (model, car) = car.into_model()?;
            let (code, _) = car.into_activation_code()?;
            assert_eq!(speeds, vec![10]);
            assert_eq!((mfr, model, code), (&b"M"[..], &b"N"[..], &b"P"[..]));
        "#,
        );
    }
    Ok(())
}

/// Domain-conversion configs still compile and run `visit_entries`.
#[test]
fn visit_entries_with_domain_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate_domain_with(&Paths::example_schema(), "visit_conv", |c| {
        c.with_conversion(ConversionSelector::named_type("Model"))
    });
    compile_and_run(
        "visit_conv",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(1, |g| {
                g.add(|mut e| {
                    e.speed(10).mpg(1.0);
                    e.usage_description(b"aa")
                })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let car = CarDecoder::try_decode(encoded, 0)?;
        let (mfr, car) = car
            .into_fuel_figures()?
            .visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
                let _ = entry.speed();
                entry.into_usage_description().map(|(_, c)| c)
            })?
            .into_performance_figures()?
            .visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
                entry
                    .into_acceleration()?
                    .visit_entries(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })
            })?
            .into_manufacturer()?;
        let (model, car) = car.into_model()?;
        let (code, _) = car.into_activation_code()?;
        assert_eq!((mfr, model, code), (&b"M"[..], &b"N"[..], &b"P"[..]));
    "#,
    );
    Ok(())
}

/// Version-absent groups occupy zero bytes and are immediately complete;
/// version-absent var-data returns an empty slice; random access still
/// reports `FieldNotInVersion`; offset walkers skip absent preceding tails.
#[test]
fn version_absent_groups_and_var_data() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(
        &Paths::ordered_decoder_version_tails_schema(),
        "version_absent_tails",
    );
    compile_and_run(
        "version_absent_tails",
        &src,
        r#"
        // v0 wire: seq + figures(1, speed+label) + note. extras / extraFigures /
        // extraNote occupy zero bytes.
        let mut buf = [0u8; 64];
        buf[0..2].copy_from_slice(&4u16.to_le_bytes());
        buf[2..4].copy_from_slice(&1u16.to_le_bytes());
        buf[4..6].copy_from_slice(&77u16.to_le_bytes());
        buf[6..8].copy_from_slice(&0u16.to_le_bytes());
        buf[8..12].copy_from_slice(&7u32.to_le_bytes());
        buf[12..14].copy_from_slice(&2u16.to_le_bytes());
        buf[14..16].copy_from_slice(&1u16.to_le_bytes());
        buf[16..18].copy_from_slice(&30u16.to_le_bytes());
        buf[18] = 3;
        buf[19..22].copy_from_slice(b"urb");
        buf[22] = 2;
        buf[23..25].copy_from_slice(b"hi");
        let encoded = &buf[..25];

        let dec = VersionedTailsDecoder::wrap(encoded, 0, 4, 0);
        assert_eq!(dec.seq(), 7);
        match dec.extra_figures() {
            Err(sbe_rt::DecodeError::FieldNotInVersion { field, wire_version, since_version }) => {
                assert_eq!(field, "extra_figures");
                assert_eq!(wire_version, 0);
                assert_eq!(since_version, 1);
            }
            Err(e) => panic!("expected FieldNotInVersion for extra_figures, got {e:?}"),
            Ok(_) => panic!("expected FieldNotInVersion for extra_figures"),
        }
        match dec.extra_note() {
            Err(sbe_rt::DecodeError::FieldNotInVersion { field, wire_version, since_version }) => {
                assert_eq!(field, "extra_note");
                assert_eq!(wire_version, 0);
                assert_eq!(since_version, 1);
            }
            Err(e) => panic!("expected FieldNotInVersion for extra_note, got {e:?}"),
            Ok(_) => panic!("expected FieldNotInVersion for extra_note"),
        }
        assert_eq!(dec.note()?, b"hi");

        let figures = dec.into_figures()?;
        assert_eq!(figures.remaining_entries(), 1);
        assert!(!figures.is_empty());
        let mut speeds = Vec::new();
        let mut labels = Vec::new();
        let after_figures = figures.visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
            speeds.push(entry.speed());
            match entry.extras() {
                Err(sbe_rt::DecodeError::FieldNotInVersion { .. }) => {}
                Err(e) => panic!("expected FieldNotInVersion for extras, got {e:?}"),
                Ok(_) => panic!("expected FieldNotInVersion for extras"),
            }
            let extras = entry.into_extras()?;
            assert!(extras.is_empty());
            assert_eq!(extras.remaining_entries(), 0);
            let entry = extras.visit_entries(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })?;
            let (label, complete) = entry.into_label()?;
            labels.push(label.to_vec());
            Ok(complete)
        })?;
        assert_eq!(speeds, vec![30]);
        assert_eq!(labels, vec![b"urb".to_vec()]);

        let extra = after_figures.into_extra_figures()?;
        assert!(extra.is_empty());
        assert_eq!(extra.remaining_entries(), 0);
        let after_extra = extra.visit_entries(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })?;
        let (note, after_note) = after_extra.into_note()?;
        assert_eq!(note, b"hi");
        let (extra_note, done) = after_note.into_extra_note()?;
        assert!(extra_note.is_empty());
        assert_eq!(done.encoded_length_with_header(), encoded.len());
    "#,
    );
    Ok(())
}

/// Present v1 tails still round-trip through `visit_entries`.
#[test]
fn version_present_groups_visit_entries() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(
        &Paths::ordered_decoder_version_tails_schema(),
        "version_present_tails",
    );
    compile_and_run(
        "version_present_tails",
        &src,
        r#"
        let mut storage = [0u8; 128];
        let len = VersionedTailsEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&VersionedTailsFixedFields { seq: 9 })
            .figures(1, |g| {
                g.add(|mut e| {
                    e.speed(40);
                    e.extras(1, |x| {
                        x.add(|mut row| {
                            row.flag(7);
                            Ok(())
                        })?;
                        Ok(())
                    })?
                    .label(b"v1")
                })?;
                Ok(())
            })?
            .extra_figures(1, |g| {
                g.add(|mut e| {
                    e.amp(11);
                    Ok(())
                })?;
                Ok(())
            })?
            .note(b"ok")?
            .extra_note(b"more")?
            .encoded_length_with_header();
        let encoded = &storage[..len];

        let dec = VersionedTailsDecoder::try_decode(encoded, 0)?;
        assert_eq!(dec.seq(), 9);
        let mut flags = Vec::new();
        let after = dec.into_figures()?.visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
            assert_eq!(entry.speed(), 40);
            let entry = entry.into_extras()?.visit_entries(|row| -> Result<(), sbe_rt::DecodeError> {
                flags.push(row.flag());
                Ok(())
            })?;
            let (label, complete) = entry.into_label()?;
            assert_eq!(label, b"v1");
            Ok(complete)
        })?;
        assert_eq!(flags, vec![7u8]);
        let mut amps = Vec::new();
        let after = after.into_extra_figures()?.visit_entries(|e| -> Result<(), sbe_rt::DecodeError> {
            amps.push(e.amp());
            Ok(())
        })?;
        assert_eq!(amps, vec![11]);
        let (note, after) = after.into_note()?;
        assert_eq!(note, b"ok");
        let (extra, done) = after.into_extra_note()?;
        assert_eq!(extra, b"more");
        assert_eq!(done.encoded_length_with_header(), len);
    "#,
    );
    Ok(())
}
