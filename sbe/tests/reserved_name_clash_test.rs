//! Regression test for reserved-method / field-name collisions.
//!
//! Placement utilities (`remaining`, `buffer`, `limit`, `message_offset`) live
//! only on `{Name}DecoderMetadata` / `{Name}EncoderMetadata` via
//! `get_metadata()`. Schema fields may therefore use those names without a
//! `_field` rename. Reserved renames still apply to true inherent methods
//! (`wrap`, `decode`, `encoded_length`, `fixed`, …).

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
  <message name="Msg" id="1" blockLength="29">
    <field name="remaining" id="1" type="uint32" presence="optional" offset="0"/>
    <field name="buffer" id="2" type="Quad" offset="4"/>
    <field name="limit" id="3" type="uint32" offset="20"/>
    <field name="messageOffset" id="4" type="uint32" offset="24"/>
    <field name="normal" id="5" type="uint8" offset="28"/>
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

    // Placement names are not reserved on the message decoder — fields keep
    // their natural names; utils are on get_metadata().
    for natural in [
        "fn remaining(&self) -> Option<u32>",
        "fn buffer(&self) -> [u32; 4]",
        "fn limit(&self) -> u32",
        "fn message_offset(&self) -> u32",
    ] {
        assert!(
            src.contains(natural),
            "placement-named field must keep natural accessor `{natural}` (not *_field)"
        );
    }
    for renamed in [
        "fn remaining_field",
        "fn buffer_field",
        "fn limit_field",
        "fn message_offset_field",
    ] {
        assert!(
            !src.contains(renamed),
            "must not rename placement-named field to `{renamed}`"
        );
    }
    assert!(
        src.contains("fn get_metadata("),
        "decoder must expose get_metadata for placement utils"
    );

    // The real proof: the generated crate compiles and every path works.
    compile_and_run(
        "clash",
        &src,
        r#"
        let mut buf = [0u8; MsgEncoder::compute_length_with_header()];
        let len = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
            .fixed(&MsgFixedFields {
                remaining: Some(7),
                buffer: [10, 20, 30, 40],
                limit: 100,
                message_offset: 200,
                normal: 9,
            })
            .encoded_length_with_header();

        let dec = MsgDecoder::try_from(&buf[..len]).expect("decode");
        // Field accessors use natural names (no _field).
        assert_eq!(dec.remaining(), Some(7));
        assert_eq!(dec.buffer(), [10, 20, 30, 40]);
        assert_eq!(dec.limit(), 100);
        assert_eq!(dec.message_offset(), 200);
        assert_eq!(dec.normal(), 9);
        // Placement utils are on metadata and do not collide with fields.
        let meta = dec.get_metadata();
        let _all: &[u8] = meta.buffer();
        let _tail: &[u8] = meta.remaining();
        let _lim: usize = meta.limit();
        let _off: usize = meta.message_offset();
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
        let n = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
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
        let n = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
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
        let n = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
            .fixed(&MsgFixedFields { x: 1, maybe: Some(2) })
            .encoded_length_with_header();
        let dec = MsgDecoder::try_from(&buf[..n]).expect("decode");
        assert_eq!(dec.x(), 1);
        assert_eq!(dec.maybe(), Some(2));

        // `fixed(None)` writes the schema null image for optional fields.
        let n2 = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
            .fixed(&MsgFixedFields { x: 99, maybe: None })
            .encoded_length_with_header();
        let dec2 = MsgDecoder::try_from(&buf[..n2]).expect("decode");
        assert_eq!(dec2.x(), 99);
        assert_eq!(dec2.maybe(), None);
        "#,
    );

    Ok(())
}

/// Schema fields named after Rust keywords (`type`, `fn`, `match`, etc.) must
/// have the `keyword_append_token` (`_`) appended so the generated crate
/// compiles. This tests the `is_rust_keyword` path in `to_snake_case`.
#[test]
fn rust_keyword_field_names_compile() -> Result<(), Box<dyn std::error::Error>> {
    let keyword_schema = r#"<messageSchema package="kw" id="1" version="0" byteOrder="littleEndian">
      <types>
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
      </types>
      <message name="Msg" id="1" blockLength="16">
        <field name="type"   id="1" type="uint32" offset="0"/>
        <field name="fn"     id="2" type="uint32" offset="4"/>
        <field name="match"  id="3" type="uint32" offset="8"/>
        <field name="impl"   id="4" type="uint32" offset="12"/>
      </message>
    </messageSchema>"#;

    let schema = Schema::from_ir(parse(keyword_schema)?);
    let src = Generator::new(GenerationConfig::new("kw"))
        .generate(&schema)?
        .modules()
        .next()
        .expect("one module")
        .source
        .clone();

    // Field names get the keyword append token.
    assert!(
        src.contains("fn type_(&self)"),
        "keyword field 'type' must be type_"
    );
    assert!(
        src.contains("fn fn_(&self)"),
        "keyword field 'fn' must be fn_"
    );
    assert!(
        src.contains("fn match_(&self)"),
        "keyword field 'match' must be match_"
    );
    assert!(
        src.contains("fn impl_(&self)"),
        "keyword field 'impl' must be impl_"
    );

    // The real proof: the generated crate compiles and runs.
    compile_and_run(
        "kw",
        &src,
        r#"
        let mut buf = [0u8; MsgEncoder::compute_length_with_header()];
        let len = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
            .fixed(&MsgFixedFields {
                type_: 1,
                fn_: 2,
                match_: 3,
                impl_: 4,
            })
            .encoded_length_with_header();

        let dec = MsgDecoder::try_from(&buf[..len]).expect("decode");
        assert_eq!(dec.type_(), 1);
        assert_eq!(dec.fn_(), 2);
        assert_eq!(dec.match_(), 3);
        assert_eq!(dec.impl_(), 4);
        "#,
    );

    Ok(())
}

