//! The example hooks in `tests/common/ext_recipes.rs` (labels and field pairs)
//! and the `with_extension_consts` switch, run on the codegen matrix fixture
//! and small schemas.
//!
//! Compiling the generated module is the load-bearing assertion (the matrix
//! rows prove the same recipes across all three codegen locations); the
//! runtime asserts here name the value that broke.

#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    unused
)]
mod common;
#[path = "common/ext_recipes.rs"]
mod ext_recipes;
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
) -> Result<String, GenerateError> {
    let schema = Schema::from_ir(parse(SchemaSource::xml(xml)).expect("schema parses"));
    let mut g = Generator::new(configure(GenerationConfig::new(module)));
    let ms = g.generate(&schema)?;
    Ok(ms.modules().next().expect("one module").source.clone())
}

/// The labels recipe adds a label method, falling back to the variant name
/// for unmarked values; `as_str` and `Display` keep the variant name.
#[test]
fn labels_render_labels_with_variant_fallback() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate("ext_labels", |c| {
        c.with_try_hook(ext_recipes::labels("jsonValue", "label"))
    })?;
    compile_and_run(
        "ext_labels",
        &src,
        r#"
        assert_eq!(Model::A.label(), "alpha");
        assert_eq!(Model::B.label(), "B");
        assert_eq!(Model::NullVal.label(), "NullVal");
        assert_eq!(BooleanType::F.label(), "f-label");
        assert_eq!(BooleanType::T.label(), "t-label");
        // The built-in rendering never changes.
        assert_eq!(Model::A.as_str(), "A");
        assert_eq!(format!("{}", Model::A), "A");
        "#,
    );
    Ok(())
}

/// The label fallback is the PascalCase variant name, what the built-in
/// `as_str` returns and `FromStr` parses.
#[test]
fn labels_fallback_matches_the_builtin_as_str() -> Result<(), Box<dyn std::error::Error>> {
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
        c.with_try_hook(ext_recipes::labels("jsonValue", "json_label"))
    })?;
    compile_and_run(
        "ext_label_fb",
        &src,
        r#"
        assert_eq!(Side::MinQty.json_label(), "MinQty");
        assert_eq!(Side::MinQty.json_label(), Side::MinQty.as_str());
        assert_eq!("MinQty".parse::<Side>().ok(), Some(Side::MinQty));
        assert_eq!(Side::MaxAsset.json_label(), "MAX_ASSET");
        assert_eq!(Side::MaxAsset.as_str(), "MaxAsset");
        "#,
    );
    Ok(())
}

/// Labels on a set: choices fall back to their schema name, and anything
/// that is not exactly one choice bit returns "UNKNOWN".
#[test]
fn labels_on_a_set_match_single_bits() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
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
    let src = generate_from_xml(xml, "ext_set_labels", |c| {
        c.with_try_hook(ext_recipes::labels("jsonValue", "label"))
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

/// A method name that is not an identifier fails generation in the hook
/// (`GenerateError::Hook`); a keyword fails the generated-source check.
#[test]
fn labels_reject_a_non_identifier_method() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(SchemaSource::file(&matrix_schema()))?);
    for bad in ["not an ident", ""] {
        let mut g = Generator::new(
            GenerationConfig::new("ext_bad").with_try_hook(ext_recipes::labels("jsonValue", bad)),
        );
        let err = g.generate(&schema).expect_err("invalid method must fail");
        assert!(
            matches!(&err, GenerateError::Hook { item, reason }
                if item.starts_with("Enum ") && reason.contains("identifier")),
            "expected GenerateError::Hook for {bad:?}, got {err:?}"
        );
    }
    let mut g = Generator::new(
        GenerationConfig::new("ext_bad").with_try_hook(ext_recipes::labels("jsonValue", "type")),
    );
    let err = g.generate(&schema).expect_err("a keyword method must fail");
    assert!(
        matches!(err, GenerateError::InvalidGeneratedSource { .. }),
        "expected InvalidGeneratedSource for a keyword, got {err:?}"
    );
    Ok(())
}

