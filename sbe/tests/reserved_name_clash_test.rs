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
        let len = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&MsgFixedFields {
                remaining: Some(7),
                buffer: [10, 20, 30, 40],
                limit: 100,
                message_offset: 200,
                normal: 9,
            })
            .encoded_length_with_header();

        let dec = MsgDecoder::try_from(&buf[..len])?;
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
        r"
        let mut buf = [0u8; MsgEncoder::compute_length_with_header()];
        let n = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0)?
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
        let dec = MsgDecoder::try_from(&buf[..n])?;
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
        ",
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
        let n = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&MsgFixedFields { rewind: 42, normal: 99 })
            .payload(payload)?
            .encoded_length_with_header();

        let dec = MsgDecoder::try_from(&buf[..n])?;
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
        r"
        let mut buf = [0u8; MsgEncoder::compute_length_with_header()];
        let n = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&MsgFixedFields { x: 1, maybe: Some(2) })
            .encoded_length_with_header();
        let dec = MsgDecoder::try_from(&buf[..n])?;
        assert_eq!(dec.x(), 1);
        assert_eq!(dec.maybe(), Some(2));

        // `fixed(None)` writes the schema null image for optional fields.
        let n2 = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&MsgFixedFields { x: 99, maybe: None })
            .encoded_length_with_header();
        let dec2 = MsgDecoder::try_from(&buf[..n2])?;
        assert_eq!(dec2.x(), 99);
        assert_eq!(dec2.maybe(), None);
        ",
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
        r"
        let mut buf = [0u8; MsgEncoder::compute_length_with_header()];
        let len = MsgEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&MsgFixedFields {
                type_: 1,
                fn_: 2,
                match_: 3,
                impl_: 4,
            })
            .encoded_length_with_header();

        let dec = MsgDecoder::try_from(&buf[..len])?;
        assert_eq!(dec.type_(), 1);
        assert_eq!(dec.fn_(), 2);
        assert_eq!(dec.match_(), 3);
        assert_eq!(dec.impl_(), 4);
        ",
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
        r"
        let mut buf = [0u8; Self_Encoder::compute_length_with_header()];
        let len = Self_Encoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&Self_FixedFields { value: 42 })
            .encoded_length_with_header();

        let dec = Self_Decoder::try_from(&buf[..len])?;
        assert_eq!(dec.value(), 42);
        ",
    );

    Ok(())
}

