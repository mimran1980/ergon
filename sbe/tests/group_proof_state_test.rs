//! Group trust-boundary proofs.
//!
//! A generated group decoder hands out entries whose required getters read
//! without bounds checks. That is only sound while three things hold, and this
//! suite pins all three:
//!
//! 1. **Proof-only constructors are unreachable.** `EntryDecoder::wrap`,
//!    `EntryEncoder::wrap`, and `GroupDecoder::wrap_with_parent` depend on a
//!    proof their caller must already hold, so they are private `unsafe fn`s in
//!    the generated module. Consumer code cannot name them.
//! 2. **A group's extent covers what its getters read.** A dimension header may
//!    declare `blockLength = 0, count = 1`. The compiled getters would then read
//!    schema-width fields past a region the wrap "proved". The checked
//!    constructors reject that.
//! 3. **Attachment is a proof, not a claim.** Only a group reached through its
//!    message's tail can complete into the next message stage.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

mod common;
use common::{Paths, compile_and_run, compile_fails_with_diagnostics, generate};

// ─── 1. Proof-only constructors ────────────────────────────────────────────

#[test]
fn consumer_cannot_name_the_entry_decoder_proof_constructor()
-> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "gp_entry_dec");
    compile_fails_with_diagnostics(
        "gp_entry_dec",
        &src,
        r#"
        let buf = [0u8; 64];
        // ILLEGAL: proof-only constructor, private to the generated module.
        let _ = FuelFiguresEntryDecoder::wrap(&buf, 0, 6, 0);
        "#,
        &["wrap"],
    );
    Ok(())
}

#[test]
fn consumer_cannot_name_the_entry_encoder_proof_constructor()
-> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "gp_entry_enc");
    compile_fails_with_diagnostics(
        "gp_entry_enc",
        &src,
        r#"
        let mut buf = [0u8; 64];
        // ILLEGAL: proof-only constructor, private to the generated module.
        let _ = FuelFiguresEntryEncoder::wrap(&mut buf, 0);
        "#,
        &["wrap"],
    );
    Ok(())
}

#[test]
fn consumer_cannot_invent_parent_proof_state() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "gp_parent");
    compile_fails_with_diagnostics(
        "gp_parent",
        &src,
        r#"
        let buf = [0u8; 64];
        // ILLEGAL: a consumer cannot claim a parent it has not proven.
        let _ = FuelFiguresDecoder::wrap_with_parent(&buf, 0, 0, 999, 999);
        "#,
        &["wrap_with_parent"],
    );
    Ok(())
}

// ─── 2. Group extent covers what the getters read ──────────────────────────

#[test]
fn count_one_dimension_only_group_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "gp_extent");
    compile_and_run(
        "gp_extent",
        &src,
        r#"
        // Dimension header claiming one entry of zero width. The compiled
        // FuelFigures entry reads speed(u16 @0) + mpg(f32 @2): six bytes that
        // are simply not there.
        let mut buf = [0u8; 4];
        buf[0..2].copy_from_slice(&0u16.to_le_bytes()); // blockLength = 0
        buf[2..4].copy_from_slice(&1u16.to_le_bytes()); // count       = 1

        let Err(err) = FuelFiguresDecoder::wrap(&buf, 0, 0) else {
            panic!("blockLength=0 with count=1 cannot hold the required fields");
        };
        match err {
            sbe_rt::DecodeError::BufferTooShort { needed, available, .. } => {
                assert!(
                    needed > available,
                    "error must report the required extent it could not prove"
                );
            }
            other => panic!("expected BufferTooShort, got {other:?}"),
        }

        // Zero count is fine: no entry is ever exposed, so nothing is read.
        let mut empty = [0u8; 4];
        empty[0..2].copy_from_slice(&0u16.to_le_bytes());
        empty[2..4].copy_from_slice(&0u16.to_le_bytes());
        let Ok(group) = FuelFiguresDecoder::wrap(&empty, 0, 0) else {
            panic!("an empty group exposes no entry, so it needs no extent");
        };
        assert!(group.is_empty());
        "#,
    );
    Ok(())
}

