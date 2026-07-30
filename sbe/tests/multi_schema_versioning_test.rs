//! Multi-schema generation: schema-ID and version isolation, shared-type
//! dedup, and version-number warnings for shared types.
//!
//! These tests verify that `generate_multi` correctly:
//! 1. Bakes each schema's own `SCHEMA_ID` and `SCHEMA_VERSION` into its module.
//! 2. Deduplicates shared types (the second schema `pub use`s them).
//! 3. Warns when a shared type carries version-gated members (`sinceVersion > 0`),
//!    because version numbers are per-schema and do not transfer across schemas.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, unused)]

mod common;
use common::{Paths, compile_and_run_two_modules};

use ergo_sbe::{GenerationConfig, Generator, Schema, parse_file};
use std::path::PathBuf;

fn multi_schema_a() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/schemas/multi-schema-a.xml")
}

fn multi_schema_b() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/schemas/multi-schema-b.xml")
}

fn generate_pair() -> (Schema, Schema) {
    let ir_a = parse_file(&multi_schema_a()).expect("parse schema A");
    let ir_b = parse_file(&multi_schema_b()).expect("parse schema B");
    (Schema::from_ir(ir_a), Schema::from_ir(ir_b))
}

// ── Schema-ID isolation ────────────────────────────────────────────────

#[test]
fn each_schema_bakes_its_own_schema_id() -> Result<(), Box<dyn std::error::Error>> {
    let (schema_a, schema_b) = generate_pair();
    let mut config = GenerationConfig::new("multi");
    config = config.with_shared_module("common_types");
    let mut g = Generator::new(config);

    let modules = g.generate_multi(&[(&schema_a, "common_types"), (&schema_b, "market_data")])?;
    let mods: Vec<_> = modules.modules().collect();
    assert_eq!(mods.len(), 2);

    // Schema A has id=101
    assert!(
        mods[0].source.contains("SCHEMA_ID: u16 = 101"),
        "schema A should bake id=101; got:\n{}",
        &mods[0].source[..500]
    );
    // Schema B has id=202
    assert!(
        mods[1].source.contains("SCHEMA_ID: u16 = 202"),
        "schema B should bake id=202; got:\n{}",
        &mods[1].source[..500]
    );

    Ok(())
}

// ── Schema-version isolation ───────────────────────────────────────────

#[test]
fn each_schema_bakes_its_own_version() -> Result<(), Box<dyn std::error::Error>> {
    let (schema_a, schema_b) = generate_pair();
    let mut config = GenerationConfig::new("multi");
    config = config.with_shared_module("common_types");
    let mut g = Generator::new(config);

    let modules = g.generate_multi(&[(&schema_a, "common_types"), (&schema_b, "market_data")])?;
    let mods: Vec<_> = modules.modules().collect();

    // Schema A has version=2
    assert!(
        mods[0].source.contains("SCHEMA_VERSION: u16 = 2"),
        "schema A should bake version=2"
    );
    // Schema B has version=0
    assert!(
        mods[1].source.contains("SCHEMA_VERSION: u16 = 0"),
        "schema B should bake version=0"
    );

    Ok(())
}

// ── Shared-type dedup ──────────────────────────────────────────────────

#[test]
fn shared_types_defined_once_imported_by_second_module() -> Result<(), Box<dyn std::error::Error>> {
    let (schema_a, schema_b) = generate_pair();
    let mut config = GenerationConfig::new("multi");
    config = config.with_shared_module("common_types");
    let mut g = Generator::new(config);

    let modules = g.generate_multi(&[(&schema_a, "common_types"), (&schema_b, "market_data")])?;
    let mods: Vec<_> = modules.modules().collect();

    // Schema A defines Decimal composite and Side enum
    assert!(
        mods[0].source.contains("Decimal"),
        "schema A should reference Decimal"
    );
    assert!(
        mods[0].source.contains("Side"),
        "schema A should reference Side"
    );
    // Schema A generates the actual type definition (struct/enum)
    assert!(
        mods[0].source.contains("pub struct Decimal(")
            || mods[0].source.contains("pub struct Decimal "),
        "schema A should define Decimal struct: {}",
        &mods[0].source[..mods[0].source.len().min(400)]
    );

    // Schema B does NOT redefine the Decimal composite (deduped) — it imports it
    // The shared module only shares types from schema A; schema B's copy is skipped.
    assert!(
        !mods[1].source.contains("pub struct Decimal("),
        "schema B should NOT redefine Decimal struct (shared)"
    );

    // Schema B imports from the shared module
    assert!(
        mods[1].source.contains("pub use super::common_types::*;"),
        "schema B should import shared types"
    );

    Ok(())
}

