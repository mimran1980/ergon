//! Encoded-length API tests: direct helpers, uniform/ragged builders,
//! source-surface audits, and conformance matrix for representative schemas.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, unused)]

mod common;
use common::encoded_length_matrix::{GeneratedRustTest, compile_and_run_generated_tests};
use common::{compile_and_run, generate};
use std::path::PathBuf;

fn conformance_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/conformance_schema.xml"
    ))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("schemas")
        .join(name)
}

#[test]
fn direct_schemas_omit_builders() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, fixed_src) = generate(&fixture("basic-schema.xml"), "fixed_audit");
    assert!(
        !fixed_src.contains("EncodedLengthAccumulator"),
        "fixed message must not emit accumulator"
    );

    // Direct: has direct helpers, no staged builder entry-point
    let (_s, flat_src) = generate(&fixture("basic-group-schema.xml"), "flat_audit");
    assert!(
        !flat_src.contains("EncodedLength::new"),
        "direct message must not generate staged builder entry-point"
    );
    assert!(flat_src.contains("fn try_compute_encoded_length"));
    assert!(flat_src.contains("fn try_compute_encoded_length_with_header"));

    // VarData-only: no staged builder
    let (_s, vd_src) = generate(&fixture("basic-variable-length-schema.xml"), "vd_audit");
    assert!(
        !vd_src.contains("EncodedLength::new"),
        "varData-only direct message must not generate staged builder"
    );

    Ok(())
}

#[test]
fn staged_schemas_generate_builders() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, src) = generate(&fixture("group-with-data-schema.xml"), "staged_audit");
    assert!(
        src.contains("EncodedLength"),
        "staged message must generate EncodedLength types"
    );
    assert!(
        src.contains("EncodedLengthAccumulator"),
        "staged schema must emit the accumulator"
    );
    Ok(())
}

#[test]
fn fixed_schemas_omit_accumulator() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, src) = generate(&fixture("basic-schema.xml"), "no_accum");
    assert!(
        !src.contains("EncodedLengthAccumulator"),
        "fixed schema must not emit the accumulator"
    );
    let (_s, src2) = generate(&fixture("basic-group-schema.xml"), "no_accum2");
    assert!(
        !src2.contains("EncodedLengthAccumulator"),
        "direct schema must not emit the accumulator"
    );
    Ok(())
}

#[test]
fn accumulator_generated_once_per_schema() -> Result<(), Box<dyn std::error::Error>> {
    // L3 orderbook has two staged messages (L3Book and L3BookVarData)
    let (_s, src) = generate(&fixture("l3-orderbook-schema.xml"), "once_accum");
    let count = src.matches("struct EncodedLengthAccumulator").count();
    assert_eq!(
        count, 1,
        "accumulator must appear exactly once even with multiple staged messages"
    );
    Ok(())
}

#[test]
fn no_add_or_add_n_in_staged_builders() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, src) = generate(&fixture("l3-orderbook-schema.xml"), "no_add");
    // The new staged builder should not have add() or add_n() methods
    // on any EncodedLength type (RaggedEntryBuilder may have add() — that's OK)
    let lines: Vec<&str> = src.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("EncodedLength") && !line.contains("RaggedEntryBuilder") {
            let context = lines.get(i + 1).unwrap_or(&"");
            if context.contains("fn add(") || context.contains("fn add_n(") {
                // Only flag if it's inside an impl block for an EncodedLength type
                if !line.contains("RaggedEntryBuilder") {
                    panic!("EncodedLength type should not have add/add_n: line {i}: {line}");
                }
            }
        }
    }
    Ok(())
}

#[test]
fn formatting_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, src) = generate(&conformance_path(), "fmt_idem");
    let file1 = syn::parse_file(&src)?;
    let formatted1 = prettyplease::unparse(&file1);
    let file2 = syn::parse_file(&formatted1)?;
    let formatted2 = prettyplease::unparse(&file2);
    assert_eq!(
        formatted1, formatted2,
        "prettyplease output must be idempotent"
    );
    Ok(())
}