#[test]
fn undersized_stride_is_rejected_before_any_entry_is_exposed()
-> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "gp_stride");
    compile_and_run(
        "gp_stride",
        &src,
        r#"
        // FuelFigures requires six bytes per entry (speed u16 + mpg f32).
        assert_eq!(FuelFiguresDecoder::min_readable_fixed_extent(0), 6);

        for block_length in 0u16..6 {
            let mut buf = [0u8; 64];
            buf[0..2].copy_from_slice(&block_length.to_le_bytes());
            buf[2..4].copy_from_slice(&1u16.to_le_bytes());
            assert!(
                FuelFiguresDecoder::wrap(&buf, 0, 0).is_err(),
                "blockLength {block_length} is too small for the required fields"
            );
        }

        // Exact boundary: six bytes per entry, and the buffer holds them.
        let mut exact = [0u8; 4 + 6];
        exact[0..2].copy_from_slice(&6u16.to_le_bytes());
        exact[2..4].copy_from_slice(&1u16.to_le_bytes());
        let Ok(group) = FuelFiguresDecoder::wrap(&exact, 0, 0) else {
            panic!("exact stride must be accepted");
        };
        assert_eq!(group.remaining(), 1);

        // FuelFigures carries a var-data tail, so it has no constant stride and
        // its whole region cannot be proven at wrap time. A declared stride
        // running past the buffer must therefore be caught before the entry is
        // exposed, not silently iterated.
        let mut truncated = [0u8; 4 + 8];
        truncated[0..2].copy_from_slice(&16u16.to_le_bytes());
        truncated[2..4].copy_from_slice(&1u16.to_le_bytes());
        let mut dynamic = FuelFiguresDecoder::wrap(&truncated, 0, 0)
            .expect("a dynamic group's extent is proven per entry, not at wrap");
        assert!(
            matches!(dynamic.next(), Some(Err(_))),
            "an entry whose acting fixed block runs past the buffer must not be              handed out"
        );

        // `acceleration` is flat (mph u16 + seconds f32), so its whole entry
        // region is proven once, at wrap.
        assert_eq!(
            PerformanceFiguresAccelerationDecoder::min_readable_fixed_extent(0),
            6
        );

        let mut wider = [0u8; 4 + 16];
        wider[0..2].copy_from_slice(&16u16.to_le_bytes());
        wider[2..4].copy_from_slice(&1u16.to_le_bytes());
        assert!(
            PerformanceFiguresAccelerationDecoder::wrap(&wider, 0, 0).is_ok(),
            "a forward-compatible wider stride is accepted when the buffer holds it"
        );

        let mut short = [0u8; 4 + 8];
        short[0..2].copy_from_slice(&16u16.to_le_bytes());
        short[2..4].copy_from_slice(&1u16.to_le_bytes());
        assert!(
            PerformanceFiguresAccelerationDecoder::wrap(&short, 0, 0).is_err(),
            "a fixed-stride group must prove its whole region at wrap"
        );

        // Count overflow cannot wrap into a small allocation.
        let mut overflow = [0u8; 64];
        overflow[0..2].copy_from_slice(&8u16.to_le_bytes());
        overflow[2..4].copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(PerformanceFiguresAccelerationDecoder::wrap(&overflow, 0, 0).is_err());
        "#,
    );
    Ok(())
}

#[test]
fn dynamic_group_proves_each_entry_before_exposing_it() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::l3_orderbook_schema(), "gp_dynamic");
    compile_and_run(
        "gp_dynamic",
        &src,
        r#"
        // A dynamic group has no constant stride, so its region cannot be
        // proven once. Each entry must be proven before it is exposed.
        let mut buf = [0u8; 4];
        buf[0..2].copy_from_slice(&0u16.to_le_bytes()); // blockLength = 0
        buf[2..4].copy_from_slice(&1u16.to_le_bytes()); // count       = 1
        assert!(
            BidsDecoder::wrap(&buf, 0, 0).is_err(),
            "a dynamic entry whose block length cannot hold its required \
             fields must be rejected at the trust boundary"
        );
        "#,
    );
    Ok(())
}

// ─── 3. Attachment is a proof ──────────────────────────────────────────────

#[test]
fn detached_group_cannot_finish_into_a_message_stage() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "gp_detached");
    compile_fails_with_diagnostics(
        "gp_detached",
        &src,
        r#"
        let mut buf = [0u8; 64];
        buf[0..2].copy_from_slice(&6u16.to_le_bytes());
        buf[2..4].copy_from_slice(&0u16.to_le_bytes());
        let group = FuelFiguresDecoder::wrap(&buf, 0, 0).unwrap();
        // ILLEGAL: a standalone group has no parent message to complete into.
        let _ = group.finish();
        "#,
        &["finish"],
    );
    Ok(())
}

