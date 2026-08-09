//! Feature integration tests: null_as_option, DomainVarData variants, chrono,
//! compact_str/smol_str/bytes edge cases.
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::error::Error;

const MSG_HEADER_XML: &str = r#"<composite name="messageHeader">
    <type name="blockLength" primitiveType="uint16"/>
    <type name="templateId" primitiveType="uint16"/>
    <type name="schemaId" primitiveType="uint16"/>
    <type name="version" primitiveType="uint16"/>
</composite>"#;

fn generate_with_config(
    config: ergo_sbe::GenerationConfig,
    types_xml: &str,
    fields: &str,
) -> Result<String, Box<dyn Error>> {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2024/sbe"
    package="test_pkg" id="1" version="0" byteOrder="littleEndian">
    <types>{MSG_HEADER_XML}{types_xml}</types>
    <sbe:message name="Msg" id="1">{fields}</sbe:message>
</sbe:messageSchema>"#
    );
    let ir = ergo_sbe::parse(&xml)?;
    let schema = ergo_sbe::Schema::from_ir(ir);
    let modules = ergo_sbe::Generator::new(config).generate(&schema)?;
    let source = modules.modules().next().ok_or("no module generated")?.source.clone();
    Ok(source)
}

// ── Null-as-option ────────────────────────────────────────────────────────

#[test]
fn null_as_option_individual_selector() -> Result<(), Box<dyn Error>> {
    let types = r#"<enum name="Side" encodingType="uint8">
        <validValue name="Buy">1</validValue><validValue name="Sell">2</validValue>
    </enum><enum name="Type" encodingType="uint8">
        <validValue name="Market">1</validValue>
    </enum>"#;
    let fields = r#"<field name="side" id="1" type="Side" offset="0"/>
        <field name="type" id="2" type="Type" offset="1"/>"#;
    let config = ergo_sbe::GenerationConfig::new("nao_ind")
        .with_null_as_option(ergo_sbe::ConversionSelector::named_type("Side"));
    let src = generate_with_config(config.clone(), types, fields)?;
    assert!(src.contains("-> Option<Side>"), "Side must be Option");
    assert!(
        !src.contains("-> Option<Type>"),
        "Type must NOT be Option (not selected)"
    );
    Ok(())
}

#[test]
fn null_as_option_blanket_catches_all() -> Result<(), Box<dyn Error>> {
    let types = r#"<enum name="Side" encodingType="uint8">
        <validValue name="Buy">1</validValue>
    </enum><enum name="Type" encodingType="uint8">
        <validValue name="Market">1</validValue>
    </enum>"#;
    let fields = r#"<field name="side" id="1" type="Side" offset="0"/>
        <field name="type" id="2" type="Type" offset="1"/>"#;
    let config = ergo_sbe::GenerationConfig::new("nao_all").with_all_enums_as_option();
    let src = generate_with_config(config.clone(), types, fields)?;
    assert!(src.contains("-> Option<Side>"), "Side must be Option");
    assert!(src.contains("-> Option<Type>"), "Type must be Option");
    let as_opt_count = src.matches("fn as_option").count();
    assert!(
        as_opt_count >= 2,
        "as_option must be on both enums, found {as_opt_count}"
    );
    Ok(())
}

#[test]
fn as_option_method_on_all_enums() -> Result<(), Box<dyn Error>> {
    let types = r#"<enum name="Status" encodingType="uint8" nullValue="99">
        <validValue name="Ok">0</validValue>
    </enum>"#;
    let fields = r#"<field name="status" id="1" type="Status" offset="0"/>"#;
    let config = ergo_sbe::GenerationConfig::new("asopt");
    let src = generate_with_config(config.clone(), types, fields)?;
    assert!(
        src.contains("fn as_option"),
        "as_option() must always be generated"
    );
    assert!(
        src.contains("NullVal = 99"),
        "custom NullVal = 99 must be present"
    );
    Ok(())
}