/// When a schema message is literally named `Self`, the generated type name
/// gets the `keyword_append_token` suffix because `Self` is a Rust keyword.
/// `PascalCase` names like `Type` don't collide with the lowercase keyword
/// `type` — they compile fine without the suffix.
#[test]
fn rust_keyword_message_name_self_compiles() -> Result<(), Box<dyn std::error::Error>> {
    let keyword_msg_schema = r#"<messageSchema package="kwmsg" id="1" version="0" byteOrder="littleEndian">
      <types>
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
      </types>
      <message name="Self" id="1" blockLength="4">
        <field name="value" id="1" type="uint32" offset="0"/>
      </message>
    </messageSchema>"#;

    let schema = Schema::from_ir(parse(keyword_msg_schema)?);
    let src = Generator::new(GenerationConfig::new("kwmsg"))
        .generate(&schema)?
        .modules()
        .next()
        .expect("one module")
        .source
        .clone();

    assert!(
        src.contains("Self_Encoder") && src.contains("Self_Decoder"),
        "keyword message 'Self' must become Self_Encoder / Self_Decoder"
    );

    compile_and_run(
        "kwmsg",
        &src,
        r#"
        let mut buf = [0u8; Self_Encoder::compute_length_with_header()];
        let len = Self_Encoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
            .fixed(&Self_FixedFields { value: 42 })
            .encoded_length_with_header();

        let dec = Self_Decoder::try_from(&buf[..len]).expect("decode");
        assert_eq!(dec.value(), 42);
        "#,
    );

    Ok(())
}

/// When `keyword_append_token` is set to empty, a schema field named after a
/// Rust keyword produces generated code that fails to compile. This is the
/// intended failure mode: the user must either rename the schema field or
/// keep the default `_` append token. The test proves that the compiler
/// rejection is clear (mentions the keyword or the field name).
#[test]
fn keyword_field_fails_compile_without_append_token() -> Result<(), Box<dyn std::error::Error>> {
    let keyword_schema = r#"<messageSchema package="kwfail" id="1" version="0" byteOrder="littleEndian">
      <types>
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
      </types>
      <message name="Msg" id="1" blockLength="8">
        <field name="type"  id="1" type="uint32" offset="0"/>
        <field name="fn"    id="2" type="uint32" offset="4"/>
      </message>
    </messageSchema>"#;

    let schema = Schema::from_ir(parse(keyword_schema)?);
    // Empty append token — generated code will have `fn type()` which is
    // invalid Rust because `type` is a reserved keyword.
    let config = GenerationConfig::new("kwfail").with_keyword_append_token("");
    let result = Generator::new(config).generate(&schema);

    // 0.1.14+: generate() returns Err instead of silently emitting a
    // comment-banner module. The keyword-affixed-field path already handles
    // this case (append "_"); the empty-token path exposes the defect.
    match result {
        Err(ergo_sbe::codegen::GenerateError::InvalidGeneratedSource { module, error }) => {
            assert!(module == "kwfail", "error module name must match");
            assert!(
                error.contains("keyword") || error.contains("type"),
                "error must mention the keyword issue: {error}"
            );
        }
        other => unreachable!("expected InvalidGeneratedSource, got {other:?}"),
    }

    Ok(())
}

fn generate_src(xml: &str, pkg: &str) -> Result<String, Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(xml)?);
    Ok(Generator::new(GenerationConfig::new(pkg))
        .generate(&schema)?
        .modules()
        .next()
        .expect("one module")
        .source
        .clone())
}

