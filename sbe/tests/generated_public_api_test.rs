//! Generated public-API snapshots for the 1.0 freeze gate.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

use std::error::Error;
use std::fmt::Write as _;

mod common;
use common::Paths;
use ergo_sbe::{DomainVarData, GenerationConfig, GenerationProfile, Generator, Schema, parse_file};

fn public_names(source: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let file = syn::parse_file(source)?;
    let mut names = Vec::new();
    collect("", &file.items, &mut names);
    names.sort();
    names.dedup();
    Ok(names)
}

const fn is_public(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn collect(prefix: &str, items: &[syn::Item], out: &mut Vec<String>) {
    for item in items {
        match item {
            syn::Item::Struct(s) if is_public(&s.vis) => {
                out.push(format!("{prefix}struct {}", s.ident));
            }
            syn::Item::Enum(e) if is_public(&e.vis) => {
                out.push(format!("{prefix}enum {}", e.ident));
                for v in &e.variants {
                    out.push(format!("{prefix}enum {}::{}", e.ident, v.ident));
                }
            }
            syn::Item::Fn(f) if is_public(&f.vis) => {
                out.push(format!("{prefix}fn {}", f.sig.ident));
            }
            syn::Item::Const(c) if is_public(&c.vis) => {
                out.push(format!("{prefix}const {}", c.ident));
            }
            syn::Item::Type(t) if is_public(&t.vis) => {
                out.push(format!("{prefix}type {}", t.ident));
            }
            syn::Item::Trait(t) if is_public(&t.vis) => {
                out.push(format!("{prefix}trait {}", t.ident));
            }
            syn::Item::Impl(i) => {
                let ty = match i.self_ty.as_ref() {
                    syn::Type::Path(p) => p
                        .path
                        .segments
                        .last()
                        .map_or_else(|| "impl".into(), |s| s.ident.to_string()),
                    _ => "impl".into(),
                };
                for ii in &i.items {
                    if let syn::ImplItem::Fn(f) = ii
                        && is_public(&f.vis)
                    {
                        out.push(format!("{prefix}{ty}::{}", f.sig.ident));
                    }
                }
            }
            syn::Item::Mod(m) if is_public(&m.vis) => {
                out.push(format!("{prefix}mod {}", m.ident));
                if let Some((_, nested)) = &m.content {
                    collect(&format!("{prefix}{}::", m.ident), nested, out);
                }
            }
            _ => {}
        }
    }
}

fn generate(
    path: &std::path::Path,
    _module: &str,
    cfg: GenerationConfig,
) -> Result<String, Box<dyn Error>> {
    let ir = parse_file(path)?;
    let schema = Schema::from_ir(ir);
    let (modules, _) = Generator::new(cfg).generate(&schema)?.into_parts();
    Ok(modules
        .into_iter()
        .next()
        .ok_or("no generated module")?
        .source)
}

fn snapshot_path(name: &str) -> std::path::PathBuf {
    Paths::workspace_root()
        .join("api")
        .join("generated")
        .join(format!("{name}.txt"))
}

fn assert_snapshot(name: &str, source: &str) -> Result<(), Box<dyn Error>> {
    let names = public_names(source)?;
    let body = names.join("\n") + "\n";
    let path = snapshot_path(name);
    if std::env::var_os("UPDATE_GENERATED_PUBLIC_API").is_some() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &body)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        expected, body,
        "generated public API snapshot {name} drifted. Review the diff and run \
         UPDATE_GENERATED_PUBLIC_API=1 cargo test -p ergo-sbe --test generated_public_api_test"
    );
    Ok(())
}

#[test]
fn car_full_public_api_snapshot() -> Result<(), Box<dyn Error>> {
    let src = generate(
        &Paths::example_schema(),
        "car_example",
        GenerationConfig::new("car_example"),
    )?;
    assert_snapshot("car_full", &src)
}

#[test]
fn car_lean_public_api_snapshot() -> Result<(), Box<dyn Error>> {
    let src = generate(
        &Paths::example_schema(),
        "car_lean",
        GenerationConfig::new("car_lean").profile(GenerationProfile::Lean),
    )?;
    assert_snapshot("car_lean", &src)
}

#[test]
fn car_domain_public_api_snapshot() -> Result<(), Box<dyn Error>> {
    let src = generate(
        &Paths::example_schema(),
        "car_domain",
        GenerationConfig::new("car_domain").with_domain_objects(DomainVarData::Bytes),
    )?;
    assert_snapshot("car_domain", &src)
}

#[test]
fn multi_schema_public_api_snapshot() -> Result<(), Box<dyn Error>> {
    let a = parse_file(&Paths::sbe_tool_test_resource("multi-schema-a.xml"))?;
    let b = parse_file(&Paths::sbe_tool_test_resource("multi-schema-b.xml"))?;
    let schema_a = Schema::from_ir(a);
    let schema_b = Schema::from_ir(b);
    let (modules, _) =
        Generator::new(GenerationConfig::new("multi_shared").with_shared_module("common_types"))
            .generate_multi(&[(&schema_a, "common_types"), (&schema_b, "market_data")])?
            .into_parts();
    let mut combined = String::new();
    for m in modules {
        writeln!(combined, "# {}", m.path)?;
        combined.push_str(&public_names(&m.source)?.join("\n"));
        combined.push('\n');
    }
    let path = snapshot_path("multi_schema_shared");
    if std::env::var_os("UPDATE_GENERATED_PUBLIC_API").is_some() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &combined)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(expected, combined, "multi_schema_shared snapshot drifted");
    Ok(())
}

#[test]
fn cluster_session_public_api_snapshot() -> Result<(), Box<dyn Error>> {
    let schema = Paths::workspace_root().join("cluster/schemas/aeron-cluster-codecs.xml");
    let src = generate(&schema, "session", GenerationConfig::new("session"))?;
    assert_snapshot("cluster_session", &src)
}

#[test]
fn snapshot_removal_of_serial_number_would_fail() -> Result<(), Box<dyn Error>> {
    let src = generate(
        &Paths::example_schema(),
        "car_example",
        GenerationConfig::new("car_example"),
    )?;
    let names = public_names(&src)?;
    assert!(
        names.iter().any(|n| n.contains("serial_number")),
        "car Full snapshot must include CarDecoder::serial_number so removing it is a detectable freeze break"
    );
    Ok(())
}
