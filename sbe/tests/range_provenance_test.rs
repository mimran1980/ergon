//! Range provenance on the IR: `Encoding::min_value_declared` /
//! `max_value_declared` say whether a bound came from the schema's
//! `minValue` / `maxValue` or is the SBE primitive default that
//! `resolve_schema` fills in. Both bounds are always `Some` after parsing,
//! so the value alone cannot tell them apart.
//!
//! `Encoding::referenced_type` names the `<type>` alias a primitive field was
//! declared with (`None` for a bare primitive), which is what
//! `ConversionSelector::named_type` on an alias matches.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

use ergo_sbe::{Encoding, Ir, Signal, parse, resolve_schema};

const SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="ranges" id="74" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="Pct" primitiveType="uint8" minValue="1" maxValue="99"/>
    <type name="Level" primitiveType="int16" maxValue="5"/>
    <type name="Timestamp" primitiveType="uint64"/>
    <composite name="Band">
      <type name="lo" primitiveType="int32" minValue="-10"/>
      <type name="hi" primitiveType="int32"/>
    </composite>
  </types>
  <sbe:message name="M" id="1">
    <field name="plain" id="1" type="uint64"/>
    <field name="pct" id="2" type="Pct"/>
    <field name="level" id="3" type="Level"/>
    <field name="ts" id="4" type="Timestamp"/>
    <field name="band" id="5" type="Band"/>
  </sbe:message>
</sbe:messageSchema>"#;

fn token<'a>(ir: &'a Ir, signal: Signal, name: &str) -> &'a Encoding {
    &ir.tokens
        .iter()
        .find(|t| t.signal == signal && t.name == name)
        .unwrap_or_else(|| panic!("no {signal:?} token named {name}"))
        .encoding
}

/// `(min, min_declared, max, max_declared)` of a field's `BeginField` token.
fn field_range(ir: &Ir, name: &str) -> (Option<u64>, bool, Option<u64>, bool) {
    let e = token(ir, Signal::BeginField, name);
    (
        e.min_value,
        e.min_value_declared,
        e.max_value,
        e.max_value_declared,
    )
}

#[test]
fn bare_primitive_range_is_the_sbe_default() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(SCHEMA)?;
    // uint64: 0 ..= 2^64 - 2 (2^64 - 1 is the null value).
    assert_eq!(
        field_range(&ir, "plain"),
        (Some(0), false, Some(u64::MAX - 1), false)
    );
    Ok(())
}

#[test]
fn alias_ranges_record_each_declared_bound() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(SCHEMA)?;
    assert_eq!(field_range(&ir, "pct"), (Some(1), true, Some(99), true));
    // Only maxValue declared: min is int16's default -32767, as two's-complement bits.
    assert_eq!(
        field_range(&ir, "level"),
        (Some((-32767_i64) as u64), false, Some(5), true)
    );
    assert_eq!(
        field_range(&ir, "ts"),
        (Some(0), false, Some(u64::MAX - 1), false)
    );
    Ok(())
}

#[test]
fn composite_member_ranges_record_each_declared_bound() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(SCHEMA)?;
    let lo = token(&ir, Signal::BeginField, "lo");
    assert_eq!(
        (lo.min_value_declared, lo.max_value_declared),
        (true, false)
    );
    assert_eq!(lo.min_value, Some((-10_i64) as u64));
    let hi = token(&ir, Signal::BeginField, "hi");
    assert_eq!(
        (hi.min_value_declared, hi.max_value_declared),
        (false, false)
    );
    Ok(())
}

#[test]
fn resolving_again_keeps_the_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let mut ir = parse(SCHEMA)?;
    let before = ir.clone();
    resolve_schema(&mut ir, None)?;
    assert_eq!(ir, before, "resolve_schema is idempotent, flags included");
    assert_eq!(
        field_range(&ir, "plain"),
        (Some(0), false, Some(u64::MAX - 1), false)
    );
    Ok(())
}

#[test]
fn primitive_fields_name_the_alias_they_were_declared_with()
-> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(SCHEMA)?;
    let named = |field: &str| {
        token(&ir, Signal::BeginField, field)
            .referenced_type
            .clone()
    };
    assert_eq!(named("ts").as_deref(), Some("Timestamp"));
    assert_eq!(named("pct").as_deref(), Some("Pct"));
    assert_eq!(named("plain"), None, "a bare primitive has no alias");
    assert_eq!(
        named("band"),
        None,
        "a composite field is named by its own token"
    );
    Ok(())
}