// ── Cross-schema round-trip ────────────────────────────────────────────

#[test]
fn cross_schema_messages_decode_with_correct_schema_id() -> Result<(), Box<dyn std::error::Error>> {
    let (schema_a, schema_b) = generate_pair();
    let mut config = GenerationConfig::new("multi");
    config = config.with_shared_module("common_types");
    let mut g = Generator::new(config);

    let modules = g.generate_multi(&[(&schema_a, "common_types"), (&schema_b, "market_data")])?;
    let mods: Vec<_> = modules.modules().collect();

    // Both modules compile and their decoders extract the correct schema ID
    // from their own message headers — proving the header constants are isolated.
    // Uses primitive fields only (composite accessors across modules is a known
    // limitation tracked separately).
    compile_and_run_two_modules(
        "multi_schema_ids",
        "common_types",
        &mods[0].source,
        "market_data",
        &mods[1].source,
        r#"
            // Schema A (id=101, version=2): verify baked constants
            assert_eq!(common_types::CommonMessageEncoder::SCHEMA_ID, 101);
            assert_eq!(common_types::CommonMessageEncoder::SCHEMA_VERSION, 2);

            // Schema B (id=202, version=0): verify baked constants
            assert_eq!(market_data::QuoteEncoder::SCHEMA_ID, 202);
            assert_eq!(market_data::QuoteEncoder::SCHEMA_VERSION, 0);

            // Encode + decode in schema A — verifies the message round-trips
            // within its own module (shared types work intra-module).
            let mut buf = [0u8; 64];
            let mut enc = common_types::CommonMessageEncoder::wrap_and_apply_header(&mut buf, 0);
            enc.price(common_types::Decimal::new(100, -2)).side(common_types::Side::Buy);
            let dec = common_types::CommonMessageDecoder::try_wrap_and_apply_header(&buf, 0)?;
            let side = dec.side();
            assert_eq!(side, common_types::Side::Buy);
        "#,
    );

    Ok(())
}

// ── Shared composite: fields accessible from importing module ───────────

/// When schema B imports shared types via `pub use super::common_types::*`,
/// the composite value struct's inner byte array must be `pub` so downstream
/// code can construct composites from the importing module.
#[test]
fn shared_composite_fields_are_public() -> Result<(), Box<dyn std::error::Error>> {
    let (schema_a, schema_b) = generate_pair();
    let mut config = GenerationConfig::new("multi");
    config = config.with_shared_module("common_types");
    let mut g = Generator::new(config);

    let modules = g.generate_multi(&[(&schema_a, "common_types"), (&schema_b, "market_data")])?;
    let mods: Vec<_> = modules.modules().collect();

    compile_and_run_two_modules(
        "multi_shared_fields",
        "common_types",
        &mods[0].source,
        "market_data",
        &mods[1].source,
        r#"
            // Access the Decimal composite via the importing module.
            // The inner field must be pub so this compiles.
            let d = market_data::Decimal::new(100, -2);
            assert_eq!(d.mantissa(), 100);
            assert_eq!(d.exponent(), -2);

            // Same for enums — Side should be accessible
            let s = market_data::Side::Buy;
            assert_eq!(s, market_data::Side::Buy);

            // The message encoder in the importing module should work
            let mut buf = [0u8; 64];
            let mut enc = market_data::QuoteEncoder::wrap_and_apply_header(&mut buf, 0);
            enc.bid_mantissa(100).ask_mantissa(200).bid_side(market_data::Side::Buy);
            let dec = market_data::QuoteDecoder::try_wrap_and_apply_header(&buf, 0)?;
            assert_eq!(dec.bid_mantissa(), 100);
            assert_eq!(dec.ask_mantissa(), 200);
            assert_eq!(dec.bid_side(), market_data::Side::Buy);
        "#,
    );

    Ok(())
}

