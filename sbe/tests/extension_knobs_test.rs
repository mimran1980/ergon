//! The three opt-in extension built-ins: variant labels (B1), exponent
//! links (B2) and field extension defaults (B3).
//!
//! Compilation of the generated module is the load-bearing assertion (the
//! matrix row proves the same knobs across all three codegen locations);
//! the runtime asserts here name the value that broke. The fail-closed
//! errors are checked directly against `GenerateError`.

#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    unused
)]
mod common;
use common::compile_and_run;
use ergo_sbe::{GenerateError, GenerationConfig, Generator, Schema, SchemaSource, parse};
use std::path::PathBuf;

fn matrix_schema() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/codegen-matrix-schema.xml"
    ))
}

fn generate(
    module: &str,
    configure: impl FnOnce(GenerationConfig) -> GenerationConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(SchemaSource::file(&matrix_schema()))?);
    let mut g = Generator::new(configure(GenerationConfig::new(module)));
    let ms = g.generate(&schema)?;
    Ok(ms.modules().next().ok_or("one module")?.source.clone())
}

fn generate_from_xml(
    xml: &str,
    module: &str,
    configure: impl FnOnce(GenerationConfig) -> GenerationConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(SchemaSource::xml(xml))?);
    let mut g = Generator::new(configure(GenerationConfig::new(module)));
    let ms = g.generate(&schema)?;
    Ok(ms.modules().next().ok_or("one module")?.source.clone())
}

/// B1: `with_variant_labels("jsonValue", "as_str")` makes `as_str` (and
/// `Display`, which delegates to it) render the extension label, falling
/// back to the value's variant name — what the built-in `as_str` returns —
/// for values that carry none.
#[test]
fn variant_labels_render_labels_with_variant_fallback() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate("ext_labels", |c| {
        c.with_variant_labels("jsonValue", "as_str")
    })?;
    compile_and_run(
        "ext_labels",
        &src,
        r#"
        assert_eq!(Model::A.as_str(), "alpha");
        assert_eq!(format!("{}", Model::A), "alpha");
        assert_eq!(Model::B.as_str(), "B");
        assert_eq!(Model::NullVal.as_str(), "NullVal");
        assert_eq!(BooleanType::F.as_str(), "f-label");
        assert_eq!(BooleanType::T.as_str(), "t-label");
        "#,
    );
    Ok(())
}

/// B1 fallback is transparent: an unmarked snake_case value renders its
/// PascalCase variant name — byte-for-byte what the built-in `as_str`
/// returned before the knob — so enabling `as_str` labels changes only the
/// marked values and `Display` → `FromStr` still round-trips the rest.
#[test]
fn variant_labels_fallback_matches_the_builtin_as_str() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema xmlns:x="urn:example" package="ext" id="9" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <enum name="Side" encodingType="uint8">
      <validValue name="maxAsset" x:jsonValue="MAX_ASSET">0</validValue>
      <validValue name="minQty">1</validValue>
    </enum>
  </types>
  <message name="Ping" id="1"><field name="side" id="1" type="Side"/></message>
</messageSchema>"#;
    let src = generate_from_xml(xml, "ext_label_fb", |c| {
        c.with_variant_labels("jsonValue", "as_str")
    })?;
    compile_and_run(
        "ext_label_fb",
        &src,
        r#"
        // Unmarked snake_case value: identical to the built-in as_str, and
        // what FromStr parses — so Display → parse round-trips.
        assert_eq!(Side::MinQty.as_str(), "MinQty");
        assert_eq!(format!("{}", Side::MinQty), "MinQty");
        assert_eq!("MinQty".parse::<Side>().ok(), Some(Side::MinQty));
        // Marked value renders its label.
        assert_eq!(Side::MaxAsset.as_str(), "MAX_ASSET");
        "#,
    );
    Ok(())
}

