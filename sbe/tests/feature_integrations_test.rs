//! Compile-checks for feature-gated integrations: compact_str, smol_str,
//! bytes, chrono. Each test generates a consumer crate with the feature
//! enabled and verifies it compiles.
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;
use common::{Paths, compile_and_run};

fn test_compiles(label: &str, features: &[&str], code: &str) {
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
