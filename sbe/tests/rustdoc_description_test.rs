//! Rustdoc description provenance — verify every SBE description attribute
//! produces a `#[doc = "..."]` on the generated Rust item.
//!
//! Tests inline XML so every element is guaranteed to have a description.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, unused)]

fn src(xml: &str, name: &str) -> String {
    let ir = ergo_sbe::parse(xml).expect("parse schema");
    let schema = ergo_sbe::Schema::from_ir(ir);
    ergo_sbe::Generator::new(ergo_sbe::GenerationConfig::new(name))
        .generate(&schema)
        .unwrap()
        .modules()
        .next()
        .unwrap()
        .source
        .clone()
}

fn assert_doc(source: &str, needle: &str, item_prefix: &str) {
    let lines: Vec<&str> = source.lines().collect();
    for gap in 0..8 {
        for w in lines.windows(gap + 1) {
            if w[gap].contains(item_prefix) {
                for i in 0..=gap {
                    if (w[i].contains("#[doc") || w[i].contains("///"))
                        && w[i].contains(needle)
                    {
                        return;
                    }
                }
            }
        }
    }
    panic!("expected doc '{needle}' within 8 lines before '{item_prefix}'");
}

fn schema(types: &str, messages: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
    package="t" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
{types}  </types>
{messages}</sbe:messageSchema>"#
    )
}

// ── Field ──────────────────────────────────────────────────────────────────

#[test]
fn field_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let s = src(&schema("", r#"<sbe:message name="M" id="1">
    <field name="price" id="1" type="uint32" description="Price in cents"/>
  </sbe:message>"#), "t");
    
    assert_doc(&s, "Price in cents", "pub fn price(&self)");
    Ok(())
}

#[test]
fn field_optional() -> Result<(), Box<dyn std::error::Error>> {
    let s = src(&schema("", r#"<sbe:message name="M" id="1">
    <field name="opt" id="1" type="uint32" presence="optional" nullValue="0"
           description="Optional value"/>
  </sbe:message>"#), "t");
    assert_doc(&s, "Optional value", "pub fn opt(&self)");
    Ok(())
}

// ── Composite + members ────────────────────────────────────────────────────

#[test]
fn composite_and_members() -> Result<(), Box<dyn std::error::Error>> {
    let s = src(&schema(
        r#"<composite name="Price" description="A price composite">
      <type name="mantissa" primitiveType="int64" description="The mantissa"/>
      <type name="exponent" primitiveType="int8" description="The exponent"/>
    </composite>
"#,
        r#"<sbe:message name="M" id="1">
    <field name="price" id="1" type="Price"/>
  </sbe:message>"#,
    ), "t");
    assert_doc(&s, "A price composite", "pub struct Price(");
    assert_doc(&s, "A price composite", "pub struct PriceDecoder");
    for (i, line) in s.lines().enumerate() { if line.contains("mantissa") { eprintln!("L{i}: {line}"); } }
    assert_doc(&s, "The mantissa", "pub fn mantissa(&self)");
    assert_doc(&s, "The exponent", "pub fn exponent(&self)");
    Ok(())
}

// ── Enum + valid values ────────────────────────────────────────────────────

#[test]
fn enum_and_valid_values() -> Result<(), Box<dyn std::error::Error>> {
    let s = src(&schema(
        r#"<enum name="Side" encodingType="uint8" description="Buy or sell">
      <validValue name="Buy" description="Buy order">1</validValue>
      <validValue name="Sell" description="Sell order">2</validValue>
    </enum>
"#,
        r#"<sbe:message name="M" id="1">
    <field name="side" id="1" type="Side"/>
  </sbe:message>"#,
    ), "t");
    assert_doc(&s, "Buy or sell", "pub struct Side");
    assert_doc(&s, "Buy order", "Buy => 1");
    assert_doc(&s, "Sell order", "Sell => 2");
    Ok(())
}

// ── Group + entry ──────────────────────────────────────────────────────────

#[test]
fn group_and_entry() -> Result<(), Box<dyn std::error::Error>> {
    let s = src(&schema(
        r#"<composite name="groupSizeEncoding">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="numInGroup" primitiveType="uint16"/>
    </composite>
"#,
        r#"<sbe:message name="Book" id="1">
    <group name="levels" id="1" dimensionType="groupSizeEncoding"
           description="Price levels">
      <field name="px" id="2" type="uint32" description="Price"/>
      <field name="sz" id="3" type="uint32" description="Size"/>
    </group>
  </sbe:message>"#,
    ), "t");
    assert_doc(&s, "Price levels", "pub struct LevelsDecoder");
    assert_doc(&s, "Price", "pub fn px(&self)");
    assert_doc(&s, "Size", "pub fn sz(&self)");
    Ok(())
}

// ── Var-data ───────────────────────────────────────────────────────────────

#[test]
fn var_data_field() -> Result<(), Box<dyn std::error::Error>> {
    let s = src(&schema(
        r#"<composite name="varStringEncoding">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="ASCII"/>
    </composite>
"#,
        r#"<sbe:message name="M" id="1">
    <field name="tag" id="1" type="uint32"/>
    <data name="symbol" id="2" type="varStringEncoding" description="Ticker"/>
  </sbe:message>"#,
    ), "t");
    for (i, line) in s.lines().enumerate() { if line.contains("symbol") { eprintln!("L{i}: {line}"); } }
    assert_doc(&s, "Ticker", "pub fn symbol");
    Ok(())
}

// ── Set ────────────────────────────────────────────────────────────────────

#[test]
fn set_field() -> Result<(), Box<dyn std::error::Error>> {
    let s = src(&schema(
        r#"<set name="Flags" encodingType="uint8" description="Event flags">
      <choice name="Trade">0</choice>
      <choice name="Quote">1</choice>
    </set>
"#,
        r#"<sbe:message name="M" id="1">
    <field name="flags" id="1" type="Flags"/>
  </sbe:message>"#,
    ), "t");
    assert_doc(&s, "Event flags", "pub struct Flags");
    Ok(())
}

// ── Message ────────────────────────────────────────────────────────────────

#[test]
fn message_level() -> Result<(), Box<dyn std::error::Error>> {
    let s = src(&schema("", r#"<sbe:message name="M" id="1" description="A test message">
    <field name="val" id="1" type="uint32"/>
  </sbe:message>"#), "t");
    assert_doc(&s, "A test message", "pub struct MDecoder");
    Ok(())
}

// ── Multi-line ─────────────────────────────────────────────────────────────

#[test]
fn multi_line() -> Result<(), Box<dyn std::error::Error>> {
    let s = src(&schema("", r#"<sbe:message name="M" id="1"
      description="Line one.&#10;Line two.">
    <field name="val" id="1" type="uint32"
        description="First.&#10;Second."/>
  </sbe:message>"#), "t");
    assert!(s.contains("Line one"));
    assert!(s.contains("Line two"));
    assert!(s.contains("First"));
    assert!(s.contains("Second"));
    Ok(())
}