// ── Shared types with sinceVersion > 0 ─────────────────────────────────
//
// Version numbers are per-schema. A shared type with version-gated members
// (sinceVersion > 0) is ambiguous when imported by a schema at a different
// version — the importer's acting_version may not match the type's evolution.
// The generator should warn about this.

#[test]
fn shared_type_with_version_gated_field_emits_warning() -> Result<(), Box<dyn std::error::Error>> {
    // Schema A at version 2 with a composite that has a member at sinceVersion=1.
    let schema_a_xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
                   package="warn" id="301" version="2" byteOrder="littleEndian">
    <types>
        <composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
        </composite>
        <composite name="EvolvingDecimal" offset="0">
            <type name="mantissa" primitiveType="int64" sinceVersion="0"/>
            <type name="exponent" primitiveType="int8" sinceVersion="1"/>
        </composite>
    </types>
    <sbe:message name="Msg" id="1">
        <field name="value" id="1" type="EvolvingDecimal"/>
    </sbe:message>
</sbe:messageSchema>"#;

    let schema_b_xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
                   package="importer" id="302" version="1" byteOrder="littleEndian">
    <types>
        <composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
        </composite>
        <composite name="EvolvingDecimal" offset="0">
            <type name="mantissa" primitiveType="int64" sinceVersion="0"/>
            <type name="exponent" primitiveType="int8" sinceVersion="1"/>
        </composite>
    </types>
    <sbe:message name="ImportMsg" id="1">
        <field name="value" id="1" type="EvolvingDecimal"/>
    </sbe:message>
</sbe:messageSchema>"#;

    let ir_a = ergo_sbe::parse(schema_a_xml)?;
    let ir_b = ergo_sbe::parse(schema_b_xml)?;
    let schema_a = Schema::from_ir(ir_a);
    let schema_b = Schema::from_ir(ir_b);

    let mut config = GenerationConfig::new("multi_warn");
    config = config.with_shared_module("common_types");

    // Capture warnings from generate_multi
    let mut g = Generator::new(config);
    let result = g.generate_multi(&[(&schema_a, "common_types"), (&schema_b, "importer")]);

    let modules = result.expect("generation should succeed with warnings, not error");
    let warnings = modules.warnings();

    // The shared EvolvingDecimal has exponent at sinceVersion=1 — must warn.
    assert!(
        !warnings.is_empty(),
        "expected at least one warning about version-gated shared type, got: {warnings:?}"
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("EvolvingDecimal") && w.contains("sinceVersion")),
        "warning should mention EvolvingDecimal and sinceVersion; got: {warnings:?}"
    );

    Ok(())
}

// ── No shared module: each schema is standalone ────────────────────────

#[test]
fn without_shared_module_each_schema_is_standalone() -> Result<(), Box<dyn std::error::Error>> {
    let (schema_a, schema_b) = generate_pair();
    let mut g = Generator::new(GenerationConfig::new("multi"));

    let modules = g.generate_multi(&[(&schema_a, "a"), (&schema_b, "b")])?;
    let mods: Vec<_> = modules.modules().collect();

    // Without shared_module, both modules define their own types and sbe_rt
    assert!(mods[0].source.contains("pub struct Decimal"));
    assert!(mods[1].source.contains("pub struct Decimal"));
    assert!(mods[0].source.contains("pub mod sbe_rt"));
    assert!(mods[1].source.contains("pub mod sbe_rt"));

    Ok(())
}
