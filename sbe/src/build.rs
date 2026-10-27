//! The generate front door: [`generate`], then write the result.
//!
//! ```rust,no_run
//! // build.rs — ergo_sbe::miette::Result renders schema errors with a source
//! // snippet by default; Box<dyn std::error::Error> prints a raw Debug dump.
//! fn main() -> ergo_sbe::miette::Result<()> {
//!     ergo_sbe::generate(
//!         ergo_sbe::SchemaSource::file("schemas/messages.xml"),
//!         ergo_sbe::GenerationConfig::new("messages"),
//!     )?
//!     .write_to_out_dir()?;
//!     Ok(())
//! }
//! ```
//!
//! Then include the generated module from `lib.rs` or `main.rs`:
//!
//! ```text
//! ergo_sbe::sbe_mod!(messages);
//! ```
//!
//! | Step | Call |
//! |---|---|
//! | Parse + generate one schema | [`generate`]`(source, config)` |
//! | Write into Cargo's `OUT_DIR` | [`GeneratedModuleSet::write_to_out_dir`] |
//! | Write into a directory (e.g. `src/generated` for IDE navigation) | [`GeneratedModuleSet::write_to`] |
//! | Several schemas sharing types | [`crate::parse`] each (consumers [`with_shared`](SchemaSource::with_shared)), [`Generator::generate_multi`], then write |

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::codegen::{GenerateError, GeneratedModuleSet, Generator};
use crate::config::GenerationConfig;
use crate::schema::Schema;
use crate::xml::{ParseError, SchemaSource, parse};

/// Errors from [`generate`] and the [`GeneratedModuleSet`] writers.
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
    /// `OUT_DIR` is unset: [`GeneratedModuleSet::write_to_out_dir`] is for
    /// Cargo `build.rs` only.
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

/// Parse one schema and generate its codec module in memory.
///
/// `config.module_name` names the module (`"messages"` → `messages.rs`).
/// Nothing is written: call [`GeneratedModuleSet::write_to_out_dir`] from a
/// `build.rs`, [`GeneratedModuleSet::write_to`] for a fixed directory, or
/// read [`GeneratedModuleSet::modules`] directly.
///
/// Same as [`parse`] → [`Schema::from_ir`] → [`Generator::generate`].
///
/// # Errors
///
/// [`BuildError::Parse`] or [`BuildError::Generate`].
///
/// # Example
///
/// ```rust
/// use ergo_sbe::{GenerationConfig, SchemaSource, generate};
///
/// # fn main() -> Result<(), ergo_sbe::BuildError> {
/// let xml = r#"<messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
///   <types><composite name="messageHeader">
///     <type name="blockLength" primitiveType="uint16"/>
///     <type name="templateId" primitiveType="uint16"/>
///     <type name="schemaId" primitiveType="uint16"/>
///     <type name="version" primitiveType="uint16"/>
///   </composite></types>
///   <message name="Ping" id="1"><field name="seq" id="1" type="uint32"/></message>
/// </messageSchema>"#;
/// let set = generate(SchemaSource::xml(xml), GenerationConfig::new("ping"))?;
/// let module = set.modules().next().ok_or(ergo_sbe::BuildError::Empty)?;
/// assert_eq!(module.path, "ping.rs");
/// assert!(module.source.contains("PingEncoder"));
/// # Ok(())
/// # }
/// ```
pub fn generate<'a>(
    source: impl Into<SchemaSource<'a>>,
    config: GenerationConfig,
) -> Result<GeneratedModuleSet, BuildError> {
    let schema = Schema::from_ir(parse(source)?);
    Ok(Generator::new(config).generate(&schema)?)
}

impl GeneratedModuleSet {
    /// Write every module into Cargo's `OUT_DIR` (see [`Self::write_to`]).
    ///
    /// # Errors
    ///
    /// [`BuildError::MissingOutDir`] outside a build script, else as
    /// [`Self::write_to`].
    pub fn write_to_out_dir(&self) -> Result<(), BuildError> {
        let out = env::var_os("OUT_DIR").ok_or(BuildError::MissingOutDir)?;
        self.write_to(PathBuf::from(out))
    }

