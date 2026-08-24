//! Helpers for Cargo `build.rs` scripts.
//!
//! Prefer these over hand-rolling parse → generate → write → `rerun-if-changed`.
//!
//! ```rust,no_run
//! // build.rs — ergo_sbe::miette::Result renders schema errors with a source
//! // snippet by default; Box<dyn std::error::Error> prints a raw Debug dump.
//! fn main() -> ergo_sbe::miette::Result<()> {
//!     ergo_sbe::generate_to_out_dir(
//!         "schemas/messages.xml",
//!         ergo_sbe::GenerationConfig::new("messages"),
//!     )?;
//!     Ok(())
//! }
//! ```
//!
//! Then include the generated module from `lib.rs` or `main.rs`:
//!
//! ```text
//! ergo_sbe::sbe_mod!(messages);
//! ```

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::codegen::{GenerateError, GeneratedModuleSet, Generator};
use crate::config::GenerationConfig;
use crate::schema::Schema;
use crate::xml::{ParseError, parse, parse_file_with_deps, parse_file_with_shared_deps};

/// Errors from [`generate_to_out_dir`] / [`generate_str_to_out_dir`].
///
/// Implements [`miette::Diagnostic`] so `fn main() -> miette::Result<()>` in a
/// `build.rs` renders schema parse errors with a source snippet and span
/// instead of a raw `Debug` dump. Plain `Box<dyn std::error::Error>` prints
/// `{:?}` on failure — use `miette::Result` to get the readable form.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[non_exhaustive]
pub enum BuildError {
    /// Schema XML could not be parsed or resolved.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(#[from] ParseError),
    /// Code generation failed (e.g. invalid conversion config).
    #[error(transparent)]
    Generate(#[from] GenerateError),
    /// `OUT_DIR` is unset — this helper is meant for Cargo `build.rs` only.
    #[error("OUT_DIR is not set (run from a Cargo build.rs script)")]
    MissingOutDir,
    /// Failed to create, write, back up, or promote a generated output.
    /// `action` names the attempted step (e.g. "create output directory",
    /// "write generated module") and `path` is the exact destination or
    /// staging file involved.
    #[error("failed to {action} at {}: {source}", path.display())]
    Io {
        /// What was being attempted.
        action: &'static str,
        /// The path involved.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Generator produced no modules.
    #[error("schema generated no modules")]
    Empty,
}

/// Build a [`BuildError::Io`] from an I/O result at a known action/path.
fn io_err(action: &'static str, path: &Path, source: std::io::Error) -> BuildError {
    BuildError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

/// One schema file in a multi-schema generation set.
///
/// `module_name` is the generated Rust module (`orders` → `orders.rs`), not
/// derived from the file stem — so `common-types.xml` can emit `common_types.rs`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemaFile<'a> {
    /// Path to the schema XML file.
    pub path: &'a Path,
    /// Generated module identifier (no `.rs` suffix).
    pub module_name: &'a str,
}

impl<'a> SchemaFile<'a> {
    /// Construct a [`SchemaFile`].
    #[must_use]
    pub fn new(path: &'a Path, module_name: &'a str) -> Self {
        Self { path, module_name }
    }
}

/// Parse a schema **file**, generate codecs, write every module under `OUT_DIR`.
///
/// Also prints `cargo::rerun-if-changed=<schema_path>` and
/// `cargo::warning=…` for non-fatal generation warnings.
///
/// `config.module_name` becomes `{module_name}.rs` (e.g. `"messages"` →
/// `$OUT_DIR/messages.rs`).
///
/// # Errors
///
/// Parse, generate, missing `OUT_DIR`, or I/O failures.
///
/// # Example
///
/// ```rust,no_run
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     ergo_sbe::generate_to_out_dir(
///         "schemas/messages.xml",
///         ergo_sbe::GenerationConfig::new("messages")
///             .with_domain_objects(ergo_sbe::DomainVarData::Bytes),
///     )?;
///     Ok(())
/// }
/// ```
pub fn generate_to_out_dir(
    schema_path: impl AsRef<Path>,
    config: GenerationConfig,
) -> Result<GeneratedModuleSet, BuildError> {
    generate_to_dir(schema_path, config, &out_dir()?)
}

/// Parse a schema **file**, generate codecs, write every module under `out_dir`.
///
/// Same as [`generate_to_out_dir`] but with an explicit output directory.
///
/// **Samples:** write to `src/generated/` (gitignored) so rust-analyzer / IDE
/// go-to-definition works on real `.rs` files. Do **not** commit those files —
/// they are large and change whenever the generator does.
///
/// Prints `cargo::rerun-if-changed` for the root schema and every resolved
/// include, plus generation warnings.
///
/// # Errors
///
/// Parse, generate, or I/O failures.
///
/// # Example
///
/// ```rust,no_run
/// // build.rs
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
///     ergo_sbe::generate_to_dir(
///         "schemas/feature-tour.xml",
///         ergo_sbe::GenerationConfig::new("feature_tour"),
///         &out,
///     )?;
///     Ok(())
/// }
///
/// ```
///
/// In `src/lib.rs`, use the real path so the IDE can jump into the implementation:
///
/// ```text
/// #[path = "generated/feature_tour.rs"]
/// mod feature_tour;
/// ```
pub fn generate_to_dir(
    schema_path: impl AsRef<Path>,
    config: GenerationConfig,
    out_dir: impl AsRef<Path>,
) -> Result<GeneratedModuleSet, BuildError> {
    let schema_path = schema_path.as_ref();
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir)
        .map_err(|source| io_err("create output directory", out_dir, source))?;
    let parsed = parse_file_with_deps(schema_path)?;
    let modules = write_generated(Schema::from_ir(parsed.ir), config, out_dir)?;
    for watched in schema_watch_paths(schema_path, &parsed.dependencies) {
        println!("cargo::rerun-if-changed={}", watched.display());
    }
    // Point at stable IDE paths (e.g. src/generated). Skip for hashed OUT_DIR —
    // that would spam every product/sample build that only uses generate_to_out_dir.
    let is_cargo_out = env::var_os("OUT_DIR")
        .map(|od| out_dir.starts_with(Path::new(&od)))
        .unwrap_or(false);
    if !is_cargo_out {
        println!(
            "cargo::warning=ergo-sbe wrote {} module(s) under {} (open for go-to-definition)",
            modules.modules().len(),
            out_dir.display()
        );
    }
    Ok(modules)
}

/// Like [`generate_to_out_dir`], but from an XML string (e.g. `include_str!`).
///
/// Does **not** emit `rerun-if-changed` (no file path). Prefer
/// [`generate_to_out_dir`] when the schema lives on disk so Cargo rebuilds
/// when it changes. If you use `include_str!`, add your own
/// `cargo::rerun-if-changed` for that path.
///
/// # Errors
///
/// Parse, generate, missing `OUT_DIR`, or I/O failures.
pub fn generate_str_to_out_dir(
    schema_xml: &str,
    config: GenerationConfig,
) -> Result<GeneratedModuleSet, BuildError> {
    generate_str_to_dir(schema_xml, config, &out_dir()?)
}

/// Parse schema XML, generate codecs, write every module under `out_dir`.
///
/// Same as [`generate_str_to_out_dir`] but with an explicit output directory
/// (useful in tests or non-Cargo drivers). Does not emit `rerun-if-changed`.
///
/// # Errors
///
/// Parse, generate, or I/O failures.
pub fn generate_str_to_dir(
    schema_xml: &str,
    config: GenerationConfig,
    out_dir: &Path,
) -> Result<GeneratedModuleSet, BuildError> {
    let ir = parse(schema_xml)?;
    write_generated(Schema::from_ir(ir), config, out_dir)
}

/// Generate a shared schema plus consumers into `OUT_DIR`.
///
/// Parses `shared` first, then each consumer with [`crate::parse_file_with_shared`],
/// validates the complete set, then writes. A late consumer failure leaves no
/// files. Watches every root and resolved include.
///
/// # Errors
///
/// Parse, generate, missing `OUT_DIR`, I/O, or a `with_shared_module` name
/// that does not match [`SchemaFile::module_name`] on `shared`.
pub fn generate_multi_to_out_dir(
    shared: SchemaFile<'_>,
    consumers: &[SchemaFile<'_>],
    config: GenerationConfig,
) -> Result<GeneratedModuleSet, BuildError> {
    generate_multi_to_dir(shared, consumers, config, &out_dir()?)
}

/// [`generate_multi_to_out_dir`] with an explicit output directory.
///
/// # Errors
///
/// Same as [`generate_multi_to_out_dir`] except `OUT_DIR` is not required.
///
/// ```rust,no_run
/// use std::path::Path;
/// use ergo_sbe::{GenerationConfig, SchemaFile, generate_multi_to_dir};
///
/// fn main() -> ergo_sbe::miette::Result<()> {
///     let common = Path::new("schemas/common-types.xml");
///     let orders = Path::new("schemas/orders.xml");
///     generate_multi_to_dir(
///         SchemaFile::new(common, "common_types"),
///         &[SchemaFile::new(orders, "orders")],
///         GenerationConfig::new("common_types"),
///         Path::new("src/generated"),
///     )?;
///     Ok(())
/// }
/// ```
pub fn generate_multi_to_dir(
    shared: SchemaFile<'_>,
    consumers: &[SchemaFile<'_>],
    mut config: GenerationConfig,
    out_dir: impl AsRef<Path>,
) -> Result<GeneratedModuleSet, BuildError> {
    if let Some(ref name) = config.shared_module {
        if name != shared.module_name {
            return Err(BuildError::Generate(GenerateError::InvalidConfiguration {
                option: "shared_module".into(),
                value: name.clone(),
                reason: format!("must match shared.module_name {:?}", shared.module_name),
            }));
        }
    } else {
        config = config.with_shared_module(shared.module_name);
    }

    let shared_parsed = parse_file_with_deps(shared.path)?;
    let shared_ir = shared_parsed.ir;
    let mut watch = schema_watch_paths(shared.path, &shared_parsed.dependencies);
    let mut consumer_irs = Vec::with_capacity(consumers.len());
    for consumer in consumers {
        let parsed = parse_file_with_shared_deps(consumer.path, &shared_ir)?;
        watch.extend(schema_watch_paths(consumer.path, &parsed.dependencies));
        consumer_irs.push((
            Schema::from_ir(ir_with_shared_type_tokens(parsed.ir, &shared_ir)),
            consumer.module_name,
        ));
    }
    let shared_schema = Schema::from_ir(shared_ir);

    let mut schemas: Vec<(&Schema, &str)> = Vec::with_capacity(1 + consumer_irs.len());
    schemas.push((&shared_schema, shared.module_name));
    for (schema, name) in &consumer_irs {
        schemas.push((schema, name));
    }

    let modules = Generator::new(config).generate_multi(&schemas)?;
    write_module_set(&modules, out_dir.as_ref())?;
    let mut seen = HashSet::new();
    for path in watch {
        let key = path.canonicalize().unwrap_or_else(|_| path.clone());
        if seen.insert(key) {
            println!("cargo::rerun-if-changed={}", path.display());
        }
    }
    Ok(modules)
}

/// Absolute path to Cargo's `OUT_DIR` (build scripts only).
///
/// # Errors
///
/// [`BuildError::MissingOutDir`] when not running under Cargo.
pub fn out_dir() -> Result<PathBuf, BuildError> {
    env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .ok_or(BuildError::MissingOutDir)
}

/// Root schema first (the path Cargo was given), then remaining unique
/// resolved includes in sorted order.
pub(crate) fn schema_watch_paths(root: &Path, dependencies: &[PathBuf]) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut push = |p: &Path| {
        let key = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        if seen.insert(key) {
            out.push(p.to_path_buf());
        }
    };
    push(root);
    let mut rest: Vec<&PathBuf> = dependencies.iter().collect();
    rest.sort();
    for path in rest {
        push(path);
    }
    out
}

/// Copy shared type tokens (not messages) onto a consumer IR so codegen can
/// resolve `headerType` and shared composites after `parse_file_with_shared`.
fn ir_with_shared_type_tokens(mut consumer: crate::Ir, shared: &crate::Ir) -> crate::Ir {
    let mut extra = Vec::new();
    let mut i = 0;
    while i < shared.tokens.len() {
        if shared.tokens[i].signal == crate::Signal::BeginMessage {
            while i < shared.tokens.len() && shared.tokens[i].signal != crate::Signal::EndMessage {
                i += 1;
            }
            i += 1;
            continue;
        }
        extra.push(shared.tokens[i].clone());
        i += 1;
    }
    extra.append(&mut consumer.tokens);
    consumer.tokens = extra;
    consumer
}

fn write_generated(
    schema: Schema,
    config: GenerationConfig,
    out: &Path,
) -> Result<GeneratedModuleSet, BuildError> {
    let modules = Generator::new(config).generate(&schema)?;
    write_module_set(&modules, out)?;
    Ok(modules)
}

/// One module's destination, staging file, and whether `dest` already
/// existed before this write (so a rollback knows whether to restore a
/// backup or remove a file that should never have been created).
struct StagedWrite {
    dest: PathBuf,
    temp: PathBuf,
    pre_existed: bool,
}

/// Publish a complete [`GeneratedModuleSet`] to `out` as one all-or-nothing
/// unit: shared and consumer modules form one generated protocol graph, so a
/// failure partway through must never leave a mixture of old and new
/// generations on disk.
///
/// Three phases: validate every basename first (no writes yet); stage every
/// source to a unique sibling temp file; only once every temp file is
/// written, back up existing destinations and promote the staged set. Any
/// failure in staging or commit rolls back — restoring backups and removing
/// promoted/temp files — before returning the path-aware [`BuildError::Io`].
/// This guarantees rollback for a failure reported to this process, not
/// crash-atomicity across power loss.
fn write_module_set(modules: &GeneratedModuleSet, out: &Path) -> Result<(), BuildError> {
    if modules.modules().len() == 0 {
        return Err(BuildError::Empty);
    }

    // Phase 1: validate every basename before any write.
    let mut dests = Vec::with_capacity(modules.modules().len());
    for m in modules.modules() {
        // Defense in depth: reject paths with directory components.
        // Generated module paths must be simple basenames like "car.rs".
        let path_str = &m.path;
        if path_str.contains('/') || path_str.contains('\\') || path_str.contains("..") {
            return Err(BuildError::Generate(
                crate::codegen::GenerateError::InvalidConfiguration {
                    option: "module_path".into(),
                    value: path_str.clone(),
                    reason: "module path must be a plain .rs basename — no path separators".into(),
                },
            ));
        }
        dests.push(out.join(path_str));
    }

    fs::create_dir_all(out).map_err(|source| io_err("create output directory", out, source))?;

    // Phase 2: stage every source to a unique sibling temp file. Nothing at
    // `dest` is touched yet.
    let pid = std::process::id();
    let mut staged: Vec<StagedWrite> = Vec::with_capacity(dests.len());
    for (idx, (m, dest)) in modules.modules().zip(&dests).enumerate() {
        let temp = dest.with_extension(format!(
            "{}.tmp.{pid}.{idx}",
            dest.extension().and_then(|e| e.to_str()).unwrap_or("rs")
        ));
        if let Some(parent) = temp.parent()
            && let Err(source) = fs::create_dir_all(parent)
        {
            remove_staged_temps(&staged);
            return Err(io_err("create output directory", parent, source));
        }
        if let Err(source) = fs::write(&temp, &m.source) {
            remove_staged_temps(&staged);
            return Err(io_err("write generated module", &temp, source));
        }
        staged.push(StagedWrite {
            dest: dest.clone(),
            temp,
            pre_existed: dest.exists(),
        });
    }

    // Phase 3: commit. Back up every pre-existing destination, then promote
    // every staged temp file over its destination.
    let mut backed_up: Vec<(PathBuf, PathBuf)> = Vec::new(); // (backup, dest)
    let mut promoted: Vec<&StagedWrite> = Vec::new();
    let commit_err = 'commit: {
        for sw in &staged {
            if sw.pre_existed {
                let backup = sw.dest.with_extension(format!(
                    "{}.bak.{pid}",
                    sw.dest.extension().and_then(|e| e.to_str()).unwrap_or("rs")
                ));
                if let Err(source) = fs::rename(&sw.dest, &backup) {
                    break 'commit Some(io_err(
                        "back up existing generated module",
                        &sw.dest,
                        source,
                    ));
                }
                backed_up.push((backup, sw.dest.clone()));
            }
        }
        for sw in &staged {
            if let Err(source) = fs::rename(&sw.temp, &sw.dest) {
                break 'commit Some(io_err("promote generated module", &sw.dest, source));
            }
            promoted.push(sw);
        }
        None
    };