/// When `keyword_append_token` is set to empty (or another value that can't
/// turn a keyword into a valid non-keyword identifier), generation must
/// reject the *configuration* before emitting anything, rather than let a
/// schema field named after a Rust keyword produce generated code that fails
/// to compile with a generic, hard-to-diagnose error.
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
    // Empty append token — "type" + "" is still the keyword `type`, which is
    // now rejected as configuration before any code is emitted.
    let config = GenerationConfig::new("kwfail").with_keyword_append_token("");
    let result = Generator::new(config).generate(&schema);

    match result {
        Err(ergo_sbe::codegen::GenerateError::InvalidConfiguration { option, reason, .. }) => {
            assert_eq!(option, "keyword_append_token");
            assert!(
                reason.contains("keyword"),
                "error must mention the keyword issue: {reason}"
            );
        }
        other => unreachable!("expected InvalidConfiguration, got {other:?}"),
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

/// A tail-derived convenience accessor must never define a method that a fixed
/// field already defines. A group `orders` wants `orders_count()`; a field
/// `ordersCount` already has it. Before this was guarded, both were emitted and
/// the generated module failed to compile with E0592.
///
/// The field wins and keeps its name — renaming it would change an accessor
/// that worked before `*_count()` existed. The same rule covers `<field>_len`
/// for var-data, at message, memoized and group-entry level.
///
/// Sibling *tails* collide the same way and are covered too: a group `fills`
/// beside a group `fillsCount`, and a var-data `blob` beside a var-data
/// `blobLen`. Checking only fields left those two cases generating duplicate
/// methods.
const TAIL_CLASH_XML: &str = r#"<messageSchema package="tailclash" id="9" version="0" byteOrder="littleEndian">
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
    <composite name="varString">
      <type name="length" primitiveType="uint16"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="Msg" id="1" blockLength="8">
    <field name="ordersCount" id="1" type="uint32" offset="0"/>
    <field name="noteLen" id="2" type="uint32" offset="4"/>
    <group name="orders" id="3" dimensionType="groupSizeEncoding" blockLength="8">
      <field name="fillsCount" id="4" type="uint32" offset="0"/>
      <field name="memoLen" id="5" type="uint32" offset="4"/>
      <group name="fills" id="6" dimensionType="groupSizeEncoding" blockLength="4">
        <field name="qty" id="7" type="uint32" offset="0"/>
      </group>
      <data name="memo" id="8" type="varString"/>
    </group>
    <group name="fills" id="20" dimensionType="groupSizeEncoding" blockLength="4">
      <field name="qty" id="21" type="uint32" offset="0"/>
    </group>
    <group name="fillsCount" id="22" dimensionType="groupSizeEncoding" blockLength="4">
      <field name="qty" id="23" type="uint32" offset="0"/>
    </group>
    <data name="note" id="9" type="varString"/>
    <data name="blob" id="24" type="varString"/>
    <data name="blobLen" id="25" type="varString"/>
  </message>
</messageSchema>"#;

#[test]
fn tail_accessors_yield_to_colliding_field_names() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(TAIL_CLASH_XML)?);
    let src = Generator::new(GenerationConfig::new("tailclash"))
        .generate(&schema)?
        .modules()
        .next()
        .expect("one module")
        .source
        .clone();

    // The field keeps its natural name and its own return type on every type
    // that carries it.
    for (name, signature) in [
        ("orders_count", "pub fn orders_count(&self) -> u32"),
        ("note_len", "pub fn note_len(&self) -> u32"),
        ("fills_count", "pub fn fills_count(&self) -> u32"),
        ("memo_len", "pub fn memo_len(&self) -> u32"),
    ] {
        assert!(
            src.contains(signature),
            "field accessor `{name}` must keep its natural name and type"
        );
    }

    // Sibling tails occupy the name just as fields do.
    for sig in [
        "pub fn fills_count(&self) -> Result<FillsCountDecoder<'a>, sbe_rt::DecodeError>",
        "pub fn blob_len(&self) -> Result<&'a [u8], sbe_rt::DecodeError>",
    ] {
        assert!(
            src.contains(sig),
            "sibling tail accessor must keep its own name and type: {sig}"
        );
    }

    // Consuming stages carry no fields, so they still get the tail accessor —
    // the guard is per-type, not per-schema.
    assert!(
        src.contains("pub fn note_len(&self) -> Result<usize, sbe_rt::DecodeError>"),
        "a stage with no colliding field must still get the tail length accessor"
    );

    // Compilation is the assertion: emitting both on one type is E0592.
    // Verified to fail before the guard existed.
    compile_and_run("tail_clash_rt", &src, "let _ = 1;");
    Ok(())
}

/// Group-entry metadata getters must yield to schema fields of the same name.
/// A field `actingVersion` / `actingBlockLength` keeps `acting_version()` /
/// `acting_block_length()`; the convenience methods are omitted. Unconditional
/// emission is E0592 (HFT review 2026-09-15).
#[test]
fn entry_fields_named_acting_version_and_block_length_compile()
-> Result<(), Box<dyn std::error::Error>> {
    const XML: &str = r#"<messageSchema package="entrynames" id="1" version="0" byteOrder="littleEndian">
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
  </types>
  <message name="M" id="1">
    <group name="rows" id="1" dimensionType="groupSizeEncoding">
      <field name="actingVersion" id="2" type="uint16"/>
      <field name="actingBlockLength" id="3" type="uint16"/>
    </group>
  </message>
</messageSchema>"#;
    let schema = Schema::from_ir(parse(XML)?);
    let src = Generator::new(GenerationConfig::new("entrynames"))
        .generate(&schema)?
        .modules()
        .next()
        .expect("one module")
        .source
        .clone();
    assert!(
        src.contains("pub fn acting_version(&self) -> u16"),
        "entry field actingVersion keeps acting_version()"
    );
    assert!(
        src.contains("pub fn acting_block_length(&self) -> u16"),
        "entry field actingBlockLength keeps acting_block_length()"
    );
    compile_and_run(
        "entrynames_rt",
        &src,
        r"
        let len = MEncoder::compute_length_with_header(1);
        let mut storage = [0u8; 32];
        let buf = &mut storage[..len];
        let actual = MEncoder::try_wrap_and_apply_header(buf, 0)?
            .fixed(&MFixedFields {})
            .rows(1, |g| {
                g.add(|e| { e.acting_version(7u16).acting_block_length(9u16); Ok(()) })?;
                Ok(())
            })?
            .encoded_length_with_header();
        assert_eq!(len, actual);
        let dec = MDecoder::try_decode(&storage[..actual], 0)?;
        let mut n = 0;
        for row in dec.rows()? {
            assert_eq!(row.acting_version(), 7);
            assert_eq!(row.acting_block_length(), 9);
            n += 1;
        }
        assert_eq!(n, 1);
        ",
    );
    Ok(())
}

