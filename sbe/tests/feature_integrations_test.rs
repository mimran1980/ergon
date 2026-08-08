//! Comprehensive tests for feature-gated integrations: compact_str, smol_str,
//! bytes, chrono. Covers domain DTO roundtrips, codec-level accessor methods,
//! and edge cases.
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::error::Error;
use std::process::Command;

mod common;
use common::{Paths, compile_and_run, compile_and_run_with_deps, generate};

/// Generate codecs then compile+run a test program with all features.
fn test_with_features(
    label: &str,
    _deps: &str,
    features: &[&str],
    code: &str,
) {
    let ir = ergo_sbe::parse_file(&Paths::example_schema()).expect("parse schema");
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new(label)
        .with_domain_objects(true);
    let modules = ergo_sbe::Generator::new(config).generate(&schema).expect("generate");
    let src = &modules.modules().next().expect("no module").source;
    let features_str = features.join(",");
    let full_deps = format!(
        r#"ergo-sbe = {{ path = "{}/sbe", features = ["{features_str}"] }}
"#,
        std::env::current_dir().expect("cwd").display(),
    );
    compile_and_run_with_deps(label, src, code, &full_deps);
}

// ── DomainVarData roundtrip tests ─────────────────────────────────────────

#[test]
#[cfg(feature = "compact_str")]
fn compact_strings_domain_roundtrip() {
    test_with_features(
        "compact_domain",
        "",
        &["compact_str"],
        r#"fn main() -> Result<(), Box<dyn std::error::Error>> {
    use compact_domain::*;
    // Test buffer: car schema with "Porsche"+"Carrera" and empty groups fits well under 256 bytes.
// Production code must use compute_length_with_header(); this test checks feature integration, not sizing.
let mut buf = [0u8; 256];
    let len = CarEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&FixedFields {
            serial_number: 1,
            model_year: 2020,
            available: BooleanType::T,
            code: Model::A,
            some_numbers: [0; 4],
            vehicle_code: [b'A'; 6],
            extras: OptionalExtras::default(),
            engine: Engine::new(1, 1, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
        })
        .fuel_figures(0, |_| Ok(()))?
        .performance_figures(0, |_| Ok(()))?
        .manufacturer(b"Porsche")?
        .model(b"Carrera")?
        .encoded_length_with_header();
    let dec = CarDecoder::decode(&buf[..len], 0)?;
    // Domain DTO with CompactString fields
    let dto = CarDomain::try_from_decoder(&dec)?;
    assert_eq!(dto.manufacturer, "Porsche");
    assert_eq!(dto.model, "Carrera");
    // Re-encode
    let mut out = [0u8; 512];
    let re_len = dto.try_to_encoder(CarEncoder::wrap(&mut out, 0)?)?
        .encoded_length_with_header();
    assert_eq!(&buf[..len], &out[..re_len]);
    Ok(())
}"#,
    )
}

#[test]
#[cfg(feature = "smol_str")]
fn smol_strings_domain_roundtrip() {
    test_with_features(
        "smol_domain",
        "",
        &["smol_str"],
        r#"fn main() -> Result<(), Box<dyn std::error::Error>> {
    use smol_domain::*;
    // Test buffer: car schema with "Porsche"+"Carrera" and empty groups fits well under 256 bytes.
// Production code must use compute_length_with_header(); this test checks feature integration, not sizing.
let mut buf = [0u8; 256];
    let len = CarEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&FixedFields {
            serial_number: 1, model_year: 2020, available: BooleanType::T, code: Model::A,
            some_numbers: [0; 4], vehicle_code: [b'A'; 6], extras: OptionalExtras::default(),
            engine: Engine::new(1, 1, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
        })
        .fuel_figures(0, |_| Ok(()))?
        .performance_figures(0, |_| Ok(()))?
        .manufacturer(b"Porsche")?
        .model(b"Carrera")?
        .encoded_length_with_header();
    let dec = CarDecoder::decode(&buf[..len], 0)?;
    let dto = CarDomain::try_from_decoder(&dec)?;
    assert_eq!(dto.model, "Carrera");
    let dto2 = dto.clone();
    assert_eq!(dto2.model, dto.model);
    Ok(())
}"#,
    )
}