const PAIR_TYPES: &str = r#"<types>
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
    <composite name="Pair">
      <type name="lo" primitiveType="uint32"/>
      <type name="hi" primitiveType="uint32"/>
    </composite>
    <type name="mantissa128" length="16" primitiveType="uint8"/>
  </types>"#;

fn pair_schema(version: u16, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema xmlns:x="urn:example" package="ext" id="9" version="{version}" byteOrder="littleEndian">
  {PAIR_TYPES}
  {body}
</messageSchema>"#
    )
}

/// A target that names no field anywhere fails generation, naming the
/// field and the target.
#[test]
fn pairs_reject_a_dangling_target() {
    let xml = pair_schema(
        0,
        r#"<message name="Filter" id="1">
    <field name="maxQty" id="1" type="uint64" x:exponent="noSuchField"/>
  </message>"#,
    );
    let err = generate_from_xml(&xml, "ext_pair", |c| {
        c.with_try_hook(ext_recipes::pairs("exponent", "with_exponent"))
    })
    .expect_err("a dangling target must fail");
    assert!(
        matches!(&err, GenerateError::Hook { item, reason }
            if item == "MessageDecoder FilterDecoder"
                && reason.contains("maxQty") && reason.contains("noSuchField")),
        "dangling target: {err:?}"
    );
}

/// The helper's tuple is exactly what the two accessors return: plain,
/// version-gated (`Option`), a `[u8; 16]` array, a composite and a constant.
#[test]
fn pairs_return_the_accessors_exact_types() -> Result<(), Box<dyn std::error::Error>> {
    let xml = pair_schema(
        1,
        r#"<message name="Filter" id="1">
    <field name="maxQty" id="1" type="uint64" x:exponent="qtyExponent"/>
    <field name="qtyExponent" id="2" type="int8"/>
    <field name="maxPx" id="3" type="uint64" x:exponent="priceExponent" sinceVersion="1"/>
    <field name="priceExponent" id="4" type="int8"/>
    <field name="volume" id="5" type="mantissa128" x:exponent="qtyExponent"/>
    <field name="span" id="6" type="uint64" x:exponent="px"/>
    <field name="px" id="7" type="Pair"/>
    <field name="k" id="8" type="uint64" presence="constant" constantValue="7" x:exponent="qtyExponent"/>
  </message>"#,
    );
    let src = generate_from_xml(&xml, "ext_pair_ok", |c| {
        c.with_try_hook(ext_recipes::pairs("exponent", "with_exponent"))
    })?;
    compile_and_run(
        "ext_pair_ok",
        &src,
        r#"
        fn pair_types<'a>(d: &'a FilterDecoder<'a>)
            -> ((u64, i8), (Option<u64>, i8), ([u8; 16], i8), (u64, PairDecoder<'a>), (u64, i8)) {
            (
                d.max_qty_with_exponent(),
                d.max_px_with_exponent(),
                d.volume_with_exponent(),
                d.span_with_exponent(),
                d.k_with_exponent(),
            )
        }
        "#,
    );
    Ok(())
}

/// A group field whose target is in the enclosing message gets a helper
/// that takes the enclosing decoder; a same-block pair on the same entry
/// takes none. Encoded values round-trip through both.
#[test]
fn pairs_reach_targets_in_enclosing_blocks() -> Result<(), Box<dyn std::error::Error>> {
    let xml = pair_schema(
        0,
        r#"<message name="Depth" id="1">
    <field name="priceExponent" id="1" type="int8"/>
    <group name="bids" id="2" dimensionType="groupSizeEncoding">
      <field name="px" id="1" type="int64" x:exponent="priceExponent"/>
      <field name="qty" id="2" type="int64" x:exponent="qtyLocal"/>
      <field name="qtyLocal" id="3" type="int8"/>
    </group>
  </message>"#,
    );
    let src = generate_from_xml(&xml, "ext_cross", |c| {
        c.with_try_hook(ext_recipes::pairs("exponent", "with_exponent"))
    })?;
    compile_and_run(
        "ext_cross",
        &src,
        r#"
        let mut buf = vec![0u8; 128];
        let enc = DepthEncoder::try_wrap_and_apply_header(&mut buf, 0).expect("wrap")
            .fixed(&DepthFixedFields { price_exponent: -2 });
        let done = enc.bids(1, |g| {
            g.add(|mut e| { e.px(-12345).qty(7).qty_local(-1); Ok(()) })?;
            Ok(())
        }).expect("bids");
        let dec = DepthDecoder::try_from(done.as_bytes_with_header()).expect("decode");
        let entry = dec.bids().expect("group").next().expect("one entry");
        assert_eq!(entry.px_with_exponent(&dec), (-12345, -2));
        assert_eq!(entry.qty_with_exponent(), (7, -1));
        "#,
    );
    Ok(())
}

/// `with_extension_consts` on the matrix fixture: the schema's own strings at all
/// three codegen locations.
#[test]
fn extension_consts_on_the_matrix_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate("ext_defaults", |c| c.with_extension_consts(true))?;
    compile_and_run(
        "ext_defaults",
        &src,
        r#"
        assert_eq!(FlatSchema::X_DEF_V0_JSON_DEFAULT_VALUE, "-1");
        assert_eq!(FlatSchema::X_DEF_V1_JSON_DEFAULT_VALUE, "-1");
        assert_eq!(FlatSchema::X_DEF_V0_JSON_OMIT_NULL, "true");
        assert_eq!(NestedRowsDecoder::X_DEF_V0_JSON_DEFAULT_VALUE, "-1");
        assert_eq!(NestedRowsDecoder::X_DEF_V0_JSON_OMIT_NULL, "true");
        assert_eq!(NestedRowsCellsDecoder::X_DEF_V0_JSON_DEFAULT_VALUE, "-1");
        assert_eq!(NestedRowsCellsDecoder::X_DEF_V1_JSON_OMIT_NULL, "true");
        assert_eq!(Model::A_JSON_VALUE, "alpha");
        "#,
    );
    Ok(())
}

