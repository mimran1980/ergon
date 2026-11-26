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

use ergo_sbe::{
    DomainVarData, GenerationConfig, Generator, ItemContext, Schema, SchemaSource, parse,
};

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
    MessageDecoder {
        name: String,
        fields: Vec<(String, String, ergo_sbe::FieldKind)>,
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
            ItemContext::MessageDecoder { name, fields, .. } => Some(Captured::MessageDecoder {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|f| (f.name.clone(), f.rust_type.clone(), f.kind()))
                    .collect(),
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
    let schema = Schema::from_ir(parse(SchemaSource::xml(SCHEMA_XML))?);
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
/// types and report nested composites/enums by type name, never omit the
/// primitive or label the container and enum ref as `u8`.
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

    // The real primitive field must be present with its true type.
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
    // Nothing in this composite is a u8, so a stray u8 means a container or
    // ref leaked through as a mislabelled field.
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

/// Message decoder hook metadata must report groups and var-data as real Rust
/// types, classified by [`ergo_sbe::FieldKind`], never the sentinels `"group"`
/// / `"data"`.
#[test]
fn message_decoder_hook_reports_group_and_data_as_real_types()
-> Result<(), Box<dyn std::error::Error>> {
    let captured = capture_all()?;
    let fields = captured
        .iter()
        .find_map(|c| match c {
            Captured::MessageDecoder { name, fields } if name == "MsgDecoder" => Some(fields),
            _ => None,
        })
        .expect("MsgDecoder context must be emitted");
    let levels = fields
        .iter()
        .find(|(n, _, _)| n == "levels")
        .expect("levels group must appear");
    assert_ne!(
        levels.1, "group",
        "group rust_type must not be a kind sentinel, got {fields:?}"
    );
    assert_eq!(levels.2, ergo_sbe::FieldKind::Group);
    assert!(
        levels.1.contains("Decoder") || levels.1.contains("Levels"),
        "group rust_type must name a generated type, got {}",
        levels.1
    );
    let note = fields
        .iter()
        .find(|(n, _, _)| n == "note")
        .expect("note var-data must appear");
    assert_ne!(
        note.1, "data",
        "var-data rust_type must not be a kind sentinel"
    );
    assert_eq!(note.2, ergo_sbe::FieldKind::Data);
    assert_eq!(note.1, "&[u8]");
    let code = fields
        .iter()
        .find(|(n, _, _)| n == "code")
        .expect("fixed field must appear");
    assert_eq!(code.2, ergo_sbe::FieldKind::Fixed);
    Ok(())
}

/// `DomainStruct` hooks must fire for the top-level DTO *and* each generated
/// `*EntryDomain`; the top-level context must include group and var-data
/// fields, which a fixed-only DTO does not exercise.
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
    // The group field is present, and its Rust type names the generated
    // entry-DTO struct (prefixed with the message name); a bare
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

    let schema = Schema::from_ir(parse(SchemaSource::xml(SCHEMA_XML)).expect("parse"));
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

    let ir = parse(SchemaSource::xml(&std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/schemas/example-schema.xml"),
    )?))?;
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

fn field_deprecated(fields: &[ergo_sbe::FieldInfo], name: &str) -> Option<u16> {
    fields
        .iter()
        .find(|f| f.name == name)
        .and_then(|f| f.deprecated)
}

/// Hooks see schema-deprecated flags for types, messages, fields, groups,
/// and var-data; `FieldInfo::deprecated` carries the schema version.
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
                if field_deprecated(fields, "legacy") == Some(2) {
                    s.items.push("field");
                }
                if field_deprecated(fields, "rows") == Some(7) {
                    s.items.push("group");
                }
                if field_deprecated(fields, "note") == Some(8) {
                    s.items.push("data");
                }
            }
            _ => {}
        }
        drop(s);
        vec![]
    });
    let schema = Schema::from_ir(parse(SchemaSource::xml(DEP_SCHEMA_XML))?);
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