#[test]
fn null_as_option_generates_option_getter() -> Result<(), Box<dyn Error>> {
    let types = r#"<enum name="Side" encodingType="uint8">
        <validValue name="Buy">1</validValue><validValue name="Sell">2</validValue>
    </enum>"#;
    let fields = r#"<field name="side" id="1" type="Side" offset="0"/>"#;
    let config = ergo_sbe::GenerationConfig::new("nao_rt")
        .with_null_as_option(ergo_sbe::ConversionSelector::named_type("Side"));
    let src = generate_with_config(config.clone(), types, fields)?;
    assert!(src.contains("as_option()"), "getter must use as_option() for NullVal→None mapping");
    assert!(src.contains("-> Option<Side>"), "getter must return Option<Side>");
    Ok(())
}

// ── DomainVarData variants ───────────────────────────────────────────────

#[test]
fn domain_vardata_variants_all_compile() -> Result<(), Box<dyn Error>> {
    let _bytes =
        ergo_sbe::GenerationConfig::new("a").with_domain_objects(ergo_sbe::DomainVarData::Bytes);
    let _strings =
        ergo_sbe::GenerationConfig::new("b").with_domain_objects(ergo_sbe::DomainVarData::Strings);
    #[cfg(feature = "compact_str")]
    {
        let _c = ergo_sbe::GenerationConfig::new("c")
            .with_domain_objects(ergo_sbe::DomainVarData::CompactStrings);
    }
    #[cfg(feature = "smol_str")]
    {
        let _d = ergo_sbe::GenerationConfig::new("d")
            .with_domain_objects(ergo_sbe::DomainVarData::SmolStrings);
    }
    #[cfg(feature = "bytes")]
    {
        let _e = ergo_sbe::GenerationConfig::new("e")
            .with_domain_objects(ergo_sbe::DomainVarData::BytesCrate);
    }
    Ok(())
}

// ── Chrono ────────────────────────────────────────────────────────────────

#[test]
#[cfg(feature = "chrono")]
fn chrono_converter_roundtrip() -> Result<(), Box<dyn Error>> {
    let now_ns: i64 = 1_720_000_000_000_000_000;
    let dt = ergo_sbe::chrono_converters::i64_nanos_to_datetime(now_ns);
    let back = ergo_sbe::chrono_converters::datetime_to_i64_nanos(dt);
    assert_eq!(back, now_ns, "DateTime roundtrip must be exact");
    let now_us: i64 = 1_720_000_000_000_000;
    let naive = ergo_sbe::chrono_converters::i64_micros_to_naive(now_us);
    assert_eq!(
        ergo_sbe::chrono_converters::naive_to_i64_micros(naive),
        now_us
    );
    Ok(())
}

#[test]
#[cfg(feature = "chrono")]
fn chrono_edge_cases() -> Result<(), Box<dyn Error>> {
    let micro_epoch: i64 = -378_691_200_000_000;
    let naive = ergo_sbe::chrono_converters::i64_micros_to_naive(micro_epoch);
    assert_eq!(
        ergo_sbe::chrono_converters::naive_to_i64_micros(naive),
        micro_epoch
    );
    Ok(())
}

// ── Edge cases ────────────────────────────────────────────────────────────

#[test]
#[cfg(feature = "compact_str")]
fn compact_str_edge_cases() {
    assert!(compact_str::CompactString::new("").is_empty());
    assert_eq!(
        compact_str::CompactString::new("ABCDEFGHIJKLMNOPQRSTUVWX").len(),
        24
    );
    assert_eq!(
        compact_str::CompactString::new("ABCDEFGHIJKLMNOPQRSTUVWXY").len(),
        25
    );
}

#[test]
#[cfg(feature = "bytes")]
fn bytes_zero_copy_semantics() {
    let data: Vec<u8> = b"Hello, World!".to_vec();
    let b = bytes::Bytes::copy_from_slice(&data);
    let b2 = b.clone();
    assert_eq!(&b[..], &b2[..]);
    assert_eq!(&b.slice(0..5)[..], b"Hello");
}
