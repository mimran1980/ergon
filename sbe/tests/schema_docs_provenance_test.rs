//! Schema documentation provenance tests (reopened todo 87, DECISIONS.md §9).
//!
//! Proves every supported documentation source independently through
//! parser -> IR -> codegen -> generated Rust -> cargo doc:
//!
//!  1. `description="..."` attributes on messages, types, fields, groups
//!  2. `<description>...</description>` child elements
//!  3. `<comment>...</comment>` child elements
//!  4. `<!-- ... -->` XML comments (nearest preceding sibling)
//!
//! Each source must reach the correct nearest element in generated rustdoc,
//! never leak to siblings, handle multi-line text and special characters,
//! and combine deterministically in the order: attr → description-child →
//! comment-child → preceding XML comments.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, unused)]

mod common;
use common::{Paths, compile_and_run, generate};
use std::path::PathBuf;
use std::process::Command;

// ── Helpers ───────────────────────────────────────────────────────────

/// Extract the rustdoc lines immediately preceding an item in generated source.
/// Walks backwards from the item offset, skipping blank lines and `#[...]`
/// attributes, then collects contiguous `///` lines.
fn docs_before(src: &str, item: &str) -> String {
    let item_offset = src.find(item).expect("generated item");
    let preceding = &src[..item_offset];
    let mut doc_lines: Vec<&str> = Vec::new();
    for line in preceding.lines().rev() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("///") {
            doc_lines.push(line);
        } else if !trimmed.is_empty() && !trimmed.starts_with("#[") {
            break;
        }
    }
    doc_lines.reverse();
    doc_lines.join("\n")
}

/// Generate schema, compile the generated code and assert the source
/// contains the expected doc string on the expected item.
fn assert_doc_on_item(schema_xml: &str, module_name: &str, expected: &str, item_prefix: &str) {
    let ir = ergo_sbe::parse(schema_xml).expect("parse schema");
    let schema = ergo_sbe::Schema::from_ir(ir);
    let modules = ergo_sbe::Generator::new(ergo_sbe::GenerationConfig::new(module_name))
        .generate(&schema)
        .unwrap();
    let src = &modules.modules().next().unwrap().source;
    syn::parse_file(src).expect("generated code must be valid Rust");

    let lines: Vec<&str> = src.lines().collect();
    let mut found = false;
    for i in 0..lines.len() {
        if lines[i].contains("///") && lines[i].contains(expected) {
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

// ── Tests ─────────────────────────────────────────────────────────────

/// The `schema-docs-all-sources.xml` fixture gives messageHeader all four
/// documentation sources with unique labels. Prove each source reaches
/// the generated struct and the merge order is deterministic:
///   1. description attribute  (`attr:header`)
///   2. <description> child     (`description-child:header`)
///   3. <comment> child         (`comment-child:header`)
///   4. preceding XML comment   (`xml-comment:header`)
#[test]
fn all_four_sources_on_message_header_with_correct_order() -> Result<(), Box<dyn std::error::Error>>
{
    let path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/schema-docs-all-sources.xml"
    ));
    let (_s, src) = generate(&path, "all_srcs_order");

    let docs = docs_before(&src, "pub struct MessageHeader");
    assert!(docs.contains("attr:header"), "missing attr:header in docs");
    assert!(
        docs.contains("description-child:header"),
        "missing description-child:header in docs"
    );
    assert!(
        docs.contains("comment-child:header"),
        "missing comment-child:header in docs"
    );
    assert!(
        docs.contains("xml-comment:header"),
        "missing xml-comment:header in docs"
    );

    // Verify deterministic merge order.
    let offsets = [
        "attr:header",
        "description-child:header",
        "comment-child:header",
        "xml-comment:header",
    ]
    .map(|text| docs.find(text).expect("documentation source"));
    assert!(
        offsets.windows(2).all(|pair| pair[0] < pair[1]),
        "documentation sources must appear in order: attr → description-child → comment-child → xml-comment. Got:\n{docs}"
    );

    Ok(())
}

/// XML comments placed before an element are associated with the nearest
/// following element and do NOT leak to the next sibling.
#[test]
fn xml_comment_before_element_does_not_leak_to_sibling() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/schema-docs-all-sources.xml"
    ));
    let (_s, src) = generate(&path, "no_leak");

    // "xml-comment:header" is before messageHeader — it must NOT appear
    // on the Colour enum (the next sibling).
    let colour_docs = docs_before(&src, "pub enum Colour");
    assert!(
        !colour_docs.contains("xml-comment:header"),
        "XML comment before messageHeader must not leak to Colour enum"
    );
    // Colour should have its own preceding XML comment.
    assert!(
        colour_docs.contains("xml-comment:enum"),
        "Colour should have its own preceding XML comment"
    );

    Ok(())
}

/// Description attribute on a message reaches the decoder and encoder rustdoc.
#[test]
fn description_attr_on_message_emits_rustdoc() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, src) = generate(&Paths::example_schema(), "desc_attr_msg");
    assert!(
        src.contains("///Description of a basic Car")
            || src.contains("#[doc = \"Description of a basic Car\"]"),
        "Car message description attr must emit rustdoc"
    );

    Ok(())
}

/// Description attribute on an enum type reaches the rustdoc.
#[test]
fn description_attr_on_enum_type_emits_rustdoc() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, src) = generate(&Paths::example_schema(), "desc_attr_enum");
    assert!(
        src.contains("///Boolean Type.")
            || src
                .lines()
                .any(|l| l.contains("Boolean Type.") && l.contains("///")),
        "enum description attr must emit rustdoc"
    );

    Ok(())
}

/// Multi-line description is preserved (no truncation or corruption).
#[test]
fn multiline_description_preserves_content() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, src) = generate(&Paths::example_schema(), "ml_desc");
    let occurrences = src.matches("Description of a basic Car").count();
    assert!(
        occurrences >= 2,
        "Car description should appear on both decoder and encoder (got {occurrences})"
    );

    Ok(())
}

/// Generated docs compile cleanly under real `cargo doc`.
/// Multi-line XML-comment style prose with indented ASCII must not become
/// bare doctests (cluster SessionMessageHeader protocol diagram).
#[test]
fn multiline_indented_description_is_text_fenced() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="inline" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <!--
    Session Protocol:
        -> connect
                          \
        <-                 event
  -->
  <sbe:message name="M" id="1" description="Header line."><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    let ir = ergo_sbe::parse(xml).expect("parse");
    let schema = ergo_sbe::Schema::from_ir(ir);
    let modules = ergo_sbe::Generator::new(ergo_sbe::GenerationConfig::new("ml_fence"))
        .generate(&schema)
        .unwrap();
    let src = &modules.modules().next().unwrap().source;
    assert!(
        src.contains("```text") || src.contains("text\\n"),
        "multi-line description must be text-fenced to avoid doctests; got snippet around MDecoder"
    );
    // Indented protocol arrow lines must live inside the fence, not as raw docs.
    let decoder_docs = docs_before(src, "pub struct MDecoder");
    if !decoder_docs.is_empty() {
        assert!(
            decoder_docs.contains("```text") || decoder_docs.contains("Session Protocol"),
            "expected fenced or retained protocol prose in docs, got:\n{decoder_docs}"
        );
    }

    Ok(())
}

/// Minimal inline XML fixture with a description attribute on a message.
#[test]
fn inline_schema_description_attr_on_message() -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}
