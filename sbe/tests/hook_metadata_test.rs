//! Regression tests for hook metadata correctness.
//!
//! Guards hook metadata correctness:
//!   * enum discriminants above `i64::MAX` must reach the hook;
//!   * composite hooks list real primitive fields with correct Rust types and
//!     report nested composites/enums by type name;
//!   * `DomainStruct` hooks fire for the top-level DTO and every
//!     `*EntryDomain`, including group and var-data fields.

#![allow(clippy::expect_used)]

mod common;
use common::compile_and_run;

use std::sync::{Arc, Mutex};

use ergo_sbe::{DomainVarData, GenerationConfig, Generator, ItemContext, Schema, parse};

/// A snapshot of one hooked item, captured for assertions.
#[derive(Debug, Clone)]
enum Captured {
    Enum {
        name: String,
        variants: Vec<(String, i128)>,
    },
    Composite {
        name: String,
        fields: Vec<(String, String)>, // (snake_name, rust_type)
    },
    Domain {
        name: String,
        fields: Vec<(String, String)>,
    },
}

#[allow(clippy::type_complexity)]
fn capture_hook(
    sink: Arc<Mutex<Vec<Captured>>>,
) -> impl Fn(&ItemContext) -> Vec<proc_macro2::TokenStream> + Send + Sync + 'static {
    move |ctx: &ItemContext| {
        let field_pairs = |fields: &[ergo_sbe::FieldInfo]| -> Vec<(String, String)> {
            fields
                .iter()
                .map(|f| (f.name.clone(), f.rust_type.clone()))
                .collect()
        };
        let item = match ctx {
            ItemContext::Enum { name, variants, .. } => Some(Captured::Enum {
                name: name.clone(),
                variants: variants.iter().map(|v| (v.name.clone(), v.value)).collect(),
            }),
            ItemContext::Composite { name, fields, .. } => Some(Captured::Composite {
                name: name.clone(),
                fields: field_pairs(fields),
            }),
            ItemContext::DomainStruct { name, fields, .. } => Some(Captured::Domain {
                name: name.clone(),
                fields: field_pairs(fields),
            }),
            _ => None,
        };
        if let Some(item) = item {
            sink.lock().expect("capture lock").push(item);
        }
        vec![]
    }
}

/// Schema exercising every metadata edge: a uint64 enum with a discriminant
/// above `i64::MAX`, a nested composite containing a primitive + nested
/// composite + enum ref, and a message with a group and a var-data field.
const SCHEMA_XML: &str = r#"<messageSchema package="hookmeta" id="7" version="0" byteOrder="littleEndian">
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
    <composite name="varStringEncoding">
      <type name="length" primitiveType="uint32"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
    <enum name="BigEnum" encodingType="uint64">
      <validValue name="Low">1</validValue>
      <validValue name="High">9223372036854775808</validValue>
    </enum>
    <composite name="Outer">
      <type name="realField" primitiveType="int64" semanticType="UTCTimestamp" description="a real field"/>
      <composite name="inner" description="the inner composite">
        <type name="x" primitiveType="uint16"/>
      </composite>
      <enum name="flag" encodingType="uint8" description="the flag enum">
        <validValue name="Off">0</validValue>
        <validValue name="On">1</validValue>
      </enum>
    </composite>
  </types>
  <message name="Msg" id="1" blockLength="8">
    <field name="code" id="1" type="BigEnum" offset="0"/>
    <group name="levels" id="2" dimensionType="groupSizeEncoding">
      <field name="price" id="3" type="int64" offset="0"/>
    </group>
    <data name="note" id="4" type="varStringEncoding"/>
  </message>
</messageSchema>"#;

fn capture_all() -> Result<Vec<Captured>, Box<dyn std::error::Error>> {
    let sink = Arc::new(Mutex::new(Vec::new()));
    let config = GenerationConfig::new("hookmeta")
        .with_domain_objects(DomainVarData::Bytes)
        .with_hook(capture_hook(Arc::clone(&sink)));
    let schema = Schema::from_ir(parse(SCHEMA_XML)?);
    let _ = Generator::new(config).generate(&schema)?;
    let out = sink.lock().expect("lock").clone();
    Ok(out)
}

/// A `uint64` enum discriminant above `i64::MAX` must still reach the hook.
#[test]
fn enum_discriminant_above_i64_max_reaches_hook() -> Result<(), Box<dyn std::error::Error>> {
    let captured = capture_all()?;
    let big = captured
        .iter()
        .find_map(|c| match c {
            Captured::Enum { name, variants } if name == "BigEnum" => Some(variants),
            _ => None,
        })
        .expect("BigEnum context must be emitted");

    let names: Vec<&str> = big.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        names.contains(&"Low") && names.contains(&"High"),
        "both variants must survive; the >i64::MAX one must not be dropped. got: {names:?}"
    );
    // 9223372036854775808 = 2^63 is above i64::MAX; as i128 it is preserved
    // faithfully (rather than wrapping to i64::MIN or being dropped entirely).
    let high = big.iter().find(|(n, _)| n == "High").expect("High present");
    assert_eq!(
        high.1, 9_223_372_036_854_775_808_i128,
        "High = 2^63 preserved"
    );
    Ok(())
}

