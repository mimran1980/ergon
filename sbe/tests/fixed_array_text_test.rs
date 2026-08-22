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
