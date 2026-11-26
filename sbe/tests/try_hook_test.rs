//! `GenerationConfig::with_try_hook`: a hook that can fail generation.
//!
//! An `Err` aborts generation with `GenerateError::Hook`, which names the
//! item (kind and generated type name) and carries the hook's reason.

#![allow(clippy::expect_used)]

use ergo_sbe::{
    DomainVarData, GenerateError, GenerationConfig, Generator, ItemContext, Schema, SchemaSource,
    parse,
};

const XML: &str = r#"<messageSchema package="tryhook" id="3" version="0" byteOrder="littleEndian">
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
  <message name="Quote" id="1">
    <field name="px" id="1" type="int64"/>
    <group name="legs" id="2" dimensionType="groupSizeEncoding">
      <field name="qty" id="1" type="int64"/>
    </group>
  </message>
</messageSchema>"#;

fn generate(config: GenerationConfig) -> Result<(), GenerateError> {
    let schema = Schema::from_ir(parse(SchemaSource::xml(XML)).expect("schema parses"));
    Generator::new(config).generate(&schema).map(|_| ())
}

/// An `Err` from a group-entry context names that entry decoder.
#[test]
fn try_hook_error_names_the_item() -> Result<(), Box<dyn std::error::Error>> {
    let config = GenerationConfig::new("tryhook").with_try_hook(|ctx: &ItemContext| match ctx {
        ItemContext::GroupEntryDecoder { .. } => Err("no entries here".to_string()),
        _ => Ok(vec![]),
    });
    let err = generate(config).expect_err("the hook must fail generation");
    let GenerateError::Hook { item, reason } = &err else {
        return Err(format!("expected GenerateError::Hook, got {err:?}").into());
    };
    assert_eq!(item, "GroupEntryDecoder LegsEntryDecoder");
    assert_eq!(reason, "no entries here");
    assert_eq!(
        err.to_string(),
        "hook failed on GroupEntryDecoder LegsEntryDecoder: no entries here"
    );
    Ok(())
}

/// An `Err` from a domain DTO context propagates too.
#[test]
fn try_hook_error_propagates_from_domain_structs() {
    let config = GenerationConfig::new("tryhook")
        .with_domain_objects(DomainVarData::Bytes)
        .with_try_hook(|ctx: &ItemContext| match ctx {
            ItemContext::DomainStruct { name, .. } => Err(format!("{name} refused")),
            _ => Ok(vec![]),
        });
    let err = generate(config).expect_err("the hook must fail generation");
    assert!(
        matches!(&err, GenerateError::Hook { item, .. } if item.starts_with("DomainStruct ")),
        "{err:?}"
    );
}

/// A try-hook that emits nothing leaves the output byte-identical, on every
/// fixture schema that parses (a schema that fails to generate must fail the
/// same way with the hook).
#[test]
fn noop_try_hook_leaves_every_fixture_byte_identical() -> Result<(), Box<dyn std::error::Error>> {
    fn sources(schema: &Schema, config: GenerationConfig) -> Result<Vec<String>, String> {
        Generator::new(config)
            .generate(schema)
            .map(|set| set.modules().map(|m| m.source.clone()).collect())
            .map_err(|e| e.to_string())
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/schemas");
    let mut compared = 0;
    for entry in std::fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "xml") {
            continue;
        }
        let Ok(ir) = parse(SchemaSource::file(&path)) else {
            continue;
        };
        let schema = Schema::from_ir(ir);
        let plain = sources(&schema, GenerationConfig::new("noop"));
        let hooked = sources(
            &schema,
            GenerationConfig::new("noop").with_try_hook(|_: &ItemContext| Ok(vec![])),
        );
        assert_eq!(plain, hooked, "{}", path.display());
        compared += 1;
    }
    assert!(compared > 100, "only {compared} fixture schemas parsed");
    Ok(())
}

/// `with_hook` and `with_try_hook` run in registration order, and an `Ok`
/// try-hook's tokens land after the item like a plain hook's.
#[test]
fn try_hooks_and_hooks_run_in_registration_order() -> Result<(), Box<dyn std::error::Error>> {
    let config = GenerationConfig::new("tryhook")
        .with_hook(|ctx: &ItemContext| match ctx {
            ItemContext::MessageDecoder { .. } => vec![quote::quote! { pub const FIRST: u8 = 1; }],
            _ => vec![],
        })
        .with_try_hook(|ctx: &ItemContext| match ctx {
            ItemContext::MessageDecoder { .. } => {
                Ok(vec![quote::quote! { pub const SECOND: u8 = 2; }])
            }
            _ => Ok(vec![]),
        });
    let schema = Schema::from_ir(parse(SchemaSource::xml(XML))?);
    let modules = Generator::new(config).generate(&schema)?;
    let src = &modules.modules().next().ok_or("one module")?.source;
    let first = src.find("FIRST").ok_or("FIRST")?;
    let second = src.find("SECOND").ok_or("SECOND")?;
    assert!(first < second, "registration order");
    Ok(())
}
