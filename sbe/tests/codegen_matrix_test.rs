//! Codegen combination matrix.
//!
//! Domain-object coverage used to assert on **source strings**, which stay
//! green while the generated module fails to compile. Five defects reached
//! 0.1.22/0.1.23 that way, and this matrix found ten more of the same family
//! on its first run. Every case here therefore *compiles* the generated
//! module; the string assertions only name the cell that broke.
//!
//! Two axes, crossed:
//!
//! * **Shape** — field kind x `sinceVersion` x mapped/unmapped, repeated at
//!   message, group-entry, and nested-group-entry level. `converter_impls` has
//!   separate message-level and group-entry loops and the domain-DTO generator
//!   recurses separately for entry DTOs, so a cell proven at one location
//!   proves nothing about the others.
//! * **Config** — the `GenerationConfig` knobs that select different codegen
//!   paths (domain objects, domain types, conversion-only, profile, var-data
//!   representation, null-as-option, display/meta/dispatch).
//!
//! [`expected_cells`] recomputes the shape cross product independently of the
//! fixture. Adding a shape there fails the suite until the fixture and the
//! expectations catch up — the fixture cannot silently fall behind.
#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    unused
)]
mod common;
use common::{compile_and_run_with_deps, generate_domain_with};
use ergo_sbe::{ConversionSelector, DomainVarData, GenerationConfig, GenerationProfile};
use std::path::PathBuf;

fn matrix_schema() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/codegen-matrix-schema.xml"
    ))
}

/// Field shapes under test. Keep in lockstep with
/// `scripts/gen-codegen-matrix-fixture.py`; the fixture is generated from the
/// same list and [`fixture_covers_every_shape`] proves they agree.
///
/// `(prefix, dto type at sinceVersion=0, dto type at sinceVersion>0)`
const SHAPES: &[(&str, &str, &str)] = &[
    // Domain type applies: the DTO materialises the mapped type.
    (
        "s_dom",
        "rust_decimal::Decimal",
        "Option<rust_decimal::Decimal>",
    ),
    ("s_plain", "u32", "Option<u32>"),
    // Optional scalars keep the wire type even when a selector matches.
    ("s_opt_dom", "Option<i64>", "Option<i64>"),
    ("s_opt_plain", "Option<i32>", "Option<i32>"),
    // Fixed arrays keep the wire type even when a selector matches.
    ("a_dom", "[u8; 4]", "[u8; 4]"),
    ("a_plain", "[u8; 4]", "[u8; 4]"),
    (
        "c_dom",
        "rust_decimal::Decimal",
        "Option<rust_decimal::Decimal>",
    ),
    ("c_plain", "Pair", "Option<Pair>"),
    ("e_bool", "bool", "Option<bool>"),
    // An optional bool enum is `Option<bool>` at every version: its accessor
    // can return `None` for the schema null value, not only for absence.
    ("e_bool_opt", "bool", "Option<bool>"),
    ("e_norm", "Model", "Option<Model>"),
    ("set_opt", "Opts", "Option<Opts>"),
];

const VERSIONS: &[u16] = &[0, 1];

/// The shape cross product, computed independently of the fixture.
fn expected_cells() -> Vec<(String, &'static str)> {
    let mut out = Vec::new();
    for (prefix, at_v0, at_vn) in SHAPES {
        for v in VERSIONS {
            let ty = if *v == 0 { *at_v0 } else { *at_vn };
            out.push((format!("{prefix}_v{v}"), ty));
        }
    }
    out
}