/// Composite metadata must list real primitive members with correct Rust
/// types and report nested composites/enums by type name — the earlier code
/// omitted the primitive and mislabelled the container and enum ref as `u8`.
#[test]
fn composite_hook_reports_real_fields_not_containers() -> Result<(), Box<dyn std::error::Error>> {
    let captured = capture_all()?;
    let outer = captured
        .iter()
        .find_map(|c| match c {
            Captured::Composite { name, fields } if name == "Outer" => Some(fields),
            _ => None,
        })
        .expect("Outer composite context must be emitted");

    // The real primitive field must be present with its true type — not omitted.
    assert!(
        outer.contains(&("real_field".to_string(), "i64".to_string())),
        "primitive member real_field must be reported as i64, got: {outer:?}"
    );
    // The nested composite is reported by type name, never as a bare u8.
    assert!(
        outer.contains(&("inner".to_string(), "Inner".to_string())),
        "nested composite must be reported as Inner, got: {outer:?}"
    );
    // The enum ref is reported by type name, never as a bare u8.
    assert!(
        outer.contains(&("flag".to_string(), "Flag".to_string())),
        "enum ref must be reported as Flag, got: {outer:?}"
    );
    // Nothing in this composite is a u8 — a stray u8 means a container/ref
    // leaked through as a mislabelled field (the old bug).
    assert!(
        !outer.iter().any(|(_, ty)| ty == "u8"),
        "no member should be mislabelled u8, got: {outer:?}"
    );
    // Exactly the three declared members, no phantom container entries.
    assert_eq!(
        outer.len(),
        3,
        "Outer has exactly 3 members, got: {outer:?}"
    );
    Ok(())
}

/// `DomainStruct` hooks must fire for the top-level DTO *and* each generated
/// `*EntryDomain`; the top-level context must include group and var-data
/// fields — the serde test only had a fixed-only DTO.
#[test]
fn domain_hook_covers_message_group_and_vardata() -> Result<(), Box<dyn std::error::Error>> {
    let captured = capture_all()?;
    let domains: Vec<(&str, &Vec<(String, String)>)> = captured
        .iter()
        .filter_map(|c| match c {
            Captured::Domain { name, fields } => Some((name.as_str(), fields)),
            _ => None,
        })
        .collect();

    let msg = domains
        .iter()
        .find(|(n, _)| *n == "MsgDomain")
        .map(|(_, f)| *f)
        .expect("top-level MsgDomain hook must fire");
    // Group field is present, and its Rust type names the ACTUAL generated
    // entry-DTO struct (prefixed with the message name) — a bare
    // `Vec<LevelsEntryDomain>` would name a type that does not exist.
    assert!(
        msg.iter()
            .any(|(n, ty)| n == "levels" && ty == "Vec<MsgLevelsEntryDomain>"),
        "group field type must be the fully-qualified entry DTO, got: {msg:?}"
    );
    // Var-data field is present.
    assert!(
        msg.iter().any(|(n, _)| n == "note"),
        "var-data field must appear in the message DTO context, got: {msg:?}"
    );

    // The entry DTO gets its own hook invocation.
    assert!(
        domains
            .iter()
            .any(|(n, _)| n.ends_with("LevelsEntryDomain")),
        "entry DTO hook must fire; got domains: {:?}",
        domains.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
    Ok(())
}

/// End-to-end proof the advertised general extension mechanism works for
/// groups, var-data, and entry DTOs: a hook attaches a method to *every*
/// domain struct, and the generated code compiles and the methods run.
#[test]
fn hook_extends_message_and_entry_domain_and_compiles() {
    let sink = Arc::new(Mutex::new(Vec::new()));
    let config = GenerationConfig::new("hookext")
        .with_domain_objects(DomainVarData::Bytes)
        .with_hook({
            let sink = Arc::clone(&sink);
            move |ctx: &ItemContext| {
                if let ItemContext::DomainStruct { name, fields, .. } = ctx {
                    sink.lock().expect("lock").push(name.clone());
                    let ident = quote::format_ident!("{name}");
                    let n = fields.len();
                    let label = name.clone();
                    return vec![quote::quote! {
                        impl #ident {
                            pub fn hooked_name() -> &'static str { #label }
                            pub fn hooked_field_count() -> usize { #n }
                        }
                    }];
                }
                vec![]
            }
        });

    let schema = Schema::from_ir(parse(SCHEMA_XML).expect("parse"));
    let src = Generator::new(config)
        .generate(&schema)
        .expect("generate")
        .modules()
        .next()
        .expect("module")
        .source
        .clone();

    // Both the message DTO and the entry DTO must have received the hook.
    let names = sink.lock().expect("lock").clone();
    assert!(
        names.iter().any(|n| n == "MsgDomain"),
        "message DTO must be hooked, got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n.ends_with("LevelsEntryDomain")),
        "entry DTO must be hooked, got: {names:?}"
    );

    // Compile the generated code and exercise the hook-added methods on both
    // the message DTO (3 fields: code, levels, note) and the entry DTO.
    compile_and_run(
        "hookext",
        &src,
        r#"
        assert_eq!(MsgDomain::hooked_name(), "MsgDomain");
        assert_eq!(MsgDomain::hooked_field_count(), 3);
        assert_eq!(MsgLevelsEntryDomain::hooked_field_count(), 1);
        "#,
    );
}

// ── Debug impls on the hook/config surface ─────────────────────────────
//
// `GenerationConfig`, `Hooks` and `ItemContext` all hand-write `Debug`
// (`Hooks` holds `Arc<dyn Fn>` and `ItemContext` holds the whole schema, so
// neither can derive it). Nothing called any of the three, so a panic or an
// accidental dump of the entire IR into a hook author's log would have gone
// unnoticed.

#[test]
fn config_and_item_context_debug_stay_short_and_useful() -> Result<(), Box<dyn std::error::Error>> {
    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);

    let config = GenerationConfig::new("dbg").with_hook(move |ctx: &ItemContext| {
        sink.lock().expect("hook sink").push(format!("{ctx:?}"));
        Vec::new()
    });

    let rendered = format!("{config:?}");
    assert!(rendered.starts_with("GenerationConfig {"), "{rendered}");
    assert!(rendered.contains("module_name: \"dbg\""), "{rendered}");
    // Hooks must report its arity, never try to format the closures.
    assert!(rendered.contains("hooks: Hooks(1)"), "{rendered}");

    let ir = parse(&std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/schemas/example-schema.xml"),
    )?)?;
    Generator::new(config).generate(&Schema::from_ir(ir))?;

    let captured = seen.lock().expect("hook sink").clone();
    assert!(!captured.is_empty(), "hooks must have fired");
    // ItemContext carries the full schema; Debug must print kind + name only.
    for line in &captured {
        assert!(line.starts_with("ItemContext { kind: "), "{line}");
        assert!(line.contains("name: "), "{line}");
        assert!(!line.contains("tokens"), "must not dump the IR: {line}");
    }
    assert!(
        captured.iter().any(|l| l.contains("MessageEncoder")),
        "expected a MessageEncoder context, got {captured:?}"
    );
    Ok(())
}

