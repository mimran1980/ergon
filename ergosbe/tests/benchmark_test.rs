//! Benchmark smoke tests (Item 3 of todo 00).
//!
//! Port of upstream `simple-binary-encoding/rust/benches/car_benchmark.rs`.
//! The upstream benchmarks use `criterion` on the upstream Rust codegen's
//! generated API (pre-compiled modules).  Since `ErgoSBE` generates code on
//! the fly with a fundamentally different API (closure-based group setters,
//! `wrap_and_apply_header`), we:
//!
//! 1. Generate and compile the Car codec via `compile_and_run`
//! 2. Run a best-case encode/decode warmup loop + timed loop
//! 3. Assert that even in debug mode the round-trip completes promptly
//!
//! Real criterion benchmarks should be added once the codegen stabilises
//! and `criterion` is added to dev-dependencies.  For now this smoke test
//! ensures the encode/decode paths are functional and not pathologically
//! slow.

mod common;
use common::{Paths, compile_and_run, generate};

#[test]
fn car_encode_decode_perf_smoke() {
    let (_, src) = generate(&Paths::example_schema(), "perf_car");

    compile_and_run(
        "perf_car",
        &src,
        r#"
        // The CarEncoder type-state requires all tail fields (groups + var-data)
        // to reach the Complete state before as_bytes() is available.
        // We build one Car, then benchmark decode on it.

        // ── Build reference encoded Car, return a Vec so ownership is clean ──
        fn encode_car_vec() -> Vec<u8> {
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
            extras.set_sports_pack(true);
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

        let encoded = encode_car_vec();

        // ── Timed encode ──
        let encode_start = std::time::Instant::now();
        for _ in 0..50 {
            encode_car_vec();
        }
        let encode_dur = encode_start.elapsed();
        eprintln!("encode 50x: {:?}", encode_dur);

        // ── Timed decode ──
        let decode_start = std::time::Instant::now();
        for _ in 0..50 {
            let car2 = CarDecoder::wrap_and_apply_header(&encoded, 0).unwrap();
            assert_eq!(1234, car2.serial_number().unwrap());
            assert_eq!(2013, car2.model_year().unwrap());
            assert_eq!(BooleanType::T, car2.available().unwrap());
            assert_eq!(Model::A, car2.code().unwrap());
            assert_eq!([1u32, 2, 3, 4], car2.some_numbers().unwrap());
            assert_eq!([97, 98, 99, 100, 101, 102], car2.vehicle_code().unwrap());
        }
        let decode_dur = decode_start.elapsed();
        eprintln!("decode 50x: {:?}", decode_dur);

        // Smoke check: even in debug mode, 50 iterations under 30 seconds.
        assert!(encode_dur.as_secs() < 30, "encode too slow: {:?}", encode_dur);
        assert!(decode_dur.as_secs() < 30, "decode too slow: {:?}", decode_dur);
    "#,
    );
}