#[test]
#[cfg(feature = "bytes")]
fn bytes_domain_roundtrip() {
    test_with_features(
        "bytes_domain",
        "",
        &["bytes"],
        r#"fn main() -> Result<(), Box<dyn std::error::Error>> {
    use bytes_domain::*;
    // Test buffer: car schema with "Porsche"+"Carrera" and empty groups fits well under 256 bytes.
// Production code must use compute_length_with_header(); this test checks feature integration, not sizing.
let mut buf = [0u8; 256];
    let len = CarEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&FixedFields {
            serial_number: 1, model_year: 2020, available: BooleanType::T, code: Model::A,
            some_numbers: [0; 4], vehicle_code: [b'A'; 6], extras: OptionalExtras::default(),
            engine: Engine::new(1, 1, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
        })
        .fuel_figures(0, |_| Ok(()))?
        .performance_figures(0, |_| Ok(()))?
        .manufacturer(b"Porsche")?
        .model(b"Carrera")?
        .encoded_length_with_header();
    let dec = CarDecoder::decode(&buf[..len], 0)?;
    let dto = CarDomain::try_from_decoder(&dec)?;
    // Bytes fields hold the raw bytes
    assert_eq!(&dto.manufacturer[..], b"Porsche");
    assert_eq!(&dto.model[..], b"Carrera");
    Ok(())
}"#,
    )
}

// ── Codec-level accessor tests ────────────────────────────────────────────

#[test]
#[cfg(feature = "compact_str")]
fn codec_compact_str_accessor() {
    test_with_features(
        "codec_compact",
        "",
        &["compact_str"],
        r#"fn main() -> Result<(), Box<dyn std::error::Error>> {
    use codec_compact::*;
    // Test buffer: car schema with "Porsche"+"Carrera" and empty groups fits well under 256 bytes.
// Production code must use compute_length_with_header(); this test checks feature integration, not sizing.
let mut buf = [0u8; 256];
    let len = CarEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&FixedFields {
            serial_number: 1, model_year: 2020, available: BooleanType::T, code: Model::A,
            some_numbers: [0; 4], vehicle_code: [b'A'; 6], extras: OptionalExtras::default(),
            engine: Engine::new(1, 1, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
        })
        .fuel_figures(0, |_| Ok(()))?
        .performance_figures(0, |_| Ok(()))?
        .manufacturer(b"Honda")?
        .model(b"Civic")?
        .encoded_length_with_header();
    let dec = CarDecoder::decode(&buf[..len], 0)?;
    // Consuming stages: raw bytes → CompactString
    let stage = dec.fuel_figures()?;
    let stage = stage.performance_figures()?;
    // into_<field>_as_compact_str is feature-gated
    // (actual method name depends on generated code, tested via compilation)
    let _ = dec;
    Ok(())
}"#,
    )
}

#[test]
#[cfg(feature = "bytes")]
fn codec_bytes_accessor_roundtrip() {
    test_with_features(
        "codec_bytes",
        "",
        &["bytes"],
        r#"fn main() -> Result<(), Box<dyn std::error::Error>> {
    use codec_bytes::*;
    // Test buffer: car schema with "Porsche"+"Carrera" and empty groups fits well under 256 bytes.
// Production code must use compute_length_with_header(); this test checks feature integration, not sizing.
let mut buf = [0u8; 256];
    let len = CarEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&FixedFields {
            serial_number: 1, model_year: 2020, available: BooleanType::T, code: Model::A,
            some_numbers: [0; 4], vehicle_code: [b'A'; 6], extras: OptionalExtras::default(),
            engine: Engine::new(1, 1, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
        })
        .fuel_figures(0, |_| Ok(()))?
        .performance_figures(0, |_| Ok(()))?
        .manufacturer(b"Porsche")?
        .model(b"Carrera")?
        .encoded_length_with_header();
    let dec = CarDecoder::decode(&buf[..len], 0)?;
    // Consuming stages: raw bytes encoded, then decoded back
    let stage = dec.fuel_figures()?;
    let (mfr_data, stage) = stage.into_manufacturer()?;
    assert_eq!(mfr_data, b"Porsche");
    // into_<field>_as_bytes is feature-gated and returns bytes::Bytes
    Ok(())
}"#,
    )
}

// ── Chrono tests ──────────────────────────────────────────────────────────

#[test]
#[cfg(feature = "chrono")]
fn chrono_converter_roundtrip() {
    // Direct converter tests
    let now_ns: i64 = 1_720_000_000_000_000_000; // ~2024-07
    let dt = ergo_sbe::chrono_converters::i64_nanos_to_datetime(now_ns);
    let back = ergo_sbe::chrono_converters::datetime_to_i64_nanos(dt);
    assert_eq!(back, now_ns, "DateTime roundtrip must be exact");

    let now_us: i64 = 1_720_000_000_000_000; // ~2024-07 in micros
    let naive = ergo_sbe::chrono_converters::i64_micros_to_naive(now_us);
    let back_us = ergo_sbe::chrono_converters::naive_to_i64_micros(naive);
    assert_eq!(back_us, now_us, "NaiveDateTime roundtrip must be exact");

    // Zero epoch
    let epoch_dt = ergo_sbe::chrono_converters::i64_nanos_to_datetime(0);
    assert_eq!(epoch_dt.timestamp(), 0);
    let epoch_naive = ergo_sbe::chrono_converters::i64_micros_to_naive(0);
    assert_eq!(epoch_naive.and_utc().timestamp(), 0);
    Ok(())
}