/// Read a reserved-name list straight out of the generator source.
///
/// Compile-time include (no `CARGO_MANIFEST_DIR`) — the generator's own list is
/// the source of truth, so the test cannot drift from it by holding a copy.
fn parse_reserved_list(marker: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    const HELPERS: &str = include_str!("../src/codegen/conversion_helpers.rs");
    let start = HELPERS
        .find(marker)
        .ok_or_else(|| format!("missing {marker}"))?;
    let rest = &HELPERS[start..];
    let end = rest
        .find("];")
        .ok_or_else(|| format!("unterminated {marker}"))?;
    let mut names = Vec::new();
    for line in rest[..end].lines() {
        if let Some(s) = line.trim().strip_prefix('"')
            && let Some(name) = s.split('"').next()
            && !name.is_empty()
        {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

/// Placement utilities live on the metadata facet, so reserving their names
/// would rename a real schema field for no reason (zombie rename regression).
#[test]
fn placement_names_are_never_reserved() -> Result<(), Box<dyn std::error::Error>> {
    let decoder_reserved = parse_reserved_list("const DECODER_RESERVED")?;
    let encoder_reserved = parse_reserved_list("const ENCODER_RESERVED")?;

    for placement in [
        "remaining",
        "buffer",
        "limit",
        "message_offset",
        "as_fixed_body_bytes",
        "as_fixed_region_with_header",
    ] {
        assert!(
            !decoder_reserved.iter().any(|n| n == placement),
            "DECODER_RESERVED must not contain placement util `{placement}`"
        );
        assert!(
            !encoder_reserved.iter().any(|n| n == placement),
            "ENCODER_RESERVED must not contain placement util `{placement}`"
        );
    }
    assert!(
        !decoder_reserved.iter().any(|n| n == "header"),
        "stale reserved `header` must stay removed"
    );
    Ok(())
}

/// Representative tailed message shape for reserved-name coverage.
const TAILED_SCHEMA: &str = r#"<messageSchema package="rsub" id="1" version="0" byteOrder="littleEndian">
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
    <composite name="varDataEncoding">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="Tailed" id="1" blockLength="4">
    <field name="x" id="1" type="uint32" offset="0"/>
    <group name="g" id="2" dimensionType="groupSizeEncoding">
      <field name="y" id="3" type="uint16" offset="0"/>
      <data name="note" id="4" type="varDataEncoding"/>
    </group>
  </message>
</messageSchema>"#;

/// Representative fixed message shape for reserved-name coverage.
const FIXED_SCHEMA: &str = r#"<messageSchema package="rfix" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Fixed" id="1" blockLength="4">
    <field name="x" id="1" type="uint32" offset="0"/>
  </message>
</messageSchema>"#;

/// Representative optional message shape for reserved-name coverage.
const OPTIONAL_SCHEMA: &str = r#"<messageSchema package="ropt" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Opt" id="1" blockLength="4">
    <field name="x" id="1" type="uint16" offset="0"/>
    <field name="maybe" id="2" type="uint16" presence="optional" offset="2"/>
  </message>
</messageSchema>"#;

/// Every reserved name must be emitted as an inherent method on some
/// representative message shape, and placement must appear on the metadata
/// facet. A name reserved but never emitted renames fields for nothing.
#[test]
fn reserved_names_match_emitted_inherent_methods() -> Result<(), Box<dyn std::error::Error>> {
    let decoder_reserved = parse_reserved_list("const DECODER_RESERVED")?;
    let encoder_reserved = parse_reserved_list("const ENCODER_RESERVED")?;

    // Staged message (group entry has var-data): compute_length factory + rewind.
    // Flat group+message-var-data is Direct strategy and does not emit compute_length().
    let tailed = generate_src(TAILED_SCHEMA, "rsub")?;

    // Fixed-only: after_this_message + wrap_into_claim.
    let fixed = generate_src(FIXED_SCHEMA, "rfix")?;

    // Optional fields → apply_nulls.
    let optional = generate_src(OPTIONAL_SCHEMA, "ropt")?;

    let has_fn = |src: &str, name: &str| {
        src.contains(&format!("fn {name}(")) || src.contains(&format!("fn {name}<"))
    };

    for name in &decoder_reserved {
        let ok = has_fn(&tailed, name) || has_fn(&fixed, name) || has_fn(&optional, name);
        assert!(
            ok,
            "DECODER_RESERVED `{name}` is not emitted as an inherent method on \
             any representative schema (tailed/fixed/optional) — remove from \
             reserved or restore emission"
        );
    }
    for name in &encoder_reserved {
        let ok = has_fn(&tailed, name) || has_fn(&fixed, name) || has_fn(&optional, name);
        assert!(
            ok,
            "ENCODER_RESERVED `{name}` is not emitted as an inherent method on \
             any representative schema (tailed/fixed/optional) — remove from \
             reserved or restore emission"
        );
    }

    // Placement lives on metadata for every shape.
    for src in [&tailed, &fixed, &optional] {
        assert!(src.contains("fn get_metadata("), "missing get_metadata");
        assert!(
            src.contains("fn remaining(&self)") || src.contains("fn remaining(&self) ->"),
            "metadata remaining missing"
        );
        // Field-safe: a placement-named method on Metadata, not reserved rename.
        assert!(
            !src.contains("fn remaining_field"),
            "must not emit remaining_field without a reserved collision"
        );
    }

    // Conditional emission spots.
    assert!(has_fn(&tailed, "rewind"), "tailed message must emit rewind");
    assert!(
        has_fn(&fixed, "after_this_message"),
        "fixed message must emit after_this_message"
    );
    assert!(
        has_fn(&fixed, "wrap_into_claim"),
        "fixed message must emit wrap_into_claim"
    );
    assert!(
        has_fn(&optional, "apply_nulls"),
        "optional fields must emit apply_nulls"
    );

    Ok(())
}
