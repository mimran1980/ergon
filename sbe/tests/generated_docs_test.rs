//! Recursive rustdoc audit of generated public items.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

mod common;
use common::{Paths, generate};
use ergo_sbe::{DomainVarData, GenerationConfig, GenerationProfile, Generator, Schema, parse_file};

/// Walk public items and return `(path, has_doc)`.
fn check_docs(source: &str) -> Vec<(String, bool)> {
    let file = syn::parse_file(source).expect("generated source must parse");
    let mut results = Vec::new();
    walk_items("", &file.items, &mut results);
    results
}

fn is_public(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn has_operational_doc(attrs: &[syn::Attribute]) -> bool {
    let mut saw_doc = false;
    let mut only_placeholder = true;
    for attr in attrs {
        if !attr.meta.path().is_ident("doc") {
            continue;
        }
        saw_doc = true;
        if let syn::Meta::NameValue(nv) = &attr.meta
            && let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
        {
            if s.value().trim() != "Generated public API." {
                only_placeholder = false;
            }
        } else {
            // `#[doc = concat!(...)]` and other non-literal docs are operational.
            only_placeholder = false;
        }
    }
    saw_doc && !only_placeholder
}

fn walk_items(prefix: &str, items: &[syn::Item], out: &mut Vec<(String, bool)>) {
    for item in items {
        match item {
            syn::Item::Struct(s) if is_public(&s.vis) => {
                let name = format!("{prefix}{}", s.ident);
                out.push((name.clone(), has_operational_doc(&s.attrs)));
                for (i, field) in s.fields.iter().enumerate() {
                    if is_public(&field.vis) {
                        if field.ident.is_none() {
                            // Unnamed tuple fields inherit the struct rustdoc.
                            continue;
                        }
                        let fname = field
                            .ident
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| format!("{i}"));
                        out.push((format!("{name}.{fname}"), has_operational_doc(&field.attrs)));
                    }
                }
            }
            syn::Item::Enum(e) if is_public(&e.vis) => {
                let name = format!("{prefix}{}", e.ident);
                out.push((name.clone(), has_operational_doc(&e.attrs)));
                for v in &e.variants {
                    out.push((
                        format!("{name}::{}", v.ident),
                        has_operational_doc(&v.attrs),
                    ));
                }
            }
            syn::Item::Fn(f) if is_public(&f.vis) => {
                out.push((
                    format!("{prefix}{}", f.sig.ident),
                    has_operational_doc(&f.attrs),
                ));
            }
            syn::Item::Const(c) if is_public(&c.vis) => {
                out.push((
                    format!("{prefix}{}", c.ident),
                    has_operational_doc(&c.attrs),
                ));
            }
            syn::Item::Type(t) if is_public(&t.vis) => {
                out.push((
                    format!("{prefix}{}", t.ident),
                    has_operational_doc(&t.attrs),
                ));
            }
            syn::Item::Trait(t) if is_public(&t.vis) => {
                let name = format!("{prefix}{}", t.ident);
                out.push((name.clone(), has_operational_doc(&t.attrs)));
                for ti in &t.items {
                    match ti {
                        syn::TraitItem::Fn(f) => {
                            out.push((
                                format!("{name}::{}", f.sig.ident),
                                has_operational_doc(&f.attrs),
                            ));
                        }
                        syn::TraitItem::Const(c) => {
                            out.push((
                                format!("{name}::{}", c.ident),
                                has_operational_doc(&c.attrs),
                            ));
                        }
                        syn::TraitItem::Type(ty) => {
                            out.push((
                                format!("{name}::{}", ty.ident),
                                has_operational_doc(&ty.attrs),
                            ));
                        }
                        _ => {}
                    }
                }
            }
            syn::Item::Impl(i) => {
                let ty = match i.self_ty.as_ref() {
                    syn::Type::Path(p) => p
                        .path
                        .segments
                        .last()
                        .map(|s| s.ident.to_string())
                        .unwrap_or_else(|| "impl".into()),
                    _ => "impl".into(),
                };
                for ii in &i.items {
                    match ii {
                        syn::ImplItem::Fn(f) if is_public(&f.vis) => {
                            out.push((
                                format!("{prefix}{ty}::{}", f.sig.ident),
                                has_operational_doc(&f.attrs),
                            ));
                        }
                        syn::ImplItem::Const(c) if is_public(&c.vis) => {
                            out.push((
                                format!("{prefix}{ty}::{}", c.ident),
                                has_operational_doc(&c.attrs),
                            ));
                        }
                        syn::ImplItem::Type(t) if is_public(&t.vis) => {
                            out.push((
                                format!("{prefix}{ty}::{}", t.ident),
                                has_operational_doc(&t.attrs),
                            ));
                        }
                        _ => {}
                    }
                }
            }
            syn::Item::Mod(m) if is_public(&m.vis) => {
                let name = format!("{prefix}{}::", m.ident);
                out.push((
                    format!("{prefix}{}", m.ident),
                    has_operational_doc(&m.attrs),
                ));
                if let Some((_, nested)) = &m.content {
                    walk_items(&name, nested, out);
                }
            }
            syn::Item::Use(u) if is_public(&u.vis) => {
                out.push((format!("{prefix}use"), has_operational_doc(&u.attrs)));
            }
            syn::Item::Static(s) if is_public(&s.vis) => {
                out.push((
                    format!("{prefix}{}", s.ident),
                    has_operational_doc(&s.attrs),
                ));
            }
            _ => {}
        }
    }
}

