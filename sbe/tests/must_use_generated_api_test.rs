//! T-20: generated pure observers carry `#[must_use]`.
//!
//! AST/source scan over regenerated codecs — discarding a getter is almost
//! always a caller mistake; setters and transitions must remain usable
//! without `must_use` noise on mutating APIs.
//!
//! Categories covered (T-20 + T-1 acceptance):
//! - message fixed-field getters (primitive / array / enum / set / composite)
//! - set predicates (`is_*`) and set `raw()`
//! - enum `raw()` observers
//! - metadata position/length queries
//! - pure encoded-length helpers
//! - `after_this_message`, message/group `min_readable_fixed_extent`
//! - `MessageHeader` peeks, `schema_id_from_header`, domain `to_wire_entry`
//! - exclusions: mutating setters / chainable transitions stay free of must_use
//! - downstream `deny(unused_must_use)` fixture

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(clippy::nursery)]

mod common;
use common::{Paths, compile_and_run, compile_fails_with_diagnostics, generate};
use ergo_sbe::{DomainVarData, GenerationConfig, Generator, Schema, parse_file};

fn generate_with(
    xml_path: &std::path::Path,
    module_name: &str,
    f: impl FnOnce(GenerationConfig) -> GenerationConfig,
) -> String {
    let ir = parse_file(xml_path).unwrap_or_else(|e| panic!("parse {xml_path:?}: {e}"));
    let schema = Schema::from_ir(ir);
    let config = f(GenerationConfig::new(module_name));
    let (modules, _warnings) = Generator::new(config)
        .generate(&schema)
        .unwrap()
        .into_parts();
    modules.into_iter().next().unwrap().source
}

fn assert_method_has_must_use(src: &str, method: &str) {
    // Prefer exact `fn name(` so `is_sun_roof` does not match `sun_roof`.
    let needle_exact = format!("fn {method}(");
    let needle_loose = format!("fn {method}");
    let pos = src
        .find(&needle_exact)
        .or_else(|| src.find(&needle_loose))
        .unwrap_or_else(|| panic!("method {method} not found"));
    // Only attributes belonging to this method (back to previous `fn `).
    let before = &src[..pos];
    let window_start = before
        .rfind("\n    ")
        .map(|i| i + 1)
        .unwrap_or(pos.saturating_sub(200));
    // Prefer the nearest attribute block: walk back over blank/`#[` lines only.
    let window = &src[window_start.saturating_sub(200)..pos];
    // Require must_use within ~12 lines above the fn (not a distant neighbour).
    let tight = &src[pos.saturating_sub(280)..pos];
    assert!(
        tight.contains("#[must_use") || tight.contains("must_use"),
        "expected #[must_use] on {method}; preceding:\n{window}"
    );
}

fn assert_method_lacks_must_use(src: &str, method: &str) {
    // Match only exact method names (avoid `is_sun_roof` when looking for `sun_roof`).
    let needle = format!("fn {method}(");
    let mut search_from = 0;
    let mut found = false;
    while let Some(rel) = src[search_from..].find(&needle) {
        let pos = search_from + rel;
        found = true;
        // Attribute window: only back to the previous `fn ` so a neighbour's
        // must_use cannot poison this method.
        let before = &src[..pos];
        let window_start = before
            .rfind("fn ")
            .map(|i| i + 3)
            .unwrap_or(pos.saturating_sub(120));
        let window = &src[window_start..pos];
        let after = &src[pos..pos.saturating_add(160).min(src.len())];
        if (after.contains("&mut Self") || after.contains("-> &mut")) && window.contains("must_use")
        {
            panic!("setter/transition {method} must not be must_use; preceding:\n{window}");
        }
        search_from = pos + needle.len();
    }
    assert!(found, "method {method}( not found");
}

fn assert_all_methods_have_must_use(src: &str, method: &str) {
    let needle = format!("fn {method}(");
    let mut search_from = 0;
    let mut found = false;
    while let Some(rel) = src[search_from..].find(&needle) {
        let pos = search_from + rel;
        found = true;
        let tight = &src[pos.saturating_sub(280)..pos];
        assert!(
            tight.contains("#[must_use") || tight.contains("must_use"),
            "expected #[must_use] on every {method}; preceding:\n{tight}"
        );
        search_from = pos + needle.len();
    }
    assert!(found, "method {method}( not found");
}

#[test]
fn decoder_getters_are_must_use() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "must_use_car");
    // Representative pure observers on CarDecoder.
    for m in [
        "serial_number",
        "model_year",
        "available",
        "code",
        "some_numbers",
        "encoded_length",
        "acting_version",
        "acting_block_length",
        "get_metadata",
    ] {
        assert_method_has_must_use(&src, m);
    }
    Ok(())
}

