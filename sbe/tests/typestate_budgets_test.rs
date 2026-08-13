//! type-state compile-fail coverage + size_of / Send budgets.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
use std::error::Error;

mod common;
use common::{Paths, compile_and_run, compile_fails_with_diagnostics, generate};

/// Wrong group order: encode asks before bids must not compile.
#[test]
fn cf_encode_asks_before_bids() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "ts_enc_order");
    compile_fails_with_diagnostics(
        "ts_enc_order",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let mut e = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap();
        e.timestamp(1).sequence(1);
        // ILLEGAL: asks is not on the initial encoder stage
        let _ = e.asks(0, |_| Ok(()));
    "#,
        &["no method named `asks`"],
    );
    Ok(())
}

/// Header-absent complete stage has no as_bytes_with_header.
#[test]
fn cf_header_absent_no_as_bytes_with_header() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ts_hdr_absent");
    compile_fails_with_diagnostics(
        "ts_hdr_absent",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let done = CarEncoder::try_wrap(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2020,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0; 4],
                vehicle_code: [b'x'; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(
                    1, 1, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0),
                ),
            })
            .fuel_figures(0, |_| Ok(()))
            .unwrap()
            .performance_figures(0, |_| Ok(()))
            .unwrap()
            .manufacturer(b"a")
            .unwrap()
            .model(b"b")
            .unwrap()
            .activation_code(b"")
            .unwrap();
        let _ = done.as_bytes_with_header(); // ILLEGAL on HeaderAbsent
    "#,
        &["no method named `as_bytes_with_header`"],
    );
    Ok(())
}

/// Consumed encoder stage cannot be reused (moved).
#[test]
fn cf_consumed_encoder_stage_reuse() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ts_enc_moved");
    compile_fails_with_diagnostics(
        "ts_enc_moved",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let enc = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2020,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0; 4],
                vehicle_code: [b'x'; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(
                    1, 1, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0),
                ),
            });
        let after = enc.fuel_figures(0, |_| Ok(())).unwrap();
        let _again = enc.fuel_figures(0, |_| Ok(())); // ILLEGAL: enc moved
        let _ = after;
    "#,
        &["moved value: `enc`"],
    );
    Ok(())
}

/// size_of / Send / Sync / ZST marker budgets for generated Car stages.
#[test]
fn size_of_send_sync_stage_budgets() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ts_size");
    compile_and_run(
        "ts_size",
        &src,
        r#"
        use core::mem::{size_of, align_of};

        // Header markers are zero-sized and Send+Sync.
        assert_eq!(size_of::<sbe_rt::HeaderPresent>(), 0);
        assert_eq!(size_of::<sbe_rt::HeaderAbsent>(), 0);
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<sbe_rt::HeaderPresent>();
        assert_send_sync::<sbe_rt::HeaderAbsent>();

        // Encoder stages: same layout family as a plain (buf, msg_offset, pos) carrier.
        // PhantomData must not grow the type.
        type Carrier = (*mut u8, usize, usize); // conceptual lower bound, not ABI identity
        let stage = size_of::<CarEncoder<'_>>();
        let after = size_of::<CarAfterFuelFigures<'_>>();
        let complete = size_of::<CarComplete<'_>>();
        assert_eq!(stage, after, "named stages must monomorphize to equal size");
        assert_eq!(stage, complete, "complete stage same size as initial");
        // Must be small: pointer + two usizes + ZST marker (no heap, no tag).
        assert!(
            stage <= size_of::<Carrier>() + size_of::<usize>(),
            "encoder stage unexpectedly large: {stage}"
        );
        assert_send_sync::<CarEncoder<'_>>();
        assert_send_sync::<CarComplete<'_>>();

        // Decoder: buf + pos + acting_block_length + acting_version (no stage tag).
        let dec = size_of::<CarDecoder<'_>>();
        assert!(
            dec <= size_of::<(*const u8, usize, usize, u16)>() + 8,
            "decoder unexpectedly large: {dec}"
        );
        assert_send_sync::<CarDecoder<'_>>();

        // Drop is pure (no custom Drop with heap).
        assert!(!std::mem::needs_drop::<CarEncoder<'_>>());
        assert!(!std::mem::needs_drop::<CarDecoder<'_>>());
        let _ = align_of::<CarEncoder<'_>>();
    "#,
    );
    Ok(())
}

/// Generated source-size budget for the default Full Car module (pinned noise band).
#[test]
fn car_full_generated_source_size_budget() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ts_budget");
    let bytes = src.len();
    // Full Car has historically been hundreds of KiB of pretty source. Bound
    // growth without inventing a tiny number that fails on every docstring.
    const MAX_BYTES: usize = 400_000;
    const MIN_BYTES: usize = 20_000;
    assert!(
        bytes <= MAX_BYTES,
        "Full Car generated source {bytes} exceeds budget {MAX_BYTES}"
    );
    assert!(
        bytes >= MIN_BYTES,
        "Full Car generated source {bytes} below floor {MIN_BYTES} (empty emit?)"
    );
    Ok(())
}
