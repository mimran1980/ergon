//! T-11: `*_str` is encoding-gated; ASCII rejects non-ASCII before write.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

mod common;
use common::{Paths, compile_and_run, compile_fails_with_diagnostics, generate};
use ergo_sbe::{GenerationConfig, Generator, Schema, parse_file};

fn generate_demo() -> Result<String, Box<dyn std::error::Error>> {
    let ir = parse_file(&Paths::fixed_array_schema())?;
    let schema = Schema::from_ir(ir);
    let (modules, _) = Generator::new(GenerationConfig::new("arr_txt"))
        .generate(&schema)?
        .into_parts();
    Ok(modules.into_iter().next().ok_or("no module")?.source)
}

#[test]
fn fixed_text_helpers_yield_to_schema_fields_at_every_depth()
-> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="collision" id="1" version="0" byteOrder="littleEndian">
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
        <type name="Code" primitiveType="char" length="4"/>
    </types>
    <sbe:message name="Collision" id="1">
        <field name="code" id="1" type="Code"/>
        <field name="codeAsStr" id="2" type="uint32"/>
        <group name="entries" id="3" dimensionType="groupSizeEncoding">
            <field name="code" id="4" type="Code"/>
            <field name="codeAsStr" id="5" type="uint32"/>
            <group name="levels" id="6" dimensionType="groupSizeEncoding">
                <field name="code" id="7" type="Code"/>
                <field name="codeAsStr" id="8" type="uint32"/>
            </group>
        </group>
    </sbe:message>
</sbe:messageSchema>"#;
    let mut ir = ergo_sbe::parse(xml)?;
    ergo_sbe::resolve_schema(&mut ir, Some(xml))?;
    let (modules, _) = Generator::new(GenerationConfig::new("collision"))
        .generate(&Schema::from_ir(ir))?
        .into_parts();
    let src = modules.into_iter().next().ok_or("no module")?.source;
    compile_and_run(
        "fixed_text_collision",
        &src,
        r#"
        // Header + 8-byte body + group dimensions + 8-byte entry + nested dimensions + 8-byte entry.
        let mut frame = [0u8; 40];
        frame[..8].copy_from_slice(&[8, 0, 1, 0, 1, 0, 0, 0]);
        frame[8..12].copy_from_slice(b"ROOT");
        frame[12..16].copy_from_slice(&11u32.to_le_bytes());
        frame[16..20].copy_from_slice(&[8, 0, 1, 0]);
        frame[20..24].copy_from_slice(b"GRUP");
        frame[24..28].copy_from_slice(&22u32.to_le_bytes());
        frame[28..32].copy_from_slice(&[8, 0, 1, 0]);
        frame[32..36].copy_from_slice(b"LEAF");
        frame[36..40].copy_from_slice(&33u32.to_le_bytes());
        let message = CollisionDecoder::try_from(frame.as_slice())?;
        assert_eq!(message.code(), *b"ROOT");
        assert_eq!(message.code_as_str(), 11u32);
        let mut entries = message.entries()?;
        let entry = entries.next().ok_or("missing entry")??;
        assert_eq!(entry.code(), *b"GRUP");
        assert_eq!(entry.code_as_str(), 22u32);
        let mut levels = entry.levels()?;
        let level = levels.next().ok_or("missing level")?;
        assert_eq!(level.code(), *b"LEAF");
        assert_eq!(level.code_as_str(), 33u32);
        "#,
    );
    Ok(())
}

#[test]
fn a_group_char_field_reads_and_writes_text() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="g" id="1" version="0" byteOrder="littleEndian">
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
        <type name="Venue" primitiveType="char" length="12"/>
    </types>
    <sbe:message name="Book" id="1">
        <group name="bids" id="2" dimensionType="groupSizeEncoding">
            <field name="venue" id="1" type="Venue"/>
        </group>
    </sbe:message>
