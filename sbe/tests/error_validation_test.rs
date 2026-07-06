//! Error validation tests: assert that invalid SBE schemas produce specific
//! [`ParseError`] / [`ResolveError`] variants rather than panicking or
//! silently succeeding.
//!
//! Also validates that errors render with miette source context.

use std::path::PathBuf;

/// Walk up to find the workspace root (where the top-level Cargo.toml lives).
fn workspace_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap();
    loop {
        if dir.join("Cargo.toml").exists() {
            // Check known layouts: sbe/, ergosbe/, crates/ergosbe/
            if dir.join("sbe").exists() || dir.join("ergosbe").exists() || dir.join("crates/ergosbe").exists() {
                return dir;
            }
        }
        assert!(
            dir.pop(),
            "cannot find workspace root from {:?}",
            std::env::current_dir()
        );
    }
}

fn crate_root() -> PathBuf {
    let ws = workspace_root();
    if ws.join("crates/ergosbe").exists() {
        ws.join("crates/ergosbe")
    } else {
        ws.join("sbe")
    }
}

fn fixture_path(name: &str) -> PathBuf {
    crate_root()
        .join("tests")
        .join("fixtures")
        .join("schemas")
        .join(name)
}

// ── missing-required-attr.xml ──────────────────────────────────────────────
//
// A <field> element without the required @name attribute.
// Expected: ParseError::Missing { what: "field @name", .. }

#[test]
fn missing_required_attr_returns_missing_error() {
    let path = fixture_path("missing-required-attr.xml");
    let err = ergosbe::parse_file(&path).unwrap_err();
    assert!(
        matches!(&err, ergosbe::ParseError::Missing { what, .. } if what == "field @name"),
        "expected Missing(field @name), got {err:?}"
    );
    // The error message should mention "field @name"
    let msg = format!("{err}");
    assert!(
        msg.contains("field @name"),
        "error message should mention the missing attribute: {msg}"
    );
}

// ── invalid-type-ref.xml ───────────────────────────────────────────────────
//
// A <field> referencing a type name that has no definition in the schema.
// Expected: ParseError::Invalid { what: "primitive type", value: "NonExistentType", .. }

#[test]
fn invalid_type_ref_returns_invalid_error() {
    let path = fixture_path("invalid-type-ref.xml");
    let err = ergosbe::parse_file(&path).unwrap_err();
    assert!(
        matches!(&err, ergosbe::ParseError::Invalid { what, value, .. }
            if what == "primitive type" && value == "NonExistentType"),
        "expected Invalid(primitive type, NonExistentType), got {err:?}"
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("NonExistentType"),
        "error message should name the bad type: {msg}"
    );
}

// ── duplicate-message-id.xml ───────────────────────────────────────────────
//
// Two <message> elements sharing the same template id.
// Expected: ResolveError::DuplicateTemplateId { id: 1, name: "AnotherMessageWithId1" }

#[test]
fn duplicate_message_id_returns_duplicate_template_id() {
    let path = fixture_path("duplicate-message-id.xml");
    let err = ergosbe::parse_file(&path).unwrap_err();
    assert!(
        matches!(&err, ergosbe::ParseError::Resolve { error, .. }
            if matches!(error.as_ref(), ergosbe::ResolveError::DuplicateTemplateId { id: 1, name, .. } if name == "AnotherMessageWithId1")),
        "expected Resolve(DuplicateTemplateId), got {err:?}"
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("duplicate template id"),
        "error message should mention duplicate template id: {msg}"
    );
    assert!(
        msg.contains("AnotherMessageWithId1"),
        "error message should name the duplicate message: {msg}"
    );
}

// ── duplicate-message-id miette rendering ─────────────────────────────────────
//
// Verify that a ResolveError rendered through miette includes source context.

#[test]
fn duplicate_message_id_renders_miette_diagnostic() {
    let path = fixture_path("duplicate-message-id.xml");
    let err = ergosbe::parse_file(&path).unwrap_err();

    // Verify the error carries source_code via the Diagnostic trait.
    use miette::Diagnostic;
    assert!(
        err.source_code().is_some(),
        "expected ParseError to carry source_code, got None"
    );

    let report = miette::Report::from(err);
    let rendered = format!("{report:?}");

    // The rendered output should contain the error message.
    assert!(
        rendered.contains("duplicate template id"),
        "rendered output should contain the error message, got:\n{rendered}"
    );

    // It should name the duplicate message.
    assert!(
        rendered.contains("AnotherMessageWithId1"),
        "rendered output should name the duplicate message, got:\n{rendered}"
    );

    // The error code should be present in the rendered output.
    assert!(
        rendered.contains("ergosbe::schema_parse::resolve"),
        "rendered output should include error code, got:\n{rendered}"
    );
}

// ── version-gap.xml ────────────────────────────────────────────────────────
//
// A <field> with sinceVersion=5 when the schema version is 1.
// Expected: ResolveError::SinceVersionBeyondSchema { version: 5, schema_version: 1, .. }

#[test]
fn version_gap_returns_since_version_beyond_schema() {
    let path = fixture_path("version-gap.xml");
    let err = ergosbe::parse_file(&path).unwrap_err();
    assert!(
        matches!(&err, ergosbe::ParseError::Resolve { error, .. }
            if matches!(error.as_ref(), ergosbe::ResolveError::SinceVersionBeyondSchema { version: 5, schema_version: 1, .. })),
        "expected Resolve(SinceVersionBeyondSchema), got {err:?}"
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("sinceVersion 5"),
        "error message should mention sinceVersion 5: {msg}"
    );
    assert!(
        msg.contains("schema version 1"),
        "error message should mention schema version 1: {msg}"
    );
}

// ── version-gap miette rendering ──────────────────────────────────────────────
//
// Verify that a SinceVersionBeyondSchema error rendered through miette includes
// source context.

#[test]
fn version_gap_renders_miette_diagnostic() {
    let path = fixture_path("version-gap.xml");
    let err = ergosbe::parse_file(&path).unwrap_err();

    use miette::Diagnostic;
    assert!(
        err.source_code().is_some(),
        "expected ParseError to carry source_code, got None"
    );

    let report = miette::Report::from(err);
    let rendered = format!("{report:?}");

    assert!(
        rendered.contains("schema version 1"),
        "rendered output should mention schema version, got:\n{rendered}"
    );

    // The error code should be present in the rendered output.
    assert!(
        rendered.contains("ergosbe::schema_parse::resolve"),
        "rendered output should include error code, got:\n{rendered}"
    );
}

// ── invalid-enum-value.xml ─────────────────────────────────────────────────
//
// An <enum> whose encodingType references a type not defined in the schema.
// Expected: ParseError::Invalid { what: "enum encodingType", value: "NonExistentEncodingType", .. }

#[test]
fn invalid_enum_encoding_type_returns_invalid_error() {
    let path = fixture_path("invalid-enum-value.xml");
    let err = ergosbe::parse_file(&path).unwrap_err();
    assert!(
        matches!(&err, ergosbe::ParseError::Invalid { what, value, .. }
            if what == "enum encodingType" && value == "NonExistentEncodingType"),
        "expected Invalid(enum encodingType, NonExistentEncodingType), got {err:?}"
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("enum encodingType"),
        "error message should mention enum encodingType: {msg}"
    );
    assert!(
        msg.contains("NonExistentEncodingType"),
        "error message should name the bad type: {msg}"
    );
}