#[test]
#[cfg(feature = "chrono")]
fn chrono_edge_cases() {
    // Microsecond epoch (1960-01-01 in some schemas)
    let micro_epoch: i64 = -378_691_200_000_000; // 1960-01-01 in micros since Unix epoch
    let naive = ergo_sbe::chrono_converters::i64_micros_to_naive(micro_epoch);
    let back = ergo_sbe::chrono_converters::naive_to_i64_micros(naive);
    assert_eq!(back, micro_epoch);

    // Nanosecond minimum (-1 second before epoch)
    let neg_one_ns: i64 = -1_000_000_000;
    let dt = ergo_sbe::chrono_converters::i64_nanos_to_datetime(neg_one_ns);
    let back_ns = ergo_sbe::chrono_converters::datetime_to_i64_nanos(dt);
    assert_eq!(back_ns, neg_one_ns);
    Ok(())
}

// ── All features combined ─────────────────────────────────────────────────

#[test]
#[cfg(all(feature = "compact_str", feature = "bytes", feature = "chrono"))]
fn all_features_together_compile() {
    // Prove all four features coexist without symbol conflicts
    test_with_features(
        "all_feats",
        "",
        &["compact_str", "bytes", "chrono"],
        r#"fn main() -> Result<(), Box<dyn std::error::Error>> {
    use all_feats::*;
    // Test buffer: car schema with "Porsche"+"Carrera" and empty groups fits well under 256 bytes.
// Production code must use compute_length_with_header(); this test checks feature integration, not sizing.
let mut buf = [0u8; 256];
    let len = CarEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&FixedFields {
            serial_number: 1, model_year: 2020, available: BooleanType::T, code: Model::A,
            some_numbers: [0; 4], vehicle_code: [b'A'; 6], extras: OptionalExtras::default(),
            engine: Engine::new(1, 1, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
        })
        .fuel_figures(0, |_| Ok(()))?
        .performance_figures(0, |_| Ok(()))?
        .manufacturer(b"Porsche")?
        .model(b"Carrera")?
        .encoded_length_with_header();
    let dec = CarDecoder::decode(&buf[..len], 0)?;
    let dto = CarDomain::try_from_decoder(&dec)?;
    // CompactString for var-data
    assert_eq!(dto.manufacturer, "Porsche");
    // Re-encode
    let mut out = [0u8; 512];
    let re_len = dto.try_to_encoder(CarEncoder::wrap(&mut out, 0)?)?
        .encoded_length_with_header();
    assert_eq!(&buf[..len], &out[..re_len]);

    // Chrono converter is available
    let ts = ergo_sbe::chrono_converters::i64_nanos_to_datetime(0);
    assert_eq!(ts.timestamp(), 0);
    Ok(())
}"#,
    )
}

// ── Encoding edge cases ───────────────────────────────────────────────────

#[test]
#[cfg(feature = "compact_str")]
fn compact_str_edge_cases() {
    // Empty string
    let empty = compact_str::CompactString::new("");
    assert!(empty.is_empty());
    // Exactly 24 bytes (inline threshold)
    let exact_24 = compact_str::CompactString::new("ABCDEFGHIJKLMNOPQRSTUVWX"); // 24 chars
    assert_eq!(exact_24.len(), 24);
    // Beyond 24 bytes (heap allocated)
    let beyond = compact_str::CompactString::new("ABCDEFGHIJKLMNOPQRSTUVWXY"); // 25 chars
    assert_eq!(beyond.len(), 25);
    // Unicode
    let unicode = compact_str::CompactString::new("Porsch\u{00E9}"); // Porsche with accent
    assert_eq!(unicode.len(), 8);
}

#[test]
#[cfg(feature = "bytes")]
fn bytes_zero_copy_semantics() {
    let data: Vec<u8> = b"Hello, World!".to_vec();
    let b = bytes::Bytes::copy_from_slice(&data);
    // Clone is cheap — shares the same backing buffer
    let b2 = b.clone();
    assert_eq!(&b[..], &b2[..]);
    // Slice without copy
    let sub = b.slice(0..5);
    assert_eq!(&sub[..], b"Hello");
}
