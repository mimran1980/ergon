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
    ("s_dep", "u32", "Option<u32>"),
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
    /// Appended to the generated module: items a knob's output names (an error
    /// type) or that prove its output exists (a hook's item).
    module_suffix: &'static str,
    /// Fragments the knob must put in the generated source. For a knob whose
    /// output is an attribute, compiling proves nothing on its own — a missing
    /// `#[deprecated]` compiles perfectly well.
    expect_src: &'static [&'static str],
}

/// Every `GenerationConfig` combination that selects a distinct codegen path.
/// Add a row when you add a knob — a knob with no row here is a knob whose
/// generated output nothing compiles.
const VARIANTS: &[Variant] = &[
    Variant {
        name: "domain_objects_and_domain_types",
        domain_impls: true,
        build: |c| with_matrix_domain_types(c.with_domain_objects(DomainVarData::Bytes)),
        module_suffix: "",
        expect_src: &[],
    },
    Variant {
        name: "domain_objects_strings_and_domain_types",
        domain_impls: true,
        build: |c| with_matrix_domain_types(c.with_domain_objects(DomainVarData::Strings)),
        module_suffix: "",
        expect_src: &[],
    },
    Variant {
        name: "domain_objects_no_conversions",
        domain_impls: false,
        build: |c| c.with_domain_objects(DomainVarData::Bytes),
        module_suffix: "",
        expect_src: &[],
    },
    Variant {
        name: "domain_objects_and_conversions_only",
        domain_impls: false,
        build: |c| with_matrix_conversions(c.with_domain_objects(DomainVarData::Bytes)),
        module_suffix: "",
        expect_src: &[],
    },
    Variant {
        name: "flyweight_only_domain_types",
        domain_impls: true,
        build: with_matrix_domain_types,
        module_suffix: "",
        expect_src: &[],
    },
    Variant {
        name: "flyweight_only_conversions",
        domain_impls: false,
        build: with_matrix_conversions,
        module_suffix: "",
        expect_src: &[],
    },
    Variant {
        name: "lean_profile_domain_types",
        domain_impls: true,
        build: |c| with_matrix_domain_types(c.profile(GenerationProfile::Lean)),
        module_suffix: "",
        expect_src: &[],
    },
    Variant {
        name: "flyweight_encode_version_0",
        domain_impls: false,
        build: |c| c.with_encode_version(0),
        module_suffix: "",
        expect_src: &[],
    },
    Variant {
        name: "domain_objects_null_as_option",
        domain_impls: true,
        build: |c| {
            with_matrix_domain_types(c.with_domain_objects(DomainVarData::Bytes))
                .with_all_enums_as_option()
        },
        module_suffix: "",
        expect_src: &[],
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
        module_suffix: "",
        expect_src: &[],
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
        module_suffix: "",
        expect_src: &[],
    },
    Variant {
        name: "domain_objects_bool_domain_type",
        domain_impls: true,
        build: |c| {
            with_matrix_domain_types(c.with_domain_objects(DomainVarData::Bytes))
                .with_bool_domain_type(true)
        },
        module_suffix: "",
        expect_src: &[],
    },
    Variant {
        name: "domain_objects_null_as_option_selector",
        domain_impls: true,
        build: |c| {
            with_matrix_domain_types(c.with_domain_objects(DomainVarData::Bytes))
                .with_null_as_option(ConversionSelector::named_type("Model"))
        },
        module_suffix: "",
        expect_src: &[],
    },
    Variant {
        name: "domain_objects_error_from_impls",
        domain_impls: true,
        #[allow(deprecated)]
        build: |c| {
            with_matrix_domain_types(c.with_domain_objects(DomainVarData::Bytes))
                .with_error_from_impls("self::MatrixError")
        },
        module_suffix: r#"
            #[derive(Debug)]
            pub struct MatrixError(pub String);
            impl From<String> for MatrixError {
                fn from(s: String) -> Self { Self(s) }
            }
            const _: fn(sbe_rt::DecodeError) -> MatrixError = MatrixError::from;
        "#,
        expect_src: &[],
    },
    Variant {
        name: "flyweight_deprecated_attrs_keyword_token",
        domain_impls: false,
        build: |c| {
            c.with_deprecated_attrs(true)
                .with_keyword_append_token("_kw")
        },
        module_suffix: r#"
            fn _keyword_renamed(d: &FlatDecoder<'_>) -> u8 { d.type_kw() }
            fn _keyword_renamed_row(d: &NestedRowsEntryDecoder<'_>) -> u8 { d.type_kw() }
            fn _keyword_renamed_cell(d: &NestedRowsCellsEntryDecoder<'_>) -> u8 { d.type_kw() }
        "#,
        expect_src: &["#[deprecated", "pub fn type_kw("],
    },
    Variant {
        name: "domain_objects_hook",
        domain_impls: true,
        build: |c| {
            with_matrix_domain_types(c.with_domain_objects(DomainVarData::Bytes)).with_hook(
                |ctx: &ergo_sbe::ItemContext| match ctx {
                    ergo_sbe::ItemContext::Enum { name, .. } => {
                        let ident = quote::format_ident!("{}", name);
                        vec![quote::quote! {
                            impl #ident { pub const HOOKED: &'static str = stringify!(#ident); }
                        }]
                    }
                    _ => Vec::new(),
                },
            )
        },
        module_suffix: r#"
            const _: () = assert!(Model::HOOKED.len() == 5);
        "#,
        expect_src: &[],
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

/// A runtime owner with no groups or var-data. Its `sbe_rt` is still whole
/// (`EntryInfo` / `Ordered` included) so a tail-owning consumer can share it.
const FIXED_BLOCK_OWNER: &str = r#"<messageSchema package="owner" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Ping" id="1"><field name="x" id="1" type="uint32"/></message>
</messageSchema>"#;

/// `with_external_sbe_rt` and `with_shared_module` configure a *set* of
/// modules, so they cannot be a single-module `VARIANTS` row. Each compiles the
/// whole fixture as the dependent module against a fixed-block owner and runs
/// the same iterator surface as every row.
#[test]
fn module_set_knobs_compile() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_sbe::{Generator, Schema, parse, parse_file};
    let owner_schema = Schema::from_ir(parse(FIXED_BLOCK_OWNER)?);
    let matrix = Schema::from_ir(parse_file(&matrix_schema())?);
    // The shared runtime must stay *one* runtime: a helper written against the
    // owner's `EntryInfo` has to accept the dependent module's.
    let body = |module: &str, owner: &str| {
        format!(
            "fn owner_entry_info(info: {owner}::sbe_rt::EntryInfo) -> usize {{ info.count }}\n\
             use {module}::*;\n\
             assert_eq!(owner_entry_info(sbe_rt::EntryInfo {{ index: 0, count: 1, block_length: 4 }}), 1);\n{}",
            iterator_surface_body(true)
        )
    };

    let owner = Generator::new(GenerationConfig::new("cm_rt_owner"))
        .generate(&owner_schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();
    let consumer = Generator::new(
        GenerationConfig::new("cm_external_rt").with_external_sbe_rt("super::cm_rt_owner::sbe_rt"),
    )
    .generate(&matrix)?
    .modules()
    .next()
    .ok_or("one module")?
    .source
    .clone();
    common::compile_and_run_modules(
        "cm_external_rt",
        &[("cm_rt_owner", &owner), ("cm_external_rt", &consumer)],
        &body("cm_external_rt", "cm_rt_owner"),
    );

    let set =
        Generator::new(GenerationConfig::new("cm_shared").with_shared_module("cm_shared_owner"))
            .generate_multi(&[(&owner_schema, "cm_shared_owner"), (&matrix, "cm_shared")])?;
    let modules: Vec<_> = set
        .modules()
        .map(|m| (m.path.trim_end_matches(".rs").to_owned(), m.source.clone()))
        .collect();
    let modules: Vec<(&str, &str)> = modules
        .iter()
        .map(|(n, s)| (n.as_str(), s.as_str()))
        .collect();
    common::compile_and_run_modules("cm_shared", &modules, &body("cm_shared", "cm_shared_owner"));
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
        for fragment in v.expect_src {
            assert!(
                src.contains(fragment),
                "{}: generated source must contain `{fragment}`",
                v.name
            );
        }
        let src = format!("{src}\n{}", v.module_suffix);
        let (prelude, deps) = if v.domain_impls {
            (DOMAIN_IMPLS, "rust_decimal = \"1\"\n")
        } else {
            ("", "")
        };
        // Panics with the compiler diagnostics on failure; the module name in
        // the scratch crate path names the failing variant.
        compile_and_run_with_deps(
            &module,
            &src,
            &format!(
                "{prelude}\n{}",
                iterator_surface_body(v.name != "flyweight_encode_version_0")
            ),
            deps,
        );
    }
    Ok(())
}

// Exercise both iterator shapes at all three owner depths. Removing any
// location's len/iterator emission breaks compilation; wrong remaining counts,
// cursor advancement or byte lengths fail the running scratch program.
fn iterator_surface_body(encode_extra: bool) -> String {
    fn length_entry(depth: usize, extra: &str, future: &str) -> String {
        if depth == 3 {
            return "b.add()?.payload(4)?;".into();
        }
        format!(
            "b.add()?.items(|b| {{ b.uniform(count)?; Ok(()) }})?\
            .records(|b| {{ for _ in 0..count {{ {} }} Ok(()) }})?\
            {future}.note(3)?{extra};",
            length_entry(depth + 1, extra, future)
        )
    }
    fn encode_owner(depth: usize, extra: &str, future: &str) -> String {
        if depth == 3 {
            return "e.payload(b\"leaf\")".into();
        }
        format!(
            "e.items(count as u16, |g| {{\
            for value in 0..count {{ g.add(|mut e| {{ e.value(value as u32); Ok(()) }})?; }} Ok(())\
        }})?.records(count as u16, |g| {{\
            for _ in 0..count {{ g.add(|e| {{ {} }})?; }} Ok(())\
        }})?{future}.note(b\"abc\"){extra}",
            encode_owner(depth + 1, extra, future)
        )
    }
    fn decode_owner(depth: usize) -> String {
        if depth == 3 {
            return "assert_eq!(d.payload_len()?, 4); let (bytes, done) = d.into_payload()?; assert_eq!(bytes, b\"leaf\"); Ok(done)".into();
        }
        let prefix = format!("IteratorSurface{}", "Records".repeat(depth));
        format!(
            r#"
            assert_eq!(d.items_count()?, count);
            assert_eq!(d.records_count()?, count);
            assert_eq!(d.future_items_count()?, 0);
            assert_eq!(d.future_records_count()?, 0);
            assert_eq!(d.note_len()?, 3);
            assert_eq!(d.extra_len()?, 0);
            // `items` entries have no tails: fixed stride, so a real iterator.
            let mut items = d.into_items()?;
            assert_eq!(items.remaining_entries(), count);
            exact(&mut items, count);
            for value in 0..count {{
                let entry: {prefix}ItemsEntryDecoder<'_> = (&mut items).next().unwrap();
                assert_eq!(entry.value(), value as u32);
                assert_eq!(items.remaining_entries(), count - value - 1);
                exact(&mut items, count - value - 1);
            }}
            assert!((&mut items).next().is_none());
            assert_eq!(items.remaining_entries(), 0);
            exact(&mut items, 0);
            // `records` entries carry their own tails: visit closure, and the
            // completion it returns is the next entry's offset.
            let mut visited = 0usize;
            let stage = items.into_records(|d: {prefix}RecordsEntryDecoder<'_>| -> Result<_, sbe_rt::DecodeError> {{
                visited += 1;
                {nested}
            }})?;
            assert_eq!(visited, count);
            let mut items = stage.into_future_items()?;
            assert_eq!(items.remaining_entries(), 0);
            exact(&mut items, 0);
            let stage = items.into_future_records(|_d| -> Result<_, sbe_rt::DecodeError> {{
                unreachable!("futureRecords is absent at this version")
            }})?;
            let (note, d) = stage.into_note()?;
            assert_eq!(note, b"abc");
            assert_eq!(d.extra_len()?, 0);
            let (extra, done) = d.into_extra()?;
            assert_eq!(extra, b"");
            Ok(done)
        "#,
            nested = decode_owner(depth + 1)
        )
    }
    format!(
        r#"
        fn exact(iter: impl ExactSizeIterator, expected: usize) {{
            assert_eq!(iter.len(), expected);
            assert_eq!(iter.size_hint(), (expected, Some(expected)));
        }}
        for count in [0usize, 1, 2] {{
            let length = IteratorSurfaceEncodedLength::new().items(count as u16)?
                .records_ragged(count as u16, |b| {{
                    for _ in 0..count {{ {length_entry} }} Ok(())
                }})?{future_length}.note(3)?{length_extra}.encoded_length_with_header();
            let mut buf = vec![0; length];
            let e = IteratorSurfaceEncoder::try_wrap_and_apply_header(&mut buf, 0)?
                .fixed(&IteratorSurfaceFixedFields {{}});
            let written = ({encode})?.encoded_length_with_header();
            assert_eq!(written, length);
            let d = IteratorSurfaceDecoder::wrap(&buf, 0, 0, IteratorSurfaceEncoder::SCHEMA_VERSION);
            // Depth 0 is a statement block, not a closure body, so the shared
            // walk's tail expression is typed here. `?` on the result is
            // load-bearing: binding it to `_` let a decoding failure pass.
            let _stage: IteratorSurfaceDecoderComplete<'_> =
                (|| -> Result<_, sbe_rt::DecodeError> {{ {decode} }})()?;

            // A partially read iterator still reaches the next tail: into_* /
            // finish() skip unread entries.
            let d = IteratorSurfaceDecoder::wrap(&buf, 0, 0, IteratorSurfaceEncoder::SCHEMA_VERSION);
            let mut items = d.into_items()?;
            let _ = (&mut items).next();
            let stage = items.into_records(|d| -> Result<_, sbe_rt::DecodeError> {{
                // Read nothing from this entry: skip_* reaches its completion.
                d.skip_items()?.skip_records()?.skip_future_items()?
                    .skip_future_records()?.into_note().map(|(_n, s)| s)?
                    .into_extra().map(|(_e, done)| done)
            }})?;
            let mut items = stage.into_future_items()?;
            assert_eq!(items.remaining_entries(), 0);
            exact(&mut items, 0);
            let stage = items.into_future_records(|_d| -> Result<_, sbe_rt::DecodeError> {{
                unreachable!("futureRecords is absent at this version")
            }})?;
            let (note, _) = stage.into_note()?;
            assert_eq!(note, b"abc");

            // The original fixed-stride random-access iterator also promises an exact hint.
            let d = IteratorSurfaceDecoder::wrap(&buf, 0, 0, IteratorSurfaceEncoder::SCHEMA_VERSION);
            exact(d.items()?, count);
            if count > 0 {{
                let payload = buf.windows(4).position(|w| w == b"leaf").unwrap();
                buf[payload - 4..payload].fill(0xff);
                let d = IteratorSurfaceDecoder::wrap(&buf, 0, 0, IteratorSurfaceEncoder::SCHEMA_VERSION);
                assert_eq!(d.records_count()?, count);
                let items = d.into_items()?;
                // Dynamic entry extents are validated as the walk reaches
                // them, not at construction, so the visit fails.
                let walked = items.into_records(|d| -> Result<_, sbe_rt::DecodeError> {{
                    d.skip_items()?.skip_records()?.skip_future_items()?
                        .skip_future_records()?.into_note().map(|(_n, s)| s)?
                        .into_extra().map(|(_e, done)| done)
                }});
                assert!(walked.is_err());
            }}
        }}
    "#,
        length_extra = if encode_extra { ".extra(0)?" } else { "" },
        future_length = if encode_extra {
            ".future_items(0)?.future_records(0).finish_empty()?"
        } else {
            ""
        },
        length_entry = length_entry(
            1,
            if encode_extra { ".extra(0)?" } else { "" },
            if encode_extra {
                ".future_items(|_| Ok(()))?.future_records(|_| Ok(()))?"
            } else {
                ""
            }
        ),
        encode = encode_owner(
            0,
            if encode_extra { "?.extra(b\"\")" } else { "" },
            if encode_extra {
                ".future_items(0, |_| Ok(()))?.future_records(0, |_| Ok(()))?"
            } else {
                ""
            }
        ),
        decode = decode_owner(0)
    )
}

#[test]
fn decoder_iterator_surface_matrix() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate_domain_with(&matrix_schema(), "cm_iterators", |c| c);
    compile_and_run_with_deps("cm_iterators", &src, &iterator_surface_body(true), "");
    Ok(())
}

/// Generated code must not emit Markdown block doc comments.
///
/// `prettyplease` renders a single `#[doc]` carrying a blank line as a
/// `/** … */` block whose continuation lines take the item's indentation.
/// Markdown then reads a 4-space-indented paragraph after a blank line as an
/// **indented code block**, so a consumer that checks the generated module
/// into `src/` gets a rustdoc-collected doctest of prose that cannot compile.
/// Emitting one `#[doc]` per line (`runtime::doc_lines_tokens`) renders as
/// `///`, which cannot form an indented block.
///
/// The module still *compiles*, so every `compile_and_run*` case stays green
/// while `cargo test --doc` fails. This test therefore runs the generated
/// modules' doctests, proves that runner fails on a block doc comment, and
/// keeps the source scan to name the offending line.
///
/// The schema must contain a message with **no groups and no var-data** —
/// that is the branch carrying the two-paragraph doc. A schema whose messages
/// all have tails renders the single-paragraph form and never exercises it.
#[test]
fn generated_code_has_no_block_doc_comments() -> Result<(), Box<dyn std::error::Error>> {
    let plain = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/bool-semantic-schema.xml"
    ));
    let modules = [
        (
            "cm_docs_fixed",
            generate_domain_with(&plain, "cm_docs_fixed", |c| c).1,
        ),
        (
            "cm_docs_matrix",
            generate_domain_with(&matrix_schema(), "cm_docs_matrix", with_matrix_domain_types).1,
        ),
    ];
    let mut offenders = Vec::new();
    let mut fixed_block_messages = 0usize;
    for (name, src) in &modules {
        fixed_block_messages += src
            .matches("This message is fixed-block, so there is no memoized lane")
            .count();
        for (i, line) in src.lines().enumerate() {
            if line.trim_start().starts_with("/**") {
                offenders.push(format!("  {name}:{}: {}", i + 1, line.trim()));
            }
        }
    }
    assert!(
        fixed_block_messages > 0,
        "no fixed-block message reached the two-paragraph doc branch; this \
         check would pass vacuously"
    );
    assert!(
        offenders.is_empty(),
        "generated code contains a block doc comment, which rustdoc may harvest \
         as a doctest of prose:\n{}",
        offenders.join("\n")
    );

    let as_refs: Vec<(&str, &str)> = modules.iter().map(|(n, s)| (*n, s.as_str())).collect();
    let domain_impls = format!("const _: () = {{ use cm_docs_matrix::*; {DOMAIN_IMPLS} }};");
    common::run_doctests("cm_docs", &as_refs, &domain_impls, "rust_decimal = \"1\"\n")
        .map_err(|out| format!("generated modules' doctests fail:\n{out}"))?;
    let prose = "/**\nFirst paragraph.\n\n    indented prose that rustdoc reads as code\n*/\npub struct Prose;\n";
    assert!(
        common::run_doctests("cm_docs_negative", &[("prose", prose)], "", "").is_err(),
        "the doctest runner must fail on a block doc comment with indented prose, \
         or the check above proves nothing"
    );
    Ok(())
}
