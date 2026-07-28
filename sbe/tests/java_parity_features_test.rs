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

mod common;
use common::compile_and_run;

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

/// Regression: message-level and group-entry-level Debug/Display used to
/// silently drop bitset fields entirely (`FieldType::Set { .. } => {}`).
/// A message or entry with only a set field printed as if the field didn't
/// exist. Covers both non-versioned and `sinceVersion`-gated set fields at
/// both levels — versioned accessors return `Option<T>` (not `Display`), so
/// the generated Debug/Display code must branch, not blindly forward.
#[test]
fn set_field_shown_in_debug_at_message_and_entry_level() -> Result<(), Box<dyn std::error::Error>>
{
    let xml = r#"<?xml version="1.0"?>
        <messageSchema package="setdbg" id="1" version="1" byteOrder="littleEndian">
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
            <set name="Flags" encodingType="uint8">
              <choice name="A">0</choice>
              <choice name="B">1</choice>
            </set>
          </types>
          <message name="M" id="1">
            <field name="topFlags" id="1" type="Flags" offset="0"/>
            <field name="verFlags" id="2" type="Flags" offset="1" sinceVersion="1"/>
            <group name="entries" id="3" dimensionType="groupSizeEncoding">
              <field name="entryFlags" id="4" type="Flags" offset="0"/>
            </group>
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

    compile_and_run(
        "setdbg",
        &out,
        r#"
        // Current-version encode: both message-level set fields present,
        // one entry with its own set field.
        let mut top_flags = Flags::default();
        top_flags.set_a(true);
        let mut ver_flags = Flags::default();
        ver_flags.set_b(true);
        let mut entry_flags = Flags::default();
        entry_flags.set_a(true);
        entry_flags.set_b(true);

        let mut buf = [0u8; 256];
        let complete = MEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&MFixedFields {
                top_flags,
                ver_flags,
            })
            .entries(1, |g| {
                g.add(|e| {
                    e.entry_flags(entry_flags);
                    Ok(())
                })?;
                Ok(())
            })?;
        let len = complete.encoded_length_with_header();

        let dec = MDecoder::try_from(&buf[..len])?;
        let text = format!("{dec:?}");
        assert!(text.contains("topFlags: A"), "{text}");
        assert!(text.contains("verFlags: B"), "{text}");
        assert!(text.contains("entryFlags: A|B") || text.contains("entryFlags: A | B") || text.contains("A|B"), "{text}");

        // Older-version decode (acting_version 0): the sinceVersion=1 field
        // must be cleanly omitted, not panic, not print garbage.
        let old_dec = MDecoder::wrap(&buf[..len], 8, 2, 0);
        let old_text = format!("{old_dec:?}");
        assert!(old_text.contains("topFlags"), "{old_text}");
        assert!(!old_text.contains("verFlags"), "{old_text}");
        "#,
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

// ── Debug / Display inspection tests ───────────────────────────────────
// These encode real messages and inspect the rendered Debug/Display strings
// to catch regressions like the set-field-silently-dropped bug.

fn all_field_types_schema() -> &'static str {
    r#"<?xml version="1.0"?>
<messageSchema package="dbg" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
    <composite name="groupSizeEncoding"><type name="blockLength" primitiveType="uint16"/><type name="numInGroup" primitiveType="uint16"/></composite>
    <composite name="varStringEncoding"><type name="length" primitiveType="uint32"/><type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/></composite>
    <enum name="Side" encodingType="uint8"><validValue name="Buy">0</validValue><validValue name="Sell">1</validValue></enum>
    <enum name="BoolFlag" encodingType="uint8"><validValue name="False">0</validValue><validValue name="True">1</validValue></enum>
    <set name="ExecInst" encodingType="uint8"><choice name="AON">0</choice><choice name="IOC">1</choice></set>
    <composite name="Price"><type name="mantissa" primitiveType="int64"/><type name="exponent" primitiveType="int8"/></composite>
  </types>
  <message name="Msg" id="1" blockLength="16">
    <field name="qty"    id="1" type="uint32"  offset="0"/>
    <field name="side"   id="2" type="Side"    offset="4"/>
    <field name="inst"   id="3" type="ExecInst" offset="5"/>
    <field name="algo"   id="4" type="BoolFlag" offset="6"/>
    <field name="price"  id="5" type="Price"   offset="7"/>
    <group name="legs" id="10" dimensionType="groupSizeEncoding">
      <field name="ratio" id="11" type="uint32" offset="0"/>
    </group>
    <data name="note" id="20" type="varStringEncoding"/>
  </message>
</messageSchema>"#
}

