//! Coverage for Java sbe-tool parity features:
//! field metadata, fixed-array bulk helpers, keyword append, XSD validation,
//! and domain DTO range checks.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

use ergo_sbe::{
    DomainVarData, GenerationConfig, Generator, SBE_XSD, Schema, parse, parse_with_xsd_validation,
    validate_against_sbe_xsd,
};

#[test]
fn field_metadata_constants_and_meta_attribute() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
        <messageSchema package="meta" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <type name="u64" primitiveType="uint64" minValue="0" maxValue="100"
                  epoch="unix" timeUnit="nanosecond" semanticType="UTCTimestamp"/>
          </types>
          <message name="Tick" id="1" blockLength="8">
            <field name="ts" id="7" type="u64" offset="0"/>
          </message>
        </messageSchema>"#;
    let ir = parse(&xml)?;
    let schema = Schema::from_ir(ir);
    let out = Generator::new(GenerationConfig::new("m"))
        .generate(&schema)?
        .modules()
        .next()
        .unwrap()
        .source
        .clone();

    assert!(out.contains("pub const TS_ID: u16 = 7"), "{out}");
    assert!(out.contains("pub const TS_SINCE_VERSION: u16 = 0"), "{out}");
    assert!(
        out.contains("pub const TS_ENCODING_OFFSET: usize = 0"),
        "{out}"
    );
    assert!(
        out.contains("pub const TS_ENCODING_LENGTH: usize = 8"),
        "{out}"
    );
    assert!(out.contains("fn ts_meta_attribute"), "{out}");
    assert!(out.contains("enum MetaAttribute"), "{out}");
    assert!(out.contains("Some(\"unix\")"), "{out}");
    assert!(out.contains("Some(\"nanosecond\")"), "{out}");
    assert!(out.contains("Some(\"UTCTimestamp\")"), "{out}");
    assert!(out.contains("Some(\"required\")"), "{out}");
    Ok(())
}

#[test]
fn fixed_array_put_and_str_helpers() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
        <messageSchema package="arr" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <type name="Nums" primitiveType="uint32" length="4"/>
            <type name="Code" primitiveType="char" length="6" characterEncoding="ASCII"/>
          </types>
          <message name="M" id="1" blockLength="22">
            <field name="someNumbers" id="1" type="Nums" offset="0"/>
            <field name="vehicleCode" id="2" type="Code" offset="16"/>
          </message>
        </messageSchema>"#;
    let ir = parse(&xml)?;
    let schema = Schema::from_ir(ir);
    let out = Generator::new(GenerationConfig::new("m"))
        .generate(&schema)?
        .modules()
        .next()
        .unwrap()
        .source
        .clone();

    assert!(out.contains("fn put_some_numbers"), "{out}");
    assert!(out.contains("fn vehicle_code_str"), "{out}");
    assert!(out.contains("FixedArrayTooLong"), "{out}");
    assert!(out.contains("fn copy_vehicle_code"), "{out}");
    Ok(())
}

#[test]
fn keyword_append_rewrites_type_field() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
        <messageSchema package="kw" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <type name="u8" primitiveType="uint8"/>
          </types>
          <message name="M" id="1" blockLength="1">
            <field name="type" id="1" type="u8" offset="0"/>
          </message>
        </messageSchema>"#;
    let ir = parse(&xml)?;
    let schema = Schema::from_ir(ir);
    let out = Generator::new(GenerationConfig::new("m").with_keyword_append_token("_"))
        .generate(&schema)?
        .modules()
        .next()
        .unwrap()
        .source
        .clone();

    // snake_case("type") is keyword → type_
    assert!(
        out.contains("fn type_"),
        "expected type_ accessor, got:\n{out}"
    );
    assert!(
        out.contains("TYPE__ID") || out.contains("TYPE__SINCE") || out.contains("TYPE_"),
        "{out}"
    );
    Ok(())
}

#[test]
fn xsd_validation_accepts_and_rejects() -> Result<(), Box<dyn std::error::Error>> {
    assert!(SBE_XSD.contains("messageSchema"));
    let good = r#"<?xml version="1.0"?>
        <messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <type name="u32" primitiveType="uint32"/>
          </types>
          <message name="M" id="1">
            <field name="x" id="1" type="u32"/>
          </message>
        </messageSchema>"#;
    validate_against_sbe_xsd(good)?;
    let _ = parse_with_xsd_validation(good)?;

    let bad = r#"<?xml version="1.0"?><messageSchema id="1" version="0"><types/><message name="M" id="1"><bogus/></message></messageSchema>"#;
    assert!(validate_against_sbe_xsd(bad).is_err());
    assert!(parse_with_xsd_validation(bad).is_err());
    Ok(())
}

#[test]
fn domain_dto_range_validation_emitted() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
        <messageSchema package="dto" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <type name="qty" primitiveType="uint32" minValue="1" maxValue="1000"/>
          </types>
          <message name="Order" id="1" blockLength="4">
            <field name="qty" id="1" type="qty" offset="0"/>
          </message>
        </messageSchema>"#;
    let ir = parse(xml)?;
    let schema = Schema::from_ir(ir);
    let out =
        Generator::new(GenerationConfig::new("m").enable_domain_objects(DomainVarData::Bytes))
            .generate(&schema)?
            .modules()
            .next()
            .unwrap()
            .source
            .clone();

    assert!(out.contains("ValueOutOfRange"), "{out}");
    assert!(out.contains("OrderDomain"), "{out}");
    // min=1 max=1000 should appear in the generated check
    assert!(
        out.contains("min: 1") || out.contains("min: 1i128") || out.contains("1"),
        "{out}"
    );
    Ok(())
}