#[test]
fn attached_group_reached_through_a_message_tail_can_finish()
-> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "gp_attached");
    compile_and_run(
        "gp_attached",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let len = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2020,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0; 4],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(1, 1, [0; 3], 0i8, BooleanType::F,
                                    Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(1, |g| {
                g.add(|mut e| {
                    e.speed(30).mpg(35.9);
                    e.usage_description(b"city").unwrap();
                    Ok(())
                })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"H")?
            .model(b"C")?
            .activation_code(b"A")?
            .encoded_length_with_header();

        // Reached through the message tail: attached, so completion works and
        // the next stage really is the next tail component.
        let fuel = CarDecoder::try_from(&buf[..len])?.into_fuel_figures()?;
        let after_fuel = fuel.finish()?;
        let perf = after_fuel.into_performance_figures()?;
        let after_perf = perf.skip_remaining()?;
        let (manufacturer, next) = after_perf.into_manufacturer()?;
        assert_eq!(manufacturer, b"H");
        let (model, next) = next.into_model()?;
        assert_eq!(model, b"C");
        let (code, _) = next.into_activation_code()?;
        assert_eq!(code, b"A");

        // The detached form of the same group still iterates and rewinds.
        let mut standalone = FuelFiguresDecoder::wrap(&buf[..len], 8 + 45, 0)?;
        assert_eq!(standalone.remaining(), 1);
        let Some(Ok(entry)) = standalone.next() else {
            panic!("standalone group must yield its one entry");
        };
        assert_eq!(entry.speed(), 30);
        standalone.rewind();
        assert_eq!(standalone.remaining(), 1);
        "#,
    );
    Ok(())
}

// ─── 4. Bulk rows must be representable ────────────────────────────────────

#[test]
fn bulk_rows_are_emitted_only_for_required_since_v0_flat_groups()
-> Result<(), Box<dyn std::error::Error>> {
    // Required, since-v0, flat: an owned row can represent every valid acting
    // version, so bulk materialisation is offered.
    let (_, car) = generate(&Paths::example_schema(), "gp_bulk_ok");
    assert!(
        car.contains("pub fn bulk_decode_into("),
        "a flat group of required since-v0 fields should offer bulk decoding"
    );

    // Optional / versioned fields have no representation in a plain row struct:
    // materialising one would have to fabricate a value for an absent field.
    let ineligible = r#"<messageSchema package="bulkgate" id="1" version="1" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="groupSizeEncoding">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="numInGroup" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Book" id="1" blockLength="0">
    <group name="levels" id="1" dimensionType="groupSizeEncoding">
      <field name="price" id="1" type="int64" offset="0"/>
      <field name="qty" id="2" type="int64" offset="8" presence="optional"/>
      <field name="venue" id="3" type="uint16" offset="16" sinceVersion="1"/>
    </group>
  </message>
</messageSchema>"#;
    let ir = ergo_sbe::parse(ineligible)?;
    let schema = ergo_sbe::Schema::from_ir(ir);
    let src = ergo_sbe::Generator::new(ergo_sbe::GenerationConfig::new("bulkgate"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("no module")?
        .source
        .clone();

    assert!(
        !src.contains("pub fn bulk_decode_into("),
        "a group with optional or versioned fields must not advertise bulk row \
         materialisation — the row type cannot represent an absent field"
    );
    assert!(
        !src.contains("pub fn bulk_decode("),
        "the owning-Vec convenience wrapper must be gated the same way"
    );
    // Ordinary version-aware access remains available.
    assert!(
        src.contains("pub fn entry_at("),
        "ineligible groups must keep entry_at / iteration"
    );
    Ok(())
}

#[test]
fn ineligible_bulk_group_still_iterates_with_version_awareness()
-> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::multi_nested_group_schema(), "gp_bulk_iter");
    // Nested-tail groups were never bulk-eligible and must not have gained it.
    assert!(
        src.contains("pub fn scan_entry_at("),
        "dynamic groups keep scan_entry_at"
    );
    Ok(())
}

// ── T-1: dynamic groups must not expose fixed-stride proof APIs ─────────

#[test]
fn dynamic_group_has_no_start_entry() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "gp_no_start_entry");
    compile_fails_with_diagnostics(
        "gp_no_start_entry",
        &src,
        r#"
        let mut buf = [0u8; 256];
        // FuelFigures has var-data (usage_description) — dynamic, no start_entry.
        let mut g = FuelFiguresEncoder::wrap(&mut buf, 0, 1);
        let _ = g.start_entry();
        "#,
        &["start_entry"],
    );
    Ok(())
}

#[test]
fn dynamic_group_has_no_complete_or_add_checked() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "gp_no_complete");
    compile_fails_with_diagnostics(
        "gp_no_complete",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let mut g = FuelFiguresEncoder::wrap(&mut buf, 0, 1);
        let _ = g.add_checked(|e| e.complete());
        "#,
        &["add_checked"],
    );
    Ok(())
}

#[test]
fn fixed_stride_group_keeps_add_checked() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "gp_fixed_ok");
    compile_and_run(
        "gp_fixed_ok",
        &src,
        r#"
        let mut buf = [0u8; 64];
        let mut g = PerformanceFiguresAccelerationEncoder::wrap(&mut buf, 0, 1);
        g.add_checked(|mut e| {
            e.mph(60).seconds(3.5);
            Ok(e.complete())
        }).expect("fixed-stride add_checked");
        assert_eq!(g.written(), 1);
        "#,
    );
    Ok(())
}
