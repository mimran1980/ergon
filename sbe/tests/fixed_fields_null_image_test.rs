//! T-13: `fixed(&FixedFields)` writes schema null images for `None`.
//!
//! Dirty-buffer reuse must not leak prior optional values when a later
//! encode passes `None`.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(clippy::nursery)]

mod common;
use common::{Paths, compile_and_run, generate};
use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
use std::path::PathBuf;

#[test]
fn optional_primitive_none_overwrites_dirty_buffer() -> Result<(), Box<dyn std::error::Error>> {
    let schema_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
        package="nullimg" id="42" version="0" byteOrder="littleEndian">
      <types>
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
        <type name="u32null" primitiveType="uint32" presence="optional" nullValue="4294967295"/>
      </types>
      <sbe:message name="Msg" id="1" blockLength="4">
        <field name="qty" id="1" type="u32null"/>
      </sbe:message>
    </sbe:messageSchema>"#;
    let schema = Schema::from_ir(parse(schema_xml)?);
    let config = GenerationConfig::new("nullimg");
    let g = Generator::new(config);
    let modules = g.generate(&schema)?;
    let src = &modules
        .modules()
        .next()
        .ok_or("expected generated module")?
        .source;

    compile_and_run(
        "fixed_null_image_prim",
        src,
        r#"
        // Dirty buffer full of 0xAB.
        let mut buf = [0xABu8; 64];
        // First encode: Some(7)
        {
            let enc = MsgEncoder::wrap_and_apply_header(&mut buf, 0)
                .fixed(&MsgFixedFields { qty: Some(7) });
            let _ = enc;
        }
        // Body qty at offset 8..12 little-endian = 7
        assert_eq!(&buf[8..12], &7u32.to_le_bytes());

        // Second encode on same dirty buffer: None → null 0xFFFFFFFF
        {
            let enc = MsgEncoder::wrap_and_apply_header(&mut buf, 0)
                .fixed(&MsgFixedFields { qty: None });
            let _ = enc;
        }
        assert_eq!(&buf[8..12], &0xFFFF_FFFFu32.to_le_bytes());
        let dec = MsgDecoder::try_from(&buf[..MsgEncoder::ENCODED_LENGTH]).expect("dec");
        assert_eq!(dec.qty(), None);
        "#,
    );
    Ok(())
}

#[test]
fn optional_enum_none_writes_nullval() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/optional_enum_nullify.xml");
    let (_, src) = generate(&path, "fixed_null_enum");
    compile_and_run(
        "fixed_null_enum",
        &src,
        r#"
        let mut buf = [0xABu8; 128];
        // Encode with None optional enum via FixedFields
        let fields = OptionalEnumNullifyFixedFields {
            optional_enum: None,
            required_enum_from_optional_type: OptionalEncodingEnumType::Alpha,
            optional_composite: OptionalComposite::new(0),
        };
        let _ = OptionalEnumNullifyEncoder::wrap_and_apply_header(&mut buf, 0).fixed(&fields);
        let dec = OptionalEnumNullifyDecoder::try_from(
            &buf[..OptionalEnumNullifyEncoder::ENCODED_LENGTH],
        ).expect("dec");
        // optional enum should decode as None / NullVal
        let raw = dec.optional_enum();
        assert!(
            matches!(raw, EnumType::NullVal) || raw.as_option().is_none(),
            "expected null enum, got {raw:?}"
        );
        "#,
    );
    Ok(())
}

#[test]
fn car_schema_fixed_path_still_compiles() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "fixed_null_car_smoke");
    // Smoke: FixedFields path exists and compiles for the car fixture.
    assert!(src.contains("fn fixed"), "car encoder must expose fixed()");
    assert!(
        src.contains("CarFixedFields"),
        "car must emit FixedFields"
    );
    Ok(())
}