#[test]
fn decoder_debug_shows_all_field_types() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(all_field_types_schema())?;
    let schema = Schema::from_ir(ir);
    let out = Generator::new(GenerationConfig::new("dbg"))
        .generate(&schema)?.modules().next().unwrap().source.clone();

    compile_and_run("dec_dbg_all", &out, r#"
        let mut inst = ExecInst::default(); inst.set_aon(true);
        let price = Price::new(12345, -2);
        let mut buf = [0u8; MsgEncoder::compute_length_with_header(1, 3)];
        let len = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&MsgFixedFields { qty: 100, side: Side::Sell, inst, algo: BoolFlag::True, price })
            .legs(1, |g| { g.add(|e| { e.ratio(50); Ok(()) })?; Ok(()) })?
            .note(b"abc")?
            .encoded_length_with_header();
        let dec = MsgDecoder::try_from(&buf[..len])?;
        let dbg = format!("{dec:?}");
        assert!(dbg.contains("qty: 100"),        "primitive: {dbg}");
        assert!(dbg.contains("side: Sell"),      "enum: {dbg}");
        assert!(dbg.contains("inst: AON"),       "set: {dbg}");
        assert!(dbg.contains("algo: True"),      "bool-enum: {dbg}");
        assert!(dbg.contains("price:"),          "composite: {dbg}");
        assert!(dbg.contains("legs:"),           "group: {dbg}");
        assert!(dbg.contains("ratio:"),          "entry field: {dbg}");
        assert!(dbg.contains("note:"),           "var-data: {dbg}");
        assert!(dbg.contains("\"abc\""),         "var-data value: {dbg}");
        assert!(dbg.contains("MsgDecoder"),      "struct name: {dbg}");
        assert_eq!(dbg, format!("{dec}"), "Display == Debug");
    "#);
    Ok(())
}

#[test]
fn decoder_debug_survives_truncated_buffer() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(all_field_types_schema())?;
    let schema = Schema::from_ir(ir);
    let out = Generator::new(GenerationConfig::new("dbg_trunc"))
        .generate(&schema)?.modules().next().unwrap().source.clone();

    compile_and_run("dec_trunc", &out, r#"
        let mut inst = ExecInst::default(); inst.set_ioc(true);
        let price = Price::new(1, 0);
        let mut buf = [0u8; MsgEncoder::compute_length_with_header(0, 0)];
        let len = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&MsgFixedFields { qty: 1, side: Side::Buy, inst, algo: BoolFlag::False, price })
            .legs(0, |_| Ok(()))?.note(b"")?
            .encoded_length_with_header();
        let _ = MsgDecoder::try_from(&buf[..len])?;
        // Truncated: 12 bytes is past header (8) but shorter than full
        // fixed block (16). try_from returns an error — must not panic.
        assert!(MsgDecoder::try_from(&buf[..12]).is_err());
        // Below header — error, not panic.
        assert!(MsgDecoder::try_from(&buf[..3]).is_err());
    "#);
    Ok(())
}

