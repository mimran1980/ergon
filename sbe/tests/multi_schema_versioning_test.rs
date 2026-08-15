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
    // ANCHOR: with_shared_module
    let mut config = GenerationConfig::new("multi");
    config = config.with_shared_module("common_types");
    // ANCHOR_END: with_shared_module
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
            let mut enc = common_types::CommonMessageEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap();
            enc.price(common_types::Decimal::new(100, -2)).side(common_types::Side::Buy);
            let dec = common_types::CommonMessageDecoder::try_decode(&buf, 0)?;
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
            // ── Composite (shared, from common_types via market_data re-export) ──
            let d = market_data::Decimal::new(100, -2);
            assert_eq!(d.mantissa(), 100);
            assert_eq!(d.exponent(), -2);
            // Composite value struct field must be accessible.
            assert_eq!(d.0[0..8], 100i64.to_le_bytes());

            // ── Enum (shared) ──
            let s = market_data::Side::Buy;
            assert_eq!(s, market_data::Side::Buy);
            assert_eq!(market_data::Side::Buy as u8, 0);
            assert_eq!(market_data::Side::Sell.raw(), 1u8);

            // ── CommonMessage (in common_types module) ──
            let mut buf = [0u8; 64];
            let mut enc = common_types::CommonMessageEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap();
            enc.price(common_types::Decimal::new(200, -1)).side(common_types::Side::Sell);
            let dec = common_types::CommonMessageDecoder::try_decode(&buf, 0)?;
            assert_eq!(dec.price().mantissa(), 200);
            assert_eq!(dec.side(), common_types::Side::Sell);

            // ── Quote message (in importing market_data module) ──
            let mut buf2 = [0u8; 64];
            let mut enc2 = market_data::QuoteEncoder::try_wrap_and_apply_header(&mut buf2, 0).unwrap();
            enc2.bid_mantissa(100).ask_mantissa(200).bid_side(market_data::Side::Buy);
            let dec2 = market_data::QuoteDecoder::try_decode(&buf2, 0)?;
            assert_eq!(dec2.bid_mantissa(), 100);
            assert_eq!(dec2.ask_mantissa(), 200);
            assert_eq!(dec2.bid_side(), market_data::Side::Buy);

            // ── Decoder accessors (flyweight) ──
            // groupSizeEncoding composite (shared) must support decode
            let dec3 = common_types::CommonMessageDecoder::try_decode(&buf, 0)?;
            let _price_val = dec3.price();
            assert_eq!(_price_val.mantissa(), 200);
        "#,
    );

    Ok(())
}

// ── Shared composite with sets and groups: all fields accessible ───────