    if let Some(err) = commit_err {
        // Restore every destination we backed up.
        for (backup, dest) in &backed_up {
            let _ = fs::rename(backup, dest);
        }
        // A promoted file that had NO backup did not exist before this call
        // — remove it so a failed generation leaves no partial output.
        let backed_up_dests: std::collections::HashSet<&Path> =
            backed_up.iter().map(|(_, d)| d.as_path()).collect();
        for sw in &promoted {
            if !backed_up_dests.contains(sw.dest.as_path()) {
                let _ = fs::remove_file(&sw.dest);
            }
        }
        remove_staged_temps(&staged);
        return Err(err);
    }

    // Success: drop backups and report warnings only after commit.
    for (backup, _) in &backed_up {
        let _ = fs::remove_file(backup);
    }
    for w in modules.warnings() {
        println!("cargo::warning={w}");
    }
    Ok(())
}

fn remove_staged_temps(staged: &[StagedWrite]) {
    for sw in staged {
        let _ = fs::remove_file(&sw.temp);
    }
}

/// Include a module written by [`generate_to_out_dir`] / [`generate_str_to_out_dir`].
///
/// After `generate_to_out_dir(..., GenerationConfig::new("messages"))`:
/// `ergo_sbe::include_sbe!("messages");`
///
/// → [`samples/sbe-feature-tour/build.rs`](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/build.rs)
///
/// Expands to `include!(concat!(env!("OUT_DIR"), "/messages.rs"))`.
#[macro_export]
macro_rules! include_sbe {
    ($module:literal) => {
        include!(concat!(env!("OUT_DIR"), "/", $module, ".rs"));
    };
    ($module:ident) => {
        include!(concat!(env!("OUT_DIR"), "/", stringify!($module), ".rs"));
    };
}

