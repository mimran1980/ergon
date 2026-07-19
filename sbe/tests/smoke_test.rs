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

/// Error-handler schemas that are intentionally invalid.
const ERROR_SCHEMAS: &[&str] = &[
    "error-handler-dup-message-schema.xml",
    "error-handler-group-dimensions-schema.xml",
    "error-handler-message-schema.xml",
    "error-handler-since-version.xml",
    "schema-with-bad-include.xml",
];

fn schema_dir() -> &'static Path {
    Path::new("tests/fixtures/schemas")
}

#[test]
fn all_schemas_parse() {
    let dir = schema_dir();
    let mut entries: Vec<_> = fs::read_dir(dir)
        .expect("fixtures/schemas/ dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "xml"))
        .map(|e| e.path())
        .collect();
    entries.sort();

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;
    let mut failures = Vec::new();

    for path in &entries {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();

        if EXPECTED_PARSE_ERRORS.contains(&name.as_str()) {
            println!("  SKIP  {name:<40}  (expected parse error — tested elsewhere)");
            skipped += 1;
            continue;
        }

        match parse_file(path) {
            Ok(ir) => {
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
                if INCLUDE_FRAGMENTS.contains(&name.as_str()) {
                    println!("  SKIP  {name:<40}  (include fragment — needs parent schema)");
                    skipped += 1;
                } else if ERROR_SCHEMAS.contains(&name.as_str()) {
                    println!("  SKIP  {name:<40}  (intentionally invalid error schema)");
                    skipped += 1;
                } else {
                    let err_str = format!("{e}");
                    let first_line = err_str.lines().next().unwrap_or(&err_str);
                    println!("  FAIL  {name:<40}  {first_line}");
                    failed += 1;
                    failures.push(name);
                }
            }
        }
    }

    println!();
    println!("  ───────────────────────────────────────────────────────");
    println!("  Passed: {passed}  |  Failed: {failed}  |  Skipped: {skipped}");

    if !failures.is_empty() {
        panic!(
            "{} schema(s) failed to parse:\n  {}",
            failed,
            failures.join("\n  ")
        );
    }
}