#[derive(Clone, Debug, Default)]
struct DepFlags {
    items: Vec<&'static str>,
}

const DEP_SCHEMA_XML: &str = r#"<?xml version="1.0"?>
<messageSchema package="dep" id="1" version="0" byteOrder="littleEndian">
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
    <composite name="varDataEncoding">
      <type name="length" primitiveType="uint32"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
    <enum name="OldEnum" encodingType="uint8" deprecated="3">
      <validValue name="A">1</validValue>
    </enum>
    <set name="OldSet" encodingType="uint8" deprecated="4">
      <choice name="X">0</choice>
    </set>
    <composite name="OldComp" deprecated="5">
      <type name="val" primitiveType="uint32"/>
    </composite>
  </types>
  <message name="M" id="1" deprecated="6">
    <field name="legacy" id="1" type="uint8" deprecated="2"/>
    <group name="rows" id="2" dimensionType="groupSizeEncoding" deprecated="7">
      <field name="qty" id="3" type="uint32"/>
    </group>
    <data name="note" id="4" type="varDataEncoding" deprecated="8"/>
  </message>
</messageSchema>"#;

fn field_deprecated(fields: &[ergo_sbe::FieldInfo], name: &str) -> bool {
    fields
        .iter()
        .find(|f| f.name == name)
        .is_some_and(|f| f.deprecated)
}

/// Hooks see schema-deprecated flags for types, messages, fields, groups,
/// and var-data. Exact version numbers are a 1.0 FieldInfo/ItemContext change.
#[test]
fn hooks_expose_deprecated_flags() -> Result<(), Box<dyn std::error::Error>> {
    let seen = Arc::new(Mutex::new(DepFlags::default()));
    let sink = Arc::clone(&seen);
    let config = GenerationConfig::new("depver").with_hook(move |ctx: &ItemContext| {
        let mut s = sink.lock().expect("lock");
        match ctx {
            ItemContext::Enum { name, .. } if name == "OldEnum" => s.items.push("enum"),
            ItemContext::Set { name, .. } if name == "OldSet" => s.items.push("set"),
            ItemContext::Composite { name, .. } if name == "OldComp" => s.items.push("composite"),
            ItemContext::MessageDecoder { fields, .. } => {
                s.items.push("message");
                if field_deprecated(fields, "legacy") {
                    s.items.push("field");
                }
                if field_deprecated(fields, "rows") {
                    s.items.push("group");
                }
                if field_deprecated(fields, "note") {
                    s.items.push("data");
                }
            }
            _ => {}
        }
        drop(s);
        vec![]
    });
    let schema = Schema::from_ir(parse(DEP_SCHEMA_XML)?);
    let _ = Generator::new(config).generate(&schema)?;
    let flags = seen.lock().expect("lock").clone();
    for name in [
        "enum",
        "set",
        "composite",
        "message",
        "field",
        "group",
        "data",
    ] {
        assert!(
            flags.items.contains(&name),
            "missing deprecated {name}: {:?}",
            flags.items
        );
    }
    Ok(())
}
