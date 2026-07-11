//! Integration tests for `ErgoSBE` code generation.

#![allow(missing_docs)]
#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, compile_and_run, generate};
use ergosbe::{GenerationConfig, Generator, Schema, parse};
use std::fs;

#[test]
fn test_generate_car_example() {
    let xml_path = Paths::example_schema();

    let xml_content = fs::read_to_string(&xml_path).expect("Failed to read example schema");

    let ir = parse(&xml_content).expect("Failed to parse SBE schema");
    let schema = Schema::from_ir(ir);

    let generator = Generator::new(GenerationConfig::new("car_example"));
    let module_set = generator.generate(&schema);

    let module = module_set.modules().next().unwrap();
    assert_eq!(module.path, "car_example.rs");

    // Check that expected generated components exist in the source code
    assert!(module.source.contains("pub struct CarDecoder"));
    assert!(module.source.contains("pub struct CarEncoder"));
    assert!(module.source.contains("pub struct MessageHeader"));
    assert!(module.source.contains("pub struct Booster"));
    assert!(module.source.contains("pub struct Engine"));
    assert!(module.source.contains("pub struct OptionalExtras"));
    assert!(module.source.contains("pub enum Model"));
    assert!(module.source.contains("pub enum BooleanType"));
}

#[test]
fn test_fixed_entry_group_access() {
    let (_schema, src) = generate(&Paths::example_schema(), "fixed_entry_group");

    compile_and_run(
        "fixed_entry_group",
        &src,
        r#"
        fn encode_car() -> Vec<u8> {
            let mut buf = vec![0u8; 512];
            let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
            car.serial_number(1234);
            car.model_year(2013);
            car.available(BooleanType::T);
            car.code(Model::A);
            car.some_numbers([1u32, 2, 3, 4]);
            car.vehicle_code([97, 98, 99, 100, 101, 102]);
            let mut extras = OptionalExtras::default();
            extras.set_cruise_control(true);
            car.extras(extras);
            car.engine(Engine::new(2000, 4, [49, 0, 0]));

            let car = car.fuel_figures(1, |g| {
                g.add(|e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban Cycle").unwrap(); }).unwrap();
            }).unwrap();

            let car = car.performance_figures(1, |g| {
                g.add(|e| {
                    e.octane_rating(95);
                    e.acceleration(3, |a| {
                        a.add(|x| { x.mph(30).seconds(4.0); }).unwrap();
                        a.add(|x| { x.mph(60).seconds(7.5); }).unwrap();
                        a.add(|x| { x.mph(100).seconds(12.2); }).unwrap();
                    }).unwrap();
                }).unwrap();
            }).unwrap();

            let car = car.manufacturer(b"Honda").unwrap();
            let car = car.model(b"Civic VTi").unwrap();
            let encoded = car.activation_code(b"abcdef").unwrap();
            encoded.as_bytes().to_vec()
        }

        let encoded = encode_car();
        let car = CarDecoder::wrap_and_apply_header(&encoded, 0).unwrap();

        // Navigate to acceleration group. Wire order is fuel_figures then
        // performance_figures; traverse fuel first via the consuming stages.
        let mut perf = car
            .into_fuel_figures()
            .unwrap()
            .finish()
            .unwrap()
            .into_performance_figures()
            .unwrap();
        let pf = perf.next().unwrap().unwrap();
        let mut accel = pf.acceleration().unwrap();

        // Use group decoder's Iterator impl (replaces as_chunks)
        {
            let mut acc_iter = pf.acceleration().unwrap();
            assert_eq!(acc_iter.len(), 3, "iterator should have 3 entries");
            let e0 = acc_iter.next().unwrap();
            assert_eq!(e0.mph(), 30, "mph of entry 0");
            let e1 = acc_iter.next().unwrap();
            assert_eq!(e1.mph(), 60, "mph of entry 1");
            let e2 = acc_iter.next().unwrap();
            assert_eq!(e2.mph(), 100, "mph of entry 2");
            assert!(acc_iter.next().is_none(), "should be no more entries");
        }

        // Test nth() random access (same as entry_at)
        let entry0 = accel.nth(0).unwrap();
        assert_eq!(entry0.mph(), 30, "nth(0) mph");
        assert!((entry0.seconds() - 4.0).abs() < 0.001, "nth(0) seconds");

        let entry_last = accel.nth(2).unwrap();
        assert_eq!(entry_last.mph(), 100, "nth(2) mph");
        assert!((entry_last.seconds() - 12.2).abs() < 0.001, "nth(2) seconds");

        // nth() out of bounds returns error
        assert!(accel.nth(3).is_err(), "nth(3) should be out of bounds");

        // Test iterating yields same results
        let mut accel2 = pf.acceleration().unwrap();
        let count = accel2.count();
        assert_eq!(count, 3, "iterator should yield 3 entries");
    "#,
    );
}