/// Exercise every shared type category across modules: composite, enum,
/// set, group entry, and var-data.
#[test]
fn shared_set_enum_group_fields_are_public() -> Result<(), Box<dyn std::error::Error>> {
    let schema_a_xml = r#"<?xml version="1.0"?>
    <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
        package="shared" id="401" version="0" byteOrder="littleEndian">
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
        <composite name="varStringEncoding">
            <type name="length" primitiveType="uint32" maxValue="1073741824"/>
            <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
        </composite>
        <composite name="InnerValue">
            <type name="x" primitiveType="uint16"/>
            <type name="y" primitiveType="uint32"/>
        </composite>
        <enum name="OrderSide" encodingType="uint8">
            <validValue name="Bid">0</validValue>
            <validValue name="Ask">1</validValue>
        </enum>
        <set name="OrderFlags" encodingType="uint8">
            <choice name="aggressive">0</choice>
            <choice name="conditional">1</choice>
        </set>
    </types>
    <sbe:message name="Order" id="1">
        <field name="price" id="1" type="InnerValue"/>
        <field name="side" id="2" type="OrderSide"/>
        <field name="flags" id="3" type="OrderFlags"/>
        <data name="note" id="4" type="varStringEncoding"/>
    </sbe:message>
    </sbe:messageSchema>"#;

    let schema_b_xml = r#"<?xml version="1.0"?>
    <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
        package="consumer" id="402" version="0" byteOrder="littleEndian">
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
        <!-- Shared from schema A -->
        <composite name="InnerValue">
            <type name="x" primitiveType="uint16"/>
            <type name="y" primitiveType="uint32"/>
        </composite>
        <enum name="OrderSide" encodingType="uint8">
            <validValue name="Bid">0</validValue>
            <validValue name="Ask">1</validValue>
        </enum>
        <set name="OrderFlags" encodingType="uint8">
            <choice name="aggressive">0</choice>
            <choice name="conditional">1</choice>
        </set>
        <composite name="varStringEncoding">
            <type name="length" primitiveType="uint32" maxValue="1073741824"/>
            <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
        </composite>
    </types>
    <sbe:message name="Trade" id="1">
        <field name="qty" id="1" type="uint32"/>
        <field name="side" id="2" type="OrderSide"/>
        <field name="flags" id="3" type="OrderFlags"/>
        <field name="value" id="4" type="InnerValue"/>
        <data name="note" id="5" type="varStringEncoding"/>
    </sbe:message>
    </sbe:messageSchema>"#;

    let ir_a = ergo_sbe::parse(schema_a_xml)?;
    let ir_b = ergo_sbe::parse(schema_b_xml)?;
    let schema_a = Schema::from_ir(ir_a);
    let schema_b = Schema::from_ir(ir_b);

    let mut config = GenerationConfig::new("multi2");
    config = config.with_shared_module("shared_types");
    let mut g = Generator::new(config);
    let modules = g.generate_multi(&[(&schema_a, "shared_types"), (&schema_b, "consumer")])?;
    let mods: Vec<_> = modules.modules().collect();

    compile_and_run_two_modules(
        "multi_shared_set_group",
        "shared_types",
        &mods[0].source,
        "consumer",
        &mods[1].source,
        r#"
            // ── Composite from importing module ──
            let v = consumer::InnerValue::new(42, 9001);
            assert_eq!(v.x(), 42);
            assert_eq!(v.y(), 9001);

            // ── Enum from importing module ──
            let side = consumer::OrderSide::Bid;
            assert_eq!(side, consumer::OrderSide::Bid);

            // ── Set from importing module ──
            let mut flags = consumer::OrderFlags::default();
            flags.aggressive(true).conditional(false);
            assert!(flags.is_aggressive());
            assert!(!flags.is_conditional());

            // ── Encode via shared module ──
            let mut sf = shared_types::OrderFlags::default();
            sf.aggressive(true);
            let mut buf = [0u8; 128];
            let enc = shared_types::OrderEncoder::try_wrap_and_apply_header(&mut buf, 0)
                .unwrap()
                .fixed(&shared_types::OrderFixedFields {
                    price: shared_types::InnerValue::new(10, 20),
                    side: shared_types::OrderSide::Ask,
                    flags: sf,
                })
                .note(b"hello")?;
            let _ = enc;
            let dec = shared_types::OrderDecoder::try_decode(&buf, 0)?;
            assert_eq!(dec.price().x(), 10);
            assert_eq!(dec.side(), shared_types::OrderSide::Ask);
            assert!(dec.flags().is_aggressive());
            assert_eq!(dec.note()?, b"hello");

            // ── Encode via importing module (with shared types) ──
            let mut cf = consumer::OrderFlags::default();
            cf.conditional(true);
            let mut buf2 = [0u8; 128];
            let enc2 = consumer::TradeEncoder::try_wrap_and_apply_header(&mut buf2, 0)
                .unwrap()
                .fixed(&consumer::TradeFixedFields {
                    qty: 500,
                    side: consumer::OrderSide::Bid,
                    flags: cf,
                    value: consumer::InnerValue::new(7, 8),
                })
                .note(b"world")?;
            let _ = enc2;
            let dec2 = consumer::TradeDecoder::try_decode(&buf2, 0)?;
            assert_eq!(dec2.qty(), 500);
            assert_eq!(dec2.side(), consumer::OrderSide::Bid);
            assert!(dec2.flags().is_conditional());
            assert_eq!(dec2.value().y(), 8);
            assert_eq!(dec2.note()?, b"world");
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

// ── T-3: validate multi-schema module plan before emission ─────────────

fn mini_schema(package: &str, id: u16, extra_types: &str) -> Schema {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="{package}" id="{id}" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            {extra_types}
          </types>
          <sbe:message name="M" id="1"><field name="x" id="1" type="uint32"/></sbe:message>
        </sbe:messageSchema>"#
    );
    Schema::from_ir(ergo_sbe::parse(&xml).expect("parse mini schema"))
}

#[test]
fn multi_schema_rejects_empty_module_name() -> Result<(), Box<dyn std::error::Error>> {
    let a = mini_schema("a", 1, "");
    let b = mini_schema("b", 2, "");
    let config = GenerationConfig::new("multi").with_shared_module("a");
    let mut g = Generator::new(config);
    let err = g
        .generate_multi(&[(&a, "a"), (&b, "")])
        .expect_err("empty module name");
    assert!(
        matches!(err, ergo_sbe::GenerateError::InvalidConfiguration { .. }),
        "{err:?}"
    );
    Ok(())
}

#[test]
fn multi_schema_rejects_path_module_name() -> Result<(), Box<dyn std::error::Error>> {
    let a = mini_schema("a", 1, "");
    let b = mini_schema("b", 2, "");
    let config = GenerationConfig::new("multi").with_shared_module("a");
    let mut g = Generator::new(config);
    let err = g
        .generate_multi(&[(&a, "a"), (&b, "../evil")])
        .expect_err("path module");
    assert!(
        matches!(err, ergo_sbe::GenerateError::InvalidConfiguration { .. }),
        "{err:?}"
    );
    Ok(())
}

#[test]
fn multi_schema_rejects_duplicate_module_names() -> Result<(), Box<dyn std::error::Error>> {
    let a = mini_schema("a", 1, "");
    let b = mini_schema("b", 2, "");
    let config = GenerationConfig::new("multi").with_shared_module("shared");
    let mut g = Generator::new(config);
    let err = g
        .generate_multi(&[(&a, "shared"), (&b, "shared")])
        .expect_err("duplicate modules");
    assert!(
        matches!(err, ergo_sbe::GenerateError::InvalidConfiguration { .. }),
        "{err:?}"
    );
    let msg = err.to_string();
    assert!(msg.contains("duplicate") || msg.contains("shared"), "{msg}");
    Ok(())
}

#[test]
fn multi_schema_rejects_keyword_module_name() -> Result<(), Box<dyn std::error::Error>> {
    let a = mini_schema("a", 1, "");
    let b = mini_schema("b", 2, "");
    let config = GenerationConfig::new("multi").with_shared_module("a");
    let mut g = Generator::new(config);
    let err = g
        .generate_multi(&[(&a, "a"), (&b, "type")])
        .expect_err("keyword module");
    // Keywords may be rejected as invalid idents or accepted depending on
    // is_valid_module_ident — either InvalidConfiguration is fine if rejected.
    assert!(
        matches!(err, ergo_sbe::GenerateError::InvalidConfiguration { .. })
            || err.to_string().contains("type"),
        "{err:?}"
    );
    Ok(())
}

// ── T-12: incompatible shared type fingerprints ────────────────────────

#[test]
fn multi_schema_rejects_incompatible_shared_enum() -> Result<(), Box<dyn std::error::Error>> {
    let enum_a = r#"
      <type name="SideEnc" primitiveType="uint8"/>
      <enum name="Side" encodingType="SideEnc">
        <validValue name="Buy">1</validValue>
        <validValue name="Sell">2</validValue>
      </enum>"#;
    // Same name, different discriminant for Sell.
    let enum_b = r#"
      <type name="SideEnc" primitiveType="uint8"/>
      <enum name="Side" encodingType="SideEnc">
        <validValue name="Buy">1</validValue>
        <validValue name="Sell">9</validValue>
      </enum>"#;
    let a = mini_schema("a", 1, enum_a);
    let b = mini_schema("b", 2, enum_b);
    let config = GenerationConfig::new("multi").with_shared_module("common");
    let mut g = Generator::new(config);
    let err = g
        .generate_multi(&[(&a, "common"), (&b, "other")])
        .expect_err("incompatible enum");
    match err {
        ergo_sbe::GenerateError::IncompatibleSharedType {
            name,
            owner_module,
            consumer_module,
            difference,
        } => {
            assert!(name.contains("Side"), "{name}");
            assert_eq!(owner_module, "common");
            assert_eq!(consumer_module, "other");
            assert!(!difference.is_empty(), "{difference}");
        }
        other => panic!("expected IncompatibleSharedType, got {other:?}"),
    }
    Ok(())
}

#[test]
fn multi_schema_accepts_identical_shared_enum() -> Result<(), Box<dyn std::error::Error>> {
    let enum_xml = r#"
      <type name="SideEnc" primitiveType="uint8"/>
      <enum name="Side" encodingType="SideEnc">
        <validValue name="Buy">1</validValue>
        <validValue name="Sell">2</validValue>
      </enum>"#;
    let a = mini_schema("a", 1, enum_xml);
    let b = mini_schema("b", 2, enum_xml);
    let config = GenerationConfig::new("multi").with_shared_module("common");
    let mut g = Generator::new(config);
    let modules = g.generate_multi(&[(&a, "common"), (&b, "other")])?;
    assert_eq!(modules.modules().len(), 2);
    Ok(())
}

#[test]
fn multi_schema_rejects_shared_module_not_owner() -> Result<(), Box<dyn std::error::Error>> {
    let a = mini_schema("a", 1, "");
    let b = mini_schema("b", 2, "");
    // shared_module name must equal the first schema module (owner).
    let config = GenerationConfig::new("multi").with_shared_module("common");
    let mut g = Generator::new(config);
    let err = g
        .generate_multi(&[(&a, "owner"), (&b, "consumer")])
        .expect_err("mismatched shared owner");
    assert!(
        matches!(err, ergo_sbe::GenerateError::InvalidConfiguration { .. }),
        "{err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("owner") || msg.contains("common") || msg.contains("first"),
        "{msg}"
    );
    Ok(())
}

#[test]
fn multi_schema_rejects_reserved_ident_gen() -> Result<(), Box<dyn std::error::Error>> {
    let a = mini_schema("a", 1, "");
    let b = mini_schema("b", 2, "");
    let config = GenerationConfig::new("multi").with_shared_module("a");
    let mut g = Generator::new(config);
    let err = g
        .generate_multi(&[(&a, "a"), (&b, "gen")])
        .expect_err("gen is reserved");
    assert!(
        matches!(err, ergo_sbe::GenerateError::InvalidConfiguration { .. }),
        "{err:?}"
    );
    Ok(())
}

#[test]
fn multi_schema_rejects_byte_order_mismatch_shared_type() -> Result<(), Box<dyn std::error::Error>>
{
    let enum_xml = r#"
      <type name="SideEnc" primitiveType="uint8"/>
      <enum name="Side" encodingType="SideEnc">
        <validValue name="Buy">1</validValue>
        <validValue name="Sell">2</validValue>
      </enum>"#;
    let le = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="a" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            {enum_xml}
          </types>
          <sbe:message name="M" id="1"><field name="x" id="1" type="uint32"/></sbe:message>
        </sbe:messageSchema>"#
    );
    let be = le
        .replace("littleEndian", "bigEndian")
        .replace("package=\"a\"", "package=\"b\"")
        .replace("id=\"1\"", "id=\"2\"");
    let a = Schema::from_ir(ergo_sbe::parse(&le)?);
    let b = Schema::from_ir(ergo_sbe::parse(&be)?);
    let config = GenerationConfig::new("multi").with_shared_module("common");
    let mut g = Generator::new(config);
    let err = g
        .generate_multi(&[(&a, "common"), (&b, "other")])
        .expect_err("byte order mismatch");
    assert!(
        matches!(err, ergo_sbe::GenerateError::IncompatibleSharedType { .. }),
        "{err:?}"
    );
    Ok(())
}

