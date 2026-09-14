//! Benchmark smoke tests (Item 3 of).
//!
//! Port of upstream `simple-binary-encoding/rust/benches/car_benchmark.rs`.
//! The upstream benchmarks use `criterion` on the upstream Rust codegen's
//! generated API (pre-compiled modules).  Since `ergon` generates code on
//! the fly with a fundamentally different API (closure-based group setters,
//! `wrap_and_apply_header`), we:
//!
//! 1. Generate and compile the Car codec via `compile_and_run`
//! 2. Run a best-case encode/decode warmup loop + timed loop
//! 3. Assert that even in debug mode the round-trip completes promptly
//!
//! **Known trap — the nested build's scratch directory can vanish mid-run.**
//! `compile_and_run` spawns a *nested* cargo build under the system temp dir
//! (`/var/folders/.../T/ergo_test_<name>/target_ci` on macOS). That tree is not
//! owned by cargo's usual locking, and it can be reclaimed or truncated while a
//! build is in flight. Observed 2026-09-14 across a whole `just test` run:
//!
//! ```text
//! error: extern location for miette does not exist: .../target_ci/.../libmiette-*.rmeta
//! error: Unable to proceed. Could not locate working directory.
//! error: failed to write .../target_ci/debug/.fingerprint/...
//! error: linking with `cc` failed: exit status: 1
//! ```
//!
//! Every affected test failed inside `compile_and_run`, never on an assertion.
//! A single earlier instance (2026-09-12) hit this binary alone and was recorded
//! as unexplained; this is that mechanism. It cleared after freeing build
//! artifacts, and the same suite then passed 126/126.
//!
//! The timing assertions here are *not* a plausible cause of a failure: they
//! allow 30 seconds for 50 debug-mode iterations, and the binary survives
//! deliberate concurrent-cargo load (74s versus ~13s isolated, still passing).
//! So if this binary fails, read the nested cargo's stderr first — an I/O or
//! missing-file error means the scratch tree went away, not that the codec got
//! slow. Free disk space and re-run before investigating the code.
//!
//! Real criterion benchmarks should be added once the codegen stabilises
//! and `criterion` is added to dev-dependencies.  For now this smoke test
//! ensures the encode/decode paths are functional and not pathologically
//! slow.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, compile_and_run, generate};

#[test]
fn car_encode_decode_perf_smoke() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "perf_car");

    compile_and_run(
        "perf_car",
        &src,
        r#"
        // The CarEncoder type-state requires all tail fields (groups + var-data)
        // to reach the Complete state before as_bytes() is available.
        // We build one Car, then benchmark decode on it.

        fn encode_car_vec() -> Vec<u8> {
            let mut buf = [0u8; 512];
            let mut extras = OptionalExtras::default();
            extras.cruise_control(true);
            extras.sports_pack(true);

            // Tails are reachable only after `fixed()` — the CarEncoder
            // typestate moves FieldsUnfixed -> FieldsFixed.
            let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
                .fixed(&CarFixedFields {
                    serial_number: 1234,
                    model_year: 2013,
                    available: BooleanType::T,
                    code: Model::A,
                    some_numbers: [1u32, 2, 3, 4],
                    vehicle_code: [97, 98, 99, 100, 101, 102],
                    extras,
                    engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
                });

            let car = car.fuel_figures(1, |g| {
                g.add(|mut e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban Cycle") })?;
                Ok(())
            }).unwrap();

            let car = car.performance_figures(1, |g| {
                g.add(|mut e| {
                    e.octane_rating(95);
                    e.acceleration(3, |a| {
                        a.add(|x| { x.mph(30).seconds(4.0); Ok(()) })?;
                        a.add(|x| { x.mph(60).seconds(7.5); Ok(()) })?;
                        a.add(|x| { x.mph(100).seconds(12.2); Ok(()) })
                    })
                })?;
                Ok(())
            }).unwrap();

            let car = car.manufacturer(b"Honda").unwrap();
            let car = car.model(b"Civic VTi").unwrap();
            let encoded = car.activation_code(b"abcdef").unwrap();
            encoded.as_bytes_with_header().to_vec()
        }

        let encoded = encode_car_vec();

        let encode_start = std::time::Instant::now();
        for _ in 0..50 {
            encode_car_vec();
        }
        let encode_dur = encode_start.elapsed();
        eprintln!("encode 50x: {:?}", encode_dur);

        let decode_start = std::time::Instant::now();
        for _ in 0..50 {
            let car2 = CarDecoder::try_decode(&encoded, 0).unwrap();
            assert_eq!(1234, car2.serial_number());
            assert_eq!(2013, car2.model_year());
            assert_eq!(BooleanType::T, car2.available());
            assert_eq!(Model::A, car2.code());
            assert_eq!([1u32, 2, 3, 4], car2.some_numbers());
            assert_eq!([97, 98, 99, 100, 101, 102], car2.vehicle_code());
        }
        let decode_dur = decode_start.elapsed();
        eprintln!("decode 50x: {:?}", decode_dur);

        // Smoke check: even in debug mode, 50 iterations under 30 seconds.
        assert!(encode_dur.as_secs() < 30, "encode too slow: {:?}", encode_dur);
        assert!(decode_dur.as_secs() < 30, "decode too slow: {:?}", decode_dur);
    "#,
    );

    Ok(())
}