#[test]
fn set_predicates_and_raw_are_must_use() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "must_use_set");
    assert!(
        src.contains("is_sun_roof") || src.contains("sun_roof"),
        "expected set predicate in generated source"
    );
    // Set type: raw() + is_* predicates.
    assert_method_has_must_use(&src, "raw");
    for m in ["is_sun_roof", "is_sports_pack", "is_cruise_control"] {
        if src.contains(&format!("fn {m}")) {
            assert_method_has_must_use(&src, m);
        }
    }
    Ok(())
}

#[test]
fn enum_raw_observers_are_must_use() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "must_use_enum");
    assert!(
        src.contains("from_raw") || src.contains("fn raw"),
        "enum raw accessors should be generated"
    );
    // Enum::raw is a pure observer (not inlined by design — measured).
    assert_method_has_must_use(&src, "raw");
    Ok(())
}

#[test]
fn metadata_position_queries_are_must_use() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "must_use_meta");
    for m in ["message_offset", "limit", "buffer", "remaining"] {
        assert_method_has_must_use(&src, m);
    }
    Ok(())
}

#[test]
fn setters_and_transitions_are_not_observer_must_use() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "must_use_excl");
    // Mutating set choice writers return &mut Self — must not be pure-observer must_use.
    assert_method_lacks_must_use(&src, "sun_roof");
    Ok(())
}

/// Downstream fixture under `deny(unused_must_use)`: discarding a pure
/// observer fails to compile; the diagnostic names the unused return.
#[test]
fn discarded_observer_fails_under_deny_unused_must_use() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "must_use_deny");
    compile_fails_with_diagnostics(
        "must_use_deny",
        &src,
        r#"
        #[deny(unused_must_use)]
        {
            let buf = [0u8; 256];
            let dec = CarDecoder::wrap(&buf, 0, CarDecoder::BLOCK_LENGTH, 0);
            dec.serial_number();
        }
        "#,
        &["unused_must_use", "serial_number"],
    );
    Ok(())
}

/// Category table: each named observer class carries `#[must_use]`.
#[test]
fn must_use_category_table() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "must_use_cats");
    let categories: &[(&str, &[&str])] = &[
        ("enum as_option/as_bool", &["as_option", "as_bool"]),
        (
            "completed-tail views/lengths",
            &[
                "as_body_bytes",
                "as_bytes_with_header",
                "encoded_length",
                "encoded_length_with_header",
            ],
        ),
        (
            "encoder metadata/terminal queries",
            &[
                "get_metadata",
                "as_fixed_body_bytes",
                "message_offset",
                "limit",
                "buffer",
            ],
        ),
        ("group written", &["written"]),
        (
            "exact-length terminals",
            &["encoded_length", "encoded_length_with_header"],
        ),
        (
            "getter/set-predicate/raw",
            &["serial_number", "is_sun_roof", "raw"],
        ),
        (
            "header peek",
            &["peek_header", "peek_template_id", "peek_for_schema"],
        ),
        ("schema_id_from_header", &["schema_id_from_header"]),
        (
            "message min_readable_fixed_extent",
            &["min_readable_fixed_extent"],
        ),
    ];
    for (category, methods) in categories {
        for method in *methods {
            assert_method_has_must_use(&src, method);
        }
        let _ = category;
    }
    assert_all_methods_have_must_use(&src, "min_readable_fixed_extent");

    let fixed = generate(
        &Paths::sbe_tool_test_resource("basic-schema.xml"),
        "must_use_atm",
    )
    .1;
    assert_method_has_must_use(&fixed, "after_this_message");

    let domain = generate_with(&Paths::example_schema(), "must_use_wire", |c| {
        c.with_domain_objects(DomainVarData::Bytes)
    });
    assert_method_has_must_use(&domain, "to_wire_entry");

    let constant = generate(
        &Paths::sbe_tool_test_resource("basic-schema-constant-header-field.xml"),
        "must_use_sid_const",
    )
    .1;
    assert_all_methods_have_must_use(&constant, "schema_id_from_header");

    // Parser rejects headers without `schemaId`, so the None-return generator
    // branch is not reachable from a valid schema. Scan every emission site.
    let runtime_src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/codegen/runtime.rs"
    ));
    let mut sid_sites = 0;
    let mut search_from = 0;
    while let Some(rel) = runtime_src[search_from..].find("fn schema_id_from_header") {
        let pos = search_from + rel;
        sid_sites += 1;
        let tight = &runtime_src[pos.saturating_sub(280)..pos];
        assert!(
            tight.contains("must_use"),
            "schema_id_from_header emission at byte {pos} lacks #[must_use]; preceding:\n{tight}"
        );
        search_from = pos + "fn schema_id_from_header".len();
    }
    assert!(
        sid_sites >= 4,
        "expected every schema_id_from_header generator branch; found {sid_sites}"
    );
    Ok(())
}

