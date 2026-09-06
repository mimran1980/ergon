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

        // Decoder at the default config: buf + pos + acting_block_length +
        // acting_version, nothing else. Memoization is opt-in, so the default
        // decoder carries no `Cell` cache and stays `Sync`.
        // `memoized_decoder_pays_one_boundary_cache` pins the other side.
        let dec = size_of::<CarDecoder<'_>>();
        let carrier = size_of::<(*const u8, usize, usize, u16)>() + 8;
        assert!(
            dec <= carrier,
            "default decoder larger than its carrier: {dec} > {carrier}"
        );
        assert_send_sync::<CarDecoder<'_>>();
        // Entry decoders with tails keep the one-shot extent cache in every
        // lane, so they are `Send` and never `Sync`. The base message decoder
        // carries no cache at all, so it stays `Sync`.
        fn assert_send<T: Send>() {}
        assert_send::<FuelFiguresEntryDecoder<'_>>();

        // Drop is pure (no custom Drop with heap).
        assert!(!std::mem::needs_drop::<CarEncoder<'_>>());
        assert!(!std::mem::needs_drop::<CarDecoder<'_>>());
        let _ = align_of::<CarEncoder<'_>>();
    "#,
    );
    Ok(())
}

/// The other side of the lane split: `Decoder::memoized(self)`.
///
/// Opting in must cost exactly one boundary cache and no more, and it is what
/// makes the wrapper `Send` but not `Sync` (`Cell` interior mutability). The
/// base budget is pinned in `size_of_send_sync_stage_budgets`, so the two
/// tests together bound both lanes rather than relaxing either.
#[test]
fn memoized_decoder_pays_one_boundary_cache() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ts_size_memoized");
    compile_and_run(
        "ts_size_memoized",
        &src,
        r#"
        use core::mem::size_of;

        fn assert_send<T: Send>() {}

        let base = size_of::<CarDecoder<'_>>();
        let memo = size_of::<CarMemoizedDecoder<'_>>();
        let cache = size_of::<sbe_rt::TailBoundaryCache<5>>();
        assert!(
            memo <= base + cache,
            "memoized lane costs more than one boundary cache: {memo} > {base} + {cache}"
        );
        assert!(memo > base, "memoized lane must actually carry a cache");

        assert_send::<CarMemoizedDecoder<'_>>();
        assert!(!std::mem::needs_drop::<CarMemoizedDecoder<'_>>());
    "#,
    );
    Ok(())
}

/// The third lane's budget: `Decoder::ordered(self)`.
///
/// The ordered cursor must store the base decoder plus its own position —
/// a tail offset and the next expected ordinal — and nothing else. It must
/// never grow a boundary cache: it already carries its current offset and
/// never re-walks an earlier tail, so a cache would be pure overhead.
#[test]
fn ordered_decoder_stores_only_the_decoder_core_and_a_cursor() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ts_size_ordered");
    compile_and_run(
        "ts_size_ordered",
        &src,
        r#"
        use core::mem::size_of;

        let base = size_of::<CarDecoder<'_>>();
        let ord = size_of::<CarOrderedDecoder<'_>>();
        let cursor = size_of::<usize>() + size_of::<u16>();
        assert!(
            ord <= base + cursor + size_of::<usize>(),
            "ordered lane carries more than the decoder core plus a cursor: \
             {ord} > {base} + {cursor} (+ padding)"
        );
        // Strictly smaller than the memoized lane. Compare the two concrete
        // generated types — comparing against `base + size_of::<cache>()` only
        // restates the memoized budget and would stay green if the ordered
        // lane grew a cache of its own.
        let memo = size_of::<CarMemoizedDecoder<'_>>();
        assert!(
            ord < memo,
            "ordered lane is not smaller than the memoized lane: {ord} >= {memo}"
        );
        // And it must not have grown a cache-sized field by any other name.
        let cache = size_of::<sbe_rt::TailBoundaryCache<5>>();
        assert!(
            ord < base + cache,
            "ordered lane looks like it grew a boundary cache: {ord} vs {base} + {cache}"
        );

        // Sequential-only state is plain data, so the cursor stays shareable.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CarOrderedDecoder<'_>>();
        assert!(!std::mem::needs_drop::<CarOrderedDecoder<'_>>());
    "#,
    );
    Ok(())
}

fn generate_fixed_only(module: &str) -> Result<String, Box<dyn Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="fixedonly" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
          </types>
          <sbe:message name="KeepAlive" id="5" description="fixed-only 16-byte body">
            <field name="leadershipTermId" id="1" type="int64"/>
            <field name="clusterSessionId" id="2" type="int64"/>
          </sbe:message>
        </sbe:messageSchema>"#;
    let schema = ergo_sbe::Schema::from_ir(ergo_sbe::parse(xml)?);
    let src = ergo_sbe::Generator::new(ergo_sbe::GenerationConfig::new(module))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("no generated module")?
        .source
        .clone();
    Ok(src)
}

/// Fixed-only completion views must not be available before `fixed()`.
/// Otherwise wrap + as_bytes_with_header publishes leftover buffer bytes.
#[test]
fn cf_fixed_only_as_bytes_requires_fixed() -> Result<(), Box<dyn Error>> {
    let src = generate_fixed_only("ts_fixed_as_bytes")?;
    compile_fails_with_diagnostics(
        "ts_fixed_as_bytes",
        &src,
        r#"
        let mut buf = [0xA5u8; KeepAliveEncoder::ENCODED_LENGTH];
        let enc = KeepAliveEncoder::wrap_and_apply_header(&mut buf, 0);
        let _ = enc.as_bytes_with_header();
    "#,
        &["no method named `as_bytes_with_header`"],
    );
    compile_fails_with_diagnostics(
        "ts_fixed_encoded_length",
        &src,
        r#"
        let mut buf = [0xA5u8; KeepAliveEncoder::ENCODED_LENGTH];
        let enc = KeepAliveEncoder::wrap_and_apply_header(&mut buf, 0);
        let _ = enc.encoded_length();
        let _ = enc.encoded_length_with_header();
        let _ = enc.into_remaining_mut();
    "#,
        &["no method named"],
    );
    Ok(())
}

/// After `fixed()`, the body is the required fields — not leftover 0xA5.
#[test]
fn fixed_only_fixed_then_as_bytes_writes_fields() -> Result<(), Box<dyn Error>> {
    let src = generate_fixed_only("ts_fixed_writes")?;
    compile_and_run(
        "ts_fixed_writes",
        &src,
        r#"
        let mut buf = [0xA5u8; KeepAliveEncoder::ENCODED_LENGTH];
        let enc = KeepAliveEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&KeepAliveFixedFields {
                leadership_term_id: 5,
                cluster_session_id: 10,
            });
        let bytes = enc.as_bytes_with_header();
        assert_eq!(bytes.len(), KeepAliveEncoder::ENCODED_LENGTH);
        assert_ne!(&bytes[8..], &[0xA5u8; 16], "body must not stay stale");
        assert_eq!(&bytes[8..16], &5i64.to_le_bytes());
        assert_eq!(&bytes[16..24], &10i64.to_le_bytes());
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
    // The mutable ordered lane (`CarOrderedDecoder` + group guards) is always
    // generated and is the bulk of the post-400 KiB growth.
    const MAX_BYTES: usize = 450_000;
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