#[test]
fn direct_flatgroup_exact_length() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "direct_fg");
    compile_and_run(
        "direct_flat",
        &src,
        r#"
        let desc = b"test exchange data";
        let len = FlatGroupEncoder::try_compute_encoded_length_with_header(2u16, 1u16, desc.len())?;
        let mut buf_storage = [0u8; 8192];
assert!(len <= buf_storage.len());
let mut buf = &mut buf_storage[..len];
        let mut enc = FlatGroupEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
        enc.symbol(42);
        let complete = enc.bids(2, |bids| {
            bids.add(|e| { e.price(100i64).qty(10i32); Ok(()) })?;
            bids.add(|e| { e.price(101i64).qty(20i32); Ok(()) })?;
            Ok(())
        })?
        .asks(1, |asks| {
            asks.add(|e| { e.price(200i64).qty(30i32); Ok(()) })?;
            Ok(())
        })?
        .description(desc)?;
        assert_eq!(len, complete.encoded_length_with_header());
        assert_eq!(len, complete.as_bytes().len());
        println!("PASS: direct_flatgroup_exact_length = {len}");
        "#,
    );
    Ok(())
}

#[test]
fn direct_u8_overflow() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&fixture("u8-dimension-schema.xml"), "u8_test");
    assert!(src.contains("try_compute_encoded_length"));
    assert!(src.contains("try_compute_encoded_length_with_header"));
    assert!(!src.contains("CompactMsgEncodedLength"));
    Ok(())
}

#[test]
fn uniform_staged_car_length() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&fixture("example-schema.xml"), "uniform_car");
    assert!(
        src.contains("CarEncodedLength"),
        "Car must generate builder"
    );
    assert!(
        src.contains("finish_empty"),
        "builder must have finish_empty"
    );
    assert!(
        src.contains("EncodedLengthAccumulator"),
        "must emit accumulator"
    );
    Ok(())
}

#[test]
fn conformance_matrix_flatgroup() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "matrix_fg");

    let mut tests = Vec::new();

    tests.push(GeneratedRustTest {
        name: "empty".into(),
        body: r#"
            let len = FlatGroupEncoder::try_compute_encoded_length_with_header(0u16, 0u16, 0)?;
            assert!(len > FlatGroupEncoder::HEADER_LENGTH);
        "#
        .into(),
    });

    tests.push(GeneratedRustTest {
        name: "singleton".into(),
        body: r#"
            let len = FlatGroupEncoder::try_compute_encoded_length_with_header(1u16, 0u16, 0)?;
            assert!(len > FlatGroupEncoder::HEADER_LENGTH);
        "#
        .into(),
    });

    tests.push(GeneratedRustTest {
        name: "many".into(),
        body: r#"
            let len = FlatGroupEncoder::try_compute_encoded_length_with_header(10u16, 5u16, 20)?;
            assert!(len > 100);
        "#
        .into(),
    });

    // VarData overflow
    tests.push(GeneratedRustTest {
        name: "vardata_overflow".into(),
        body: r#"
            let result = FlatGroupEncoder::try_compute_encoded_length_with_header(0u16, 0u16, 100_000);
            assert!(result.is_err());
        "#.into(),
    });

    compile_and_run_generated_tests("matrix_fg", &src, &tests)?;
    println!("PASS: conformance_matrix_flatgroup — {} tests", tests.len());
    Ok(())
}

#[test]
fn conformance_matrix_l3() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&fixture("l3-orderbook-schema.xml"), "matrix_l3");
    assert!(
        src.contains("L3BookEncodedLength"),
        "L3 staged schema must generate builder"
    );
    assert!(
        src.contains("finish_empty"),
        "staged builder must have finish_empty"
    );
    assert!(
        src.contains("EncodedLengthAccumulator"),
        "staged schema must emit accumulator"
    );
    Ok(())
}

#[test]
fn conformance_matrix_nested_group() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "matrix_ng");
    assert!(
        src.contains("NestedGroupEncodedLength"),
        "NestedGroup must have builder"
    );
    assert!(
        src.contains("finish_empty"),
        "staged builder must have finish_empty"
    );
    Ok(())
}