    /// Write every module into `dir`, all or none.
    ///
    /// Prints `cargo::rerun-if-changed` for every [`Self::sources`] file and
    /// `cargo::warning` for every [`Self::warnings`] entry. A directory outside
    /// `OUT_DIR` (e.g. a gitignored `src/generated/` so rust-analyzer can jump
    /// into real files) also gets a `cargo::warning` naming it. Do not commit
    /// those files: they are large and change with the generator.
    ///
    /// # Errors
    ///
    /// [`BuildError::Empty`] for an empty set, [`BuildError::Generate`] for a
    /// module path with directory components, or [`BuildError::Io`] naming the
    /// failed step and path. A failed write leaves the previous outputs in place.
    pub fn write_to(&self, dir: impl AsRef<Path>) -> Result<(), BuildError> {
        let dir = dir.as_ref();
        write_module_set(self, dir)?;
        for path in self.sources() {
            println!("cargo::rerun-if-changed={}", path.display());
        }
        // Point at stable IDE paths (e.g. src/generated). Skip for hashed
        // OUT_DIR, which every product and sample build uses.
        let is_cargo_out = env::var_os("OUT_DIR").is_some_and(|od| dir.starts_with(Path::new(&od)));
        if !is_cargo_out {
            println!(
                "cargo::warning=ergo-sbe wrote {} module(s) under {} (open for go-to-definition)",
                self.modules().len(),
                dir.display()
            );
        }
        Ok(())
    }
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
/// A cleanup step that itself fails is reported in preference to the failure
/// that triggered it: silently swallowing it would claim a rollback that left
/// debris behind.
///
/// The final step — deleting the backups of a *successful* commit — is past
/// the point of no return and does not roll back: the new outputs are
/// installed and correct. It removes every backup it can and then fails the
/// publication naming the first it could not, so success is never reported
/// with stray `.bak` files on disk.
///
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
            remove_staged_temps(&staged)?;
            return Err(io_err("create output directory", parent, source));
        }
        if let Err(source) = fs::write(&temp, &m.source) {
            remove_staged_temps(&staged)?;
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
        // Restore every destination we backed up, then remove promotions that
        // had no backup and every staged temp. Prefer a rollback error over
        // the original commit error: a half-applied graph or leftover debris
        // is worse than a failed promote, and the rollback error names the
        // exact path still needing attention.
        let rollback_err = first_err(
            [
                restore_backed_up(&backed_up),
                remove_unbacked_promotions(&promoted, &backed_up),
                remove_staged_temps(&staged),
            ]
            .into_iter(),
        );
        return Err(rollback_err.err().unwrap_or(err));
    }

    // Success: drop backups. The new outputs are installed and correct, so
    // there is nothing to roll back — but a leftover `.bak` is debris. Remove
    // every backup we can, then fail the publication naming the first we
    // could not, rather than report success with stray files.
    let mut backup_err = None;
    for (backup, _) in &backed_up {
        if let Err(err) = remove_if_present(backup, "remove generated module backup") {
            println!("cargo::warning={err}");
            backup_err = backup_err.or(Some(err));
        }
    }
    if let Some(err) = backup_err {
        return Err(err);
    }
    for w in modules.warnings() {
        println!("cargo::warning={w}");
    }
    Ok(())
}

/// Remove `path`, treating an already-absent file as success. A staged temp
/// that was promoted, or a destination the commit never reached, is simply not
/// there; any other error is real debris and must not be hidden behind a
/// rollback that reports itself as clean.
fn remove_if_present(path: &Path, action: &'static str) -> Result<(), BuildError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_err(action, path, source)),
    }
}

fn remove_staged_temps(staged: &[StagedWrite]) -> Result<(), BuildError> {
    first_err(
        staged.iter().map(|sw| {
            remove_if_present(&sw.temp, "remove staged generated module during rollback")
        }),
    )
}

/// A promoted file that had NO backup did not exist before this call — remove
/// it so a failed generation leaves no partial output.
fn remove_unbacked_promotions(
    promoted: &[&StagedWrite],
    backed_up: &[(PathBuf, PathBuf)],
) -> Result<(), BuildError> {
    let backed_up_dests: std::collections::HashSet<&Path> =
        backed_up.iter().map(|(_, d)| d.as_path()).collect();
    first_err(
        promoted
            .iter()
            .filter(|sw| !backed_up_dests.contains(sw.dest.as_path()))
            .map(|sw| {
                remove_if_present(&sw.dest, "remove promoted generated module during rollback")
            }),
    )
}