/// `TryFromSbe`/`TryToSbe` impls for every wire type the matrix maps to a
/// domain type. Domain `try_*` accessors name these in their signatures, so the
/// generated module does not compile without them.
const DOMAIN_IMPLS: &str = r#"
        use rust_decimal::Decimal;

        impl TryFromSbe<i64> for Decimal {
            type Error = &'static str;
            fn try_from_sbe(w: i64) -> Result<Self, Self::Error> { Ok(Decimal::new(w, 3)) }
        }
        impl TryToSbe<i64> for Decimal {
            type Error = &'static str;
            fn try_to_sbe(&self) -> Result<i64, Self::Error> { Ok(self.mantissa() as i64) }
        }
        impl TryFromSbe<[u8; 4]> for u32 {
            type Error = &'static str;
            fn try_from_sbe(w: [u8; 4]) -> Result<Self, Self::Error> { Ok(u32::from_le_bytes(w)) }
        }
        impl TryToSbe<[u8; 4]> for u32 {
            type Error = &'static str;
            fn try_to_sbe(&self) -> Result<[u8; 4], Self::Error> { Ok(self.to_le_bytes()) }
        }
        impl TryFromSbe<Model> for u8 {
            type Error = &'static str;
            fn try_from_sbe(w: Model) -> Result<Self, Self::Error> { Ok(w as u8) }
        }
        impl TryToSbe<Model> for u8 {
            type Error = &'static str;
            fn try_to_sbe(&self) -> Result<Model, Self::Error> { Ok(Model::A) }
        }
        impl TryFromSbe<Opts> for u16 {
            type Error = &'static str;
            fn try_from_sbe(w: Opts) -> Result<Self, Self::Error> { Ok(u8::from(w) as u16) }
        }
        impl TryToSbe<Opts> for u16 {
            type Error = &'static str;
            fn try_to_sbe(&self) -> Result<Opts, Self::Error> { Ok(Opts::default()) }
        }
        impl TryFromSbe<Money> for Decimal {
            type Error = &'static str;
            fn try_from_sbe(w: Money) -> Result<Self, Self::Error> {
                Ok(Decimal::new(w.mantissa(), (-w.exponent()) as u32))
            }
        }
        impl TryToSbe<Money> for Decimal {
            type Error = &'static str;
            fn try_to_sbe(&self) -> Result<Money, Self::Error> {
                Ok(Money::new(self.mantissa() as i64, -(self.scale() as i8)))
            }
        }
"#;

/// Apply the matrix's domain-type selectors to a config.
fn with_matrix_domain_types(c: GenerationConfig) -> GenerationConfig {
    c.with_manual_domain_type(
        ConversionSelector::semantic_type("ScaledPrice"),
        "rust_decimal::Decimal",
    )
    .with_manual_domain_type(
        ConversionSelector::named_type("Money"),
        "rust_decimal::Decimal",
    )
    .with_manual_domain_type(ConversionSelector::semantic_type("TagText"), "u32")
}

/// Conversion selectors only — no domain type, so the generated surface is
/// `*_as` / `*_from` plus `*_wire` raw renames rather than `try_*`.
fn with_matrix_conversions(c: GenerationConfig) -> GenerationConfig {
    c.with_conversion(ConversionSelector::semantic_type("ScaledPrice"))
        .with_conversion(ConversionSelector::named_type("Money"))
        .with_conversion(ConversionSelector::semantic_type("TagText"))
}

/// One generation configuration under test.
struct Variant {
    /// Names the cell in a failure and the scratch crate on disk.
    name: &'static str,
    /// Whether the scratch crate needs the domain impls and `rust_decimal`.
    domain_impls: bool,
    build: fn(GenerationConfig) -> GenerationConfig,
}

/// Every `GenerationConfig` combination that selects a distinct codegen path.
/// Add a row when you add a knob — a knob with no row here is a knob whose
/// generated output nothing compiles.
const VARIANTS: &[Variant] = &[
    Variant {
        name: "domain_objects_and_domain_types",
        domain_impls: true,
        build: |c| with_matrix_domain_types(c.with_domain_objects(DomainVarData::Bytes)),
    },
    Variant {
        name: "domain_objects_strings_and_domain_types",
        domain_impls: true,
        build: |c| with_matrix_domain_types(c.with_domain_objects(DomainVarData::Strings)),
    },
    Variant {
        name: "domain_objects_no_conversions",
        domain_impls: false,
        build: |c| c.with_domain_objects(DomainVarData::Bytes),
    },
    Variant {
        name: "domain_objects_and_conversions_only",
        domain_impls: false,
        build: |c| with_matrix_conversions(c.with_domain_objects(DomainVarData::Bytes)),
    },
    Variant {
        name: "flyweight_only_domain_types",
        domain_impls: true,
        build: with_matrix_domain_types,
    },
    Variant {
        name: "flyweight_only_conversions",
        domain_impls: false,
        build: with_matrix_conversions,
    },
    Variant {
        name: "lean_profile_domain_types",
        domain_impls: true,
        build: |c| with_matrix_domain_types(c.profile(GenerationProfile::Lean)),
    },
    Variant {
        name: "flyweight_encode_version_0",
        domain_impls: false,
        build: |c| c.with_encode_version(0),
    },
    Variant {
        name: "domain_objects_null_as_option",
        domain_impls: true,
        build: |c| {
            with_matrix_domain_types(c.with_domain_objects(DomainVarData::Bytes))
                .with_all_enums_as_option()
        },
    },
    Variant {
        name: "domain_objects_display_meta_dispatch",
        domain_impls: true,
        build: |c| {
            with_matrix_domain_types(c.with_domain_objects(DomainVarData::Bytes))
                .with_display_debug(true)
                .with_meta_attributes(true)
                .with_dispatch(true)
        },
    },
    Variant {
        name: "domain_objects_null_as_option_with_enum_domain_types",
        domain_impls: true,
        build: |c| {
            with_matrix_domain_types(c.with_domain_objects(DomainVarData::Bytes))
                .with_all_enums_as_option()
                .with_manual_domain_type(ConversionSelector::named_type("Model"), "u8")
                .with_manual_domain_type(ConversionSelector::named_type("Opts"), "u16")
        },
    },
    Variant {
        name: "domain_objects_bool_domain_type",
        domain_impls: true,
        build: |c| {
            with_matrix_domain_types(c.with_domain_objects(DomainVarData::Bytes))
                .with_bool_domain_type(true)
        },
    },
];