/// Schema for the decoder-context tests: two messages (so group types carry
/// the message prefix), a nested group, a constant, a keyword field, a
/// reserved-name field, optional and versioned fields, an array, an enum, a
/// set, a composite, a conversion (`*_wire`), a domain type, and a
/// composite named `quoteSchema` so the `{Name}Schema` marker must be renamed.
const BLOCK_SCHEMA_XML: &str = r#"<messageSchema xmlns:x="urn:acme:ext" package="acme" id="7" version="1" byteOrder="littleEndian">
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
    <enum name="Venue" encodingType="uint8">
      <validValue name="Lse">1</validValue>
      <validValue name="Xetra">2</validValue>
    </enum>
    <set name="Flags" encodingType="uint8">
      <choice name="fast">0</choice>
    </set>
    <composite name="Pair">
      <type name="lo" primitiveType="uint32"/>
      <type name="hi" primitiveType="uint32"/>
    </composite>
    <composite name="quoteSchema">
      <type name="z" primitiveType="uint8"/>
    </composite>
    <type name="id16" primitiveType="uint8" length="16"/>
  </types>
  <message name="Quote" id="1">
    <field name="venue" id="1" type="Venue"/>
    <field name="priceScale" id="2" type="int8"/>
    <field name="price" id="3" type="int64" x:scaleBy="priceScale" x:wire="y"/>
    <field name="amount" id="4" type="uint64" x:dom="y"/>
    <field name="type" id="5" type="uint8"/>
    <field name="wrap" id="6" type="uint8"/>
    <field name="maybe" id="7" type="int32" presence="optional"/>
    <field name="late" id="8" type="uint32" sinceVersion="1"/>
    <field name="key" id="9" type="id16"/>
    <field name="flags" id="10" type="Flags"/>
    <field name="pair" id="11" type="Pair"/>
    <field name="k" id="12" type="uint16" presence="constant" constantValue="7"/>
    <group name="legs" id="20" dimensionType="groupSizeEncoding" x:unit="lots">
      <field name="qtyScale" id="1" type="int8"/>
      <field name="qty" id="2" type="int64" x:scaleBy="qtyScale"/>
      <group name="fills" id="3" dimensionType="groupSizeEncoding">
        <field name="px" id="1" type="int64" x:scaleBy="priceScale"/>
      </group>
    </group>
    <data name="note" id="30" type="varStringEncoding"/>
  </message>
  <message name="Other" id="2">
    <field name="n" id="1" type="uint8"/>
    <group name="legs" id="2" dimensionType="groupSizeEncoding">
      <field name="q" id="1" type="uint8"/>
    </group>
  </message>
</messageSchema>"#;

/// One decoder-side context, flattened for assertions.
#[derive(Debug, Clone)]
struct Block {
    kind: &'static str,
    name: String,
    group_decoder: Option<String>,
    schema_marker: Option<String>,
    enclosing: Vec<(String, Vec<String>)>,
    fields: Vec<ergo_sbe::FieldInfo>,
    group_unit: Option<String>,
}

fn capture_blocks() -> Result<(Vec<Block>, String), Box<dyn std::error::Error>> {
    let sink: Arc<Mutex<Vec<Block>>> = Arc::new(Mutex::new(Vec::new()));
    let hook_sink = Arc::clone(&sink);
    let config = GenerationConfig::new("blocks")
        .with_conversion(ergo_sbe::ConversionSelector::extension_attr("wire"))
        .with_manual_domain_type(ergo_sbe::ConversionSelector::extension_attr("dom"), "u128")
        .with_hook(move |ctx: &ItemContext| {
            let block = match ctx {
                ItemContext::MessageDecoder {
                    name,
                    fields,
                    schema_marker,
                    ..
                } => Some(Block {
                    kind: "message",
                    name: name.clone(),
                    group_decoder: None,
                    schema_marker: Some(schema_marker.clone()),
                    enclosing: vec![],
                    fields: fields.clone(),
                    group_unit: None,
                }),
                ItemContext::GroupEntryDecoder {
                    name,
                    group_decoder,
                    fields,
                    enclosing,
                    extensions,
                    ..
                } => Some(Block {
                    kind: "entry",
                    name: name.clone(),
                    group_decoder: Some(group_decoder.clone()),
                    schema_marker: None,
                    enclosing: enclosing
                        .iter()
                        .map(|b| {
                            (
                                b.decoder.clone(),
                                b.fields.iter().map(|f| f.schema_name.clone()).collect(),
                            )
                        })
                        .collect(),
                    fields: fields.clone(),
                    group_unit: extensions.attr("unit").map(str::to_string),
                }),
                _ => None,
            };
            if let Some(b) = block {
                hook_sink.lock().expect("lock").push(b);
            }
            vec![]
        });
    let schema = Schema::from_ir(parse(SchemaSource::xml(BLOCK_SCHEMA_XML))?);
    let modules = Generator::new(config).generate(&schema)?;
    let src = modules.modules().next().ok_or("one module")?.source.clone();
    let out = sink.lock().expect("lock").clone();
    Ok((out, src))
}