/// B1 on a set: the method name is config (no `as_str` exists on sets),
/// choices fall back to their raw schema name, and anything that is not
/// exactly one choice bit returns `"UNKNOWN"`.
#[test]
fn variant_labels_on_a_set_match_single_bits() -> Result<(), Box<dyn std::error::Error>> {
    const XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema xmlns:x="urn:example" package="ext" id="9" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <set name="Opts" encodingType="uint8">
      <choice name="fast" x:jsonValue="F">0</choice>
      <choice name="safe">1</choice>
    </set>
  </types>
  <message name="Ping" id="1">
    <field name="opts" id="1" type="Opts"/>
  </message>
</messageSchema>"#;
    let src = generate_from_xml(XML, "ext_set_labels", |c| {
        c.with_variant_labels("jsonValue", "label")
    })?;
    compile_and_run(
        "ext_set_labels",
        &src,
        r#"
        assert_eq!(Opts(1).label(), "F");
        assert_eq!(Opts(2).label(), "safe");
        assert_eq!(Opts(3).label(), "UNKNOWN");
        assert_eq!(Opts(0).label(), "UNKNOWN");
        "#,
    );
    Ok(())
}

/// B1: the method name is validated as a Rust identifier at generation
/// time (`GenerateError::InvalidConfiguration`), not left to fail as an
/// unreadable compile error in the consumer's crate.
#[test]
fn variant_labels_reject_a_non_identifier_method() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(SchemaSource::file(&matrix_schema()))?);
    for bad in ["not an ident", "type", ""] {
        let mut g =
            Generator::new(GenerationConfig::new("ext_bad").with_variant_labels("jsonValue", bad));
        let err = g.generate(&schema).expect_err("invalid method must fail");
        assert!(
            matches!(
                err,
                GenerateError::InvalidConfiguration {
                    ref option,
                    ..
                } if option == "with_variant_labels"
            ),
            "expected InvalidConfiguration for {bad:?}, got {err:?}"
        );
    }
    Ok(())
}

/// Generate `xml` with `with_exponent_links("exponent")` and return the
/// error it must produce.
fn exponent_link_error(xml: &str) -> GenerateError {
    let schema = Schema::from_ir(parse(SchemaSource::xml(xml)).expect("schema parses"));
    let mut g = Generator::new(GenerationConfig::new("ext_pair").with_exponent_links("exponent"));
    g.generate(&schema).expect_err("reference must fail")
}

const PAIR_TYPES: &str = r#"<types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="Pair">
      <type name="lo" primitiveType="uint32"/>
      <type name="hi" primitiveType="uint32"/>
    </composite>
  </types>"#;

/// B2 fails closed: a dangling target, a non-integer target, a non-integer
/// carrying field and a constant field are all `ExtensionReference` at
/// generation time — never a silently skipped helper.
#[test]
fn exponent_links_reject_invalid_references() -> Result<(), Box<dyn std::error::Error>> {
    // Dangling: no sibling field of that name in the block.
    let err = exponent_link_error(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema xmlns:x="urn:example" package="ext" id="9" version="0" byteOrder="littleEndian">
  {PAIR_TYPES}
  <message name="Filter" id="1">
    <field name="maxQty" id="1" type="uint64" x:exponent="noSuchField"/>
  </message>
</messageSchema>"#
    ));
    assert!(
        matches!(
            &err,
            GenerateError::ExtensionReference { field, target, .. }
                if field == "maxQty" && target == "noSuchField"
        ),
        "dangling target: {err:?}"
    );

    // The target exists but is a composite, not an integer scalar.
    let err = exponent_link_error(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema xmlns:x="urn:example" package="ext" id="9" version="0" byteOrder="littleEndian">
  {PAIR_TYPES}
  <message name="Filter" id="1">
    <field name="maxQty" id="1" type="uint64" x:exponent="px"/>
    <field name="px" id="2" type="Pair"/>
  </message>
</messageSchema>"#
    ));
    assert!(
        matches!(&err, GenerateError::ExtensionReference { field, reason, .. }
            if field == "maxQty" && reason.contains("integer")),
        "non-integer target: {err:?}"
    );

    // The carrying field itself is not an integer scalar.
    let err = exponent_link_error(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema xmlns:x="urn:example" package="ext" id="9" version="0" byteOrder="littleEndian">
  {PAIR_TYPES}
  <message name="Filter" id="1">
    <field name="px" id="1" type="Pair" x:exponent="maxQty"/>
    <field name="maxQty" id="2" type="uint64"/>
  </message>
</messageSchema>"#
    ));
    assert!(
        matches!(&err, GenerateError::ExtensionReference { field, .. }
            if field == "px"),
        "non-integer carrier: {err:?}"
    );

    // A constant has no varying value to pair.
    let err = exponent_link_error(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema xmlns:x="urn:example" package="ext" id="9" version="0" byteOrder="littleEndian">
  {PAIR_TYPES}
  <message name="Filter" id="1">
    <field name="k" id="1" type="uint64" presence="constant" constantValue="7" x:exponent="maxQty"/>
    <field name="maxQty" id="2" type="uint64"/>
  </message>
</messageSchema>"#
    ));
    assert!(
        matches!(&err, GenerateError::ExtensionReference { field, reason, .. }
            if field == "k" && reason.contains("constant")),
        "constant carrier: {err:?}"
    );
    Ok(())
}

