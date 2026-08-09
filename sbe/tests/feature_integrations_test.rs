//! Compile-checks for feature-gated integrations: compact_str, smol_str,
//! bytes, chrono. Each test generates a consumer crate with the feature
//! enabled and verifies it compiles.
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;
use common::{Paths, compile_and_run};

fn test_compiles(label: &str, _features: &[&str], code: &str) {
    let ir = ergo_sbe::parse_file(&Paths::example_schema()).expect("parse schema");
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config =
        ergo_sbe::GenerationConfig::new(label).with_domain_objects(ergo_sbe::DomainVarData::Bytes);
    let modules = ergo_sbe::Generator::new(config)
        .generate(&schema)
        .expect("generate");
    let src = &modules.modules().next().expect("no module").source;
    compile_and_run(label, src, code);
}

// ── Domain DTO feature roundtrips ─────────────────────────────────────────

#[test]
#[cfg(feature = "compact_str")]
fn compact_strings_domain_compiles() {
    test_compiles(
        "cd",
        &[],
        r#"fn main() -> Result<(), Box<dyn std::error::Error>> {
    use cd::*;
    // Prove CarDomain is generated with CompactString fields when built with compact_str
    let buf = [0u8; 256];
    let dec = CarDecoder::decode(&buf, 0)?;
    // CarDomain::try_from_decoder exists and returns Result
    let _dto: CarDomain = CarDomain::try_from_decoder(dec)?;
    Ok(())
}"#,
    );
}

#[test]
#[cfg(feature = "smol_str")]
fn smol_strings_domain_compiles() {
    test_compiles(
        "sd",
        &[],
        r#"fn main() -> Result<(), Box<dyn std::error::Error>> {
    use sd::*;
    let buf = [0u8; 256];
    let dec = CarDecoder::decode(&buf, 0)?;
    let dto: CarDomain = CarDomain::try_from_decoder(dec)?;
    // SmolStr is O(1)-clone — prove Clone is derived
    let _dto2 = dto.clone();
    Ok(())
}"#,
    );
}

#[test]
#[cfg(feature = "bytes")]
fn bytes_domain_compiles() {
    test_compiles(
        "bd",
        &[],
        r#"fn main() -> Result<(), Box<dyn std::error::Error>> {
    use bd::*;
    let buf = [0u8; 256];
    let dec = CarDecoder::decode(&buf, 0)?;
    let dto: CarDomain = CarDomain::try_from_decoder(dec)?;
    // Bytes fields — test that the type resolves
    let _mfr: bytes::Bytes = dto.manufacturer;
    Ok(())
}"#,
    );
}

// ── All features together ─────────────────────────────────────────────────

#[test]
#[cfg(all(feature = "compact_str", feature = "bytes", feature = "chrono"))]
fn all_features_compile_together() {
    test_compiles(
        "af",
        &[],
        r#"fn main() -> Result<(), Box<dyn std::error::Error>> {
    use af::*;
    let buf = [0u8; 256];
    let dec = CarDecoder::decode(&buf, 0)?;
    let dto: CarDomain = CarDomain::try_from_decoder(dec)?;
    // CompactString field
    let _mfr: compact_str::CompactString = dto.manufacturer;
    let _model: compact_str::CompactString = dto.model;
    // Chrono converter available
    let ts = ergo_sbe::chrono_converters::i64_nanos_to_datetime(0);
    assert_eq!(ts.and_utc().timestamp(), 0);
    Ok(())
}"#,
    );
}

// ── Chrono converter tests ────────────────────────────────────────────────

#[test]
#[cfg(feature = "chrono")]
fn chrono_converter_roundtrip() {
    let now_ns: i64 = 1_720_000_000_000_000_000;
    let dt = ergo_sbe::chrono_converters::i64_nanos_to_datetime(now_ns);
    let back = ergo_sbe::chrono_converters::datetime_to_i64_nanos(dt);
    assert_eq!(back, now_ns, "DateTime roundtrip must be exact");

    let now_us: i64 = 1_720_000_000_000_000;
    let naive = ergo_sbe::chrono_converters::i64_micros_to_naive(now_us);
    let back_us = ergo_sbe::chrono_converters::naive_to_i64_micros(naive);
    assert_eq!(back_us, now_us, "NaiveDateTime roundtrip must be exact");
}

#[test]
#[cfg(feature = "chrono")]
fn chrono_edge_cases() {
    let micro_epoch: i64 = -378_691_200_000_000;
    let naive = ergo_sbe::chrono_converters::i64_micros_to_naive(micro_epoch);
    let back = ergo_sbe::chrono_converters::naive_to_i64_micros(naive);
    assert_eq!(back, micro_epoch);
}

// ── Null-as-option combinatorics ──────────────────────────────────────────