fn field<'a>(b: &'a Block, schema_name: &str) -> &'a ergo_sbe::FieldInfo {
    let found = b.fields.iter().find(|f| f.schema_name == schema_name);
    assert!(
        found.is_some(),
        "{}: no field {schema_name}: {:?}",
        b.name,
        b.fields
    );
    found.expect("asserted above")
}

fn compact(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// G1, G5, the multi-message bug: a hook fires once per group entry, nested
/// included, depth-first in schema order straight after the message decoder;
/// entry and group decoder names are the generated ones (message prefix and
/// parent-group prefix applied); `enclosing` is outermost first; the
/// message's group row names the prefixed group decoder; `schema_marker` is
/// the renamed `{Name}Schema` type.
#[test]
fn group_entry_contexts_name_the_generated_types() -> Result<(), Box<dyn std::error::Error>> {
    let (blocks, src) = capture_blocks()?;
    let order: Vec<(&str, &str)> = blocks.iter().map(|b| (b.kind, b.name.as_str())).collect();
    assert_eq!(
        order,
        vec![
            ("message", "QuoteDecoder"),
            ("entry", "QuoteLegsEntryDecoder"),
            ("entry", "QuoteLegsFillsEntryDecoder"),
            ("message", "OtherDecoder"),
            ("entry", "OtherLegsEntryDecoder"),
        ]
    );
    for b in blocks.iter().filter(|b| b.kind == "entry") {
        assert!(
            src.contains(&format!("pub struct {}<", b.name)),
            "entry type {} must exist in the generated source",
            b.name
        );
        let g = b.group_decoder.as_deref().ok_or("group decoder")?;
        assert!(
            src.contains(&format!("pub struct {g}<")),
            "group decoder {g} must exist in the generated source"
        );
    }
    let fills = blocks
        .iter()
        .find(|b| b.name == "QuoteLegsFillsEntryDecoder")
        .ok_or("fills")?;
    assert_eq!(
        fills.group_decoder.as_deref(),
        Some("QuoteLegsFillsDecoder")
    );
    let enclosing: Vec<&str> = fills.enclosing.iter().map(|(d, _)| d.as_str()).collect();
    assert_eq!(enclosing, vec!["QuoteDecoder", "QuoteLegsEntryDecoder"]);
    assert!(fills.enclosing[0].1.contains(&"priceScale".to_string()));
    let legs = blocks
        .iter()
        .find(|b| b.name == "QuoteLegsEntryDecoder")
        .ok_or("legs")?;
    assert_eq!(legs.group_unit.as_deref(), Some("lots"), "marks on <group>");

    let quote = &blocks[0];
    let marker = quote.schema_marker.as_deref().ok_or("marker")?;
    assert_ne!(marker, "QuoteSchema", "the composite took the plain name");
    assert!(
        src.contains(&format!("pub struct {marker}")),
        "marker {marker} must be the generated type"
    );
    let legs_row = field(quote, "legs");
    assert_eq!(legs_row.kind(), ergo_sbe::FieldKind::Group);
    assert_eq!(
        legs_row.rust_type, "QuoteLegsDecoder",
        "multi-message prefix"
    );
    Ok(())
}

/// G3, G4: `schema_name` keeps the schema spelling; constants are listed
/// with presence "constant"; fixed fields come first, then groups, then data.
#[test]
fn decoder_contexts_list_constants_and_schema_names() -> Result<(), Box<dyn std::error::Error>> {
    let (blocks, _) = capture_blocks()?;
    let quote = &blocks[0];
    let ps = field(quote, "priceScale");
    assert_eq!(ps.name, "price_scale");
    let k = field(quote, "k");
    assert_eq!(k.presence, "constant");
    let kinds: Vec<ergo_sbe::FieldKind> =
        quote.fields.iter().map(ergo_sbe::FieldInfo::kind).collect();
    let first_group = kinds
        .iter()
        .position(|k| *k != ergo_sbe::FieldKind::Fixed)
        .ok_or("tail")?;
    assert!(
        kinds[first_group..]
            .iter()
            .all(|k| *k != ergo_sbe::FieldKind::Fixed)
    );
    assert_eq!(field(quote, "note").kind(), ergo_sbe::FieldKind::Data);
    Ok(())
}

/// G2: `accessor` / `accessor_type` are the generated decoder method and its
/// exact return type for every fixed-field kind, at message and entry level.
#[test]
fn decoder_contexts_name_accessors_and_their_types() -> Result<(), Box<dyn std::error::Error>> {
    let (blocks, _) = capture_blocks()?;
    let quote = &blocks[0];
    let cases: &[(&str, &str, &str)] = &[
        ("priceScale", "price_scale", "i8"),
        ("price", "price_wire", "i64"),
        (
            "amount",
            "amount",
            "Result<u128, <u128 as TryFromSbe<u64>>::Error>",
        ),
        ("type", "type_", "u8"),
        ("wrap", "wrap_field", "u8"),
        ("maybe", "maybe", "Option<i32>"),
        ("late", "late", "Option<u32>"),
        ("key", "key", "[u8; 16]"),
        ("k", "k", "u16"),
    ];
    for (schema_name, accessor, ty) in cases {
        let f = field(quote, schema_name);
        assert_eq!(f.accessor.as_deref(), Some(*accessor), "{schema_name}");
        assert_eq!(
            f.accessor_type.as_deref().map(compact),
            Some(compact(ty)),
            "{schema_name}"
        );
    }
    for schema_name in ["venue", "flags", "pair"] {
        let f = field(quote, schema_name);
        assert!(
            f.accessor.is_some() && f.accessor_type.is_some(),
            "{schema_name}: {f:?}"
        );
    }
    let legs = field(quote, "legs");
    assert_eq!(legs.accessor.as_deref(), Some("legs"));
    assert!(
        legs.accessor_type
            .as_deref()
            .is_some_and(|t| compact(t).contains("QuoteLegsDecoder<")),
        "{legs:?}"
    );
    let entry = &blocks[1];
    let qty = field(entry, "qty");
    assert_eq!(qty.accessor.as_deref(), Some("qty"));
    assert_eq!(qty.accessor_type.as_deref(), Some("i64"));
    Ok(())
}

/// Encoder and composite contexts have no decoder accessor.
#[test]
fn non_decoder_contexts_have_no_accessor() -> Result<(), Box<dyn std::error::Error>> {
    let seen: Arc<Mutex<Vec<(String, bool)>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let config = GenerationConfig::new("noacc").with_hook(move |ctx: &ItemContext| {
        let any = |fields: &[ergo_sbe::FieldInfo]| {
            fields
                .iter()
                .any(|f| f.accessor.is_some() || f.accessor_type.is_some())
        };
        let row = match ctx {
            ItemContext::MessageEncoder { name, fields, .. }
            | ItemContext::Composite { name, fields, .. } => Some((name.clone(), any(fields))),
            _ => None,
        };
        if let Some(r) = row {
            sink.lock().expect("lock").push(r);
        }
        vec![]
    });
    let schema = Schema::from_ir(parse(SchemaSource::xml(BLOCK_SCHEMA_XML))?);
    let _ = Generator::new(config).generate(&schema)?;
    let rows = seen.lock().expect("lock").clone();
    assert_ne!(rows.len(), 0, "encoder and composite contexts fire");
    assert!(rows.iter().all(|(_, any)| !any), "{rows:?}");
    Ok(())
}
