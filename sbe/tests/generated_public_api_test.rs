//! Generated public-API snapshots for the 1.0 freeze gate.
//!
//! Fixtures are the `[[fixtures]]` records in `api/public-api-baseline.toml`.
//! Generation uses each record's schema, profile, domain, and shared topology.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

use std::error::Error;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

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

struct Fixture {
    name: String,
    schema: PathBuf,
    profile: GenerationProfile,
    generated_module: String,
    snapshot: PathBuf,
    domain_objects: Option<DomainVarData>,
    shared_schema: Option<PathBuf>,
    shared_topology: Option<String>,
    consumer_module: Option<String>,
}

fn workspace() -> PathBuf {
    Paths::workspace_root()
}

fn manifest_path() -> PathBuf {
    workspace().join("api/public-api-baseline.toml")
}

fn fixtures_from_manifest() -> Result<Vec<Fixture>, Box<dyn Error>> {
    let text = std::fs::read_to_string(manifest_path())?;
    let table: toml::Table = toml::from_str(&text)?;
    let rows = table
        .get("fixtures")
        .and_then(|v| v.as_array())
        .ok_or("manifest missing [[fixtures]]")?;
    let mut out = Vec::new();
    for row in rows {
        let name = row
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("fixture missing name")?
            .to_owned();
        let schema = workspace().join(
            row.get("schema")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("fixture {name} missing schema"))?,
        );
        let profile = match row
            .get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("Full")
        {
            "Lean" => GenerationProfile::Lean,
            _ => GenerationProfile::Full,
        };
        let generated_module = row
            .get("generated_module")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("fixture {name} missing generated_module"))?
            .to_owned();
        let snapshot = workspace().join(
            row.get("snapshot")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("fixture {name} missing snapshot"))?,
        );
        let domain_objects = match row.get("domain_objects").and_then(|v| v.as_str()) {
            Some("Bytes") => Some(DomainVarData::Bytes),
            Some("Strings") => Some(DomainVarData::Strings),
            Some(other) => {
                return Err(format!("fixture {name}: unknown domain_objects {other}").into());
            }
            None => None,
        };
        let shared_schema = row
            .get("shared_schema")
            .and_then(|v| v.as_str())
            .map(|p| workspace().join(p));
        let shared_topology = row
            .get("shared_topology")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        let consumer_module = row
            .get("consumer_module")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        if shared_schema.is_some() && shared_topology.is_none() {
            return Err(format!("fixture {name}: shared_schema requires shared_topology").into());
        }
        out.push(Fixture {
            name,
            schema,
            profile,
            generated_module,
            snapshot,
            domain_objects,
            shared_schema,
            shared_topology,
            consumer_module,
        });
    }
    Ok(out)
}

fn config_for(fix: &Fixture) -> GenerationConfig {
    let mut cfg = GenerationConfig::new(&fix.generated_module).profile(fix.profile);
    if let Some(domain) = fix.domain_objects {
        cfg = cfg.with_domain_objects(domain);
    }
    if fix.shared_schema.is_some() {
        cfg = cfg.with_shared_module(&fix.generated_module);
    }
    cfg
}

fn generate_source(fix: &Fixture) -> Result<String, Box<dyn Error>> {
    if let Some(shared) = &fix.shared_schema {
        match fix.shared_topology.as_deref() {
            Some("shared-first") => {}
            other => {
                return Err(format!(
                    "fixture {}: unsupported shared_topology {other:?}",
                    fix.name
                )
                .into());
            }
        }
        let consumer = fix.consumer_module.as_deref().ok_or_else(|| {
            format!(
                "fixture {}: shared_schema requires consumer_module",
                fix.name
            )
        })?;
        let a = parse_file(&fix.schema)?;
        let b = parse_file(shared)?;
        let schema_a = Schema::from_ir(a);
        let schema_b = Schema::from_ir(b);
        let (modules, _) = Generator::new(config_for(fix))
            .generate_multi(&[
                (&schema_a, fix.generated_module.as_str()),
                (&schema_b, consumer),
            ])?
            .into_parts();
        let mut combined = String::new();
        for m in modules {
            writeln!(combined, "# {}", m.path)?;
            combined.push_str(&public_names(&m.source)?.join("\n"));
            combined.push('\n');
        }
        return Ok(combined);
    }
    let ir = parse_file(&fix.schema)?;
    let schema = Schema::from_ir(ir);
    let (modules, _) = Generator::new(config_for(fix))
        .generate(&schema)?
        .into_parts();
    let names = public_names(
        &modules
            .into_iter()
            .next()
            .ok_or("no generated module")?
            .source,
    )?;
    Ok(names.join("\n") + "\n")
}

fn assert_snapshot(fix: &Fixture, body: &str) -> Result<(), Box<dyn Error>> {
    if std::env::var_os("UPDATE_GENERATED_PUBLIC_API").is_some() {
        if let Some(parent) = fix.snapshot.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&fix.snapshot, body)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&fix.snapshot).unwrap_or_default();
    assert_eq!(
        expected, body,
        "generated public API snapshot {} drifted. Review the diff and run \
         UPDATE_GENERATED_PUBLIC_API=1 cargo test -p ergo-sbe --test generated_public_api_test",
        fix.name
    );
    Ok(())
}

#[test]
fn snapshot_corpus_excludes_private_and_golden_duplicates() -> Result<(), Box<dyn Error>> {
    let manifest = std::fs::read_to_string(manifest_path())?;
    assert!(
        !manifest.contains("name = \"cluster_session\""),
        "cluster_session pins crate-private codecs; drop it from the public-API freeze"
    );
    assert!(
        !manifest.contains("name = \"car_full\""),
        "car_full duplicates just check-golden; drop it from the snapshot corpus"
    );
    assert!(
        !workspace()
            .join("api/generated/cluster_session.txt")
            .exists(),
        "api/generated/cluster_session.txt must not exist"
    );
    assert!(
        !workspace().join("api/generated/car_full.txt").exists(),
        "api/generated/car_full.txt must not exist"
    );
    Ok(())
}

#[test]
fn manifest_fixtures_match_snapshots() -> Result<(), Box<dyn Error>> {
    let fixtures = fixtures_from_manifest()?;
    assert!(
        !fixtures.is_empty(),
        "manifest must declare at least one fixture"
    );
    let names: Vec<_> = fixtures.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"car_lean")
            && names.contains(&"car_domain")
            && names.contains(&"multi_schema_shared"),
        "kept fixtures missing from manifest: {names:?}"
    );
    for fix in &fixtures {
        assert!(
            Path::new(&fix.schema).is_file(),
            "fixture {} schema missing: {}",
            fix.name,
            fix.schema.display()
        );
        let body = generate_source(fix)?;
        assert_snapshot(fix, &body)?;
    }
    Ok(())
}

#[test]
fn snapshot_removal_of_serial_number_would_fail() -> Result<(), Box<dyn Error>> {
    let fixtures = fixtures_from_manifest()?;
    let lean = fixtures
        .iter()
        .find(|f| f.name == "car_lean")
        .ok_or("car_lean fixture missing")?;
    let body = generate_source(lean)?;
    assert!(
        body.lines().any(|n| n.contains("serial_number")),
        "car_lean snapshot must include serial_number so removing it is a detectable freeze break"
    );
    Ok(())
}