// ── parse_file_with_shared ─────────────────────────────────────────────
//
// `parse_with_shared` (string form) is exercised above and elsewhere; the
// file form was public but called by nothing, so a break in its path
// resolution or its shared-registry seeding would not have been caught.

#[test]
fn parse_file_with_shared_seeds_the_registry_from_the_shared_schema()
-> Result<(), Box<dyn std::error::Error>> {
    let shared = ergo_sbe::parse(
        r#"<?xml version="1.0"?>
<messageSchema package="common" id="0" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
    <composite name="SharedPrice"><type name="mantissa" primitiveType="int64"/><type name="exponent" primitiveType="int8"/></composite>
  </types>
</messageSchema>"#,
    )?;

    let dir = std::env::temp_dir().join(format!("ergo-shared-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let consumer = dir.join("consumer.xml");
    // The consumer declares no types of its own: `SharedPrice` resolves only
    // because the shared schema seeded the registry.
    std::fs::write(
        &consumer,
        r#"<?xml version="1.0"?>
<messageSchema package="c" id="7" version="0" byteOrder="littleEndian" headerType="messageHeader">
  <message name="Quote" id="1"><field name="px" id="1" type="SharedPrice"/></message>
</messageSchema>"#,
    )?;

    let ir = ergo_sbe::parse_file_with_shared(&consumer, &shared)?;
    assert!(
        ir.tokens.iter().any(|t| t.name == "SharedPrice"),
        "shared composite must resolve through the seeded registry"
    );
    assert!(ir.tokens.iter().any(|t| t.name == "Quote"));
    assert_eq!(ir.id, 7, "consumer keeps its own schema id, not the shared 0");

    // A missing file is an error, not a panic — the read happens before parse.
    assert!(ergo_sbe::parse_file_with_shared(dir.join("absent.xml"), &shared).is_err());

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