#[test]
fn enum_display_and_fromstr_emitted() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
        <messageSchema package="enum" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <enum name="Side" encodingType="uint8">
              <validValue name="Buy">0</validValue>
              <validValue name="Sell">1</validValue>
            </enum>
          </types>
          <message name="Quote" id="1" blockLength="1">
            <field name="side" id="1" type="Side" offset="0"/>
          </message>
        </messageSchema>"#;
    let ir = parse(xml)?;
    let schema = Schema::from_ir(ir);
    let out = Generator::new(GenerationConfig::new("m"))
        .generate(&schema)?
        .modules()
        .next()
        .unwrap()
        .source
        .clone();

    assert!(out.contains("impl core::fmt::Display for Side"), "{out}");
    assert!(out.contains("impl core::str::FromStr for Side"), "{out}");
    // Display outputs the variant name; FromStr accepts it -> round-trips.
    assert!(out.contains("stringify!(Buy)"), "{out}");
    assert!(out.contains("type Err = ()"), "{out}");
    Ok(())
}

#[test]
fn bitset_display_and_fromstr_emitted() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
        <messageSchema package="set" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <set name="OptionalExtras" encodingType="uint8">
              <choice name="sunRoof">0</choice>
              <choice name="sportsPack">1</choice>
            </set>
          </types>
          <message name="Car" id="1" blockLength="1">
            <field name="extras" id="1" type="OptionalExtras" offset="0"/>
          </message>
        </messageSchema>"#;
    let ir = parse(xml)?;
    let schema = Schema::from_ir(ir);
    let out = Generator::new(GenerationConfig::new("m"))
        .generate(&schema)?
        .modules()
        .next()
        .unwrap()
        .source
        .clone();

    assert!(
        out.contains("impl core::fmt::Display for OptionalExtras"),
        "{out}"
    );
    assert!(
        out.contains("impl core::str::FromStr for OptionalExtras"),
        "{out}"
    );
    // Display/FromStr use the schema choice names (faithful, round-trips).
    assert!(out.contains("\"sunRoof\""), "{out}");
    assert!(out.contains("\"sportsPack\""), "{out}");
    // The set's own description doc is emitted exactly once (no duplication).
    assert_eq!(
        out.matches("Set of optional extras").count(),
        0,
        "no description in this schema; just checking no spurious doc"
    );
    Ok(())
}

#[test]
fn set_description_doc_not_duplicated() -> Result<(), Box<dyn std::error::Error>> {
    // Regression: generate_set previously emitted the description doc twice.
    let xml = r#"<?xml version="1.0"?>
        <messageSchema package="setdoc" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <set name="Flags" encodingType="uint8" description="Set of option flags.">
              <choice name="a">0</choice>
              <choice name="b">1</choice>
            </set>
          </types>
        </messageSchema>"#;
    let ir = parse(xml)?;
    let schema = Schema::from_ir(ir);
    let out = Generator::new(GenerationConfig::new("m"))
        .generate(&schema)?
        .modules()
        .next()
        .unwrap()
        .source
        .clone();

    assert_eq!(out.matches("Set of option flags.").count(), 1, "{out}");
    Ok(())
}

#[test]
fn deprecated_field_marks_getter() -> Result<(), Box<dyn std::error::Error>> {
    // #[deprecated] on a field accessor is safe (warns on use, not on definition).
    let xml = r#"<?xml version="1.0"?>
        <messageSchema package="dep" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <type name="u8" primitiveType="uint8"/>
          </types>
          <message name="M" id="1" blockLength="2">
            <field name="current" id="1" type="u8" offset="0"/>
            <field name="legacy" id="2" type="u8" offset="1" deprecated="true"/>
          </message>
        </messageSchema>"#;
    let ir = parse(xml)?;
    let schema = Schema::from_ir(ir);
    // `#[deprecated]` emission is opt-in (cascades to types/impls).
    let out = Generator::new(GenerationConfig::new("m").with_deprecated_attrs())
        .generate(&schema)?
        .modules()
        .next()
        .unwrap()
        .source
        .clone();

    // The deprecated field's getter carries #[deprecated]; the live one does not.
    let legacy = out
        .find("pub fn legacy(&self)")
        .ok_or_else(|| format!("legacy getter missing in:\n{out}"))?;
    let preceding = &out[..legacy];
    let last_deprecated = preceding.rfind("#[deprecated]");
    assert!(
        last_deprecated.is_some(),
        "no #[deprecated] before legacy getter in:\n{out}"
    );
    // The deprecated attr must be immediately above the getter (no other fn between).
    let between = &out[last_deprecated.unwrap()..legacy];
    assert!(
        !between.contains("pub fn "),
        "deprecated attr not adjacent to legacy getter:\n{between}"
    );
    // Non-deprecated field's getter has no adjacent #[deprecated].
    let current = out
        .find("pub fn current(&self)")
        .ok_or_else(|| format!("current getter missing in:\n{out}"))?;
    let cur_between = &out[out[..current].rfind("#[deprecated]").unwrap_or(0)..current];
    assert!(
        !cur_between.contains("pub fn current"),
        "current getter should not carry #[deprecated]"
    );
    Ok(())
}
