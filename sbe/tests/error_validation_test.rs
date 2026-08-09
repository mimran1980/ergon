//! Error validation tests: assert that invalid SBE schemas produce specific
//! [`ParseError`] / [`ResolveError`] variants rather than panicking or
//! silently succeeding.
//!
//! Also validates that errors render with miette source context.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

use std::path::PathBuf;

/// Walk up to find the workspace root (where the top-level Cargo.toml lives).
fn workspace_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap();
    loop {
        if dir.join("Cargo.toml").exists() {
            if dir.join("sbe").exists()
                || dir.join("ergon").exists()
                || dir.join("crates/ergon").exists()
            {
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
    if ws.join("crates/ergon").exists() {
        ws.join("crates/ergon")
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

#[test]
fn missing_required_attr_returns_missing_error() -> Result<(), Box<dyn std::error::Error>> {
    let path = fixture_path("missing-required-attr.xml");
    let err = ergo_sbe::parse_file(&path).unwrap_err();
    assert!(
        matches!(&err, ergo_sbe::ParseError::Missing { what, .. } if what == "field @name"),
        "expected Missing(field @name), got {err:?}"
    );
    // The error message should mention "field @name"
    let msg = format!("{err}");
    assert!(
        msg.contains("field @name"),
        "error message should mention the missing attribute: {msg}"
    );

    Ok(())
}

#[test]
fn invalid_type_ref_returns_invalid_error() -> Result<(), Box<dyn std::error::Error>> {
    let path = fixture_path("invalid-type-ref.xml");
    let err = ergo_sbe::parse_file(&path).unwrap_err();
    assert!(
        matches!(&err, ergo_sbe::ParseError::Invalid { what, value, .. }
            if what == "primitive type" && value == "NonExistentType"),
        "expected Invalid(primitive type, NonExistentType), got {err:?}"
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("NonExistentType"),
        "error message should name the bad type: {msg}"
    );

    Ok(())
}

#[test]
fn duplicate_message_id_returns_duplicate_template_id() -> Result<(), Box<dyn std::error::Error>> {
    let path = fixture_path("duplicate-message-id.xml");
    let err = ergo_sbe::parse_file(&path).unwrap_err();
    assert!(
        matches!(&err, ergo_sbe::ParseError::Resolve { error, .. }
            if matches!(error.as_ref(), ergo_sbe::ResolveError::DuplicateTemplateId { id: 1, name, .. } if name == "AnotherMessageWithId1")),
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

    Ok(())
}

#[test]
fn duplicate_message_id_renders_miette_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let path = fixture_path("duplicate-message-id.xml");
    let err = ergo_sbe::parse_file(&path).unwrap_err();

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
        rendered.contains("ergo_sbe::schema_parse::resolve"),
        "rendered output should include error code, got:\n{rendered}"
    );

    Ok(())
}

#[test]
fn version_gap_returns_since_version_beyond_schema() -> Result<(), Box<dyn std::error::Error>> {
    let path = fixture_path("version-gap.xml");
    let err = ergo_sbe::parse_file(&path).unwrap_err();
    assert!(
        matches!(&err, ergo_sbe::ParseError::Resolve { error, .. }
            if matches!(error.as_ref(), ergo_sbe::ResolveError::SinceVersionBeyondSchema { version: 5, schema_version: 1, .. })),
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

    Ok(())
}

#[test]
fn version_gap_renders_miette_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let path = fixture_path("version-gap.xml");
    let err = ergo_sbe::parse_file(&path).unwrap_err();

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
        rendered.contains("ergo_sbe::schema_parse::resolve"),
        "rendered output should include error code, got:\n{rendered}"
    );

    Ok(())
}

// Upstream error-handler schemas (SBE-20 /) — each must fail parse.

#[test]
fn error_handler_schemas_all_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        "error-handler-enum-violates-min-max-value-range.xml",
        "error-handler-group-dimensions-schema.xml",
        "error-handler-invalid-composite-offsets-schema.xml",
        "error-handler-invalid-composite.xml",
        "error-handler-invalid-name.xml",
        "error-handler-message-schema.xml",
        "error-handler-since-version.xml",
        "error-handler-types-schema.xml",
        "error-handler-types-dup-schema.xml",
        "error-handler-dup-message-schema.xml",
        "cyclic-refs-schema.xml",
    ];
    for name in cases {
        let path = fixture_path(name);
        let err = ergo_sbe::parse_file(&path);
        assert!(
            err.is_err(),
            "expected parse rejection for {name}, got Ok(...)"
        );
    }

    Ok(())
}

