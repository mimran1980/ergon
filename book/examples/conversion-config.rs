//! Self-contained demo: `with_conversion` vs `with_domain_type`.
//!
//! Compiles against `ergo-sbe`.  The book pages include anchored subsets;
//! this full file is what the book-fence test compiles to keep the examples
//! in sync with the real API.

use ergo_sbe::{ConversionSelector, GenerationConfig, Generator, Schema, parse};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -- setup: parse a minimal schema that has a Decimal composite ---------
    let xml = r#"<?xml version="1.0"?>
    <messageSchema package="demo" id="1" version="0" byteOrder="littleEndian">
      <types>
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
        <composite name="Decimal">
          <type name="mantissa" primitiveType="int64"/>
          <type name="exponent" primitiveType="int8"/>
        </composite>
      </types>
      <message name="Quote" id="1" blockLength="10">
        <field name="price" id="1" type="Decimal" offset="0"/>
      </message>
    </messageSchema>"#;
    let ir = parse(xml)?;
    let schema = Schema::from_ir(ir);

    // ANCHOR: with_conversion
    use ergo_sbe::{ConversionSelector, GenerationConfig};
    // A — generic converter: one wire type, many app types
    let _cfg =
        GenerationConfig::new("msgs").with_conversion(ConversionSelector::named_type("Decimal"));
    // ANCHOR_END: with_conversion

    // ANCHOR: with_domain_type
    use ergo_sbe::{ConversionSelector, GenerationConfig};
    // B — concrete mapping: one Rust type per wire type (already enables conversion)
    let _cfg = GenerationConfig::new("msgs").with_domain_type(
        ConversionSelector::named_type("Decimal"),
        "rust_decimal::Decimal",
    );
    // ANCHOR_END: with_domain_type

    // Prove both configs generate successfully.
    let _ = Generator::new(
        GenerationConfig::new("msgs_a").with_conversion(ConversionSelector::named_type("Decimal")),
    )
    .generate(&schema)?;
    let _ = Generator::new(GenerationConfig::new("msgs_b").with_domain_type(
        ConversionSelector::named_type("Decimal"),
        "rust_decimal::Decimal",
    ))
    .generate(&schema)?;
    Ok(())
}
