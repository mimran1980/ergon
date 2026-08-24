//! A message-decoder hook may invoke a nested `Generator` (e.g. to emit a
//! companion crate, or just because a hook author's code happens to call
//! back into the library). Regression coverage for T-13: nested generation
//! must not leak its sealing path into the outer generation's remaining
//! messages. See `sbe/src/codegen/runtime.rs::GenerationContext`.
#![allow(clippy::restriction)]

use ergo_sbe::{GenerationConfig, Generator, ItemContext, Schema, parse};

const TWO_MESSAGE_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="reentrant" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="First" id="1" blockLength="4">
    <field name="value" id="1" type="int32"/>
  </message>
  <message name="Second" id="2" blockLength="4">
    <field name="value" id="1" type="int32"/>
  </message>
</messageSchema>"#;

const NESTED_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="nested" id="2" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Ping" id="1" blockLength="4">
    <field name="value" id="1" type="int32"/>
  </message>
</messageSchema>"#;

/// The outer generator's own sealing module (bare, no `with_external_sbe_rt`).
const OUTER_SEALED: &str = "__sbe_message_sealed";

fn nested_generate_hook(ctx: &ItemContext) -> Vec<proc_macro2::TokenStream> {
    if let ItemContext::MessageDecoder { name, .. } = ctx
        && name == "First"
    {
        // Run a complete nested generation with a DIFFERENT sealed path
        // than the outer one, synchronously, from inside the hook.
        let nested_ir = parse(NESTED_SCHEMA).expect("parse nested schema");
        let nested_schema = Schema::from_ir(nested_ir);
        let nested_config =
            GenerationConfig::new("nested").with_external_sbe_rt("crate::nested_owner::sbe_rt");
        Generator::new(nested_config)
            .generate(&nested_schema)
            .expect("nested generation must succeed");
    }
    vec![]
}

/// Outer generation emits its own `sbe_rt` (no `with_external_sbe_rt`); a
/// hook on its first message runs a nested generation that uses an
/// *external* sealed path. Every outer decoder/encoder — including the
/// second message, generated after the hook ran — must still seal against
/// the outer module, never the nested one.
#[test]
fn nested_generation_does_not_leak_sealed_path_into_outer_messages()
-> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(TWO_MESSAGE_SCHEMA)?;
    let schema = Schema::from_ir(ir);
    let config = GenerationConfig::new("reentrant_outer").with_hook(nested_generate_hook);
    let modules = Generator::new(config).generate(&schema)?;
    let src = modules.modules().next().ok_or("no module")?.source.clone();

    assert!(
        !src.contains("nested_owner"),
        "outer generation must never reference the nested generation's external sealed path:\n{src}"
    );
    let sealed_impl_count = src.matches(&format!("{OUTER_SEALED}::Sealed")).count();
    // One `SbeMessage: super::__sbe_message_sealed::Sealed` supertrait
    // declaration, plus two messages × (decoder + encoder) = 4 `Sealed`
    // impls — all against the outer module, including First's encoder and
    // Second's decoder/encoder, generated *after* the hook ran the nested
    // generation.
    assert_eq!(
        sealed_impl_count, 5,
        "expected 1 trait decl + 4 outer-sealed impls (First+Second, decoder+encoder):\n{src}"
    );

    Ok(())
}

/// Inverse case: the OUTER generation is the one using an external sealed
/// path; a hook-triggered nested generation emits its own *local* sealing
/// module. The outer messages generated after the hook must keep naming the
/// external path, not fall back to a local one.
#[test]
fn nested_local_generation_does_not_leak_into_outer_external_path()
-> Result<(), Box<dyn std::error::Error>> {
    fn hook(ctx: &ItemContext) -> Vec<proc_macro2::TokenStream> {
        if let ItemContext::MessageDecoder { name, .. } = ctx
            && name == "First"
        {
            let nested_ir = parse(NESTED_SCHEMA).expect("parse nested schema");
            let nested_schema = Schema::from_ir(nested_ir);
            // Nested generation is LOCAL this time (no external_sbe_rt).
            Generator::new(GenerationConfig::new("nested_local"))
                .generate(&nested_schema)
                .expect("nested generation must succeed");
        }
        vec![]
    }

    let ir = parse(TWO_MESSAGE_SCHEMA)?;
    let schema = Schema::from_ir(ir);
    let config = GenerationConfig::new("reentrant_outer_ext")
        .with_external_sbe_rt("crate::outer_owner::sbe_rt")
        .with_hook(hook);
    let modules = Generator::new(config).generate(&schema)?;
    let src = modules.modules().next().ok_or("no module")?.source.clone();

    // If the nested (local) generation leaked, at least one of these 4
    // outer impls would be missing the "crate::outer_owner::" qualification
    // (falling back to a bare, unqualified local sealed path instead), and
    // this count would drop below 4.
    let outer_sealed_count = src
        .matches("crate::outer_owner::__sbe_message_sealed::Sealed")
        .count();
    assert_eq!(
        outer_sealed_count, 4,
        "expected all 4 outer impls (First+Second, decoder+encoder) to keep \
         naming the external outer sealed path:\n{src}"
    );

    Ok(())
}