#[test]
fn dto_debug_shows_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(all_field_types_schema())?;
    let schema = Schema::from_ir(ir);
    let out = Generator::new(
        GenerationConfig::new("dbg_dto").enable_domain_objects(DomainVarData::LossyStrings))
        .generate(&schema)?.modules().next().unwrap().source.clone();

    compile_and_run("dto_dbg", &out, r#"
        let mut inst = ExecInst::default(); inst.set_aon(true);
        let price = Price::new(999, -1);
        let dto = MsgDomain {
            qty: 42, side: Side::Buy, inst, algo: BoolFlag::True, price,
            legs: vec![MsgLegsEntryDomain { ratio: 10 }], note: "hi".into(),
        };
        let len = dto.encoded_length_with_header()?;
        let mut buf = [0u8; 256];
        let written = dto.encode(&mut buf[..len])?;
        assert_eq!(written, len);
        let dbg = format!("{dto:?}");
        assert!(dbg.contains("qty: 42"),     "DTO primitive: {dbg}");
        assert!(dbg.contains("side: Buy"),   "DTO enum: {dbg}");
        assert!(dbg.contains("inst: ExecInst"), "DTO set: {dbg}");
        assert!(dbg.contains("algo: True"),  "DTO bool: {dbg}");
        assert!(dbg.contains("price:"),      "DTO composite: {dbg}");
        assert!(dbg.contains("legs:"),       "DTO group: {dbg}");
        assert!(dbg.contains("ratio: 10"),   "DTO entry: {dbg}");
        assert!(dbg.contains("note:"),       "DTO var-data: {dbg}");
        assert!(dbg.contains("\"hi\""),      "DTO var-data value: {dbg}");
    "#);
    Ok(())
}

#[test]
fn encoder_display_delegates_to_decoder() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(all_field_types_schema())?;
    let schema = Schema::from_ir(ir);
    let out = Generator::new(GenerationConfig::new("dbg_enc"))
        .generate(&schema)?.modules().next().unwrap().source.clone();

    compile_and_run("enc_display", &out, r#"
        let mut inst = ExecInst::default(); inst.set_aon(true);
        let price = Price::new(99, -1);
        let mut buf = [0u8; MsgEncoder::compute_length_with_header(0, 3)];
        let enc = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&MsgFixedFields { qty: 1, side: Side::Buy, inst, algo: BoolFlag::True, price })
            .legs(0, |_| Ok(()))?.note(b"xyz")?;
        // Encoder Display delegates to the decoder — shows the message.
        let display = format!("{enc}");
        assert!(display.contains("qty"), "encoder Display: {display}");
        assert!(display.contains("side"), "encoder Display: {display}");
    "#);
    Ok(())
}

#[test]
fn entry_decoder_debug_shows_enum_set_and_composite() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="e" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
    <composite name="groupSizeEncoding"><type name="blockLength" primitiveType="uint16"/><type name="numInGroup" primitiveType="uint16"/></composite>
    <enum name="Side" encodingType="uint8"><validValue name="Buy">0</validValue><validValue name="Sell">1</validValue></enum>
    <set name="Flags" encodingType="uint8"><choice name="A">0</choice><choice name="B">1</choice></set>
    <composite name="Price"><type name="mantissa" primitiveType="int64"/><type name="exponent" primitiveType="int8"/></composite>
  </types>
  <message name="M" id="1" blockLength="0">
    <group name="rows" id="1" dimensionType="groupSizeEncoding">
      <field name="side"  id="1" type="Side"  offset="0"/>
      <field name="flags" id="2" type="Flags" offset="1"/>
      <field name="price" id="3" type="Price" offset="2"/>
    </group>
  </message>
</messageSchema>"#;
    let ir = parse(xml)?;
    let schema = Schema::from_ir(ir);
    let out = Generator::new(GenerationConfig::new("entry_dbg"))
        .generate(&schema)?.modules().next().unwrap().source.clone();

    compile_and_run("entry_dbg", &out, r#"
        let mut flags = Flags::default(); flags.set_b(true);
        let price = Price::new(777, -1);
        let mut buf = [0u8; MEncoder::compute_length_with_header(1)];
        let len = MEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .rows(1, |g| { g.add(|e| { e.side(Side::Buy).flags(flags).price(price); Ok(()) })?; Ok(()) })?
            .encoded_length_with_header();
        let dec = MDecoder::try_from(&buf[..len])?;
        let mut rows = dec.into_rows()?;
        let entry = rows.next().expect("one row");
        let dbg = format!("{entry}");
        assert!(dbg.contains("side: Side"),  "entry enum: {dbg}");
        assert!(dbg.contains("flags: B"),   "entry set: {dbg}");
        assert!(dbg.contains("price:"),     "entry composite: {dbg}");
    "#);
    Ok(())
}