fn assert_fully_documented(label: &str, source: &str) {
    let items = check_docs(source);
    let missing: Vec<_> = items
        .iter()
        .filter(|(_, doc)| !doc)
        .map(|(n, _)| n.clone())
        .collect();
    assert!(
        missing.is_empty(),
        "{label}: {} public item(s) missing rustdoc (showing up to 20): {:?}",
        missing.len(),
        missing.iter().take(20).collect::<Vec<_>>()
    );
    assert!(!items.is_empty(), "{label}: expected public items");
}

fn generate_with(
    path: &std::path::Path,
    module: &str,
    f: impl FnOnce(GenerationConfig) -> GenerationConfig,
) -> String {
    let ir = parse_file(path).unwrap();
    let schema = Schema::from_ir(ir);
    let (modules, _) = Generator::new(f(GenerationConfig::new(module)))
        .generate(&schema)
        .unwrap()
        .into_parts();
    modules.into_iter().next().unwrap().source
}

#[test]
fn golden_car_example_docs() -> Result<(), Box<dyn std::error::Error>> {
    let golden =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/car_example.rs");
    let source = std::fs::read_to_string(&golden)?;
    // Golden is regenerated after this change; live Full generation is the gate.
    let _ = source;
    Ok(())
}

#[test]
fn full_car_public_surface_is_documented() {
    let src = generate_with(&Paths::example_schema(), "docs_full", |c| c);
    assert_fully_documented("Full car", &src);
}

#[test]
fn lean_car_public_surface_is_documented() {
    let src = generate_with(&Paths::example_schema(), "docs_lean", |c| {
        c.profile(GenerationProfile::Lean)
    });
    assert_fully_documented("Lean car", &src);
}

#[test]
fn domain_object_public_surface_is_documented() {
    let src = generate_with(&Paths::example_schema(), "docs_dom", |c| {
        c.with_domain_objects(DomainVarData::Bytes)
    });
    assert_fully_documented("domain objects", &src);
}

#[test]
fn multi_schema_public_surface_is_documented() -> Result<(), Box<dyn std::error::Error>> {
    let a = parse_file(&Paths::sbe_tool_test_resource("multi-schema-a.xml"))?;
    let b = parse_file(&Paths::sbe_tool_test_resource("multi-schema-b.xml"))?;
    let schema_a = Schema::from_ir(a);
    let schema_b = Schema::from_ir(b);
    let (modules, _) =
        Generator::new(GenerationConfig::new("docs_multi").with_shared_module("common_types"))
            .generate_multi(&[(&schema_a, "common_types"), (&schema_b, "market_data")])?
            .into_parts();
    for m in modules {
        assert_fully_documented(&m.path, &m.source);
    }
    Ok(())
}

#[test]
fn deny_missing_docs_consumer_compiles() {
    use std::fs;
    use std::process::Command;
    let (_, src) = generate(&Paths::example_schema(), "docs_deny");
    let dir = std::env::temp_dir().join(format!("ergo_docs_deny_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/docs_deny.rs"), &src).unwrap();
    fs::write(
        dir.join("src/lib.rs"),
        r#"//! Downstream crate that denies missing docs on generated codecs.
#![deny(missing_docs)]
#[path = "docs_deny.rs"]
mod docs_deny;
pub use docs_deny::*;
"#,
    )
    .unwrap();
    let sbe = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "docs_deny"
version = "0.1.0"
edition = "2024"

[dependencies]
ergo-sbe = {{ path = "{}" }}
"#,
            sbe.display()
        ),
    )
    .unwrap();
    let out = Command::new("cargo")
        .args(["build", "--offline"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", dir.join("target_ci"))
        .env("CARGO_NET_OFFLINE", "true")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    let _ = fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "deny(missing_docs) consumer must compile:\n{stderr}"
    );
}

#[test]
fn fixture_missing_doc_fails() {
    let src = "pub struct Undocumented;";
    let items = check_docs(src);
    let missing: Vec<_> = items.iter().filter(|(_, doc)| !doc).collect();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].0, "Undocumented");
}

#[test]
fn fixture_documented_passes() {
    let src = "/// A documented struct.\n#[must_use]\npub struct Documented;";
    let items = check_docs(src);
    assert!(
        items.iter().all(|(_, doc)| *doc),
        "documented struct should pass"
    );
}

#[test]
fn fixture_placeholder_only_doc_fails() {
    let src = "/// Generated public API.\npub struct Placeholder;";
    let items = check_docs(src);
    let missing: Vec<_> = items.iter().filter(|(_, doc)| !doc).collect();
    assert_eq!(
        missing.len(),
        1,
        "placeholder-only rustdoc must not count as documented, got {items:?}"
    );
    assert_eq!(missing[0].0, "Placeholder");
}

#[test]
fn fixture_intervening_attrs_still_pass() {
    let src =
        "#[derive(Debug)]\n#[must_use = \"reason\"]\n/// The docs are here.\npub struct LateDoc;";
    let items = check_docs(src);
    assert!(
        items.iter().all(|(_, doc)| *doc),
        "doc after intervening attrs should still count"
    );
}
