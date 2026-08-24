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

/// Canonically serialise the semver-relevant `syn` surface of every public
/// item: struct fields (name/position + type), enum variant payloads and
/// discriminants, full free-fn/method signatures (receiver, generics, args,
/// return type, where-clause), associated types/consts, type aliases, and
/// the semver-relevant attributes (`cfg`, `non_exhaustive`, `repr`,
/// `deprecated`, `must_use`) — preserved on the item rather than causing it
/// to be dropped. One line per item/field/variant/method, matching the
/// previous names-only granularity so a change localises the same way.
fn public_surface(source: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let file = syn::parse_file(source)?;
    let mut out = Vec::new();
    collect("", &file.items, &mut out);
    out.sort();
    out.dedup();
    Ok(out)
}

const fn is_public(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

/// Render only the attributes that change the public contract. Doc
/// comments and other prose-only attributes are deliberately excluded so a
/// body/doc-only edit does not disturb the snapshot.
fn semver_attrs(attrs: &[syn::Attribute]) -> String {
    let relevant: Vec<String> = attrs
        .iter()
        .filter(|a| {
            let p = a.path();
            p.is_ident("cfg")
                || p.is_ident("non_exhaustive")
                || p.is_ident("repr")
                || p.is_ident("deprecated")
                || p.is_ident("must_use")
        })
        .map(|a| quote::quote!(#a).to_string())
        .collect();
    if relevant.is_empty() {
        String::new()
    } else {
        format!("{} ", relevant.join(" "))
    }
}

fn generics_str(g: &syn::Generics) -> String {
    quote::quote!(#g).to_string()
}

fn where_str(g: &syn::Generics) -> String {
    g.where_clause
        .as_ref()
        .map(|w| format!(" {}", quote::quote!(#w)))
        .unwrap_or_default()
}

/// `filter_visibility`: struct fields need an individual `pub` to be part of
/// the public surface. Enum variant fields have no visibility syntax at all
/// — they are implicitly as public as the variant — so filtering them by
/// `is_public` would silently drop every payload.
fn fields_str(fields: &syn::Fields, filter_visibility: bool) -> String {
    let keep = |vis: &syn::Visibility| !filter_visibility || is_public(vis);
    match fields {
        syn::Fields::Named(f) => f
            .named
            .iter()
            .filter(|fld| keep(&fld.vis))
            .map(|fld| {
                let ty = &fld.ty;
                format!(
                    "{}{}: {}",
                    semver_attrs(&fld.attrs),
                    fld.ident.as_ref().expect("named field has an ident"),
                    quote::quote!(#ty)
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
        syn::Fields::Unnamed(f) => f
            .unnamed
            .iter()
            .enumerate()
            .filter(|(_, fld)| keep(&fld.vis))
            .map(|(i, fld)| {
                let ty = &fld.ty;
                format!("{i}: {}", quote::quote!(#ty))
            })
            .collect::<Vec<_>>()
            .join(", "),
        syn::Fields::Unit => String::new(),
    }
}

fn sig_str(sig: &syn::Signature) -> String {
    quote::quote!(#sig).to_string()
}

fn collect(prefix: &str, items: &[syn::Item], out: &mut Vec<String>) {
    for item in items {
        match item {
            syn::Item::Struct(s) if is_public(&s.vis) => {
                out.push(format!(
                    "{}{prefix}struct {}{} {{ {} }}",
                    semver_attrs(&s.attrs),
                    s.ident,
                    generics_str(&s.generics),
                    fields_str(&s.fields, true)
                ));
            }
            syn::Item::Enum(e) if is_public(&e.vis) => {
                let enum_attrs = semver_attrs(&e.attrs);
                out.push(format!(
                    "{enum_attrs}{prefix}enum {}{}",
                    e.ident,
                    generics_str(&e.generics)
                ));
                for v in &e.variants {
                    let payload = fields_str(&v.fields, false);
                    let discriminant = v
                        .discriminant
                        .as_ref()
                        .map(|(_, expr)| format!(" = {}", quote::quote!(#expr)))
                        .unwrap_or_default();
                    out.push(format!(
                        "{}{prefix}enum {}::{}{}{}",
                        semver_attrs(&v.attrs),
                        e.ident,
                        v.ident,
                        if payload.is_empty() {
                            String::new()
                        } else {
                            format!("({payload})")
                        },
                        discriminant
                    ));
                }
            }
            syn::Item::Fn(f) if is_public(&f.vis) => {
                out.push(format!(
                    "{}{prefix}fn {}",
                    semver_attrs(&f.attrs),
                    sig_str(&f.sig)
                ));
            }
            syn::Item::Const(c) if is_public(&c.vis) => {
                let ty = &c.ty;
                out.push(format!(
                    "{}{prefix}const {}: {}",
                    semver_attrs(&c.attrs),
                    c.ident,
                    quote::quote!(#ty)
                ));
            }
            syn::Item::Type(t) if is_public(&t.vis) => {
                let ty = &t.ty;
                out.push(format!(
                    "{}{prefix}type {}{}{} = {}",
                    semver_attrs(&t.attrs),
                    t.ident,
                    generics_str(&t.generics),
                    where_str(&t.generics),
                    quote::quote!(#ty)
                ));
            }
            syn::Item::Trait(t) if is_public(&t.vis) => {
                let supertraits: Vec<String> = t
                    .supertraits
                    .iter()
                    .map(|b| quote::quote!(#b).to_string())
                    .collect();
                out.push(format!(
                    "{}{prefix}trait {}{}{}{}",
                    semver_attrs(&t.attrs),
                    t.ident,
                    generics_str(&t.generics),
                    if supertraits.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", supertraits.join(" + "))
                    },
                    where_str(&t.generics)
                ));
                for ti in &t.items {
                    match ti {
                        syn::TraitItem::Fn(f) => {
                            out.push(format!(
                                "{}{prefix}trait {}::{}",
                                semver_attrs(&f.attrs),
                                t.ident,
                                sig_str(&f.sig)
                            ));
                        }
                        syn::TraitItem::Type(ty) => {
                            out.push(format!("{prefix}trait {}::type {}", t.ident, ty.ident));
                        }
                        syn::TraitItem::Const(c) => {
                            let const_ty = &c.ty;
                            out.push(format!(
                                "{prefix}trait {}::const {}: {}",
                                t.ident,
                                c.ident,
                                quote::quote!(#const_ty)
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
                        .map_or_else(|| "impl".into(), |s| s.ident.to_string()),
                    _ => "impl".into(),
                };
                for ii in &i.items {
                    match ii {
                        syn::ImplItem::Fn(f) if is_public(&f.vis) => {
                            out.push(format!(
                                "{}{prefix}{ty}::{}",
                                semver_attrs(&f.attrs),
                                sig_str(&f.sig)
                            ));
                        }
                        syn::ImplItem::Const(c) if is_public(&c.vis) => {
                            let const_ty = &c.ty;
                            out.push(format!(
                                "{prefix}{ty}::const {}: {}",
                                c.ident,
                                quote::quote!(#const_ty)
                            ));
                        }
                        syn::ImplItem::Type(assoc_ty) if is_public(&assoc_ty.vis) => {
                            let underlying = &assoc_ty.ty;
                            out.push(format!(
                                "{prefix}{ty}::type {} = {}",
                                assoc_ty.ident,
                                quote::quote!(#underlying)
                            ));
                        }
                        _ => {}
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
            combined.push_str(&public_surface(&m.source)?.join("\n"));
            combined.push('\n');
        }
        return Ok(combined);
    }
    let ir = parse_file(&fix.schema)?;
    let schema = Schema::from_ir(ir);
    let (modules, _) = Generator::new(config_for(fix))
        .generate(&schema)?
        .into_parts();
    let names = public_surface(
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

/// T-12 acceptance criteria: `public_surface` must change/fail on each of
/// five mutation classes the previous names-only snapshot missed, and must
/// NOT change on a private/body/doc-only edit. Each pair below is real,
/// `syn`-parseable Rust source differing in exactly one respect.
mod mutation_sensitivity {
    use super::public_surface;

    fn wrap(body: &str) -> String {
        format!("pub mod m {{ {body} }}")
    }

    #[test]
    fn receiver_change_is_detected() -> Result<(), Box<dyn std::error::Error>> {
        let a = wrap("pub struct S; impl S { pub fn f(&self) -> i32 { 0 } }");
        let b = wrap("pub struct S; impl S { pub fn f(self) -> i32 { 0 } }");
        assert_ne!(
            public_surface(&a)?,
            public_surface(&b)?,
            "receiver change must be visible"
        );
        Ok(())
    }

    #[test]
    fn argument_and_return_type_change_is_detected() -> Result<(), Box<dyn std::error::Error>> {
        let a = wrap("pub struct S; impl S { pub fn f(&self, x: i32) -> bool { true } }");
        let b = wrap("pub struct S; impl S { pub fn f(&self, x: i64) -> bool { true } }");
        assert_ne!(
            public_surface(&a)?,
            public_surface(&b)?,
            "argument type change must be visible"
        );

        let c = wrap("pub struct S; impl S { pub fn f(&self, x: i32) -> u8 { 0 } }");
        assert_ne!(
            public_surface(&a)?,
            public_surface(&c)?,
            "return type change must be visible"
        );
        Ok(())
    }

    #[test]
    fn generic_and_where_clause_change_is_detected() -> Result<(), Box<dyn std::error::Error>> {
        let a =
            wrap("pub struct S; impl S { pub fn f<T>(&self, x: T) where T: Clone { let _ = x; } }");
        let b =
            wrap("pub struct S; impl S { pub fn f<T>(&self, x: T) where T: Copy { let _ = x; } }");
        assert_ne!(
            public_surface(&a)?,
            public_surface(&b)?,
            "where-clause bound change must be visible"
        );

        let c = wrap("pub struct S; impl S { pub fn f<T: Clone>(&self, x: T) { let _ = x; } }");
        let d = wrap("pub struct S; impl S { pub fn f<T: Copy>(&self, x: T) { let _ = x; } }");
        assert_ne!(
            public_surface(&c)?,
            public_surface(&d)?,
            "generic bound change must be visible"
        );
        Ok(())
    }

    #[test]
    fn public_field_and_enum_payload_change_is_detected() -> Result<(), Box<dyn std::error::Error>>
    {
        let a = wrap("pub struct S { pub a: i32 }");
        let b = wrap("pub struct S { pub a: i64 }");
        assert_ne!(
            public_surface(&a)?,
            public_surface(&b)?,
            "public field type change must be visible"
        );

        let c = wrap("pub enum E { A(i32) }");
        let d = wrap("pub enum E { A(i64) }");
        assert_ne!(
            public_surface(&c)?,
            public_surface(&d)?,
            "enum payload type change must be visible"
        );
        Ok(())
    }

    #[test]
    fn cfg_gated_method_change_is_detected_not_dropped() -> Result<(), Box<dyn std::error::Error>> {
        let a = wrap(r#"pub struct S; impl S { #[cfg(feature = "x")] pub fn f(&self) {} }"#);
        let b = wrap(r#"pub struct S; impl S { #[cfg(feature = "x")] pub fn g(&self) {} }"#);
        let sa = public_surface(&a)?;
        let sb = public_surface(&b)?;
        assert_ne!(sa, sb, "a cfg-gated method's own change must be visible");
        assert!(
            sa.iter().any(|l| l.contains("cfg") && l.contains("fn f (")),
            "cfg-gated item must be PRESENT in the surface (not dropped), got: {sa:?}"
        );
        Ok(())
    }

    #[test]
    fn private_field_and_doc_only_edits_do_not_change_the_snapshot()
    -> Result<(), Box<dyn std::error::Error>> {
        let a = wrap("pub struct S { pub a: i32, priv_b: i32 }");
        let b = wrap("pub struct S { pub a: i32, priv_b: i64 }");
        assert_eq!(
            public_surface(&a)?,
            public_surface(&b)?,
            "a private field's type is not part of the public surface"
        );

        let c = wrap("/// old doc\npub struct S { pub a: i32 }");
        let d = wrap("/// completely different doc\npub struct S { pub a: i32 }");
        assert_eq!(
            public_surface(&c)?,
            public_surface(&d)?,
            "doc-only edits must not change the snapshot"
        );

        let e = wrap("pub struct S; impl S { pub fn f(&self) -> i32 { 0 } }");
        let f = wrap("pub struct S; impl S { pub fn f(&self) -> i32 { 1 + 2 } }");
        assert_eq!(
            public_surface(&e)?,
            public_surface(&f)?,
            "a body-only edit must not change the snapshot"
        );
        Ok(())
    }
}