/// Declare a module that includes generated SBE codecs from `OUT_DIR`.
///
/// Applies the usual `allow`s for generated code (snake/camel, unused, …).
///
/// After build.rs generates `$OUT_DIR/messages.rs`:
/// `ergo_sbe::sbe_mod!(messages);` → `mod messages { ... include!(.../messages.rs); }`
/// `ergo_sbe::sbe_mod!(pub codecs);` → public module `codecs` → `codecs.rs`
/// `ergo_sbe::sbe_mod!(pub ergo_car = "car_bench");` → `car_bench.rs`
///
/// → [`samples/sbe-feature-tour/src/lib.rs`](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs)
#[macro_export]
macro_rules! sbe_mod {
    ($name:ident) => {
        mod $name {
            #![allow(
                dead_code,
                unused_imports,
                unused_variables,
                unused_mut,
                unused_assignments,
                unused_must_use,
                unused_comparisons,
                non_camel_case_types,
                non_snake_case,
                unexpected_cfgs,
                unused_unsafe,
                clippy::all,
                clippy::pedantic,
                clippy::nursery,
                clippy::unwrap_used,
                clippy::expect_used,
                clippy::panic
            )]
            include!(concat!(env!("OUT_DIR"), "/", stringify!($name), ".rs"));
        }
    };
    ($vis:vis $name:ident) => {
        $vis mod $name {
            #![allow(
                dead_code,
                unused_imports,
                unused_variables,
                unused_mut,
                unused_assignments,
                unused_must_use,
                unused_comparisons,
                unused_unsafe,
                non_camel_case_types,
                non_snake_case,
                unexpected_cfgs,
                clippy::all,
                clippy::pedantic,
                clippy::nursery,
                clippy::unwrap_used,
                clippy::expect_used,
                clippy::panic
            )]
            include!(concat!(env!("OUT_DIR"), "/", stringify!($name), ".rs"));
        }
    };
    ($name:ident = $file:literal) => {
        mod $name {
            #![allow(
                dead_code,
                unused_imports,
                unused_variables,
                unused_mut,
                unused_assignments,
                unused_must_use,
                unused_comparisons,
                non_camel_case_types,
                non_snake_case,
                unexpected_cfgs,
                clippy::all
            )]
            include!(concat!(env!("OUT_DIR"), "/", $file, ".rs"));
        }
    };
    ($vis:vis $name:ident = $file:literal) => {
        $vis mod $name {
            #![allow(
                dead_code,
                unused_imports,
                unused_variables,
                unused_mut,
                unused_assignments,
                unused_must_use,
                unused_comparisons,
                non_camel_case_types,
                non_snake_case,
                unexpected_cfgs,
                clippy::all
            )]
            include!(concat!(env!("OUT_DIR"), "/", $file, ".rs"));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn minimal_schema() -> &'static str {
        r#"<?xml version="1.0"?>
        <messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
          </types>
          <message name="Ping" id="1">
            <field name="seq" id="1" type="uint32" offset="0"/>
          </message>
        </messageSchema>"#
    }

    /// Proves `ergo_sbe::miette` is publicly re-exported and usable as a
    /// `build.rs` return type without the caller adding a direct `miette`
    /// dependency. If this re-export is ever removed or made private, this
    /// fails to compile.
    #[test]
    fn miette_is_reexported_for_build_rs_return_type() -> crate::miette::Result<()> {
        fn build_rs_main() -> crate::miette::Result<()> {
            Ok(())
        }
        build_rs_main()
    }

    /// The `*_to_out_dir` helpers only work inside a build script. Called
    /// anywhere else they must fail with [`BuildError::MissingOutDir`] rather
    /// than panicking or picking an arbitrary directory — ergo-sbe has no
    /// build script of its own, so `OUT_DIR` is genuinely unset here and the
    /// test needs no environment mutation.
    #[test]
    fn out_dir_helpers_report_missing_out_dir_outside_a_build_script() {
        assert!(
            env::var_os("OUT_DIR").is_none(),
            "ergo-sbe has no build script; OUT_DIR must be unset for this test"
        );
        assert!(matches!(out_dir(), Err(BuildError::MissingOutDir)));
        assert!(matches!(
            generate_str_to_out_dir(minimal_schema(), GenerationConfig::new("ping")),
            Err(BuildError::MissingOutDir)
        ));
        assert!(matches!(
            generate_to_out_dir("schemas/does-not-matter.xml", GenerationConfig::new("ping")),
            Err(BuildError::MissingOutDir)
        ));
    }

    #[test]
    fn generate_str_to_dir_writes_module() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        let set = generate_str_to_dir(minimal_schema(), GenerationConfig::new("ping"), &dir)?;
        assert_eq!(set.modules().len(), 1);
        let path = dir.join("ping.rs");
        assert!(path.is_file(), "expected {}", path.display());
        let src = fs::read_to_string(&path)?;
        assert!(src.contains("PingEncoder"), "{src}");
        assert!(src.contains("PingDecoder"), "{src}");
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn generate_to_dir_reads_schema_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        let schema_path = dir.join("messages.xml");
        fs::write(&schema_path, minimal_schema())?;

        let explicit = dir.join("explicit");
        let set = generate_to_dir(&schema_path, GenerationConfig::new("from_file"), &explicit)?;
        assert_eq!(set.modules().len(), 1);
        assert!(explicit.join("from_file.rs").is_file());

        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn schema_watch_paths_are_root_then_sorted_unique_includes()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        let root = dir.join("root.xml");
        let leaf = dir.join("leaf.xml");
        let mid = dir.join("mid.xml");
        fs::write(&root, "root")?;
        fs::write(&leaf, "leaf")?;
        fs::write(&mid, "mid")?;
        let root_canon = root.canonicalize()?;
        let leaf_canon = leaf.canonicalize()?;
        let mid_canon = mid.canonicalize()?;
        let watched = schema_watch_paths(
            &root,
            &[
                mid_canon.clone(),
                leaf_canon.clone(),
                root_canon.clone(),
                leaf_canon.clone(),
            ],
        );
        assert_eq!(watched.first(), Some(&root));
        assert_eq!(watched.len(), 3, "{watched:?}");
        let rest: Vec<_> = watched.iter().skip(1).cloned().collect();
        let mut expected_rest = vec![leaf_canon, mid_canon];
        expected_rest.sort();
        assert_eq!(rest, expected_rest);
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn generate_to_dir_rebuilds_after_include_only_edit() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile_dir()?;
        let types = dir.join("types.xml");
        fs::write(
            &types,
            r#"<?xml version="1.0"?>
<types>
  <composite name="messageHeader">
    <type name="blockLength" primitiveType="uint16"/>
    <type name="templateId" primitiveType="uint16"/>
    <type name="schemaId" primitiveType="uint16"/>
    <type name="version" primitiveType="uint16"/>
  </composite>
  <type name="Seq" primitiveType="uint32"/>
</types>
"#,
        )?;
        let schema_path = dir.join("root.xml");
        fs::write(
            &schema_path,
            r#"<?xml version="1.0"?>
<messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
  <include href="types.xml"/>
  <types/>
  <message name="Ping" id="1">
    <field name="seq" id="1" type="Seq"/>
  </message>
</messageSchema>
"#,
        )?;
        let parsed = crate::xml::parse_file_with_deps(&schema_path)?;
        let watched = schema_watch_paths(&schema_path, &parsed.dependencies);
        assert!(
            watched.iter().any(|p| p.file_name() == types.file_name()),
            "include must be watched: {watched:?}"
        );

        let out = dir.join("out");
        generate_to_dir(&schema_path, GenerationConfig::new("ping"), &out)?;
        let first = fs::read_to_string(out.join("ping.rs"))?;
        assert!(
            first.contains("u32") || first.contains("uint32"),
            "first generate must encode Seq as uint32:\n{first}"
        );

        fs::write(
            &types,
            r#"<?xml version="1.0"?>
<types>
  <composite name="messageHeader">
    <type name="blockLength" primitiveType="uint16"/>
    <type name="templateId" primitiveType="uint16"/>
    <type name="schemaId" primitiveType="uint16"/>
    <type name="version" primitiveType="uint16"/>
  </composite>
  <type name="Seq" primitiveType="uint64"/>
</types>
"#,
        )?;
        generate_to_dir(&schema_path, GenerationConfig::new("ping"), &out)?;
        let second = fs::read_to_string(out.join("ping.rs"))?;
        assert!(
            second.contains("u64") || second.contains("uint64"),
            "include-only edit must regenerate Seq as uint64:\n{second}"
        );
        assert_ne!(first, second, "generated source must change");
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    fn write_multi_schemas(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(
            dir.join("common-types.xml"),
            r#"<?xml version="1.0"?>
<messageSchema package="common" id="0" version="1" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="Price">
      <type name="mantissa" primitiveType="int64"/>
      <type name="exponent" primitiveType="int8"/>
    </composite>
  </types>
</messageSchema>
"#,
        )?;
        fs::write(
            dir.join("orders.xml"),
            r#"<?xml version="1.0"?>
<messageSchema package="orders" id="1" version="1" byteOrder="littleEndian">
  <message name="NewOrder" id="1">
    <field name="price" id="1" type="Price"/>
  </message>
</messageSchema>
"#,
        )?;
        Ok(())
    }

    #[test]
    fn generate_multi_to_dir_uses_supplied_module_names() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile_dir()?;
        write_multi_schemas(&dir)?;
        let out = dir.join("out");
        let set = generate_multi_to_dir(
            SchemaFile::new(&dir.join("common-types.xml"), "common_types"),
            &[SchemaFile::new(&dir.join("orders.xml"), "orders")],
            GenerationConfig::new("common_types"),
            &out,
        )?;
        let names: Vec<_> = set.modules().map(|m| m.path.as_str()).collect();
        assert_eq!(names, ["common_types.rs", "orders.rs"]);
        assert!(out.join("common_types.rs").is_file());
        assert!(out.join("orders.rs").is_file());
        let orders = fs::read_to_string(out.join("orders.rs"))?;
        assert!(
            orders.contains("price") || orders.contains("NewOrder"),
            "shared Price must resolve in the consumer"
        );
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn generate_multi_rejects_mismatched_shared_module() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        write_multi_schemas(&dir)?;
        let err = generate_multi_to_dir(
            SchemaFile::new(&dir.join("common-types.xml"), "common_types"),
            &[SchemaFile::new(&dir.join("orders.xml"), "orders")],
            GenerationConfig::new("common_types").with_shared_module("other"),
            dir.join("out"),
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("shared_module") || msg.contains("other"),
            "{msg}"
        );
        assert!(!dir.join("out/common_types.rs").exists());
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn generate_multi_rejects_duplicate_module_names() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        write_multi_schemas(&dir)?;
        let err = generate_multi_to_dir(
            SchemaFile::new(&dir.join("common-types.xml"), "dup"),
            &[SchemaFile::new(&dir.join("orders.xml"), "dup")],
            GenerationConfig::new("dup"),
            dir.join("out"),
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("duplicate") || msg.contains("dup"), "{msg}");
        assert!(!dir.join("out/dup.rs").exists());
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn generate_multi_late_consumer_failure_writes_nothing()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        write_multi_schemas(&dir)?;
        fs::write(
            dir.join("bad.xml"),
            r#"<?xml version="1.0"?>
<messageSchema package="bad" id="2" version="1" byteOrder="littleEndian">
  <message name="Bad" id="1">
    <field name="x" id="1" type="NotAType"/>
  </message>
</messageSchema>
"#,
        )?;
        let out = dir.join("out");
        let err = generate_multi_to_dir(
            SchemaFile::new(&dir.join("common-types.xml"), "common_types"),
            &[
                SchemaFile::new(&dir.join("orders.xml"), "orders"),
                SchemaFile::new(&dir.join("bad.xml"), "bad"),
            ],
            GenerationConfig::new("common_types"),
            &out,
        )
        .unwrap_err();
        assert!(matches!(err, BuildError::Parse(_)), "{err:?}");
        assert!(
            !out.join("common_types.rs").exists() && !out.join("orders.rs").exists(),
            "late consumer failure must not write earlier modules"
        );
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    /// First generation into an empty directory: if staging the second
    /// module fails, the first module's temp file must be cleaned up and
    /// neither destination must exist — a fresh generation either lands
    /// completely or not at all.
    #[test]
    fn generate_multi_staging_failure_rolls_back_to_no_outputs()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        write_multi_schemas(&dir)?;
        let out = dir.join("out");
        fs::create_dir_all(&out)?;

        // Block the second module's ("orders") staging temp file with a
        // pre-existing directory at that exact path — the same naming
        // scheme write_module_set uses (same process, so the same pid).
        let pid = std::process::id();
        let orders_temp = out.join(format!("orders.rs.tmp.{pid}.1"));
        fs::create_dir_all(&orders_temp)?;

        let err = generate_multi_to_dir(
            SchemaFile::new(&dir.join("common-types.xml"), "common_types"),
            &[SchemaFile::new(&dir.join("orders.xml"), "orders")],
            GenerationConfig::new("common_types"),
            &out,
        )
        .unwrap_err();
        assert!(matches!(err, BuildError::Io { .. }), "{err:?}");
        assert!(
            !out.join("common_types.rs").exists(),
            "staging failure on module two must not promote module one either"
        );
        // Any `*.tmp.*` file besides the directory this test pre-planted
        // (write_module_set only ever removes files it staged itself, so
        // the pre-planted directory is expected to remain) means module
        // one's temp file was left behind instead of being cleaned up.
        let stray_temps: Vec<_> = fs::read_dir(&out)?
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp.") && *n != format!("orders.rs.tmp.{pid}.1"))
            .collect();
        assert!(
            stray_temps.is_empty(),
            "module one's temp file must be cleaned up on rollback: {stray_temps:?}"
        );
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    /// Regenerating over an existing output set: if the commit phase fails
    /// partway (backing up module two's destination), every pre-existing
    /// destination must come back byte-identical and no backup/temp debris
    /// must remain — a failed regeneration must never leave a mixture of the
    /// old and new generated graphs.
    #[test]
    fn generate_multi_commit_failure_restores_pre_existing_outputs()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        write_multi_schemas(&dir)?;
        let out = dir.join("out");
        fs::create_dir_all(&out)?;

        // Simulate a prior successful generation already on disk.
        let original_common = b"// original common_types.rs\n".to_vec();
        let original_orders = b"// original orders.rs\n".to_vec();
        fs::write(out.join("common_types.rs"), &original_common)?;
        fs::write(out.join("orders.rs"), &original_orders)?;

        // Block module two's backup destination with a non-empty directory —
        // `fs::rename` onto that fails, forcing a commit-phase error after
        // staging has already succeeded for both modules.
        let pid = std::process::id();
        let orders_backup = out.join(format!("orders.rs.bak.{pid}"));
        fs::create_dir_all(&orders_backup)?;
        fs::write(orders_backup.join("occupied"), b"x")?;

        let err = generate_multi_to_dir(
            SchemaFile::new(&dir.join("common-types.xml"), "common_types"),
            &[SchemaFile::new(&dir.join("orders.xml"), "orders")],
            GenerationConfig::new("common_types"),
            &out,
        )
        .unwrap_err();
        assert!(matches!(err, BuildError::Io { .. }), "{err:?}");

        assert_eq!(
            fs::read(out.join("common_types.rs"))?,
            original_common,
            "module one must be restored byte-identical after rollback"
        );
        assert_eq!(
            fs::read(out.join("orders.rs"))?,
            original_orders,
            "module two must be untouched — its backup step never completed"
        );
        let debris: Vec<_> = fs::read_dir(&out)?
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp.") || (n.contains(".bak.") && !n.contains("occupied")))
            .filter(|n| n != &format!("orders.rs.bak.{pid}"))
            .collect();
        assert!(
            debris.is_empty(),
            "no leftover temp/backup files: {debris:?}"
        );
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    /// `BuildError::Io` must name the exact action and path attempted, and
    /// keep the original `std::io::Error` reachable through `source()` — not
    /// just a flattened "I/O error: ..." string with no diagnostic context.
    #[test]
    fn build_error_io_display_and_source_preserve_action_and_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        let schema_path = dir.join("messages.xml");
        fs::write(&schema_path, minimal_schema())?;

        // A regular file where the output directory should be — creating a
        // directory there fails with a real `std::io::Error`.
        let blocked = dir.join("blocked");
        fs::write(&blocked, b"not a directory")?;

        let err =
            generate_to_dir(&schema_path, GenerationConfig::new("blocked"), &blocked).unwrap_err();
        match &err {
            BuildError::Io { action, path, .. } => {
                assert_eq!(*action, "create output directory");
                assert_eq!(path, &blocked);
            }
            other => unreachable!("expected BuildError::Io, got {other:?}"),
        }
        let msg = err.to_string();
        assert!(msg.contains("create output directory"), "{msg}");
        assert!(msg.contains(&blocked.display().to_string()), "{msg}");
        assert!(
            std::error::Error::source(&err).is_some(),
            "BuildError::Io must expose the original io::Error via source()"
        );

        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn generate_multi_watches_transitive_includes() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        let nested = dir.join("nested");
        fs::create_dir_all(&nested)?;
        fs::write(
            dir.join("leaf.xml"),
            r#"<?xml version="1.0"?>
<types>
  <composite name="Price">
    <type name="mantissa" primitiveType="int64"/>
    <type name="exponent" primitiveType="int8"/>
  </composite>
</types>
"#,
        )?;
        fs::write(
            nested.join("mid.xml"),
            r#"<?xml version="1.0"?>
<messageSchema package="mid" id="9" version="0">
  <include href="../leaf.xml"/>
  <types/>
</messageSchema>
"#,
        )?;
        fs::write(
            dir.join("common-types.xml"),
            r#"<?xml version="1.0"?>
<messageSchema package="common" id="0" version="1" byteOrder="littleEndian">
  <include href="nested/mid.xml"/>
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
</messageSchema>
"#,
        )?;
        fs::write(
            dir.join("orders.xml"),
            r#"<?xml version="1.0"?>
<messageSchema package="orders" id="1" version="1" byteOrder="littleEndian">
  <message name="NewOrder" id="1">
    <field name="price" id="1" type="Price"/>
  </message>
</messageSchema>
"#,
        )?;
        let parsed = crate::xml::parse_file_with_deps(dir.join("common-types.xml"))?;
        let names: Vec<String> = parsed
            .dependencies
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"leaf.xml".into()), "{names:?}");
        let out = dir.join("out");
        generate_multi_to_dir(
            SchemaFile::new(&dir.join("common-types.xml"), "common_types"),
            &[SchemaFile::new(&dir.join("orders.xml"), "orders")],
            GenerationConfig::new("common_types"),
            &out,
        )?;
        assert!(out.join("orders.rs").is_file());
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    /// Proves `BuildError::Parse` forwards the inner `ParseError`'s source +
    /// span through `#[diagnostic(transparent)]` — the wrapped error still
    /// renders a real snippet, not just the outer `{}`/`{:?}` message. This is
    /// what a `build.rs` returning `miette::Result<()>` actually shows on a
    /// malformed schema, instead of the raw `Debug` dump you get from
    /// `Box<dyn std::error::Error>`.
    #[test]
    fn build_error_parse_variant_renders_source_snippet_via_miette()
    -> Result<(), Box<dyn std::error::Error>> {
        let bad_xml = r#"<messageSchema package="x" id="1" version="0">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <message name="M" id="1"><field name="f" id="1" type="bogus"/></message>
</messageSchema>"#;

        let dir = tempfile_dir()?;
        let err = generate_str_to_dir(bad_xml, GenerationConfig::new("bad"), &dir).unwrap_err();
        let _ = fs::remove_dir_all(&dir);

        assert!(
            matches!(err, BuildError::Parse(_)),
            "expected BuildError::Parse, got {err:?}"
        );

        let mut rendered = String::new();
        miette::GraphicalReportHandler::new_themed(miette::GraphicalTheme::unicode_nocolor())
            .render_report(&mut rendered, &err)?;

        assert!(rendered.contains("bogus"), "rendered:\n{rendered}");
        assert!(
            rendered.lines().count() > 1,
            "expected a multi-line snippet through the transparent wrapper, got:\n{rendered}"
        );

        Ok(())
    }

    fn tempfile_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = env::temp_dir().join(format!(
            "ergo_sbe_build_test_{}_{}_{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }
}
