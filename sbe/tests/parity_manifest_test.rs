//! Enforces that the independent sbe-tool oracle inventory and the supported
//! wire-feature matrix stay connected to executable tests.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn strings<'a>(
    table: &'a toml::value::Table,
    key: &str,
) -> Result<Vec<&'a str>, Box<dyn std::error::Error>> {
    table
        .get(key)
        .and_then(toml::Value::as_array)
        .ok_or_else(|| format!("missing array {key:?}").into())
        .and_then(|values| {
            values
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .ok_or_else(|| format!("{key:?} contains a non-string value").into())
                })
                .collect()
        })
}

fn test_symbols(tests_dir: &Path) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let mut symbols = BTreeSet::new();
    for entry in fs::read_dir(tests_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(path)?;
        let syntax = syn::parse_file(&source)?;
        for item in syntax.items {
            if let syn::Item::Fn(function) = item {
                let has_test_attr = function.attrs.iter().any(|a| a.path().is_ident("test"));
                let name_starts_with_test = function.sig.ident.to_string().starts_with("test_");
                if has_test_attr || name_starts_with_test {
                    symbols.insert(function.sig.ident.to_string());
                }
            }
        }
    }
    Ok(symbols)
}

#[test]
#[allow(clippy::too_many_lines)]
fn parity_manifest_covers_every_reference_crate_and_supported_feature()
-> Result<(), Box<dyn std::error::Error>> {
    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let manifest =
        toml::from_str::<toml::Value>(&fs::read_to_string(tests_dir.join("parity-cases.toml"))?)?;
    let root = manifest
        .as_table()
        .ok_or("parity-cases.toml must contain a table")?;
    assert_eq!(
        root.get("version").and_then(toml::Value::as_integer),
        Some(1),
        "update the manifest validator when its format changes"
    );

    let symbols = test_symbols(&tests_dir)?;
    let reference_root = tests_dir.join("sbe_tool_reference");
    let checked_in: BTreeSet<String> = fs::read_dir(&reference_root)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();

    let cases = root
        .get("case")
        .and_then(toml::Value::as_array)
        .ok_or("manifest must contain [[case]] entries")?;
    let mut declared = BTreeSet::new();
    for value in cases {
        let case = value.as_table().ok_or("[[case]] must be a table")?;
        let key = case
            .get("key")
            .and_then(toml::Value::as_str)
            .ok_or("[[case]] is missing key")?;
        assert!(declared.insert(key.to_owned()), "duplicate case {key}");
        assert!(
            reference_root.join(key).join("Cargo.toml").is_file(),
            "manifest case {key} has no checked-in reference crate"
        );

        for category in ["exact_encode", "cross_decode", "endian"] {
            let tests = strings(case, category)?;
            assert!(
                !tests.is_empty(),
                "reference case {key} has no {category} test"
            );
            for test in tests {
                assert!(
                    symbols.contains(test),
                    "reference case {key} names missing test {test}"
                );
            }
        }
        for test in strings(case, "versioning")? {
            assert!(
                symbols.contains(test),
                "reference case {key} names missing versioning test {test}"
            );
        }
    }
    assert_eq!(
        declared, checked_in,
        "parity manifest and checked-in sbe-tool crates differ"
    );

    let required_features: BTreeSet<&str> = [
        "big_endian",
        "custom_headers",
        "dimension_u8_u16_u32",
        "exhaustive_truncation",
        "explicit_implicit_offsets",
        "fixed_arrays",
        "float_wire_bits",
        "groups",
        "name_collisions",
        "nested_groups",
        "schema_versioning",
        "var_data",
        "write_offsets_and_canaries",
    ]
    .into_iter()
    .collect();
    let features = root
        .get("feature")
        .and_then(toml::Value::as_array)
        .ok_or("manifest must contain [[feature]] entries")?;
    let mut declared_features = BTreeSet::new();
    for value in features {
        let feature = value.as_table().ok_or("[[feature]] must be a table")?;
        let name = feature
            .get("name")
            .and_then(toml::Value::as_str)
            .ok_or("[[feature]] is missing name")?;
        assert!(
            declared_features.insert(name),
            "duplicate supported feature {name}"
        );
        let tests = strings(feature, "tests")?;
        assert!(!tests.is_empty(), "feature {name} has no executable test");
        for test in tests {
            assert!(
                symbols.contains(test),
                "feature {name} names missing test {test}"
            );
        }
    }
    assert_eq!(
        declared_features, required_features,
        "supported-feature manifest changed without updating the enforced matrix"
    );
    Ok(())
}
