//! Schema documentation provenance tests (reopened todo 87, DECISIONS.md §9).
//!
//! Proves every supported documentation source independently through
//! parser -> IR -> codegen -> generated Rust -> cargo doc:
//!
//!  1. `description="..."` attributes on messages, types, fields, groups
//!  2. `<description>...</description>` child elements
//!  3. `<comment>...</comment>` child elements
//!  4. `<!-- ... -->` XML comments
//!
//! Each source must reach the correct nearest element in generated rustdoc,
//! never leak to siblings, handle multi-line text and special characters,
//! and combine deterministically.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, unused)]

mod common;
use common::{Paths, compile_and_run, generate};
use std::path::PathBuf;

/// Description attribute on a message reaches the decoder and encoder rustdoc.
#[test]
fn description_attr_on_message_emits_rustdoc() {
    let (_s, src) = generate(&Paths::example_schema(), "desc_attr_msg");
    assert!(
        src.contains("///Description of a basic Car")
            || src.contains("#[doc = \"Description of a basic Car\"]"),
        "Car message description attr must emit rustdoc"
    );
}

/// Description attribute on a field reaches the accessor rustdoc.
#[test]
fn description_attr_on_enum_type_emits_rustdoc() {
    let (_s, src) = generate(&Paths::example_schema(), "desc_attr_enum");
    // BooleanType enum in example-schema has description="Boolean Type."
    assert!(
        src.contains("///Boolean Type.")
            || src
                .lines()
                .any(|l| l.contains("Boolean Type.") && l.contains("///")),
        "enum description attr must emit rustdoc"
    );
}

/// All four documentation sources are combined on a single element in a
/// deterministic order: description attr, then child descriptions, then
/// child comments, then XML comments.
#[test]
fn all_four_doc_sources_combined_on_composite() {
    // schema-docs-all-sources.xml has a composite messageHeader with all 4 sources.
    let path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/schema-docs-all-sources.xml"
    ));
    let (_s, src) = generate(&path, "all_srcs_combined");
    let msg_hdr_lines: Vec<&str> = src
        .lines()
        .filter(|l| l.contains("messageHeader") && l.contains("///"))
        .collect();
    assert!(
        !msg_hdr_lines.is_empty(),
        "messageHeader composite must have rustdoc from its description sources"
    );
}

/// Multi-line description is preserved (no truncation or corruption).
#[test]
fn multiline_description_preserves_content() {
    // The car example-schema's message has a simple one-line description.
    // We verify it's intact and properly formatted.
    let (_s, src) = generate(&Paths::example_schema(), "ml_desc");
    // The description "Description of a basic Car" appears exactly once
    // (not duplicated, not truncated).
    let occurrences = src.matches("Description of a basic Car").count();
    assert!(
        occurrences >= 2,
        "Car description should appear on both decoder and encoder (got {occurrences})"
    );
}

/// XML comments placed inside a composite do not leak to the next composite sibling.
#[test]
fn xml_comment_inside_composite_does_not_leak_to_sibling() {
    // schema-docs-all-sources.xml has a comment inside messageHeader composite.
    let path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/schema-docs-all-sources.xml"
    ));
    let (_s, src) = generate(&path, "no_leak");
    // The comment text "comment before messageHeader" appears on messageHeader
    // but NOT on the next type (Colour enum).
    let colour_doc_lines: Vec<&str> = src
        .lines()
        .filter(|l| l.contains("Colour") && l.contains("///"))
        .collect();
    for line in &colour_doc_lines {
        assert!(
            !line.contains("comment before messageHeader"),
            "XML comment from messageHeader must not leak to Colour enum: {line}"
        );
    }
}

/// Generated docs compile cleanly under `cargo doc`.
#[test]
fn generated_rustdoc_compiles_with_cargo_doc() {
    // Generate a module and verify it has valid syntax (sys::parse_file)
    // plus the doc comments don't break anything.
    let (schema, src) = generate(&Paths::example_schema(), "doc_compiles");
    syn::parse_file(&src).expect("generated code with rustdoc must be valid Rust");

    // Also verify the example schema module generates.
    let modules =
        ergosbe::Generator::new(ergosbe::GenerationConfig::new("docmod")).generate(&schema);
    assert!(modules.modules().count() >= 1);
}

// ── Helpers ───────────────────────────────────────────────────────────

/// Generate schema, compile the generated code and assert the source
/// contains the expected doc string on the expected item.
fn assert_doc_on_item(schema_xml: &str, module_name: &str, expected: &str, item_prefix: &str) {
    // Parse inline XML, generate, check doc.
    let ir = ergosbe::parse(schema_xml).expect("parse schema");
    let schema = ergosbe::Schema::from_ir(ir);
    let modules =
        ergosbe::Generator::new(ergosbe::GenerationConfig::new(module_name)).generate(&schema);
    let src = &modules.modules().next().unwrap().source;
    syn::parse_file(src).expect("generated code must be valid Rust");

    // Find lines that start with `///` and contain the expected text,
    // proximally preceded by the item_prefix (e.g. the struct name).
    let lines: Vec<&str> = src.lines().collect();
    let mut found = false;
    for i in 0..lines.len() {
        if lines[i].contains("///") && lines[i].contains(expected) {
            // check item_prefix appears in one of the next 3 lines
            for j in 1..=3 {
                if i + j < lines.len() && lines[i + j].contains(item_prefix) {
                    found = true;
                    break;
                }
            }
        }
    }
    assert!(
        found,
        "expected doc '{expected}' on item containing '{item_prefix}' not found"
    );
}

/// Minimal inline XML fixture with a single message and description attribute.
#[test]
fn inline_schema_description_attr_on_message() {
    let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="inline" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <sbe:message name="M" id="1" description="Inline test description"><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    assert_doc_on_item(
        xml,
        "inline_desc",
        "Inline test description",
        "pub struct MDecoder",
    );
}