/// Drain `results` and report the first failure. Deliberately *not*
/// `try_fold`: short-circuiting would strand the rest of the debris behind
/// one unremovable path.
fn first_err(results: impl Iterator<Item = Result<(), BuildError>>) -> Result<(), BuildError> {
    let mut first = None;
    for r in results {
        if let Err(e) = r {
            first = first.or(Some(e));
        }
    }
    first.map_or(Ok(()), Err)
}

/// Put `backup` back at `dest`, replacing an already-promoted file.
/// Windows `rename` will not replace an existing destination, so delete first.
fn restore_destination(backup: &Path, dest: &Path) -> Result<(), BuildError> {
    if dest.exists() {
        if dest.is_dir() {
            return Err(io_err(
                "restore generated module",
                dest,
                std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "destination is a directory",
                ),
            ));
        }
        fs::remove_file(dest).map_err(|source| {
            io_err(
                "remove promoted generated module during restore",
                dest,
                source,
            )
        })?;
    }
    fs::rename(backup, dest).map_err(|source| io_err("restore generated module", dest, source))?;
    Ok(())
}

fn restore_backed_up(backed_up: &[(PathBuf, PathBuf)]) -> Result<(), BuildError> {
    first_err(
        backed_up
            .iter()
            .map(|(backup, dest)| restore_destination(backup, dest)),
    )
}

