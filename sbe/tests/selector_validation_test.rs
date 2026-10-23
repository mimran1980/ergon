//! Conversion selectors fail closed: a selector that cannot apply is a
//! [`GenerateError`] at generation time, never a codec generated silently
//! without the mapping.
//!
//! | Config | Error |
//! |---|---|
//! | `field_path` / `semantic_type` matching no field | `InvalidConversion` |
//! | `with_null_as_option` on anything but `named_type(<enum>)` | `InvalidConfiguration` |
//! | two `with_domain_type` types for one selector | `InvalidConversion` |
//! | a domain selector every match of which a higher-rank selector takes | `InvalidConversion` |
//!
//! Each rejection has a positive twin that still generates.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

use ergo_sbe::{ConversionSelector, GenerateError, GenerationConfig, Generator, Schema, parse};

const SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="sel" id="71" version="0" byteOrder="littleEndian">
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
    <composite name="Decimal">
      <type name="mantissa" primitiveType="int64"/>
      <type name="exponent" primitiveType="int8"/>
    </composite>
    <enum name="Side" encodingType="uint8">
      <validValue name="Buy">1</validValue>
      <validValue name="Sell">2</validValue>
    </enum>
  </types>
  <sbe:message name="NewOrder" id="1">
    <field name="price" id="1" type="Decimal"/>
    <field name="side" id="2" type="Side"/>
    <field name="qty" id="3" type="uint32" semanticType="Qty"/>
    <group name="fills" id="10" dimensionType="groupSizeEncoding">
      <field name="px" id="11" type="Decimal"/>
    </group>
  </sbe:message>
</sbe:messageSchema>"#;

/// A second schema for multi-schema generation: `Cancel.reason` exists only here.
const SECOND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="sel2" id="72" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <sbe:message name="Cancel" id="1">
    <field name="reason" id="1" type="uint8"/>
  </sbe:message>
</sbe:messageSchema>"#;

fn schema(xml: &str) -> Result<Schema, Box<dyn std::error::Error>> {
    Ok(Schema::from_ir(parse(xml)?))
}

fn generate(config: GenerationConfig) -> Result<(), GenerateError> {
    let schema = schema(SCHEMA).map_err(|e| GenerateError::InvalidConversion {
        selector: "fixture".into(),
        reason: e.to_string(),
    })?;
    Generator::new(config).generate(&schema).map(|_| ())
}

fn generate_multi(config: GenerationConfig) -> Result<(), GenerateError> {
    let (a, b) = match (schema(SCHEMA), schema(SECOND)) {
        (Ok(a), Ok(b)) => (a, b),
        _ => {
            return Err(GenerateError::InvalidConversion {
                selector: "fixture".into(),
                reason: "fixture schemas must parse".into(),
            });
        }
    };
    Generator::new(config)
        .generate_multi(&[(&a, "sel_a"), (&b, "sel_b")])
        .map(|_| ())
}

fn config() -> GenerationConfig {
    GenerationConfig::new("sel")
}

/// The `(selector, reason)` of an `InvalidConversion`, or a test failure.
fn invalid_conversion(result: Result<(), GenerateError>) -> (String, String) {
    match result {
        Err(GenerateError::InvalidConversion { selector, reason }) => (selector, reason),
        other => panic!("expected InvalidConversion, got {other:?}"),
    }
}

/// The `(option, value, reason)` of an `InvalidConfiguration`, or a test failure.
fn invalid_configuration(result: Result<(), GenerateError>) -> (String, String, String) {
    match result {
        Err(GenerateError::InvalidConfiguration {
            option,
            value,
            reason,
        }) => (option, value, reason),
        other => panic!("expected InvalidConfiguration, got {other:?}"),
    }
}

// ── selectors that match no field ──────────────────────────────────────