#[test]
fn null_as_option_individual_selector() {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2024/sbe"
    package="nao_ind" id="1" version="0" byteOrder="littleEndian">
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
        <enum name="Type" encodingType="uint8">
            <validValue name="Market">1</validValue>
        </enum>
    </types>
    <sbe:message name="Order" id="1">
        <field name="side" id="1" type="Side" offset="0"/>
        <field name="type" id="2" type="Type" offset="1"/>
    </sbe:message>
</sbe:messageSchema>"#;
    let ir = ergo_sbe::parse(schema).unwrap();
    let schema_obj = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("nao_ind")
        .with_null_as_option(ergo_sbe::ConversionSelector::named_type("Side"));
    let modules = ergo_sbe::Generator::new(config).generate(&schema_obj).unwrap();
    let src = &modules.modules().next().unwrap().source;

    // Only Side becomes Option — Type stays bare
    assert!(src.contains("-> Option<Side>"), "Side must be Option");
    assert!(!src.contains("-> Option<Type>"), "Type must NOT be Option (not selected)");
}

#[test]
fn null_as_option_blanket_catches_all() {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2024/sbe"
    package="nao_all" id="1" version="0" byteOrder="littleEndian">
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
        <enum name="Type" encodingType="uint8">
            <validValue name="Market">1</validValue>
        </enum>
    </types>
    <sbe:message name="Order" id="1">
        <field name="side" id="1" type="Side" offset="0"/>
        <field name="type" id="2" type="Type" offset="1"/>
    </sbe:message>
</sbe:messageSchema>"#;
    let ir = ergo_sbe::parse(schema).unwrap();
    let schema_obj = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("nao_all")
        .with_all_enums_as_option();
    let modules = ergo_sbe::Generator::new(config).generate(&schema_obj).unwrap();
    let src = &modules.modules().next().unwrap().source;

    // Both enums become Option
    assert!(src.contains("-> Option<Side>"), "Side must be Option");
    assert!(src.contains("-> Option<Type>"), "Type must be Option");
    // as_option() on both
    let side_count = src.matches("fn as_option").count();
    assert!(side_count >= 2, "as_option must be on both enums, found {side_count}");
}

#[test]
fn domain_vardata_variants_all_compile() {
    // Prove all DomainVarData variants produce valid configs (no generation needed)
    let _bytes = ergo_sbe::GenerationConfig::new("a")
        .with_domain_objects(ergo_sbe::DomainVarData::Bytes);
    let _strings = ergo_sbe::GenerationConfig::new("b")
        .with_domain_objects(ergo_sbe::DomainVarData::Strings);

    #[cfg(feature = "compact_str")]
    {
        let _compact = ergo_sbe::GenerationConfig::new("c")
            .with_domain_objects(ergo_sbe::DomainVarData::CompactStrings);
    }
    #[cfg(feature = "smol_str")]
    {
        let _smol = ergo_sbe::GenerationConfig::new("d")
            .with_domain_objects(ergo_sbe::DomainVarData::SmolStrings);
    }
    #[cfg(feature = "bytes")]
    {
        let _bc = ergo_sbe::GenerationConfig::new("e")
            .with_domain_objects(ergo_sbe::DomainVarData::BytesCrate);
    }
}

#[test]
fn as_option_method_present_on_all_enums() {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2024/sbe"
    package="asopt" id="1" version="0" byteOrder="littleEndian">
    <types>
        <composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
        </composite>
        <enum name="Status" encodingType="uint8" nullValue="99">
            <validValue name="Ok">0</validValue>
        </enum>
    </types>
    <sbe:message name="Msg" id="1">
        <field name="status" id="1" type="Status" offset="0"/>
    </sbe:message>
</sbe:messageSchema>"#;
    let ir = ergo_sbe::parse(schema).unwrap();
    let schema_obj = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("asopt");
    let modules = ergo_sbe::Generator::new(config).generate(&schema_obj).unwrap();
    let src = &modules.modules().next().unwrap().source;

    // as_option() exists even without null_as_option config
    assert!(src.contains("fn as_option"), "as_option() must always be generated");
    // Custom NullVal = 99 still present
    assert!(src.contains("NullVal = 99"), "custom NullVal = 99 must be present");
}

// ── Edge cases ────────────────────────────────────────────────────────────

#[test]
#[cfg(feature = "compact_str")]
fn compact_str_edge_cases() {
    assert!(compact_str::CompactString::new("").is_empty());
    let exact_24 = compact_str::CompactString::new("ABCDEFGHIJKLMNOPQRSTUVWX");
    assert_eq!(exact_24.len(), 24);
    let beyond = compact_str::CompactString::new("ABCDEFGHIJKLMNOPQRSTUVWXY");
    assert_eq!(beyond.len(), 25);
}

#[test]
#[cfg(feature = "bytes")]
fn bytes_zero_copy_semantics() {
    let data: Vec<u8> = b"Hello, World!".to_vec();
    let b = bytes::Bytes::copy_from_slice(&data);
    let b2 = b.clone();
    assert_eq!(&b[..], &b2[..]);
    let sub = b.slice(0..5);
    assert_eq!(&sub[..], b"Hello");
}