/// Include a module written by [`GeneratedModuleSet::write_to_out_dir`].
///
/// After `generate(source, GenerationConfig::new("messages"))?.write_to_out_dir()?`:
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

    /// [`GeneratedModuleSet::write_to_out_dir`] only works inside a build
    /// script. Called anywhere else it must fail with
    /// [`BuildError::MissingOutDir`] rather than panicking or picking an
    /// arbitrary directory — ergo-sbe has no build script of its own, so
    /// `OUT_DIR` is genuinely unset here and the test needs no environment
    /// mutation.
    #[test]
    fn write_to_out_dir_reports_missing_out_dir_outside_a_build_script()
    -> Result<(), Box<dyn std::error::Error>> {
        assert!(
            env::var_os("OUT_DIR").is_none(),
            "ergo-sbe has no build script; OUT_DIR must be unset for this test"
        );
        let set = generate(
            SchemaSource::xml(minimal_schema()),
            GenerationConfig::new("ping"),
        )?;
        assert!(matches!(
            set.write_to_out_dir(),
            Err(BuildError::MissingOutDir)
        ));
        Ok(())
    }

    #[test]
    fn generate_from_xml_writes_module() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        let set = generate(
            SchemaSource::xml(minimal_schema()),
            GenerationConfig::new("ping"),
        )?;
        assert!(set.sources().is_empty(), "inline XML has no source files");
        set.write_to(&dir)?;
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
    fn generate_from_file_writes_module_and_lists_the_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        let schema_path = dir.join("messages.xml");
        fs::write(&schema_path, minimal_schema())?;

        let explicit = dir.join("explicit");
        let set = generate(&schema_path, GenerationConfig::new("from_file"))?;
        set.write_to(&explicit)?;
        assert_eq!(set.modules().len(), 1);
        assert!(explicit.join("from_file.rs").is_file());
        assert_eq!(set.sources(), [schema_path.canonicalize()?]);

        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    /// A consumer parsed with a shared schema generates on its own: the
    /// shared types are copied in, and the shared file is watched too.
    #[test]
    fn generate_consumer_with_shared_is_self_contained() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        write_multi_schemas(&dir)?;
        let common = parse(dir.join("common-types.xml"))?;
        let set = generate(
            SchemaSource::file(dir.join("orders.xml")).with_shared(&common),
            GenerationConfig::new("orders"),
        )?;
        let src = &set.modules().next().ok_or("one module")?.source;
        assert!(src.contains("pub struct Price"), "{src}");
        assert!(src.contains("NewOrderEncoder"), "{src}");
        let names: Vec<_> = set
            .sources()
            .iter()
            .filter_map(|p| p.file_name()?.to_str())
            .collect();
        assert_eq!(names, ["orders.xml", "common-types.xml"]);
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn generate_rebuilds_after_include_only_edit() -> Result<(), Box<dyn std::error::Error>> {
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
        let watched = parse(&schema_path)?.sources;
        assert!(
            watched.iter().any(|p| p.file_name() == types.file_name()),
            "include must be watched: {watched:?}"
        );

        let out = dir.join("out");
        generate(&schema_path, GenerationConfig::new("ping"))?.write_to(&out)?;
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
        generate(&schema_path, GenerationConfig::new("ping"))?.write_to(&out)?;
        let second = fs::read_to_string(out.join("ping.rs"))?;
        assert!(
            second.contains("u64") || second.contains("uint64"),
            "include-only edit must regenerate Seq as uint64:\n{second}"
        );
        assert_ne!(first, second, "generated source must change");
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    /// Parse `common` then each consumer against it, and generate the set.
    fn generate_set(
        common: &Path,
        common_module: &str,
        consumers: &[(&Path, &str)],
        config: GenerationConfig,
    ) -> Result<GeneratedModuleSet, BuildError> {
        let shared = parse(common)?;
        let mut parsed = Vec::with_capacity(consumers.len());
        for (path, module) in consumers {
            let ir = parse(SchemaSource::file(path).with_shared(&shared))?;
            parsed.push((Schema::from_ir(ir), *module));
        }
        let shared = Schema::from_ir(shared);
        let mut schemas = vec![(&shared, common_module)];
        schemas.extend(parsed.iter().map(|(s, m)| (s, *m)));
        Ok(Generator::new(config).generate_multi(&schemas)?)
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
    fn generate_multi_uses_supplied_module_names() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        write_multi_schemas(&dir)?;
        let out = dir.join("out");
        let set = generate_set(
            &dir.join("common-types.xml"),
            "common_types",
            &[(&dir.join("orders.xml"), "orders")],
            GenerationConfig::new("common_types").with_shared_module("common_types"),
        )
        .and_then(|set| set.write_to(&out).map(|()| set))?;
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
        let err = generate_set(
            &dir.join("common-types.xml"),
            "common_types",
            &[(&dir.join("orders.xml"), "orders")],
            GenerationConfig::new("common_types").with_shared_module("other"),
        )
        .and_then(|set| set.write_to(dir.join("out")).map(|()| set))
        .unwrap_err();
        assert!(
            matches!(
                &err,
                BuildError::Generate(GenerateError::InvalidConfiguration { option, value, .. })
                    if option == "shared_module" && value == "other"
            ),
            "{err:?}"
        );
        assert!(!dir.join("out/common_types.rs").exists());
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn generate_multi_rejects_duplicate_module_names() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        write_multi_schemas(&dir)?;
        let err = generate_set(
            &dir.join("common-types.xml"),
            "dup",
            &[(&dir.join("orders.xml"), "dup")],
            GenerationConfig::new("dup").with_shared_module("dup"),
        )
        .and_then(|set| set.write_to(dir.join("out")).map(|()| set))
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
        let err = generate_set(
            &dir.join("common-types.xml"),
            "common_types",
            &[
                (&dir.join("orders.xml"), "orders"),
                (&dir.join("bad.xml"), "bad"),
            ],
            GenerationConfig::new("common_types").with_shared_module("common_types"),
        )
        .and_then(|set| set.write_to(&out).map(|()| set))
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

        let err = generate_set(
            &dir.join("common-types.xml"),
            "common_types",
            &[(&dir.join("orders.xml"), "orders")],
            GenerationConfig::new("common_types").with_shared_module("common_types"),
        )
        .and_then(|set| set.write_to(&out).map(|()| set))
        .unwrap_err();
        match &err {
            BuildError::Io { action, path, .. } => {
                assert_eq!(*action, "write generated module");
                assert!(
                    path.to_string_lossy().contains("orders.rs.tmp."),
                    "path must name the exact staging file that failed: {}",
                    path.display()
                );
            }
            other => unreachable!("expected BuildError::Io, got {other:?}"),
        }
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

        let err = generate_set(
            &dir.join("common-types.xml"),
            "common_types",
            &[(&dir.join("orders.xml"), "orders")],
            GenerationConfig::new("common_types").with_shared_module("common_types"),
        )
        .and_then(|set| set.write_to(&out).map(|()| set))
        .unwrap_err();
        match &err {
            BuildError::Io { action, path, .. } => {
                assert_eq!(*action, "back up existing generated module");
                assert_eq!(path, &out.join("orders.rs"));
            }
            other => unreachable!("expected BuildError::Io, got {other:?}"),
        }

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

    /// The commit succeeds, but deleting one backup afterwards fails. The new
    /// outputs are installed and correct, so there is nothing to roll back —
    /// but the publication must still fail naming the backup it could not
    /// remove, and must have removed every *other* backup rather than
    /// abandoning the tail on the first error.
    #[test]
    fn generate_multi_backup_cleanup_failure_is_reported_not_swallowed()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        write_multi_schemas(&dir)?;
        let out = dir.join("out");
        fs::create_dir_all(&out)?;

        // Module one's destination is a *directory*. It counts as
        // pre-existing, so commit renames it aside to `common_types.rs.bak.
        // {pid}` — which succeeds — and promotes the staged file into the
        // vacated name. Only the final backup delete trips: `remove_file` on
        // a directory is EISDIR/EPERM.
        fs::create_dir_all(out.join("common_types.rs"))?;
        // Module two's destination is an ordinary pre-existing file, so its
        // backup must be deleted normally even though module one's failed.
        fs::write(out.join("orders.rs"), b"// original orders.rs\n")?;

        let pid = std::process::id();
        let err = generate_set(
            &dir.join("common-types.xml"),
            "common_types",
            &[(&dir.join("orders.xml"), "orders")],
            GenerationConfig::new("common_types").with_shared_module("common_types"),
        )
        .and_then(|set| set.write_to(&out).map(|()| set))
        .expect_err("undeletable backup must fail the publication");
        match &err {
            BuildError::Io { action, path, .. } => {
                assert_eq!(*action, "remove generated module backup");
                assert_eq!(path, &out.join(format!("common_types.rs.bak.{pid}")));
            }
            other => unreachable!("expected BuildError::Io, got {other:?}"),
        }
        assert!(
            !out.join(format!("orders.rs.bak.{pid}")).exists(),
            "module two's backup must still be removed — one undeletable \
             backup must not strand the rest of the debris"
        );

        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    /// Cleanup tolerates an already-absent file — a staged temp that was
    /// promoted is legitimately gone — but never swallows a real failure.
    #[test]
    fn remove_if_present_tolerates_missing_but_reports_real_failures()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        remove_if_present(&dir.join("never-existed.rs"), "remove x")?;

        let blocked = dir.join("blocked.rs");
        fs::create_dir_all(&blocked)?;
        match remove_if_present(&blocked, "remove x").expect_err("directory is not removable") {
            BuildError::Io { action, path, .. } => {
                assert_eq!(action, "remove x");
                assert_eq!(path, blocked);
            }
            other => unreachable!("expected Io, got {other:?}"),
        }
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

        let err = generate(&schema_path, GenerationConfig::new("blocked"))?
            .write_to(&blocked)
            .unwrap_err();
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
        let parsed = parse(dir.join("common-types.xml"))?;
        let names: Vec<String> = parsed
            .sources
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"leaf.xml".into()), "{names:?}");
        let out = dir.join("out");
        let set = generate_set(
            &dir.join("common-types.xml"),
            "common_types",
            &[(&dir.join("orders.xml"), "orders")],
            GenerationConfig::new("common_types").with_shared_module("common_types"),
        )
        .and_then(|set| set.write_to(&out).map(|()| set))?;
        assert!(out.join("orders.rs").is_file());
        let watched: Vec<_> = set
            .sources()
            .iter()
            .filter_map(|p| p.file_name()?.to_str())
            .collect();
        for name in ["common-types.xml", "mid.xml", "leaf.xml", "orders.xml"] {
            assert!(
                watched.contains(&name),
                "{name} must be watched: {watched:?}"
            );
        }
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
        let err = generate(SchemaSource::xml(bad_xml), GenerationConfig::new("bad")).unwrap_err();
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

    #[test]
    fn restore_destination_replaces_already_promoted_file() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile_dir()?;
        let dest = dir.join("mod.rs");
        let backup = dir.join("mod.rs.bak");
        fs::write(&dest, b"promoted")?;
        fs::write(&backup, b"original")?;
        restore_destination(&backup, &dest)?;
        assert_eq!(fs::read(&dest)?, b"original");
        assert!(!backup.exists(), "rename consumes the backup");
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn restore_destination_errors_when_dest_is_a_directory()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        let dest = dir.join("mod.rs");
        let backup = dir.join("mod.rs.bak");
        fs::create_dir_all(&dest)?;
        fs::write(&backup, b"original")?;
        let err = restore_destination(&backup, &dest).expect_err("dir dest");
        match err {
            BuildError::Io { action, path, .. } => {
                assert!(
                    action.contains("restore") || action.contains("remove"),
                    "{action}"
                );
                assert_eq!(path, dest);
            }
            other => unreachable!("expected Io, got {other:?}"),
        }
        assert!(backup.exists(), "failed restore must leave the backup");
        fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
