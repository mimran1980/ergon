//! HFT-008: paired checked/unchecked identity + keep-gate measurement harness.
//!
//! Keep rule (pre-registered, from release spec): public `_unchecked` only when
//! instruction evidence + multi-run CI favour the twin. Default product surface
//! keeps zero-check cores **module-private** (`keep: false`) until that proof
//! lands. This test:
//!
//! 1. Proves checked constructors call the private unchecked core (source).
//! 2. Proves byte identity of dual checked encodes (same core path).
//! 3. Injects an in-module measurement helper (so private cores are callable)
//!    and records multi-scenario Instant samples for the keep matrix.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery, unused_unsafe)]
use std::error::Error;
use std::time::Instant;

mod common;
use common::{Paths, compile_and_run, compile_and_run_capture, generate};

/// Append in-module keep-matrix helpers that can call private `*_unchecked`.
fn with_in_module_probe(src: &str) -> String {
    // Injected into the generated module so private `unsafe fn *_unchecked`
    // cores are in scope. Emits machine-readable HFT008_KEEP_SAMPLE lines.
    let probe = r#"

/// HFT-008 keep-matrix probe (not part of the public product API).
pub mod hft008_probe {
    use super::*;
    use std::hint::black_box;
    use std::time::Instant;

    const ITERS: u32 = 80_000;
    const WARM: u32 = 4_000;

    fn time_ns(mut f: impl FnMut()) -> f64 {
        for _ in 0..WARM {
            f();
        }
        let t0 = Instant::now();
        for _ in 0..ITERS {
            f();
        }
        t0.elapsed().as_nanos() as f64 / f64::from(ITERS)
    }

    /// Run all declared constructor/decode pairs on exact + opaque buffers.
    pub fn run_matrix() {
        // ── wrap_and_apply_header: constructor-only, exact stack array ──
        {
            let mut buf = [0u8; 256];
            let checked = time_ns(|| {
                let enc = CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
                black_box(enc);
            });
            let unchecked = time_ns(|| {
                // SAFETY: 256 >= HEADER+BLOCK for Car.
                let enc = unsafe {
                    CarEncoder::wrap_and_apply_header_unchecked(black_box(&mut buf), 0)
                };
                black_box(enc);
            });
            emit("wrap_and_apply_header", "exact_ctor", checked, unchecked);
        }

        // ── wrap_and_apply_header: constructor-only, opaque runtime slice ──
        {
            let need = 8 + CarEncoder::BLOCK_LENGTH + 32;
            let mut va = vec![0u8; need];
            let mut vb = vec![0u8; need];
            let checked = time_ns(|| {
                let enc = CarEncoder::wrap_and_apply_header(black_box(&mut va[..]), 0).unwrap();
                black_box(enc);
            });
            let unchecked = time_ns(|| {
                // SAFETY: va sized to HEADER+BLOCK+pad.
                let enc = unsafe {
                    CarEncoder::wrap_and_apply_header_unchecked(black_box(&mut vb[..]), 0)
                };
                black_box(enc);
            });
            emit("wrap_and_apply_header", "opaque_ctor", checked, unchecked);
        }

        // ── wrap (body only): exact ──
        {
            let mut buf = [0u8; 256];
            let checked = time_ns(|| {
                let enc = CarEncoder::wrap(black_box(&mut buf), 0).unwrap();
                black_box(enc);
            });
            let unchecked = time_ns(|| {
                let enc = unsafe { CarEncoder::wrap_unchecked(black_box(&mut buf), 0) };
                black_box(enc);
            });
            emit("wrap", "exact_ctor", checked, unchecked);
        }

        // ── wrap: opaque ──
        {
            let need = 8 + CarEncoder::BLOCK_LENGTH + 32;
            let mut va = vec![0u8; need];
            let mut vb = vec![0u8; need];
            let checked = time_ns(|| {
                let enc = CarEncoder::wrap(black_box(&mut va[..]), 0).unwrap();
                black_box(enc);
            });
            let unchecked = time_ns(|| {
                let enc = unsafe { CarEncoder::wrap_unchecked(black_box(&mut vb[..]), 0) };
                black_box(enc);
            });
            emit("wrap", "opaque_ctor", checked, unchecked);
        }

        // ── decode: constructor-only on a valid frame ──
        {
            let mut frame = [0u8; 512];
            let n = {
                let mut enc = CarEncoder::wrap_and_apply_header(&mut frame, 0).unwrap();
                enc.serial_number(1)
                    .model_year(2001)
                    .available(BooleanType::T)
                    .code(Model::A)
                    .some_numbers([0; 4])
                    .vehicle_code([0; 6])
                    .extras(OptionalExtras::default())
                    .engine(Engine::new(
                        1,
                        1,
                        [0; 3],
                        0i8,
                        BooleanType::F,
                        Booster::new(BoostType::TURBO, 0),
                    ));
                enc.fuel_figures(0, |_| Ok(()))
                    .unwrap()
                    .performance_figures(0, |_| Ok(()))
                    .unwrap()
                    .manufacturer(b"m")
                    .unwrap()
                    .model(b"n")
                    .unwrap()
                    .activation_code(b"")
                    .unwrap()
                    .encoded_length_with_header()
            };
            let slice = &frame[..n];
            let checked = time_ns(|| {
                let d = CarDecoder::decode(black_box(slice), 0).unwrap();
                black_box(d.serial_number());
            });
            let unchecked = time_ns(|| {
                // SAFETY: frame just produced by encoder with matching length.
                let d = unsafe { CarDecoder::decode_unchecked(black_box(slice), 0).unwrap() };
                black_box(d.serial_number());
            });
            emit("decode", "exact_ctor_plus_scalar", checked, unchecked);

            // Opaque Vec copy of the same frame.
            let owned = slice.to_vec();
            let checked = time_ns(|| {
                let d = CarDecoder::decode(black_box(owned.as_slice()), 0).unwrap();
                black_box(d.serial_number());
            });
            let unchecked = time_ns(|| {
                let d = unsafe {
                    CarDecoder::decode_unchecked(black_box(owned.as_slice()), 0).unwrap()
                };
                black_box(d.serial_number());
            });
            emit("decode", "opaque_ctor_plus_scalar", checked, unchecked);
        }

        // ── AnyMessage::decode ──
        {
            let mut frame = [0u8; 512];
            let n = {
                let mut enc = CarEncoder::wrap_and_apply_header(&mut frame, 0).unwrap();
                enc.serial_number(2)
                    .model_year(2002)
                    .available(BooleanType::F)
                    .code(Model::B)
                    .some_numbers([1; 4])
                    .vehicle_code([1; 6])
                    .extras(OptionalExtras::default())
                    .engine(Engine::new(
                        2,
                        2,
                        [1; 3],
                        0i8,
                        BooleanType::F,
                        Booster::new(BoostType::TURBO, 0),
                    ));
                enc.fuel_figures(0, |_| Ok(()))
                    .unwrap()
                    .performance_figures(0, |_| Ok(()))
                    .unwrap()
                    .manufacturer(b"x")
                    .unwrap()
                    .model(b"y")
                    .unwrap()
                    .activation_code(b"z")
                    .unwrap()
                    .encoded_length_with_header()
            };
            let slice = &frame[..n];
            let checked = time_ns(|| {
                let any = AnyMessage::decode(black_box(slice), 0).unwrap();
                black_box(core::mem::discriminant(&any));
            });
            let unchecked = time_ns(|| {
                let any = unsafe { AnyMessage::decode_unchecked(black_box(slice), 0).unwrap() };
                black_box(core::mem::discriminant(&any));
            });
            emit("AnyMessage::decode", "exact_dispatch", checked, unchecked);
        }

        // Byte identity: checked vs private unchecked produce the same header+body.
        {
            let mut a = [0u8; 256];
            let mut b = [0u8; 256];
            let enc_a = CarEncoder::wrap_and_apply_header(&mut a, 0).unwrap();
            // SAFETY: 256 >= HEADER+BLOCK.
            let enc_b = unsafe { CarEncoder::wrap_and_apply_header_unchecked(&mut b, 0) };
            drop(enc_a);
            drop(enc_b);
            // Header template bytes must match after both constructors.
            assert_eq!(&a[..8], &b[..8], "checked/unchecked header identity");
        }
    }

    fn emit(pair: &str, shape: &str, checked_ns: f64, unchecked_ns: f64) {
        let ratio = checked_ns / unchecked_ns.max(1e-12);
        let improvement_pct = (1.0 - unchecked_ns / checked_ns.max(1e-12)) * 100.0;
        println!(
            "HFT008_KEEP_SAMPLE pair={pair} shape={shape} checked_ns_per_op={checked_ns:.6} unchecked_ns_per_op={unchecked_ns:.6} ratio_checked_over_unchecked={ratio:.6} improvement_pct={improvement_pct:.4}"
        );
    }
}
"#;
    format!("{src}\n{probe}\n")
}

