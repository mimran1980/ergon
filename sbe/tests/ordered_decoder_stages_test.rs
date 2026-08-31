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
use common::{Paths, compile_and_run, generate};

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
