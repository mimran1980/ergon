//! Extension XML reaches hooks: every `ItemContext` variant and the
//! `FieldInfo` / `EnumVariantInfo` / `SetChoiceInfo` metadata it carries are
//! populated from the schema's extension content, and a hook can emit working
//! code from that data. Compilation of the emitted code is the assertion.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

mod common;
use common::compile_and_run;

use ergo_sbe::{DomainVarData, GenerationConfig, ItemContext};

/// Venue-neutral schema: namespaced attributes on every hook-visible element,
/// one foreign child element under the message, and marks that differ per
/// element so a wrong threading site is visible.
const SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema xmlns:x="urn:example" package="ext" id="9" version="0" byteOrder="littleEndian">
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
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
    </composite>
    <enum name="Side" encodingType="uint8" x:kind="labelled">
      <validValue name="Buy" x:label="B">1</validValue>
      <validValue name="Sell" x:label="S">2</validValue>
    </enum>
    <set name="Flags" encodingType="uint8" x:kind="bits">
      <choice name="frozen" x:note="F">1</choice>
      <choice name="active" x:note="A">2</choice>
    </set>
    <composite name="Pair" x:kind="pair">
      <type name="a" primitiveType="int64" x:unit="cents"/>
      <type name="b" primitiveType="int64"/>
    </composite>
  </types>
  <message name="Order" id="1" x:topic="orders" x:owner="root">
    <x:meta source="ingest"/>
    <field name="side" id="1" type="Side"/>
    <field name="qty" id="2" type="int64" x:unit="lots"/>
    <group name="legs" id="10" dimensionType="groupSizeEncoding" x:scope="leg" x:owner="legs">
      <field name="px" id="11" type="int64" x:unit="cents"/>
    </group>
    <data name="memo" id="20" type="varStringEncoding" x:encoding="utf8"/>
  </message>
</messageSchema>"#;