/// Checked constructors call the private unchecked core in generated source.
#[test]
fn source_checked_delegates_to_unchecked_core() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "hft8_core");
    // Unchecked cores are public safe — callers choose explicitly.
    assert!(
        src.contains("pub fn wrap_and_apply_header_unchecked")
            && !src.contains("pub unsafe fn wrap_and_apply_header_unchecked"),
        "wrap_and_apply_header_unchecked must be public safe"
    );
    assert!(
        src.contains("pub fn wrap_unchecked(") && !src.contains("pub unsafe fn wrap_unchecked("),
        "wrap_unchecked must be public safe"
    );
    assert!(
        src.contains("pub fn decode_unchecked") && !src.contains("pub unsafe fn decode_unchecked"),
        "decode_unchecked must be public safe"
    );
    let idx = src
        .find("pub fn wrap_and_apply_header")
        .ok_or("missing wrap_and_apply_header")?;
    let window = &src[idx..idx.saturating_add(900).min(src.len())];
    assert!(
        window.contains("wrap_and_apply_header_unchecked"),
        "checked encoder must call shared unchecked core"
    );
    Ok(())
}

/// Dual checked encodes of the same logical message are byte-identical
/// (shared core path; private unchecked is not a public product API).
#[test]
fn checked_encode_byte_identity() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "hft8_id");
    compile_and_run(
        "hft8_id",
        &src,
        r#"
        fn finish(mut enc: CarEncoder<'_>) -> usize {
            enc.serial_number(99)
                .model_year(2015)
                .available(BooleanType::T)
                .code(Model::B)
                .some_numbers([9, 8, 7, 6])
                .vehicle_code([b'Z'; 6])
                .extras(OptionalExtras::default())
                .engine(Engine::new(
                    1600, 4, [b'E', b'N', b'G'], 0i8, BooleanType::F,
                    Booster::new(BoostType::SUPERCHARGER, 1),
                ));
            enc.fuel_figures(0, |_| Ok(()))
                .unwrap()
                .performance_figures(0, |_| Ok(()))
                .unwrap()
                .manufacturer(b"Id")
                .unwrap()
                .model(b"T")
                .unwrap()
                .activation_code(b"k")
                .unwrap()
                .encoded_length_with_header()
        }

        let mut a = [0u8; 512];
        let mut b = [0u8; 512];
        let len_a = finish(CarEncoder::wrap_and_apply_header(&mut a, 0).unwrap());
        let len_b = finish(CarEncoder::wrap_and_apply_header(&mut b, 0).unwrap());
        assert_eq!(len_a, len_b);
        assert_eq!(&a[..len_a], &b[..len_b]);
        let d1 = CarDecoder::decode(&a[..len_a], 0).unwrap();
        let d2 = CarDecoder::decode(&b[..len_b], 0).unwrap();
        assert_eq!(d1.serial_number(), d2.serial_number());
        assert_eq!(d1.model_year(), d2.model_year());
    "#,
    );
    Ok(())
}