</sbe:messageSchema>"#;
    let mut ir = ergo_sbe::parse(xml)?;
    ergo_sbe::resolve_schema(&mut ir, Some(xml))?;
    let schema = Schema::from_ir(ir);
    let (modules, _) = Generator::new(GenerationConfig::new("grp_txt"))
        .generate(&schema)?
        .into_parts();
    let src = modules.into_iter().next().ok_or("no module")?.source;
    assert!(src.contains("fn venue_str"), "group encoder must take &str");
    assert!(
        src.contains("fn venue_as_str"),
        "group decoder must return &str"
    );
    compile_and_run(
        "group_char_text",
        &src,
        r#"
        let mut buf = [0u8; BookEncoder::compute_length_with_header(1)];
        let written = BookEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&BookFixedFields {})
            .bids(1, |g| {
                g.add_checked(|mut entry| {
                    entry.venue_str("BINANCE")?;
                    Ok(entry.complete())
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        assert_eq!(written, buf.len());
        let msg = AnyMessage::decode(&buf, 0)?;
        let AnyMessage::Book(book) = msg else { panic!("book") };
        let mut bids = book.bids()?;
        let entry = bids.next().unwrap();
        assert_eq!(entry.venue_as_str()?, "BINANCE");
        "#,
    );
    Ok(())
}

#[test]
fn str_setters_present_only_for_supported_text_encodings() -> Result<(), Box<dyn std::error::Error>>
{
    let src = generate_demo()?;
    for present in [
        "fn fixed16_char_str",
        "fn fixed16_ascii_char_str",
        "fn fixed16_utf8_char_str",
        "fn fixed16_ascii_u8_str",
        "fn fixed16_utf8_u8_str",
    ] {
        assert!(src.contains(present), "missing {present}");
    }
    for absent in [
        "fn fixed16_gb18030_char_str",
        "fn fixed16_u8_str",
        "fn fixed16_gb18030_u8_str",
        "fn fixed16i8_str",
        "fn fixed16i16_str",
    ] {
        assert!(!src.contains(absent), "must not emit {absent}");
    }
    assert!(src.contains("InvalidAscii"), "{src}");
    Ok(())
}

#[test]
fn ascii_str_rejects_non_ascii_before_mutating() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate_demo()?;
    compile_and_run(
        "arr_ascii_reject",
        &src,
        r#"
        let mut buf = [0xFFu8; DemoEncoder::compute_length_with_header()];
        {
            let mut writer = DemoEncoder::try_wrap_and_apply_header(&mut buf, 0)
                .unwrap()
                .raw_fixed();
            match writer.fixed16_ascii_char_str("café") {
                Err(sbe_rt::EncodeError::InvalidAscii { field }) => {
                    assert_eq!(field, "fixed16AsciiChar");
                }
                Ok(_) => panic!("non-ASCII ASCII field must fail"),
                Err(other) => panic!("expected InvalidAscii, got {other:?}"),
            }
        }
        let dec = DemoDecoder::try_from(buf.as_slice())?;
        assert_eq!(
            dec.fixed16_ascii_char(),
            [0xFFu8; 16],
            "InvalidAscii must not mutate the destination field"
        );
        "#,
    );
    Ok(())
}

#[test]
fn utf8_str_accepts_multibyte_within_capacity() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate_demo()?;
    compile_and_run(
        "arr_utf8_ok",
        &src,
        r#"
        let mut buf = [0u8; DemoEncoder::compute_length_with_header()];
        DemoEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .raw_fixed()
            .fixed16_utf8_char_str("héllo")?;
        let dec = DemoDecoder::try_from(buf.as_slice())?;
        let bytes = dec.fixed16_utf8_char();
        assert_eq!(&bytes[..6], "héllo".as_bytes());
        assert!(bytes[6..].iter().all(|b| *b == 0));
        "#,
    );
    Ok(())
}

#[test]
fn raw_setters_accept_every_byte_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate_demo()?;
    compile_and_run(
        "arr_raw_bytes",
        &src,
        r#"
        let mut buf = [0u8; DemoEncoder::compute_length_with_header()];
        let raw = [0xFFu8; 16];
        DemoEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .raw_fixed()
            .fixed16_gb18030_char(raw)
            .fixed16_u8(raw);
        let dec = DemoDecoder::try_from(buf.as_slice())?;
        assert_eq!(dec.fixed16_gb18030_char(), raw);
        assert_eq!(dec.fixed16_u8(), raw);
        "#,
    );
    Ok(())
}

#[test]
fn unsupported_encoding_str_does_not_compile() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate_demo()?;
    compile_fails_with_diagnostics(
        "arr_no_gb_str",
        &src,
        r#"
        let mut buf = [0u8; DemoEncoder::compute_length_with_header()];
        DemoEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .raw_fixed()
            .fixed16_gb18030_char_str("x")
            .unwrap();
        "#,
        &["fixed16_gb18030_char_str"],
    );
    Ok(())
}

#[test]
fn field_name_ending_str_keeps_raw_and_suffixed_text_setter()
-> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "arr_str_name");
    // Car.vehicleCode is the reserved-name check: *_str is a suffix, not a rename.
    assert!(src.contains("fn vehicle_code(") || src.contains("fn vehicle_code_str("));
    let xml = r#"<?xml version="1.0"?>
        <messageSchema package="n" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <type name="Code" primitiveType="char" length="4" characterEncoding="ASCII"/>
          </types>
          <message name="M" id="1">
            <field name="code_str" id="1" type="Code"/>
          </message>
        </messageSchema>"#;
    let ir = ergo_sbe::parse(xml)?;
    let schema = Schema::from_ir(ir);
    let (modules, _) = Generator::new(GenerationConfig::new("nstr"))
        .generate(&schema)?
        .into_parts();
    let out = modules.into_iter().next().unwrap().source;
    assert!(
        out.contains("fn code_str("),
        "raw setter must keep the field name"
    );
    assert!(
        out.contains("fn code_str_str("),
        "text convenience must suffix _str even when the field already ends in _str"
    );
    Ok(())
}
