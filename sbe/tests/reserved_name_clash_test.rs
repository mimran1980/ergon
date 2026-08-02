//! Regression test for reserved-method / field-name collisions.
//!
//! A flat message may legally declare fields whose names collide with reserved
//! decoder/encoder methods (`remaining`, `buffer`). Both the **array**
//! accessor path and the **optional primitive** accessor path must route the
//! field name through `resolve_field_ident` so it becomes `{name}_field`,
//! leaving the reserved method intact. The earlier substring-only test used a
//! single scalar field and did not exercise either path — a schema with an
//! optional `remaining` and an array `buffer` produced duplicate methods
//! and a `Display` impl referencing a non-existent `remaining_field`, so the
//! generated crate failed to compile. This test compiles and runs it.

#![allow(clippy::expect_used)]

mod common;
use common::compile_and_run;

use ergo_sbe::{GenerationConfig, Generator, Schema, parse};

const SCHEMA_XML: &str = r#"<messageSchema package="clash" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="Quad" primitiveType="uint32" length="4"/>
  </types>
  <message name="Msg" id="1" blockLength="21">
    <field name="remaining" id="1" type="uint32" presence="optional" offset="0"/>
    <field name="buffer" id="2" type="Quad" offset="4"/>
    <field name="normal" id="3" type="uint8" offset="20"/>
  </message>
</messageSchema>"#;

#[test]
fn optional_and_array_fields_named_after_reserved_methods_compile()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(SCHEMA_XML)?);
    let src = Generator::new(GenerationConfig::new("clash"))
        .generate(&schema)?
        .modules()
        .next()
        .expect("one module")
        .source
        .clone();

    // Both field accessors are renamed; the reserved methods survive.
    assert!(
        src.contains("fn remaining_field(&self) -> Option<u32>"),
        "optional field 'remaining' must be renamed to remaining_field"
    );
    assert!(
        src.contains("fn buffer_field(&self) -> [u32; 4]"),
        "array field 'buffer' must be renamed to buffer_field"
    );
    // The reserved decoder methods still exist and are not shadowed.
    assert!(
        src.contains("fn remaining(&self)"),
        "reserved decoder method remaining() must remain"
    );
    assert!(
        src.contains("fn buffer(&self)"),
        "reserved decoder method buffer() must remain"
    );

    // The real proof: the generated crate compiles and every path works,
    // including the Display impl that references the renamed accessors.
    compile_and_run(
        "clash",
        &src,
        r#"
        let mut buf = [0u8; MsgEncoder::compute_length_with_header()];
        let len = MsgEncoder::wrap_and_apply_header(&mut buf, 0).unwrap()
            .fixed(&MsgFixedFields {
                remaining: Some(7),
                buffer: [10, 20, 30, 40],
                normal: 9,
            })
            .encoded_length_with_header();

        let dec = MsgDecoder::try_from(&buf[..len]).expect("decode");
        // Renamed field accessors.
        assert_eq!(dec.remaining_field(), Some(7));
        assert_eq!(dec.buffer_field(), [10, 20, 30, 40]);
        assert_eq!(dec.normal(), 9);
        // Reserved methods still available and distinct from the fields.
        let _tail: &[u8] = dec.remaining();
        let _all: &[u8] = dec.buffer();
        // Display/Debug impl references the renamed accessors — must format.
        let shown = format!("{dec:?}");
        assert!(shown.contains("remaining"));
        "#,
    );

    Ok(())
}

/// The encoder side has its own reserved list. A fixed message with fields
/// named after inherent *encoder* methods must rename the setters, otherwise
/// they collide and the generated crate fails to compile. This covers every
/// name in `ENCODER_RESERVED` that is emitted as an inherent method on the
/// fixed-message encoder struct.
const ENCODER_CLASH_SCHEMA: &str = r#"<messageSchema package="eclash" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Msg" id="1" blockLength="18">
    <field name="encodedLength" id="1" type="uint32" offset="0"/>
    <field name="encodedLengthWithHeader" id="2" type="uint16" offset="4"/>
    <field name="asBodyBytes" id="3" type="uint16" offset="6"/>
    <field name="asBytesWithHeader" id="4" type="uint16" offset="8"/>
    <field name="wrapAndApplyHeader" id="5" type="uint16" offset="10"/>
    <field name="fixed" id="6" type="uint16" offset="12"/>
    <field name="rawFixed" id="7" type="uint16" offset="14"/>
    <field name="bufferTooShort" id="8" type="uint16" offset="16"/>
  </message>
</messageSchema>"#;

