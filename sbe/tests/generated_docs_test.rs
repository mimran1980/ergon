//! Verify that every public struct and enum in the generated golden has a
//! real `#[doc = "..."]` attribute, regardless of intervening attributes.
//!
//! This replaces the fragile shell-script grep in the justfile's
//! `check-generated-docs` recipe.
#![allow(clippy::expect_used)]

use std::path::Path;

/// Walk public items in a Rust source file and return `(name, has_doc)` for
/// every `pub struct` and `pub enum`.
fn check_docs(source: &str) -> Vec<(String, bool)> {
    let file = syn::parse_file(source).expect("golden must parse");
    let mut results = Vec::new();
    for item in &file.items {
        match item {
            syn::Item::Struct(s) if is_public(&s.vis) => {
                let has_doc = has_doc_attr(&s.attrs);
                results.push((s.ident.to_string(), has_doc));
            }
            syn::Item::Enum(e) if is_public(&e.vis) => {
                let has_doc = has_doc_attr(&e.attrs);
                results.push((e.ident.to_string(), has_doc));
            }
            _ => {}
        }
    }
    results
}

const fn is_public(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

/// Check whether any attribute is a `#[doc = "..."]` — we scan all attrs,
/// not just the immediately preceding one, because `#[must_use]`,
/// `#[derive(...)]`, `#[inline]`, etc. may sit between the doc comment
/// and the item.
fn has_doc_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.meta.path().is_ident("doc"))
}

#[test]
fn golden_car_example_docs() -> Result<(), Box<dyn std::error::Error>> {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/car_example.rs");
    let source = std::fs::read_to_string(&golden)?;
    let items = check_docs(&source);
    let missing: Vec<_> = items.iter().filter(|(_, doc)| !doc).collect();
    assert!(
        missing.is_empty(),
        "{} public item(s) missing doc comment in golden: {missing:?}\n\
         Add doc comments in the codegen emitter and re-run `just update-golden`.",
        missing.len(),
    );
    assert!(!items.is_empty(), "golden must contain public items");
    Ok(())
}

#[test]
fn fixture_missing_doc_fails() {
    let src = "pub struct Undocumented;";
    let items = check_docs(src);
    let missing: Vec<_> = items.iter().filter(|(_, doc)| !doc).collect();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].0, "Undocumented");
}

#[test]
fn fixture_documented_passes() {
    let src = "/// A documented struct.\n#[must_use]\npub struct Documented;";
    let items = check_docs(src);
    assert!(
        items.iter().all(|(_, doc)| *doc),
        "documented struct should pass"
    );
}

#[test]
fn fixture_intervening_attrs_still_pass() {
    let src =
        "#[derive(Debug)]\n#[must_use = \"reason\"]\n/// The docs are here.\npub struct LateDoc;";
    let items = check_docs(src);
    assert!(
        items.iter().all(|(_, doc)| *doc),
        "doc after intervening attrs should still count"
    );
}