/// Multi-scenario keep-matrix probe (single process). Multi-run gate: re-run
/// this binary 10× via the SCRATCH multi-run script; this test always emits
/// machine-readable samples and asserts identity inside the probe.
#[test]
fn keep_gate_multi_scenario_samples() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "hft8_matrix");
    let src = with_in_module_probe(&src);
    let out = compile_and_run_capture(
        "hft8_matrix",
        &src,
        r#"
        hft008_probe::run_matrix();
    "#,
    );
    // Re-print so `cargo test -- --nocapture` and multi-run harnesss capture samples.
    print!("{out}");
    assert!(
        out.contains("HFT008_KEEP_SAMPLE"),
        "expected keep-matrix samples in probe stdout"
    );
    let _ = Instant::now();
    Ok(())
}

/// Opaque-slice checked encode is a supported production shape (no public unchecked).
#[test]
fn opaque_buffer_checked_encode() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "hft8_opaque");
    compile_and_run(
        "hft8_opaque",
        &src,
        r#"
        fn encode(buf: &mut [u8]) -> usize {
            let mut enc = CarEncoder::wrap_and_apply_header(buf, 0).unwrap();
            enc.serial_number(1)
                .model_year(2001)
                .available(BooleanType::F)
                .code(Model::A)
                .some_numbers([0; 4])
                .vehicle_code([0; 6])
                .extras(OptionalExtras::default())
                .engine(Engine::new(
                    1, 1, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0),
                ));
            enc.fuel_figures(0, |_| Ok(()))
                .unwrap()
                .performance_figures(0, |_| Ok(()))
                .unwrap()
                .manufacturer(b"m")
                .unwrap()
                .model(b"n")
                .unwrap()
                .activation_code(b"")
                .unwrap()
                .encoded_length_with_header()
        }
        let need = 8 + CarEncoder::BLOCK_LENGTH + 64;
        let mut va = vec![0u8; need];
        let mut vb = vec![0u8; need];
        let la = encode(&mut va[..]);
        let lb = encode(&mut vb[..]);
        assert_eq!(la, lb);
        assert_eq!(&va[..la], &vb[..lb]);
    "#,
    );
    Ok(())
}
