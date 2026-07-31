//! Concrete consuming decoder tail stages (DECISIONS.md §3, Tasks A–C).
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
//! is consumed. The legacy `&self` random-access surface still coexists for now;
//! these tests exercise only the new consuming path.

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
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        car.serial_number(1234);
        car.model_year(2013);
        let car = car.fuel_figures(3, |g| -> Result<(), sbe_rt::EncodeError> {
            g.add(|e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban Cycle")?; Ok(()) })?;
            g.add(|e| { e.speed(55).mpg(49.0); e.usage_description(b"Combined Cycle")?; Ok(()) })?;
            g.add(|e| { e.speed(75).mpg(40.0); e.usage_description(b"Highway Cycle")?; Ok(()) })?;
            Ok(())
        })?;
        let car = car.performance_figures(2, |g| -> Result<(), sbe_rt::EncodeError> {
            g.add(|e| { e.octane_rating(95).acceleration(0, |_| Ok(()))?; Ok(()) })?;
            g.add(|e| { e.octane_rating(99).acceleration(0, |_| Ok(()))?; Ok(()) })?;
            Ok(())
        })?;
        let car = car.manufacturer(b"Honda")?;
        let car = car.model(b"Civic VTi")?;
        let complete = car.activation_code(b"abcdef")?;
        assert!(complete.encoded_length_with_header() > 0);
        let encoded = complete.as_bytes_with_header();
        let total_len = encoded.len();

        let dec = CarDecoder::try_wrap_and_apply_header(encoded, 0).unwrap();
        assert_eq!(dec.serial_number(), 1234);
        assert_eq!(dec.model_year(), 2013);

        // First group: consume the message stage, iterate, then finish().
        let mut fuel = dec.into_fuel_figures().unwrap();
        assert_eq!(fuel.len(), 3);
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
        assert_eq!(perf.len(), 2);
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
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        car.serial_number(7);
        let car = car.fuel_figures(3, |g| -> Result<(), sbe_rt::EncodeError> {
            g.add(|e| { e.speed(10).mpg(1.0); e.usage_description(b"aaa")?; Ok(()) })?;
            g.add(|e| { e.speed(20).mpg(2.0); e.usage_description(b"bbbb")?; Ok(()) })?;
            g.add(|e| { e.speed(30).mpg(3.0); e.usage_description(b"ccccc")?; Ok(()) })?;
            Ok(())
        })?;
        let car = car.performance_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?;
        let car = car.manufacturer(b"M")?;
        let car = car.model(b"N")?;
        let complete = car.activation_code(b"P")?;
        assert!(complete.encoded_length_with_header() > 0);
        let encoded = complete.as_bytes_with_header();

        let dec = CarDecoder::try_wrap_and_apply_header(encoded, 0).unwrap();
        let mut fuel = dec.into_fuel_figures().unwrap();
        let first = fuel.next().unwrap().unwrap();
        assert_eq!(first.speed(), 10);
        let after_fuel = fuel.skip_remaining().unwrap();

        // We must still land at performance_figures, then the var-data, correctly.
        let mut perf = after_fuel.into_performance_figures().unwrap();
        assert_eq!(perf.len(), 0);
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

/// Empty groups and empty var-data still traverse through the same stages.
#[test]
fn empty_tail_components_traverse_stages() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE_EMPTY);
    compile_and_run(
        MODULE_EMPTY,
        &src,
        r#"
        let mut buf = [0u8; 4096];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        car.serial_number(1);
        let car = car.fuel_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?;
        let car = car.performance_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?;
        let car = car.manufacturer(b"")?;
        let car = car.model(b"")?;
        let complete = car.activation_code(b"")?;
        assert!(complete.encoded_length_with_header() > 0);
        let encoded = complete.as_bytes_with_header();

        let dec = CarDecoder::try_wrap_and_apply_header(encoded, 0).unwrap();
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