/// `acting_version` / `acting_block_length` yield to a schema name at every
/// location that owns tails: entry fields beside an entry ordered lane, entry
/// tails, and message tails. The earlier entry test used a group with no tails,
/// so it never generated the ordered lane that still emitted both names
/// unconditionally (E0592 on a tail, E0015 on a non-const field getter).
#[test]
#[allow(clippy::too_many_lines)]
fn acting_names_yield_on_tail_owners_and_ordered_lanes() -> Result<(), Box<dyn std::error::Error>> {
    const XML: &str = r#"<messageSchema package="actingtails" id="1" version="0" byteOrder="littleEndian">
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
    <composite name="varStringEncoding">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
    </composite>
  </types>
  <message name="E" id="1">
    <group name="rows" id="1" dimensionType="groupSizeEncoding">
      <field name="actingVersion" id="2" type="uint16"/>
      <field name="actingBlockLength" id="3" type="uint16"/>
      <data name="note" id="4" type="varStringEncoding"/>
    </group>
  </message>
  <message name="T" id="2">
    <group name="items" id="1" dimensionType="groupSizeEncoding">
      <field name="px" id="2" type="uint32"/>
      <group name="actingBlockLength" id="3" dimensionType="groupSizeEncoding">
        <field name="q" id="4" type="uint32"/>
      </group>
      <data name="actingVersion" id="5" type="varStringEncoding"/>
    </group>
  </message>
  <message name="M" id="3">
    <field name="x" id="1" type="uint32"/>
    <group name="actingBlockLength" id="2" dimensionType="groupSizeEncoding">
      <field name="q" id="3" type="uint32"/>
    </group>
    <data name="actingVersion" id="4" type="varStringEncoding"/>
  </message>
</messageSchema>"#;
    let schema = Schema::from_ir(parse(XML)?);
    let src = Generator::new(GenerationConfig::new("actingtails"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();
    compile_and_run(
        "actingtails_rt",
        &src,
        r#"
    // E: entry fields own `acting_*`; the entry ordered lane must still exist.
    let len = EEncodedLength::new().rows(1).note(2)?.encoded_length_with_header();
    let mut storage = [0u8; 64];
    let buf = &mut storage[..len];
    let actual = EEncoder::try_wrap_and_apply_header(buf, 0)?
        .fixed(&EFixedFields {})
        .rows(1, |g| {
            g.add(|mut e| { e.acting_version(7u16).acting_block_length(9u16); e.note(b"hi") })?;
            Ok(())
        })?
        .encoded_length_with_header();
    assert_eq!(len, actual);
    let mut notes = Vec::new();
    let done = EDecoder::try_decode(&storage[..actual], 0)?
        .ordered()
        .rows(|e, _| {
            assert_eq!((e.acting_version(), e.acting_block_length()), (7, 9));
            Ok(e.ordered().note(|b| { notes.push(b.to_vec()); Ok(()) })?)
        })?;
    assert_eq!(done.acting_version(), 0);
    assert_eq!(notes, vec![b"hi".to_vec()]);

    // T: entry tails own `acting_*`.
    let len = TEncodedLength::new()
        .items_ragged(1, |b| {
            b.add()?.acting_block_length(|n| { n.uniform(1)?; Ok(()) })?.acting_version(1)?;
            Ok(())
        })?
        .encoded_length_with_header();
    let mut storage = [0u8; 64];
    let buf = &mut storage[..len];
    let actual = TEncoder::try_wrap_and_apply_header(buf, 0)?
        .fixed(&TFixedFields {})
        .items(1, |g| {
            g.add(|mut e| {
                e.px(3);
                e.acting_block_length(1, |n| { n.add(|q| { q.q(5); Ok(()) })?; Ok(()) })?
                    .acting_version(b"v")
            })?;
            Ok(())
        })?
        .encoded_length_with_header();
    assert_eq!(len, actual);
    let mut seen = Vec::new();
    TDecoder::try_decode(&storage[..actual], 0)?
        .ordered()
        .items(|e, _| {
            assert_eq!(e.acting_version_len()?, 1);
            let stage = e.ordered();
            assert_eq!(stage.acting_block_length_count()?, 1);
            let stage = stage.acting_block_length(|q, _| { seen.push(q.q()); Ok(()) })?;
            assert_eq!(stage.acting_block_length(), 4);
            Ok(stage.acting_version(|b| { seen.push(u32::from(b[0])); Ok(()) })?)
        })?;
    assert_eq!(seen, vec![5, u32::from(b'v')]);

    // M: message tails own `acting_*`; metadata keeps the header values.
    let len = MEncoder::compute_length_with_header(1, 2);
    let mut storage = [0u8; 64];
    let buf = &mut storage[..len];
    let actual = MEncoder::try_wrap_and_apply_header(buf, 0)?
        .fixed(&MFixedFields { x: 11 })
        .acting_block_length(1, |g| { g.add(|q| { q.q(6); Ok(()) })?; Ok(()) })?
        .acting_version(b"ok")?
        .encoded_length_with_header();
    assert_eq!(len, actual);
    let dec = MDecoder::try_decode(&storage[..actual], 0)?;
    assert_eq!(dec.get_metadata().acting_version(), 0);
    assert_eq!(dec.acting_version()?, b"ok");
    // `actingVersion` is the *second* tail here, so it takes the name only from
    // the wrapper that defines its own accessor. The first wrapper keeps the
    // header getter: suppression is per type, not per schema.
    assert_eq!(
        MDecoder::try_decode(&storage[..actual], 0)?.ordered().acting_version(),
        0
    );
    assert_eq!(dec.acting_block_length()?.count(), 1);
    let memo = MDecoder::try_decode(&storage[..actual], 0)?.memoized();
    assert_eq!(memo.acting_version()?, b"ok");
    let mut qs = Vec::new();
    let complete = MDecoder::try_decode(&storage[..actual], 0)?
        .ordered()
        .fixed(|v| { assert_eq!((v.x(), v.acting_version()), (11, 0)); Ok(()) })?
        .acting_block_length(|q, _| { qs.push(q.q()); Ok(()) })?
        .acting_version(|b| { assert_eq!(b, b"ok"); Ok(()) })?;
    assert_eq!(qs, vec![6]);
    assert_eq!(complete.encoded_length_with_header(), actual);
    "#,
    );
    Ok(())
}

/// Nested-entry `actingVersion` / `actingBlockLength` beside nested tails.
/// Message-level and one-level entry cells do not prove this location.
#[test]
fn nested_entry_fields_named_acting_version_compile() -> Result<(), Box<dyn std::error::Error>> {
    const XML: &str = r#"<messageSchema package="nestedacting" id="1" version="0" byteOrder="littleEndian">
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
    <composite name="varStringEncoding">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
    </composite>
  </types>
  <message name="N" id="1">
    <group name="rows" id="1" dimensionType="groupSizeEncoding">
      <field name="px" id="2" type="uint32"/>
      <group name="cells" id="3" dimensionType="groupSizeEncoding">
        <field name="actingVersion" id="4" type="uint16"/>
        <field name="actingBlockLength" id="5" type="uint16"/>
        <data name="note" id="6" type="varStringEncoding"/>
      </group>
    </group>
  </message>
</messageSchema>"#;
    let schema = Schema::from_ir(parse(XML)?);
    let src = Generator::new(GenerationConfig::new("nestedacting"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();
    compile_and_run(
        "nestedacting_rt",
        &src,
        r#"
        let len = NEncodedLength::new()
            .rows_ragged(1, |r| {
                r.add()?.cells(|c| {
                    c.add()?.note(2)?;
                    Ok(())
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        let mut storage = [0u8; 64];
        let buf = &mut storage[..len];
        let actual = NEncoder::try_wrap_and_apply_header(buf, 0)?
            .fixed(&NFixedFields {})
            .rows(1, |g| {
                g.add(|mut e| {
                    e.px(3);
                    e.cells(1, |c| {
                        c.add(|mut n| {
                            n.acting_version(7u16).acting_block_length(9u16);
                            n.note(b"hi")
                        })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        assert_eq!(len, actual);
        let dec = NDecoder::try_decode(&storage[..actual], 0)?;
        for row in dec.rows()? {
            let row = row?;
            assert_eq!(row.px(), 3);
            for cell in row.cells()? {
                let cell = cell?;
                assert_eq!((cell.acting_version(), cell.acting_block_length()), (7, 9));
                assert_eq!(cell.note()?, b"hi");
            }
        }
        "#,
    );
    Ok(())
}