/// Reads extension data off every context kind and emits it as inherent items.
/// `MessageEncoder` reuses the decoder arm: `for_encode` must carry the
/// message's extensions into the encoder view too.
fn ext_hook(ctx: &ItemContext) -> Vec<proc_macro2::TokenStream> {
    use quote::{format_ident, quote};
    match ctx {
        ItemContext::Enum {
            name,
            variants,
            extensions,
            ..
        } => {
            let ident = format_ident!("{name}");
            let mut items = Vec::new();
            if let Some(kind) = extensions.attr("kind") {
                items.push(quote! {
                    impl #ident { pub const KIND: &'static str = #kind; }
                });
            }
            let mut arms = Vec::new();
            for v in variants {
                if let Some(label) = v.extensions.attr("label") {
                    let var = format_ident!("{}", v.name);
                    arms.push(quote! { Self::#var => #label, });
                }
            }
            if !arms.is_empty() {
                // Generated enums carry a `NullVal` variant the schema never
                // declares, so the match always needs a wildcard arm.
                items.push(quote! {
                    impl #ident {
                        pub fn label(&self) -> &'static str {
                            match self { #(#arms)* _ => "UNKNOWN" }
                        }
                    }
                });
            }
            items
        }
        ItemContext::Set {
            name,
            choices,
            extensions,
            ..
        } => {
            let ident = format_ident!("{name}");
            let mut consts = Vec::new();
            if let Some(kind) = extensions.attr("kind") {
                consts.push(quote! { pub const KIND: &'static str = #kind; });
            }
            for c in choices {
                if let Some(note) = c.extensions.attr("note") {
                    let cst = format_ident!("{}_NOTE", c.name.to_uppercase());
                    consts.push(quote! { pub const #cst: &'static str = #note; });
                }
            }
            if consts.is_empty() {
                vec![]
            } else {
                vec![quote! { impl #ident { #(#consts)* } }]
            }
        }
        ItemContext::Composite {
            name,
            fields,
            extensions,
            ..
        } => {
            let ident = format_ident!("{name}");
            let mut consts = Vec::new();
            if let Some(kind) = extensions.attr("kind") {
                consts.push(quote! { pub const KIND: &'static str = #kind; });
            }
            for f in fields {
                if let Some(unit) = f.extensions.attr("unit") {
                    let cst = format_ident!("{}_UNIT", f.name.to_uppercase());
                    consts.push(quote! { pub const #cst: &'static str = #unit; });
                }
            }
            if consts.is_empty() {
                vec![]
            } else {
                vec![quote! { impl #ident { #(#consts)* } }]
            }
        }
        ItemContext::MessageDecoder {
            name,
            fields,
            extensions,
            ..
        }
        | ItemContext::MessageEncoder {
            name,
            fields,
            extensions,
            ..
        } => {
            let ident = format_ident!("{name}");
            let mut assoc = Vec::new();
            if let Some(topic) = extensions.attr("topic") {
                assoc.push(quote! { pub const TOPIC: &'static str = #topic; });
            }
            if let Some(source) = extensions.child("meta").and_then(|m| m.attr("source")) {
                assoc.push(quote! { pub const META_SOURCE: &'static str = #source; });
            }
            for f in fields {
                if let Some(unit) = f.extensions.attr("unit") {
                    let fn_ident = format_ident!("{}_unit", f.name);
                    assoc.push(quote! { pub fn #fn_ident() -> &'static str { #unit } });
                }
                if let Some(scope) = f.extensions.attr("scope") {
                    let fn_ident = format_ident!("{}_scope", f.name);
                    assoc.push(quote! { pub fn #fn_ident() -> &'static str { #scope } });
                }
                if let Some(encoding) = f.extensions.attr("encoding") {
                    let fn_ident = format_ident!("{}_encoding", f.name);
                    assoc.push(quote! { pub fn #fn_ident() -> &'static str { #encoding } });
                }
            }
            if assoc.is_empty() {
                vec![]
            } else {
                vec![quote! { impl<'a> #ident<'a> { #(#assoc)* } }]
            }
        }
        ItemContext::DomainStruct {
            name, extensions, ..
        } => {
            let ident = format_ident!("{name}");
            match extensions.attr("owner") {
                Some(owner) => vec![quote! {
                    impl #ident { pub const OWNER: &'static str = #owner; }
                }],
                None => vec![],
            }
        }
    }
}

#[test]
fn hook_emitted_code_compiles_and_runs() -> Result<(), Box<dyn std::error::Error>> {
    let ir = ergo_sbe::parse(ergo_sbe::SchemaSource::xml(SCHEMA))?;
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = GenerationConfig::new("ext_hook")
        .with_domain_objects(DomainVarData::Bytes)
        .with_hook(ext_hook);
    let modules = ergo_sbe::Generator::new(config).generate(&schema)?;
    let src = modules.modules().next().ok_or("one module")?.source.clone();

    compile_and_run(
        "ext_hook",
        &src,
        r#"
        // ItemContext::Enum: element extensions + per-variant extensions.
        assert_eq!(Side::KIND, "labelled");
        assert_eq!(Side::Buy.label(), "B");
        assert_eq!(Side::Sell.label(), "S");
        // ItemContext::Set: element extensions + per-choice extensions.
        assert_eq!(Flags::KIND, "bits");
        assert_eq!(Flags::FROZEN_NOTE, "F");
        assert_eq!(Flags::ACTIVE_NOTE, "A");
        // ItemContext::Composite: element extensions + per-member FieldInfo.
        assert_eq!(Pair::KIND, "pair");
        assert_eq!(Pair::A_UNIT, "cents");
        // ItemContext::MessageDecoder: element extensions (incl. foreign child)
        // + FieldInfo for fixed fields, group entries and var-data.
        assert_eq!(OrderDecoder::TOPIC, "orders");
        assert_eq!(OrderDecoder::META_SOURCE, "ingest");
        assert_eq!(OrderDecoder::qty_unit(), "lots");
        assert_eq!(OrderDecoder::legs_scope(), "leg");
        assert_eq!(OrderDecoder::memo_encoding(), "utf8");
        // The encoder view (`for_encode`) must carry the message extensions too.
        assert_eq!(OrderEncoder::TOPIC, "orders");
        assert_eq!(OrderEncoder::qty_unit(), "lots");
        // ItemContext::DomainStruct: message DTO gets the message's marks,
        // entry DTOs the group's.
        assert_eq!(OrderDomain::OWNER, "root");
        assert_eq!(OrderLegsEntryDomain::OWNER, "legs");
        "#,
    );
    Ok(())
}
