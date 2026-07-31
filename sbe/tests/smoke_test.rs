//! Smoke test: parse every real-world and test schema in `fixtures/schemas/`.
//!
//! For each schema we parse via `parse_file()` and count structural elements.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

use std::fs;
use std::path::Path;

use ergo_sbe::ir::Signal;
use ergo_sbe::parse_file;

/// Schemas that are intentionally invalid — tested separately.
const EXPECTED_PARSE_ERRORS: &[&str] = &[
    "cyclic-self-include.xml",
    "duplicate-message-id.xml",
    "fix_examples_v2rc3.xml", // missing xi:include targets (types-include.xml, messages-include.xml)
    "invalid-enum-value.xml",
    "invalid-type-ref.xml",
    "missing-required-attr.xml",
    "version-gap.xml",
];

/// Include fragments that cannot be parsed standalone.
const INCLUDE_FRAGMENTS: &[&str] = &[
    "common-types.xml",
    "fix_types_include.xml",
    "fix_messages_include.xml",
    "types-include.xml",
    "bad-include.xml",
];

/// Error-handler / intentionally invalid schemas.
const ERROR_SCHEMAS: &[&str] = &[
    "cyclic-refs-schema.xml",
    "error-handler-dup-message-schema.xml",
    "error-handler-enum-violates-min-max-value-range.xml",
    "error-handler-group-dimensions-schema.xml",
    "error-handler-invalid-composite-offsets-schema.xml",
    "error-handler-invalid-composite.xml",
    "error-handler-invalid-name.xml",
    "error-handler-message-schema.xml",
    "error-handler-since-version.xml",
    "error-handler-types-dup-schema.xml",
    "error-handler-types-schema.xml",
    "schema-with-bad-include.xml",
];

fn schema_dir() -> &'static Path {
    Path::new("tests/fixtures/schemas")
}

#[test]
fn all_schema_fixtures_have_the_expected_parse_outcome() -> Result<(), Box<dyn std::error::Error>> {
    let dir = schema_dir();
    let mut entries = fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.retain(|path| path.extension().is_some_and(|extension| extension == "xml"));
    entries.sort();

    for name in EXPECTED_PARSE_ERRORS
        .iter()
        .chain(INCLUDE_FRAGMENTS)
        .chain(ERROR_SCHEMAS)
    {
        assert!(
            dir.join(name).is_file(),
            "classified schema fixture is missing: {name}"
        );
    }

    let mut passed = 0u32;
    let mut rejected = 0u32;
    let mut failures = Vec::new();

    for path in &entries {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let expected_rejection = EXPECTED_PARSE_ERRORS.contains(&name.as_str())
            || INCLUDE_FRAGMENTS.contains(&name.as_str())
            || ERROR_SCHEMAS.contains(&name.as_str());

        match parse_file(path) {
            Ok(ir) => {
                if expected_rejection {
                    failures.push(format!("{name}: unexpectedly parsed"));
                    continue;
                }
                let msg_count = ir
                    .tokens
                    .iter()
                    .filter(|t| t.signal == Signal::BeginMessage)
                    .count();
                let group_count = ir
                    .tokens
                    .iter()
                    .filter(|t| t.signal == Signal::BeginGroup)
                    .count();
                let enum_count = ir
                    .tokens
                    .iter()
                    .filter(|t| t.signal == Signal::BeginEnum)
                    .count();
                let set_count = ir
                    .tokens
                    .iter()
                    .filter(|t| t.signal == Signal::BeginSet)
                    .count();
                let vardata_count = ir
                    .tokens
                    .iter()
                    .filter(|t| t.signal == Signal::BeginVarData)
                    .count();
                let composite_count = ir
                    .tokens
                    .iter()
                    .filter(|t| t.signal == Signal::BeginComposite)
                    .count();
                println!(
                    "  OK    {name:<40}  \
                     v={version:2}  \
                     messages={msg_count:3}  \
                     groups={group_count:3}  \
                     enums={enum_count:3}  \
                     sets={set_count:3}  \
                     composites={composite_count:3}  \
                     vardata={vardata_count:3}  \
                     tokens={tokens}",
                    version = ir.version,
                    tokens = ir.tokens.len(),
                );
                passed += 1;
            }
            Err(e) => {
                if expected_rejection {
                    let message = e.to_string();
                    assert!(
                        !message.trim().is_empty(),
                        "{name}: expected rejection must have a diagnostic"
                    );
                    println!(
                        "  REJECT {name:<40}  {}",
                        message.lines().next().unwrap_or(&message)
                    );
                    rejected += 1;
                } else {
                    let err_str = format!("{e}");
                    let first_line = err_str.lines().next().unwrap_or(&err_str);
                    failures.push(format!("{name}: {first_line}"));
                }
            }
        }
    }

    println!();
    println!("  ───────────────────────────────────────────────────────");
    println!("  Parsed: {passed}  |  Rejected as expected: {rejected}");

    if !failures.is_empty() {
        panic!(
            "{} schema fixture(s) had the wrong parse outcome:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }
    assert_eq!(
        passed + rejected,
        entries.len() as u32,
        "every discovered schema must have an asserted outcome"
    );

    Ok(())
}