#[test]
fn fields_named_after_encoder_methods_compile() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(ENCODER_CLASH_SCHEMA)?);
    let src = Generator::new(GenerationConfig::new("eclash"))
        .generate(&schema)?
        .modules()
        .next()
        .expect("one module")
        .source
        .clone();

    // Encoder reserved names always take `_field` on the encoder.
    // Decoder reserved now includes wrap/decode (0.1.10 dual-lane), so those
    // names are also `_field` on the decoder.
    for renamed in [
        "encoded_length_field",
        "encoded_length_with_header_field",
        "as_body_bytes_field",
        "as_bytes_with_header_field",
        "wrap_and_apply_header_field",
        "fixed_field",
        "raw_fixed_field",
        "buffer_too_short_field",
    ] {
        assert!(
            src.contains(&format!("fn {renamed}")),
            "expected renamed accessor fn {renamed}"
        );
    }

    // The inherent encoder methods still exist and are distinct.
    for inherent in [
        "fn encoded_length(",
        "fn encoded_length_with_header(",
        "fn as_body_bytes(",
        "fn as_bytes_with_header(",
        "fn wrap_and_apply_header(",
        "fn fixed(",
        "fn raw_fixed(",
        "fn buffer_too_short(",
    ] {
        assert!(
            src.contains(inherent),
            "inherent encoder method {inherent} must not be shadowed"
        );
    }

    compile_and_run(
        "eclash",
        &src,
        r#"
        let mut buf = [0u8; MsgEncoder::compute_length_with_header()];
        let n = MsgEncoder::wrap_and_apply_header(&mut buf, 0).unwrap()
            .fixed(&MsgFixedFields {
                encoded_length: 11,
                encoded_length_with_header: 22,
                as_body_bytes: 33,
                as_bytes_with_header: 44,
                wrap_and_apply_header: 55,
                fixed: 66,
                raw_fixed: 77,
                buffer_too_short: 88,
            })
            .encoded_length_with_header();
        let dec = MsgDecoder::try_from(&buf[..n]).expect("decode");
        assert_eq!(dec.encoded_length_field(), 11);
        assert_eq!(dec.encoded_length_with_header_field(), 22);
        // as_body_bytes / as_bytes_with_header are on DECODER_RESERVED too.
        assert_eq!(dec.as_body_bytes_field(), 33);
        assert_eq!(dec.as_bytes_with_header_field(), 44);
        // wrap_and_apply_header is encoder-only reserved; decoder keeps the
        // field accessor name (u16, not a Result).
        assert_eq!(dec.wrap_and_apply_header(), 55);
        assert_eq!(dec.fixed(), 66);
        assert_eq!(dec.raw_fixed(), 77);
        assert_eq!(dec.buffer_too_short(), 88);
        "#,
    );

    Ok(())
}

/// Decoder `rewind` is only emitted when the message has groups or var-data.
/// A field named `rewind` in such a message must be renamed to `rewind_field`.
const REWIND_CLASH_SCHEMA: &str = r#"<messageSchema package="rewclash" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="varDataEncoding">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="Msg" id="1" blockLength="8">
    <field name="rewind" id="1" type="uint32" offset="0"/>
    <field name="normal" id="2" type="uint32" offset="4"/>
    <data name="payload" id="3" type="varDataEncoding"/>
  </message>
</messageSchema>"#;

#[test]
fn rewind_field_vs_consuming_method() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(REWIND_CLASH_SCHEMA)?);
    let src = Generator::new(GenerationConfig::new("rewclash"))
        .generate(&schema)?
        .modules()
        .next()
        .expect("one module")
        .source
        .clone();

    assert!(
        src.contains("fn rewind_field(&self) -> u32"),
        "field 'rewind' must be renamed to rewind_field on the decoder"
    );
    assert!(
        src.contains("fn rewind("),
        "reserved decoder method rewind() must remain"
    );

    compile_and_run(
        "rewclash",
        &src,
        r#"
        let payload = b"hello";
        let len = MsgEncoder::compute_length_with_header(payload.len());
        let mut buf = vec![0u8; len];
        let n = MsgEncoder::wrap_and_apply_header(&mut buf, 0).unwrap()
            .fixed(&MsgFixedFields { rewind: 42, normal: 99 })
            .payload(payload)?
            .encoded_length_with_header();

        let dec = MsgDecoder::try_from(&buf[..n]).expect("decode");
        assert_eq!(dec.rewind_field(), 42);
        assert_eq!(dec.payload(), Ok(payload.as_slice()));
        // rewind() consumes self → returns fresh initial decoder.
        let rewound = dec.rewind();
        assert_eq!(rewound.rewind_field(), 42);
        "#,
    );

    Ok(())
}

#[test]
fn optional_fixed_field_runtime() -> Result<(), Box<dyn std::error::Error>> {
    // Minimal repro: fixed() with an optional field must not panic.
    let xml = r#"<messageSchema package="optfix" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Msg" id="1" blockLength="4">
    <field name="x" id="1" type="uint16" offset="0"/>
    <field name="maybe" id="2" type="uint16" presence="optional" offset="2"/>
  </message>
</messageSchema>"#;
    let schema = Schema::from_ir(parse(xml)?);
    let src = Generator::new(GenerationConfig::new("optfix"))
        .generate(&schema)?
        .modules()
        .next()
        .expect("one module")
        .source
        .clone();

    compile_and_run(
        "optfix",
        &src,
        r#"
        let mut buf = [0u8; MsgEncoder::compute_length_with_header()];
        let n = MsgEncoder::wrap_and_apply_header(&mut buf, 0).unwrap()
            .fixed(&MsgFixedFields { x: 1, maybe: Some(2) })
            .encoded_length_with_header();
        let dec = MsgDecoder::try_from(&buf[..n]).expect("decode");
        assert_eq!(dec.x(), 1);
        assert_eq!(dec.maybe(), Some(2));

        // apply_nulls() nullifies ALL optional fields (by design).
        let n2 = MsgEncoder::wrap_and_apply_header(&mut buf, 0).unwrap()
            .fixed(&MsgFixedFields { x: 99, maybe: None })
            .apply_nulls()
            .encoded_length_with_header();
        let dec2 = MsgDecoder::try_from(&buf[..n2]).expect("decode");
        assert_eq!(dec2.x(), 99);
        assert_eq!(dec2.maybe(), None);
        "#,
    );

    Ok(())
}