#[test]
fn domain_type_field_path_typo_is_rejected() {
    let (selector, reason) = invalid_conversion(generate(config().with_domain_type(
        ConversionSelector::field_path("NewOrder.prise"),
        "crate::Px",
    )));
    assert_eq!(selector, r#"FieldPath("NewOrder.prise")"#);
    assert_eq!(reason, "matches no field in the schema");
}

#[test]
fn conversion_field_path_typo_is_rejected() {
    let (selector, reason) = invalid_conversion(generate(
        config().with_conversion(ConversionSelector::field_path("NewOrder.quantity")),
    ));
    assert_eq!(selector, r#"FieldPath("NewOrder.quantity")"#);
    assert_eq!(reason, "matches no field in the schema");
}

#[test]
fn group_field_reached_without_its_group_segment_is_rejected() {
    let (selector, reason) = invalid_conversion(generate(
        config().with_conversion(ConversionSelector::field_path("NewOrder.px")),
    ));
    assert_eq!(selector, r#"FieldPath("NewOrder.px")"#);
    assert_eq!(reason, "matches no field in the schema");
}

#[test]
fn semantic_type_no_field_carries_is_rejected() {
    let (selector, reason) = invalid_conversion(generate(config().with_domain_type(
        ConversionSelector::semantic_type("UTCTimestamp"),
        "crate::Ts",
    )));
    assert_eq!(selector, r#"SemanticType("UTCTimestamp")"#);
    assert_eq!(reason, "matches no field in the schema");
}

#[test]
fn selectors_that_match_a_field_still_generate() -> Result<(), GenerateError> {
    generate(
        config()
            .with_conversion(ConversionSelector::field_path("NewOrder.qty"))
            .with_conversion(ConversionSelector::field_path("NewOrder.fills.px"))
            .with_conversion(ConversionSelector::semantic_type("Qty"))
            .with_domain_type(ConversionSelector::named_type("Side"), "crate::Side"),
    )
}

#[test]
fn multi_schema_field_path_resolves_against_every_schema() -> Result<(), GenerateError> {
    generate_multi(config().with_conversion(ConversionSelector::field_path("Cancel.reason")))?;
    let (selector, reason) = invalid_conversion(generate_multi(
        config().with_conversion(ConversionSelector::field_path("Cancel.reasn")),
    ));
    assert_eq!(selector, r#"FieldPath("Cancel.reasn")"#);
    assert_eq!(reason, "matches no field in the schema");
    Ok(())
}

// ── with_null_as_option: only named_type(<enum>) is applied ────────────

#[test]
fn null_as_option_on_a_field_path_is_rejected() {
    let (option, value, reason) = invalid_configuration(generate(
        config().with_null_as_option(ConversionSelector::field_path("NewOrder.side")),
    ));
    assert_eq!(option, "null_as_option");
    assert_eq!(value, r#"FieldPath("NewOrder.side")"#);
    assert_eq!(reason, "only named_type(<enum>) is applied");
}

#[test]
fn null_as_option_on_a_semantic_type_is_rejected() {
    let (option, value, reason) = invalid_configuration(generate(
        config().with_null_as_option(ConversionSelector::semantic_type("Qty")),
    ));
    assert_eq!(option, "null_as_option");
    assert_eq!(value, r#"SemanticType("Qty")"#);
    assert_eq!(reason, "only named_type(<enum>) is applied");
}

#[test]
fn null_as_option_on_an_unknown_type_is_rejected() {
    let (option, value, reason) = invalid_configuration(generate(
        config().with_null_as_option(ConversionSelector::named_type("Sid")),
    ));
    assert_eq!(option, "null_as_option");
    assert_eq!(value, r#"NamedType("Sid")"#);
    assert_eq!(reason, "names no enum in the schema");
}

#[test]
fn null_as_option_on_a_composite_is_rejected() {
    let (option, value, reason) = invalid_configuration(generate(
        config().with_null_as_option(ConversionSelector::named_type("Decimal")),
    ));
    assert_eq!(option, "null_as_option");
    assert_eq!(value, r#"NamedType("Decimal")"#);
    assert_eq!(reason, "names no enum in the schema");
}

#[test]
fn null_as_option_on_an_enum_still_generates() -> Result<(), GenerateError> {
    generate(config().with_null_as_option(ConversionSelector::named_type("Side")))?;
    generate_multi(config().with_null_as_option(ConversionSelector::named_type("Side")))
}

// ── one selector, two domain types ─────────────────────────────────────

#[test]
fn second_domain_type_for_one_selector_is_rejected() {
    let (selector, reason) = invalid_conversion(generate(
        config()
            .with_domain_type(ConversionSelector::named_type("Decimal"), "crate::A")
            .with_domain_type(ConversionSelector::named_type("Decimal"), "crate::B"),
    ));
    assert_eq!(selector, r#"NamedType("Decimal")"#);
    assert_eq!(reason, "mapped to both `crate::A` and `crate::B`");
}

#[test]
fn manual_domain_type_with_another_type_is_rejected() {
    let (selector, reason) = invalid_conversion(generate(
        config()
            .with_domain_type(ConversionSelector::field_path("NewOrder.qty"), "crate::A")
            .with_manual_domain_type(ConversionSelector::field_path("NewOrder.qty"), "crate::B"),
    ));
    assert_eq!(selector, r#"FieldPath("NewOrder.qty")"#);
    assert_eq!(reason, "mapped to both `crate::A` and `crate::B`");
}

#[test]
fn repeating_one_domain_type_still_generates() -> Result<(), GenerateError> {
    generate(
        config()
            .with_domain_type(ConversionSelector::named_type("Decimal"), "crate::A")
            .with_domain_type(ConversionSelector::named_type("Decimal"), "crate::A")
            .with_manual_domain_type(ConversionSelector::named_type("Decimal"), "crate::A"),
    )
}

// ── selectors shadowed on every field they match ───────────────────────

#[test]
fn named_type_overridden_on_every_field_is_rejected() {
    let (selector, reason) = invalid_conversion(generate(
        config()
            .with_domain_type(ConversionSelector::named_type("Decimal"), "crate::A")
            .with_domain_type(ConversionSelector::field_path("NewOrder.price"), "crate::B")
            .with_domain_type(
                ConversionSelector::field_path("NewOrder.fills.px"),
                "crate::B",
            ),
    ));
    assert_eq!(selector, r#"NamedType("Decimal")"#);
    assert_eq!(
        reason,
        "every field it matches takes a higher-precedence selector"
    );
}

#[test]
fn semantic_type_overridden_on_every_field_is_rejected() {
    let (selector, reason) = invalid_conversion(generate(
        config()
            .with_domain_type(ConversionSelector::semantic_type("Qty"), "crate::A")
            .with_domain_type(ConversionSelector::field_path("NewOrder.qty"), "crate::B"),
    ));
    assert_eq!(selector, r#"SemanticType("Qty")"#);
    assert_eq!(
        reason,
        "every field it matches takes a higher-precedence selector"
    );
}

#[test]
fn named_type_overridden_on_some_fields_still_generates() -> Result<(), GenerateError> {
    generate(
        config()
            .with_domain_type(ConversionSelector::named_type("Decimal"), "crate::A")
            .with_domain_type(ConversionSelector::field_path("NewOrder.price"), "crate::B"),
    )
}
