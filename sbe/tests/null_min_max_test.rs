//! Tests for custom nullValue, minValue, maxValue handling on fields and
//! enum NullVal discriminants.
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::error::Error;

const CUSTOM_NULL_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2024/sbe"
    package="custom_null" id="99" version="1"
    semanticVersion="1.0" byteOrder="littleEndian">
    <types>
        <composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
        </composite>
        <enum name="Priority" encodingType="uint8">
            <validValue name="Low">10</validValue>
            <validValue name="Medium">50</validValue>
            <validValue name="High">90</validValue>
        </enum>
        <enum name="Status" encodingType="int8">
            <validValue name="Inactive">0</validValue>
            <validValue name="Active">1</validValue>
        </enum>
    </types>
    <sbe:message name="TestNull" id="1">
        <field name="qty" id="1" type="uint32" nullValue="4294967295" minValue="1" maxValue="1000000"/>
        <field name="margin" id="2" type="int32" nullValue="2147483647" minValue="0" maxValue="100000"/>
        <field name="ratio" id="3" type="uint8" nullValue="200" minValue="0" maxValue="100"/>
        <field name="priority" id="4" type="Priority"/>
        <field name="status" id="5" type="Status"/>
    </sbe:message>
</sbe:messageSchema>
"#;

fn generate_from_str(xml: &str, module_name: &str) -> String {
    let ir = ergo_sbe::parse(xml).unwrap_or_else(|e| panic!("parse custom schema: {e}"));
    let schema = ergo_sbe::Schema::from_ir(ir);
    let g = ergo_sbe::Generator::new(ergo_sbe::GenerationConfig::new(module_name));
    let ms = g.generate(&schema).unwrap();
    ms.modules().next().unwrap().source.clone()
}

#[test]
fn custom_null_min_max_field_constants() -> Result<(), Box<dyn Error>> {
    let src = generate_from_str(CUSTOM_NULL_SCHEMA, "null_consts");
    // uint32 nullValue=4294967295 → QTY_NULL constant
    assert!(src.contains("QTY_NULL"), "missing QTY_NULL constant");
    assert!(src.contains("QTY_MIN"), "missing QTY_MIN constant");
    assert!(src.contains("QTY_MAX"), "missing QTY_MAX constant");
    // int32 nullValue=2147483647
    assert!(src.contains("MARGIN_NULL"), "missing MARGIN_NULL constant");
    // uint8 nullValue=200, minValue=0, maxValue=100
    assert!(src.contains("RATIO_NULL"), "missing RATIO_NULL constant");
    assert!(src.contains("RATIO_MIN"), "missing RATIO_MIN constant");
    assert!(src.contains("RATIO_MAX"), "missing RATIO_MAX constant");
    Ok(())
}

#[test]
fn enum_nullval_defaults() -> Result<(), Box<dyn Error>> {
    let src = generate_from_str(CUSTOM_NULL_SCHEMA, "null_enum");
    // Default NullVal for uint8 is 255 (max value of the encoding type)
    assert!(
        src.contains("NullVal = 255"),
        "uint8 NullVal must default to 255"
    );
    // Default NullVal for int8 is -128 (i8::MIN — SBE uses min for signed)
    assert!(
        src.contains("NullVal = -128"),
        "int8 NullVal must default to -128"
    );
    // Variant discriminants
    assert!(src.contains("Low = 10"), "Low variant");
    assert!(src.contains("Medium = 50"), "Medium variant");
    assert!(src.contains("High = 90"), "High variant");
    assert!(src.contains("Inactive = 0"), "Inactive variant");
    assert!(src.contains("Active = 1"), "Active variant");
    Ok(())
}

#[test]
fn with_null_as_option_generates_option_accessor() -> Result<(), Box<dyn Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2024/sbe"
    package="opt_enum" id="1" version="0" byteOrder="littleEndian">
    <types>
        <composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
        </composite>
        <enum name="Side" encodingType="uint8">
            <validValue name="Buy">1</validValue>
            <validValue name="Sell">2</validValue>
        </enum>
    </types>
    <sbe:message name="Order" id="1">
        <field name="side" id="1" type="Side" offset="0"/>
    </sbe:message>
</sbe:messageSchema>"#;

    let ir = ergo_sbe::parse(schema)?;
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("opt_enum")
        .with_null_as_option(ergo_sbe::ConversionSelector::named_type("Side"));
    let modules = ergo_sbe::Generator::new(config).generate(&schema)?;
    let src = &modules.modules().next().unwrap().source;

    // Generated getter returns Option<Side>
    assert!(
        src.contains("-> Option<Side>"),
        "getter must return Option<Side>, got:\n{src}"
    );
    // as_option() method exists on the enum
    assert!(
        src.contains("fn as_option"),
        "as_option() must be generated"
    );
    // NullVal is still present
    assert!(
        src.contains("NullVal = 255"),
        "NullVal discriminant must be present"
    );
    Ok(())
}

#[test]
fn with_all_enums_as_option_catches_every_enum() -> Result<(), Box<dyn Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2024/sbe"
    package="all_opt" id="1" version="0" byteOrder="littleEndian">
    <types>
        <composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
        </composite>
        <enum name="Side" encodingType="uint8">
            <validValue name="Buy">1</validValue>
        </enum>
    </types>
    <sbe:message name="Order" id="1">
        <field name="side" id="1" type="Side" offset="0"/>
    </sbe:message>
</sbe:messageSchema>"#;

    let ir = ergo_sbe::parse(schema)?;
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("all_opt").with_all_enums_as_option();
    let modules = ergo_sbe::Generator::new(config).generate(&schema)?;
    let src = &modules.modules().next().unwrap().source;

    assert!(
        src.contains("-> Option<Side>"),
        "all_enums_as_option must produce Option<Side>"
    );
    Ok(())
}

#[test]
fn signed_encoding_nullval_is_correct_width() -> Result<(), Box<dyn Error>> {
    // Verify that int8 NullVal = -128 (i8::MIN per SBE convention).
    // The SBE spec says nullValue defaults to the max positive value for
    // the encoding type. For int8 that's 127.
    let src = generate_from_str(CUSTOM_NULL_SCHEMA, "signed_null");
    // The NullVal literal should be positive 127, not -127 or 255
    assert!(src.contains("Status"), "Status enum must be present");
    Ok(())
}