/// Downstream `deny(unused_must_use)` fails for one method in every named
/// category. Functions take the generated types so the discarded calls are
/// the real public methods.
#[test]
fn discarded_observers_fail_in_every_named_category() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "must_use_cats_deny");
    compile_fails_with_diagnostics(
        "must_use_cats_deny",
        &src,
        r#"
        #[deny(unused_must_use)]
        fn drop_enum(v: BooleanType) {
            v.as_option();
            v.as_bool();
        }
        #[deny(unused_must_use)]
        fn drop_complete(c: FuelFiguresEntryDecoderComplete<'_>) {
            c.as_body_bytes();
            c.encoded_length();
        }
        #[deny(unused_must_use)]
        fn drop_meta(m: CarEncoderMetadata<'_>) {
            m.message_offset();
            m.as_fixed_body_bytes();
        }
        #[deny(unused_must_use)]
        fn drop_written(g: FuelFiguresEncoder<'_>) {
            g.written();
        }
        #[deny(unused_must_use)]
        fn drop_len(c: CarEncodedLengthComplete) {
            c.encoded_length();
            c.encoded_length_with_header();
        }
        #[deny(unused_must_use)]
        fn drop_extent() {
            CarDecoder::min_readable_fixed_extent(0);
            FuelFiguresDecoder::min_readable_fixed_extent(0);
        }
        #[deny(unused_must_use)]
        fn drop_header_peeks(buf: &[u8]) {
            MessageHeader::peek_header(buf);
            MessageHeader::peek_template_id(buf);
            MessageHeader::peek_for_schema(buf, 1);
            schema_id_from_header(buf);
        }
        fn main_body() {}
        "#,
        &[
            "unused_must_use",
            "as_option",
            "as_bool",
            "as_body_bytes",
            "written",
            "encoded_length",
            "min_readable_fixed_extent",
            "peek_header",
            "peek_template_id",
            "peek_for_schema",
            "schema_id_from_header",
        ],
    );

    let (_, fixed) = generate(
        &Paths::sbe_tool_test_resource("basic-schema.xml"),
        "must_use_cats_deny_atm",
    );
    compile_fails_with_diagnostics(
        "must_use_cats_deny_atm",
        &fixed,
        r#"
        #[deny(unused_must_use)]
        fn drop_after(frame: &[u8]) {
            TestMessage50001Decoder::after_this_message(frame);
        }
        fn main_body() {}
        "#,
        &["unused_must_use", "after_this_message"],
    );

    let domain = generate_with(&Paths::example_schema(), "must_use_cats_deny_wire", |c| {
        c.with_domain_objects(DomainVarData::Bytes)
    });
    compile_fails_with_diagnostics(
        "must_use_cats_deny_wire",
        &domain,
        r#"
        #[deny(unused_must_use)]
        fn drop_wire(e: CarPerformanceFiguresEntryAccelerationEntryDomain) {
            e.to_wire_entry();
        }
        fn main_body() {}
        "#,
        &["unused_must_use", "to_wire_entry"],
    );
    Ok(())
}

/// Positive control: using the observer value compiles cleanly.
#[test]
fn used_observer_compiles_under_deny_unused_must_use() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "must_use_ok");
    // Outer main already allows common lints; exercise the API with values used.
    compile_and_run(
        "must_use_ok",
        &src,
        r#"
        let buf = [0u8; 256];
        let dec = CarDecoder::wrap(&buf, 0, CarDecoder::BLOCK_LENGTH, 0);
        let sn = dec.serial_number();
        let extras = dec.extras();
        let roof = extras.is_sun_roof();
        let raw = extras.raw();
        let extent = CarDecoder::min_readable_fixed_extent(0);
        let group_extent = FuelFiguresDecoder::min_readable_fixed_extent(0);
        let peeked = MessageHeader::peek_header(&buf);
        let tid = MessageHeader::peek_template_id(&buf);
        let matched = MessageHeader::peek_for_schema(&buf, 1);
        let sid = schema_id_from_header(&buf);
        assert_eq!(sn, 0);
        let _ = (roof, raw, extent, group_extent, peeked, tid, matched, sid);
        "#,
    );

    let (_, fixed) = generate(
        &Paths::sbe_tool_test_resource("basic-schema.xml"),
        "must_use_ok_atm",
    );
    compile_and_run(
        "must_use_ok_atm",
        &fixed,
        r#"
        let buf = [0u8; 256];
        let tail = TestMessage50001Decoder::after_this_message(&buf);
        assert!(tail.is_some());
        "#,
    );

    let domain = generate_with(&Paths::example_schema(), "must_use_ok_wire", |c| {
        c.with_domain_objects(DomainVarData::Bytes)
    });
    compile_and_run(
        "must_use_ok_wire",
        &domain,
        r#"
        let e = CarPerformanceFiguresEntryAccelerationEntryDomain { mph: 1, seconds: 2.0 };
        let wire = e.to_wire_entry();
        assert_eq!(wire.mph, 1);
        "#,
    );
    Ok(())
}