/// The fixture must carry a field for every shape x version cell. If this fails
/// the fixture has fallen behind `SHAPES` — regenerate it with
/// `scripts/gen-codegen-matrix-fixture.py`.
#[test]
fn fixture_covers_every_shape() -> Result<(), Box<dyn std::error::Error>> {
    let xml = std::fs::read_to_string(matrix_schema())?;
    let mut missing = Vec::new();
    for (prefix, _, _) in SHAPES {
        for v in VERSIONS {
            // Fixture field names are camelCase of the snake_case cell name.
            let camel = camel(prefix);
            let field = format!("name=\"{camel}V{v}\"");
            // Every cell must appear at all three codegen locations.
            let n = xml.matches(&field).count();
            if n != 3 {
                missing.push(format!(
                    "  {camel}V{v}: found at {n} location(s), expected 3 \
                     (message, group entry, nested group entry)"
                ));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "codegen matrix fixture is missing {} cell(s):\n{}",
        missing.len(),
        missing.join("\n")
    );
    Ok(())
}

fn camel(snake: &str) -> String {
    let mut out = String::new();
    let mut up = false;
    for ch in snake.chars() {
        if ch == '_' {
            up = true;
        } else if up {
            out.extend(ch.to_uppercase());
            up = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Every DTO cell materialises the right type at message level, and the module
/// compiles. The compile is the load-bearing assertion.
#[test]
fn domain_dto_shape_matrix_types_and_compiles() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, src) = generate_domain_with(&matrix_schema(), "cm_shapes", |c| {
        with_matrix_domain_types(c.with_domain_objects(DomainVarData::Bytes))
    });

    let mut wrong = Vec::new();
    for (cell, ty) in expected_cells() {
        if !src.contains(&format!("pub {cell}: {ty},")) {
            wrong.push(format!("  {cell}: expected `pub {cell}: {ty},`"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} DTO cell(s) have the wrong type:\n{}\n--- generated ---\n{src}",
        wrong.len(),
        wrong.join("\n")
    );
    assert!(
        !src.contains("pub k_const:"),
        "a constant-presence field must not appear in the DTO"
    );

    compile_and_run_with_deps("cm_shapes", &src, DOMAIN_IMPLS, "rust_decimal = \"1\"\n");
    Ok(())
}

/// `ConversionSelector::FieldPath` selects exactly one field.
///
/// It is documented as the primary selector form and validation accepts it, but
/// codegen matched only `NamedType` and `SemanticType` — every `FieldPath`
/// selector was a silent no-op that produced a wire-typed DTO field with no
/// error. Group and nested-group fields extend the path with their group names.
#[test]
fn field_path_selector_selects_exactly_that_field() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, src) = generate_domain_with(&matrix_schema(), "cm_field_path", |c| {
        c.with_domain_objects(DomainVarData::Bytes)
            .with_manual_domain_type(
                ConversionSelector::field_path("Flat.sPlainV0"),
                "rust_decimal::Decimal",
            )
            .with_manual_domain_type(
                ConversionSelector::field_path("Nested.rows.sPlainV0"),
                "rust_decimal::Decimal",
            )
    });
    assert!(
        src.contains("pub s_plain_v0: rust_decimal::Decimal,"),
        "FieldPath must apply the domain type to the named field: {src}"
    );
    assert!(
        src.contains("pub s_plain_v1: u32,"),
        "FieldPath must not leak onto a sibling field: {src}"
    );
    compile_and_run_with_deps(
        "cm_field_path",
        &src,
        r#"
        use rust_decimal::Decimal;
        impl TryFromSbe<u32> for Decimal {
            type Error = &'static str;
            fn try_from_sbe(w: u32) -> Result<Self, Self::Error> { Ok(Decimal::new(w as i64, 0)) }
        }
        impl TryToSbe<u32> for Decimal {
            type Error = &'static str;
            fn try_to_sbe(&self) -> Result<u32, Self::Error> { Ok(self.mantissa() as u32) }
        }
        "#,
        "rust_decimal = \"1\"\n",
    );
    Ok(())
}

/// Documented selector precedence (`config.rs`): `FieldPath` beats
/// `SemanticType` beats `NamedType`. Resolution used to be "first selector in
/// registration order wins", so precedence depended on call order.
#[test]
fn selector_precedence_field_path_beats_semantic_beats_named()
-> Result<(), Box<dyn std::error::Error>> {
    // Register in *reverse* precedence order: if order decided the winner
    // rather than tier, NamedType would win for every Money field.
    let (_s, src) = generate_domain_with(&matrix_schema(), "cm_precedence", |c| {
        c.with_domain_objects(DomainVarData::Bytes)
            .with_manual_domain_type(ConversionSelector::named_type("Money"), "u64")
            .with_manual_domain_type(
                ConversionSelector::semantic_type("ScaledPrice"),
                "rust_decimal::Decimal",
            )
            .with_manual_domain_type(ConversionSelector::field_path("Flat.cDomV0"), "i128")
    });
    assert!(
        src.contains("pub c_dom_v0: i128,"),
        "FieldPath must outrank NamedType on the same field: {src}"
    );
    assert!(
        src.contains("pub c_dom_v1: Option<u64>,"),
        "NamedType still applies where no higher-precedence selector matches: {src}"
    );
    assert!(
        src.contains("pub s_dom_v0: rust_decimal::Decimal,"),
        "SemanticType still applies where no FieldPath matches: {src}"
    );
    Ok(())
}

/// A domain type configured for an **enum** or **set** field must reach the
/// DTO. `converter_impls` generated `try_*` accessors for these fields all
/// along, but the domain-DTO generator ignored them and emitted the raw
/// generated type — the configured mapping silently did nothing.
#[test]
fn enum_and_set_domain_types_reach_the_dto() -> Result<(), Box<dyn std::error::Error>> {
    let (_s, src) = generate_domain_with(&matrix_schema(), "cm_enum_set_dt", |c| {
        c.with_domain_objects(DomainVarData::Bytes)
            .with_manual_domain_type(ConversionSelector::named_type("Model"), "u8")
            .with_manual_domain_type(ConversionSelector::named_type("Opts"), "u16")
    });
    assert!(
        src.contains("pub e_norm_v0: u8,"),
        "enum domain type must reach the DTO: {src}"
    );
    assert!(
        src.contains("pub e_norm_v1: Option<u8>,"),
        "versioned enum domain type must be Option-wrapped exactly once: {src}"
    );
    assert!(
        src.contains("pub set_opt_v0: u16,"),
        "set domain type must reach the DTO: {src}"
    );
    assert!(
        src.contains("pub set_opt_v1: Option<u16>,"),
        "versioned set domain type must be Option-wrapped exactly once: {src}"
    );
    compile_and_run_with_deps(
        "cm_enum_set_dt",
        &src,
        r#"
        impl TryFromSbe<Model> for u8 {
            type Error = &'static str;
            fn try_from_sbe(w: Model) -> Result<Self, Self::Error> { Ok(w as u8) }
        }
        impl TryToSbe<Model> for u8 {
            type Error = &'static str;
            fn try_to_sbe(&self) -> Result<Model, Self::Error> { Ok(Model::A) }
        }
        impl TryFromSbe<Opts> for u16 {
            type Error = &'static str;
            fn try_from_sbe(w: Opts) -> Result<Self, Self::Error> { Ok(u8::from(w) as u16) }
        }
        impl TryToSbe<Opts> for u16 {
            type Error = &'static str;
            fn try_to_sbe(&self) -> Result<Opts, Self::Error> { Ok(Opts::default()) }
        }
        "#,
        "",
    );
    Ok(())
}

/// Every configuration variant produces a module that compiles, across all
/// three codegen locations. This is the guard that was missing: a generated
/// module that does not compile cannot pass, however good its source looks.
#[test]
fn every_config_variant_compiles() -> Result<(), Box<dyn std::error::Error>> {
    for v in VARIANTS {
        let module = format!("cm_{}", v.name);
        let (_s, src) = generate_domain_with(&matrix_schema(), &module, |c| (v.build)(c));
        let (prelude, deps) = if v.domain_impls {
            (DOMAIN_IMPLS, "rust_decimal = \"1\"\n")
        } else {
            ("", "")
        };
        // Panics with the compiler diagnostics on failure; the module name in
        // the scratch crate path names the failing variant.
        compile_and_run_with_deps(&module, &src, prelude, deps);
    }
    Ok(())
}