/// B2 positive: a minimal valid pair compiles, the helper mirrors the two
/// accessors' types (`Option` for a version-gated mantissa).
#[test]
fn exponent_link_helper_compiles_for_a_valid_pair() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate_from_xml(
        &format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema xmlns:x="urn:example" package="ext" id="9" version="1" byteOrder="littleEndian">
  {PAIR_TYPES}
  <message name="Filter" id="1">
    <field name="maxQty" id="1" type="uint64" x:exponent="qtyExponent"/>
    <field name="qtyExponent" id="2" type="int8"/>
    <field name="maxPx" id="3" type="uint64" x:exponent="priceExponent" sinceVersion="1"/>
    <field name="priceExponent" id="4" type="int8"/>
  </message>
</messageSchema>"#
        ),
        "ext_pair_ok",
        |c| c.with_exponent_links("exponent"),
    )?;
    compile_and_run(
        "ext_pair_ok",
        &src,
        r#"
        // The helpers' return types are exactly the accessors' return
        // types; mismatched tuple elements fail this definition.
        fn pair_types(d: &FilterDecoder<'_>) -> ((u64, i8), (Option<u64>, i8)) {
            (d.max_qty_with_exponent(), d.max_px_with_exponent())
        }
        "#,
    );
    Ok(())
}

/// B3: `with_field_extension_defaults` puts stringly `Option<&'static str>`
/// defaults and `bool` omit-null flags on the message's `{Name}Schema`
/// unit and on the group decoders that own group-entry fields — at all
/// three codegen locations, values exactly as the schema wrote them.
#[test]
fn field_extension_defaults_render_stringly_consts() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate("ext_defaults", |c| {
        c.with_field_extension_defaults("jsonDefaultValue", "jsonOmitNull")
    })?;
    compile_and_run(
        "ext_defaults",
        &src,
        r#"
        assert_eq!(FlatSchema::X_DEF_V0_SCHEMA_DEFAULT, Some("-1"));
        assert_eq!(FlatSchema::X_DEF_V1_SCHEMA_DEFAULT, Some("-1"));
        assert!(FlatSchema::X_DEF_V0_SCHEMA_OMIT_NULL);
        assert_eq!(NestedRowsDecoder::X_DEF_V0_SCHEMA_DEFAULT, Some("-1"));
        assert!(NestedRowsDecoder::X_DEF_V0_SCHEMA_OMIT_NULL);
        assert_eq!(NestedRowsCellsDecoder::X_DEF_V0_SCHEMA_DEFAULT, Some("-1"));
        assert!(NestedRowsCellsDecoder::X_DEF_V1_SCHEMA_OMIT_NULL);
        "#,
    );
    Ok(())
}

/// B3 emits nothing for fields that carry neither attribute, so a schema
/// without the marks is byte-identical with the knob on.
#[test]
fn field_extension_defaults_stay_silent_without_marks() -> Result<(), Box<dyn std::error::Error>> {
    let plain = generate_from_xml(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="ext" id="9" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Quiet" id="1">
    <field name="qty" id="1" type="uint64"/>
  </message>
</messageSchema>"#,
        "ext_defaults_quiet",
        |c| c.with_field_extension_defaults("jsonDefaultValue", "jsonOmitNull"),
    )?;
    let bare = generate_from_xml(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="ext" id="9" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Quiet" id="1">
    <field name="qty" id="1" type="uint64"/>
  </message>
</messageSchema>"#,
        "ext_defaults_bare",
        |c| c,
    )?;
    assert_eq!(
        plain, bare,
        "a schema with no extension marks must generate byte-identical output"
    );
    Ok(())
}