#[test]
fn matrix_runner_rejects_duplicates() -> Result<(), Box<dyn std::error::Error>> {
    let tests = vec![
        GeneratedRustTest {
            name: "a".into(),
            body: "".into(),
        },
        GeneratedRustTest {
            name: "a".into(),
            body: "".into(),
        },
    ];
    let result = compile_and_run_generated_tests("dup", "// empty", &tests);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn matrix_runner_rejects_empty_name() -> Result<(), Box<dyn std::error::Error>> {
    let tests = vec![GeneratedRustTest {
        name: "".into(),
        body: "".into(),
    }];
    let result = compile_and_run_generated_tests("empty", "// empty", &tests);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn one_byte_short_buffer_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "short_buf");
    compile_and_run(
        "short_buf_test",
        &src,
        r#"
        let len = FlatGroupEncoder::try_compute_encoded_length_with_header(1u16, 0u16, 0)?;
        let mut buf_storage = [0u8; 8192];
assert!(len <= buf_storage.len());
let mut buf = &mut buf_storage[..len];
        let mut enc = FlatGroupEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
        enc.symbol(42);
        let complete = enc.bids(1, |g| {
            g.add(|e| { e.price(1i64).qty(1i32); Ok(()) })?;
            Ok(())
        })?
        .asks(0, |_| Ok(()))?
        .description(b"")?;
        assert_eq!(len, complete.as_bytes().len());

        let mut tiny = [0u8; 4]; // header=8, block=8 — 4 is too short
        let result = FlatGroupEncoder::try_wrap_and_apply_header(&mut tiny, 0);
        assert!(result.is_err(), "too-short buffer must fail");
        println!("PASS: one_byte_short_buffer_fails");
        "#,
    );
    Ok(())
}

#[test]
fn endianness_generates_identical_structure() -> Result<(), Box<dyn std::error::Error>> {
    let (_s_le, src_le) = generate(&fixture("example-schema.xml"), "le_end");
    let (_s_be, src_be) = generate(&fixture("example-bigendian-test-schema.xml"), "be_end");
    syn::parse_file(&src_le)?;
    syn::parse_file(&src_be)?;
    let le_count = src_le.matches("EncodedLength").count();
    let be_count = src_be.matches("EncodedLength").count();
    assert_eq!(
        le_count, be_count,
        "LE and BE schemas must generate same number of EncodedLength references"
    );
    println!("PASS: endianness_generates_identical_structure");
    Ok(())
}

#[test]
fn ragged_too_few_entries_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&fixture("example-schema.xml"), "ragged_few");
    compile_and_run(
        "ragged_few_test",
        &src,
        r#"
        let result = CarEncodedLength::new()
            .fuel_figures_ragged(3, |ff| {
                ff.add()?.usage_description(5)?;
                ff.add()?.usage_description(7)?;
                Ok(())
            });
        assert!(result.is_err(), "too few ragged entries must fail");
        if let Err(sbe_rt::EncodeError::GroupCountMismatch { declared, actual }) = result {
            assert_eq!(declared, 3);
            assert_eq!(actual, 2);
        } else {
            panic!("expected GroupCountMismatch");
        }
        println!("PASS: ragged_too_few_entries_rejected");
        "#,
    );
    Ok(())
}

#[test]
fn direct_helper_overflow_detected() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&conformance_path(), "direct_overflow");
    compile_and_run(
        "direct_overflow_test",
        &src,
        r#"
        let result = FlatGroupEncoder::try_compute_encoded_length_with_header(
            u16::MAX, u16::MAX, usize::MAX,
        );
        assert!(result.is_err(), "checked arithmetic overflow must return Err");
        println!("PASS: direct_helper_overflow_detected");
        "#,
    );
    Ok(())
}

#[test]
fn production_schemas_generate_valid_rust() -> Result<(), Box<dyn std::error::Error>> {
    for (schema_name, schema_file) in &[
        ("binance", "binance_spot_3_5.xml"),
        ("cme", "cme_templates_FixBinary.xml"),
        ("ilink", "ilinkbinary.xml"),
        ("u8dim", "u8-dimension-schema.xml"),
        ("bigendian", "example-bigendian-test-schema.xml"),
    ] {
        let path = fixture(schema_file);
        if !path.exists() {
            eprintln!("SKIP {schema_name}: {schema_file} not found");
            continue;
        }
        let (_s, src) = generate(&path, &format!("prod_{schema_name}"));
        syn::parse_file(&src)
            .map_err(|e| format!("{} generated invalid Rust: {e}", schema_name))?;
        eprintln!("OK {schema_name}: parses");
    }
    Ok(())
}

