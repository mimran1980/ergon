//! Hostile-input test for flat var-data accessors.

#![allow(clippy::all, clippy::pedantic, clippy::restriction)]
use std::error::Error;

mod common;
use common::{Paths, compile_and_run, generate};

#[test]
fn flat_var_data_accessors_never_panic_on_truncated_tail() -> Result<(), Box<dyn Error>> {
    let (_schema, source) = generate(&Paths::example_schema(), "hostile_flat_accessor");

    let body = r###"
        use std::panic::{catch_unwind, AssertUnwindSafe};

        let mut ref_buf = [0u8; 512];
        let fields = CarFixedFields {
            serial_number: 1, model_year: 2024, available: BooleanType::T, code: Model::A,
            some_numbers: [0u32; 4], vehicle_code: *b"ABC123", extras: OptionalExtras(0),
            engine: Engine::new(2000, 4, *b"ENG", 0, BooleanType::F,
                Booster::new(BoostType::NITROUS, 200)),
        };
        let ref_len = CarEncoder::try_wrap_and_apply_header(&mut ref_buf, 0).unwrap()
            .fixed(&fields)
            .fuel_figures(0, |_| Ok(())).unwrap()
            .performance_figures(0, |_| Ok(())).unwrap()
            .manufacturer(b"BMW").unwrap()
            .model(b"M3").unwrap()
            .activation_code(b"XYZ").unwrap()
            .encoded_length_with_header();
        let ref_frame = &ref_buf[..ref_len];

        let min_len = 8 + 45;
        let mut err_count = 0usize;
        for len in min_len..ref_len {
            let buf = &ref_frame[..len];
            let result = catch_unwind(AssertUnwindSafe(|| {
                match CarDecoder::try_decode(buf, 0) {
                    Ok(dec) => { let _ = dec.manufacturer(); let _ = dec.model(); let _ = dec.activation_code(); }
                    Err(_) => {}
                }
            }));
            assert!(result.is_ok(), "flat var-data accessor panicked at buffer len {len}");
            if len == min_len {
                if let Ok(dec) = CarDecoder::try_decode(buf, 0) {
                    assert!(dec.manufacturer().is_err() || dec.model().is_err() || dec.activation_code().is_err());
                }
            }
            if let Ok(dec) = CarDecoder::try_decode(buf, 0) {
                if dec.manufacturer().is_err() { err_count += 1; }
            }
        }
        assert!(err_count > 0, "manufacturer() never returned Err across all truncation points");
    "###;

    compile_and_run("hostile_flat_accessor", &source, &body);
    Ok(())
}