/// The recipes emit nothing for a schema without marks: byte-identical
/// output with or without them registered.
#[test]
fn recipes_stay_silent_without_marks() -> Result<(), Box<dyn std::error::Error>> {
    let xml = pair_schema(
        0,
        r#"<message name="Quiet" id="1">
    <field name="qty" id="1" type="uint64"/>
    <group name="rows" id="2" dimensionType="groupSizeEncoding">
      <field name="px" id="1" type="int64"/>
    </group>
  </message>"#,
    );
    let with = generate_from_xml(&xml, "ext_quiet", |c| {
        c.with_try_hook(ext_recipes::labels("jsonValue", "label"))
            .with_try_hook(ext_recipes::pairs("exponent", "with_exponent"))
            .with_extension_consts(true)
    })?;
    let bare = generate_from_xml(&xml, "ext_quiet", |c| c)?;
    assert_eq!(with, bare, "no marks, no change");
    Ok(())
}

/// Both recipes and the consts switch at once: independently gated, no
/// collisions, the module compiles and runs.
#[test]
fn recipes_and_consts_together() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate("ext_all", |c| {
        c.with_try_hook(ext_recipes::labels("jsonValue", "label"))
            .with_try_hook(ext_recipes::pairs("exponent", "with_exponent"))
            .with_extension_consts(true)
    })?;
    compile_and_run(
        "ext_all",
        &src,
        r#"
        assert_eq!(Model::A.label(), "alpha");
        assert_eq!(format!("{}", BooleanType::T), "T");
        assert_eq!(FlatSchema::X_DEF_V0_JSON_DEFAULT_VALUE, "-1");
        assert_eq!(NestedRowsCellsDecoder::X_DEF_V0_JSON_OMIT_NULL, "true");
        fn pair(d: &NestedRowsEntryDecoder<'_>) -> (u64, i8) {
            d.x_link_v0_with_exponent()
        }
        "#,
    );
    Ok(())
}

/// Generation is deterministic with the recipes registered.
#[test]
fn recipes_generate_deterministically() -> Result<(), Box<dyn std::error::Error>> {
    let run = || {
        generate("ext_det", |c| {
            c.with_try_hook(ext_recipes::labels("jsonValue", "label"))
                .with_try_hook(ext_recipes::pairs("exponent", "with_exponent"))
                .with_extension_consts(true)
        })
    };
    assert_eq!(run()?, run()?);
    Ok(())
}