/// All three knobs at once: each stays independently gated, their emissions
/// do not collide, and the module compiles and runs.
#[test]
fn all_three_extension_knobs_together() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate("ext_all", |c| {
        c.with_variant_labels("jsonValue", "as_str")
            .with_exponent_links("exponent")
            .with_field_extension_defaults("jsonDefaultValue", "jsonOmitNull")
    })?;
    compile_and_run(
        "ext_all",
        &src,
        r#"
        assert_eq!(Model::A.as_str(), "alpha");
        assert_eq!(format!("{}", BooleanType::T), "t-label");
        assert_eq!(FlatSchema::X_DEF_V0_SCHEMA_DEFAULT, Some("-1"));
        assert!(NestedRowsCellsDecoder::X_DEF_V0_SCHEMA_OMIT_NULL);
        fn pair(d: &NestedRowsEntryDecoder<'_>) -> (u64, i8) {
            d.x_link_v0_with_exponent()
        }
        "#,
    );
    Ok(())
}

/// B2 reachability: a group field whose exponent attribute names a field in
/// the *enclosing message* (the venue's dominant pattern — exponents declared
/// once at message level, mantissas inside groups) gets no helper: the entry
/// decoder cannot reach message-level state. The reference is recognised, not
/// an error, because it exists somewhere in the message; a name that exists
/// nowhere still fails below.
#[test]
fn exponent_links_skip_cross_block_targets() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema xmlns:x="urn:example" package="ext" id="9" version="0" byteOrder="littleEndian">
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
  <message name="Depth" id="1">
    <field name="priceExponent" id="1" type="int8"/>
    <group name="bids" id="2" dimensionType="groupSizeEncoding">
      <field name="px" id="1" type="uint64" x:exponent="priceExponent"/>
      <field name="qty" id="2" type="uint64" x:exponent="qtyLocal"/>
      <field name="qtyLocal" id="3" type="int8"/>
    </group>
  </message>
</messageSchema>"#;
    let src = generate_from_xml(xml, "ext_cross", |c| c.with_exponent_links("exponent"))?;
    // The unreachable cross-block ref is skipped — no helper for `px` …
    assert!(
        !src.contains("px_with_exponent"),
        "a group field naming a message-level exponent gets no helper:\n{src}"
    );
    // … while a same-block pair on the same entry still emits.
    assert!(
        src.contains("qty_with_exponent"),
        "a same-block group pair still emits:\n{src}"
    );
    compile_and_run(
        "ext_cross",
        &src,
        r#"
        fn pair(d: &BidsEntryDecoder<'_>) -> (u64, i8) { d.qty_with_exponent() }
        "#,
    );
    Ok(())
}

/// B2 member types: a fixed integer *array* (the venue's `mantissa128` — a
/// 16-byte little-endian signed magnitude) is a valid pair member; the helper
/// returns exactly what the accessors return, `([u8; 16], i8)`.
#[test]
fn exponent_links_pair_fixed_integer_arrays() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema xmlns:x="urn:example" package="ext" id="9" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="mantissa128" length="16" primitiveType="uint8"/>
  </types>
  <message name="Ticker" id="1">
    <field name="qtyExponent" id="1" type="int8"/>
    <field name="volume" id="2" type="mantissa128" x:exponent="qtyExponent"/>
  </message>
</messageSchema>"#;
    let src = generate_from_xml(xml, "ext_arr", |c| c.with_exponent_links("exponent"))?;
    compile_and_run(
        "ext_arr",
        &src,
        r#"
        fn pair(d: &TickerDecoder<'_>) -> ([u8; 16], i8) { d.volume_with_exponent() }
        "#,
    );
    Ok(())
}