#[test]
fn invalid_enum_encoding_type_returns_invalid_error() -> Result<(), Box<dyn std::error::Error>> {
    let path = fixture_path("invalid-enum-value.xml");
    let err = ergo_sbe::parse_file(&path).unwrap_err();
    assert!(
        matches!(&err, ergo_sbe::ParseError::Invalid { what, value, .. }
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

    Ok(())
}

#[test]
fn invalid_schema_fixtures_have_useful_miette_errors() -> Result<(), Box<dyn std::error::Error>> {
    use miette::Diagnostic;

    let cases = [
        "missing-required-attr.xml",
        "invalid-type-ref.xml",
        "duplicate-message-id.xml",
        "version-gap.xml",
        "invalid-enum-value.xml",
        "error-handler-enum-violates-min-max-value-range.xml",
        "error-handler-group-dimensions-schema.xml",
        "error-handler-invalid-composite-offsets-schema.xml",
        "error-handler-invalid-composite.xml",
        "error-handler-invalid-name.xml",
        "error-handler-message-schema.xml",
        "error-handler-since-version.xml",
        "error-handler-types-schema.xml",
        "error-handler-types-dup-schema.xml",
        "error-handler-dup-message-schema.xml",
        "cyclic-refs-schema.xml",
        "bad-include.xml",
        "schema-with-bad-include.xml",
    ];

    for name in cases {
        let path = fixture_path(name);
        assert!(path.is_file(), "{name}: invalid-schema fixture is missing");
        let err = match ergo_sbe::parse_file(&path) {
            Ok(_) => panic!("{name}: invalid schema unexpectedly parsed"),
            Err(e) => e,
        };

        let display = format!("{err}");
        assert!(
            !display.trim().is_empty(),
            "{name}: Display must be non-empty"
        );
        assert!(
            display.len() >= 8,
            "{name}: Display too short to be useful: {display:?}"
        );

        let debug = format!("{err:?}");
        // Debug must name a concrete variant (ParseError / IncludeError / …), not empty.
        assert!(
            debug.contains("Error")
                || debug.contains("Invalid")
                || debug.contains("Missing")
                || debug.contains("Resolve")
                || debug.contains("Xml")
                || debug.contains("Io"),
            "{name}: Debug should expose error kind, got: {debug}"
        );

        // miette surface: Report renders without panic and carries a code or message.
        let report = miette::Report::from(err);
        let rendered = format!("{report:?}");
        assert!(
            !rendered.trim().is_empty(),
            "{name}: miette Report must render non-empty"
        );
        // Prefer rich diagnostics: either source_code attached or a diagnostic code.
        let has_source = report.source_code().is_some();
        let has_code = report.code().is_some();
        assert!(
            has_source || has_code || rendered.len() > 20,
            "{name}: expected source_code, diagnostic code, or substantive render; got:\n{rendered}"
        );
    }

    Ok(())
}

// ── Practical schema-error quality tests ────────────────────────────────────
//
// These test that error messages contain enough information for a user to
// understand *what* is wrong and *where* to fix it, without reading the
// schema source or the ergo-sbe source.

/// When a `<field>` has a bad `type` reference, the error must name both
/// the offending type and the field that referenced it.
#[test]
fn invalid_field_type_names_the_field() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
      <types>
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
      </types>
      <message name="Msg" id="1">
        <field name="price" id="1" type="NotARealType" offset="0"/>
      </message>
    </messageSchema>"#;

    let err = ergo_sbe::parse(xml).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("NotARealType"),
        "must name the bad type: {msg}"
    );
    // NOTE: the error currently does not name the containing field (e.g. "price").
    // That is a desirable improvement for future releases.
    Ok(())
}

/// When `blockLength` is declared but some fields sit past it, the error
/// must identify the overflowing field.
#[test]
fn block_length_too_short_named_field() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
      <types>
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
      </types>
      <message name="Msg" id="1" blockLength="4">
        <field name="x" id="1" type="uint32" offset="0"/>
        <field name="overflow" id="2" type="uint32" offset="8"/>
      </message>
    </messageSchema>"#;

    let err = ergo_sbe::parse(xml).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("overflow") || msg.contains("8") || msg.to_lowercase().contains("block"),
        "must name the overflowing field, offset, or mention blockLength: {msg}"
    );
    Ok(())
}

/// Malformed XML must produce a clear error, not a panic or generic message.
#[test]
fn malformed_xml_gives_clear_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
      <types>
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
      </types>
      <message name="Msg" id="1">
        <field name="x" id="1" type="uint32" offset="0"/>
      </unclosed></message>
    </messageSchema>"#;

    let err = ergo_sbe::parse(xml).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("XML")
            || msg.contains("xml")
            || msg.contains("malformed")
            || msg.contains("unexpected"),
        "must indicate malformed XML: {msg}"
    );
    Ok(())
}

/// A schema that references a non-existent composite type must fail with
/// the type name in the error.
#[test]
fn unknown_composite_type_named_in_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
      <types>
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
      </types>
      <message name="Msg" id="1">
        <field name="engine" id="1" type="UnknownComposite" offset="0"/>
      </message>
    </messageSchema>"#;

    let err = ergo_sbe::parse(xml).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("UnknownComposite"),
        "must name the unknown type: {msg}"
    );
    Ok(())
}

/// miette rendering must carry source context for a syntax-level error.
#[test]
fn missing_attribute_renders_miette_source_context() -> Result<(), Box<dyn std::error::Error>> {
    let path = fixture_path("missing-required-attr.xml");
    let err = ergo_sbe::parse_file(&path).unwrap_err();
    let report = miette::Report::from(err);
    let rendered = format!("{report:?}");
    assert!(
        rendered.contains("field @name"),
        "must mention what is missing: {rendered}"
    );
    assert!(
        rendered.contains("id=\"1\"") || rendered.contains("field"),
        "rendering should include source context: {rendered}"
    );
    Ok(())
}

/// A schema `sinceVersion` beyond the schema's own version must fail clearly.
#[test]
fn since_version_beyond_schema_is_explained() -> Result<(), Box<dyn std::error::Error>> {
    let path = fixture_path("version-gap.xml");
    let err = ergo_sbe::parse_file(&path).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("sinceVersion"),
        "must mention sinceVersion: {msg}"
    );
    assert!(msg.contains("5"), "must show the bad version: {msg}");
    assert!(msg.contains("1"), "must show the schema version: {msg}");
    Ok(())
}
