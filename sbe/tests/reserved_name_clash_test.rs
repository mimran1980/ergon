//! Regression test for reserved-method / field-name collisions.
//!
//! A flat message may legally declare fields whose names collide with reserved
//! decoder/encoder methods (`remaining`, `whole_buffer`). Both the **array**
//! accessor path and the **optional primitive** accessor path must route the
//! field name through `resolve_field_ident` so it becomes `{name}_field`,
//! leaving the reserved method intact. The earlier substring-only test used a
//! single scalar field and did not exercise either path — a schema with an
//! optional `remaining` and an array `wholeBuffer` produced duplicate methods
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
    <field name="wholeBuffer" id="2" type="Quad" offset="4"/>
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
        src.contains("fn whole_buffer_field(&self) -> [u32; 4]"),
        "array field 'wholeBuffer' must be renamed to whole_buffer_field"
    );
    // The reserved decoder methods still exist and are not shadowed.
    assert!(
        src.contains("fn remaining(&self) -> &'a [u8]"),
        "reserved decoder method remaining() must remain"
    );
    assert!(
        src.contains("fn whole_buffer(&self) -> &'a [u8]"),
        "reserved decoder method whole_buffer() must remain"
    );

    // The real proof: the generated crate compiles and every path works,
    // including the Display impl that references the renamed accessors.
    compile_and_run(
        "clash",
        &src,
        r#"
        let mut buf = [0u8; MsgEncoder::compute_length_with_header()];
        let len = MsgEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&MsgFixedFields {
                remaining: Some(7),
                whole_buffer: [10, 20, 30, 40],
                normal: 9,
            })
            .encoded_length_with_header();

        let dec = MsgDecoder::try_from(&buf[..len]).expect("decode");
        // Renamed field accessors.
        assert_eq!(dec.remaining_field(), Some(7));
        assert_eq!(dec.whole_buffer_field(), [10, 20, 30, 40]);
        assert_eq!(dec.normal(), 9);
        // Reserved methods still available and distinct from the fields.
        let _tail: &[u8] = dec.remaining();
        let _all: &[u8] = dec.whole_buffer();
        // Display/Debug impl references the renamed accessors — must format.
        let shown = format!("{dec:?}");
        assert!(shown.contains("remaining"));
        "#,
    );

    Ok(())
}
