//! One persistent target directory for every throwaway crate a test builds.
//!
//! Each scratch crate used to build into its own fresh `target_ci`, so every
//! call recompiled `ergo-sbe` and its whole dependency tree. Now every scratch
//! crate builds into `<outer target>/ergon-scratch/<key>`, which tests never
//! delete: dependencies compile once per feature set and each test compiles
//! only its own small crate.
//!
//! - Cargo's build-directory lock serialises concurrent builds, across test
//!   threads and across processes.
//! - Every crate gets a package name unique to this process and call
//!   ([`ScratchCrate::new`]), so two tests never run or overwrite each other's
//!   binary, and a compile-fail fixture can never pick up a stale success.
//!   [`Drop`] removes that crate's own artifacts so the directory does not grow.
//! - The workspace `Cargo.lock` is copied in ([`ScratchCrate::write_lockfile`]),
//!   so every crate resolves the same versions and shares their artifacts.
//! - `key` separates builds that strip ambient `RUSTFLAGS` from builds that
//!   keep them, so the two never invalidate each other's artifacts.
//!
//! # Scratch crates compile in parallel ([`ScratchCrate::exec`])
//!
//! Cargo holds the build-directory lock for a whole build, so scratch builds
//! through cargo run one at a time even when the tests run in parallel. On
//! Unix, [`ScratchCrate::exec`] therefore uses cargo only once per process
//! and crate shape: it builds a stub "template" crate with the same manifest,
//! features and target layout, records the exact `rustc` command cargo ran for
//! it (`RUSTC_WORKSPACE_WRAPPER`, so only the template's own crate is
//! recorded, never a dependency), and then compiles each scratch crate by
//! running that command itself, with the crate name and directory swapped,
//! outside the lock. Dependencies, editions, features, profile and
//! `RUSTFLAGS` are exactly cargo's, because they are cargo's own arguments.
//! Each crate's output goes to its own `out/` directory inside its source
//! directory. Anything the template cannot reproduce (a crate with both a lib
//! and a bin, a build script, a template that fails to build) falls back to
//! plain cargo, so a failure always surfaces with cargo's own diagnostics.
//! Set `ERGON_SCRATCH_VIA_CARGO=1` to build every scratch crate through cargo
//! (to compare the two paths).

// Each test binary that includes this file uses its own subset of it.
#![allow(dead_code)]

use std::cell::Cell;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Root of the shared scratch target directories (`<outer target>/ergon-scratch`).
pub fn scratch_root() -> PathBuf {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"));
    tmp.parent().unwrap_or(tmp).join("ergon-scratch")
}

/// The workspace `Cargo.lock` (next to the workspace `Cargo.toml`).
fn workspace_lockfile() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .map(|dir| dir.join("Cargo.lock"))
        .find(|lock| lock.is_file())
}

/// Shared target directory for builds that keep (`strip_rustflags == false`)
/// or clear the ambient `RUSTFLAGS`.
fn shared_target_dir(strip_rustflags: bool) -> PathBuf {
    let ambient: Vec<String> = ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .filter(|v| !v.is_empty())
        .collect();
    if strip_rustflags || ambient.is_empty() {
        return scratch_root().join("default");
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    ambient.hash(&mut h);
    scratch_root().join(format!("rustflags-{:016x}", h.finish()))
}

/// A throwaway crate building into the shared target directory.
pub struct ScratchCrate {
    name: String,
    target: PathBuf,
    strip_rustflags: bool,
    /// Whether cargo ever built this crate into the shared target directory
    /// (only then does [`Drop`] have artifacts there to remove).
    used_cargo: Cell<bool>,
}

/// What [`ScratchCrate::exec`] does with a crate, mirroring a cargo command.
#[derive(Clone, Copy, Debug)]
pub enum Mode<'a> {
    /// `cargo run [--features f]…`: compile the bin, then run it.
    Run(&'a [&'a str]),
    /// `cargo build`: compile the single bin or lib target.
    Build,
    /// `cargo check`: type-check the single bin or lib target.
    Check,
    /// `cargo test -- <args>`: compile the test harness, then run it with `args`.
    /// The direct path runs only the `#[test]` harness, never doctests, so a
    /// crate whose doc examples must run belongs with `cargo test --doc`.
    Test(&'a [&'a str]),
}

impl ScratchCrate {
    /// `base` plus this process id and a per-process counter, so the package
    /// name is unique among every scratch crate building right now.
    pub fn new(base: &str, strip_rustflags: bool) -> Self {
        let clean: String = base
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        Self {
            name: format!("{clean}_{}_{n}", std::process::id()),
            target: shared_target_dir(strip_rustflags),
            strip_rustflags,
            used_cargo: Cell::new(false),
        }
    }

    /// Package name (also the bin/lib crate name: no `-`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Copy the workspace `Cargo.lock` into `dir`, so the crate resolves the
    /// workspace's versions (shared artifacts, no per-crate index update).
    #[allow(clippy::unused_self)] // a method so every call site reads `krate.…`
    pub fn write_lockfile(&self, dir: &Path) {
        if let Some(lock) = workspace_lockfile() {
            let _ = fs::copy(lock, dir.join("Cargo.lock"));
        }
    }

    /// `cargo` building into the shared target directory (callers set the
    /// working directory or `--manifest-path`).
    ///
    /// Clears `CARGO_NET_OFFLINE` (see the `common` module docs) and, when the
    /// crate was created with `strip_rustflags`, the ambient `RUSTFLAGS` so a
    /// fixture's own lint attributes decide the outcome.
    pub fn cargo(&self) -> Command {
        self.used_cargo.set(true);
        let mut cmd = Command::new("cargo");
        cmd.env_remove("CARGO_NET_OFFLINE")
            .env("CARGO_TARGET_DIR", &self.target)
            // One-shot crates: an incremental cache is written and never read.
            .env("CARGO_INCREMENTAL", "0");
        if self.strip_rustflags {
            cmd.env_remove("RUSTFLAGS")
                .env_remove("CARGO_ENCODED_RUSTFLAGS");
        }
        cmd
    }
}

impl Drop for ScratchCrate {
    /// Remove this crate's own artifacts (unique names), never shared ones.
    fn drop(&mut self) {
        if !self.used_cargo.get() {
            return;
        }
        let profiles = ["debug", "release"];
        let lib = format!("lib{}", self.name);
        let prefixes = [format!("{}-", self.name), format!("{lib}-")];
        // macOS unpacked debuginfo: `<hash>-<name tail>-<hash>.<name>.<hash>-cgu.N.rcgu.o`.
        let infix = format!(".{}.", self.name);
        for profile in profiles {
            let p = self.target.join(profile);
            for exact in [
                self.name.clone(),
                format!("{}.d", self.name),
                format!("{}.exe", self.name),
                format!("{}.pdb", self.name),
                format!("{}.dSYM", self.name),
                format!("{lib}.rlib"),
                format!("{lib}.rmeta"),
                format!("{lib}.d"),
            ] {
                remove(&p.join(exact));
            }
            for sub in ["deps", ".fingerprint", "build", "incremental"] {
                let Ok(entries) = fs::read_dir(p.join(sub)) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let file = entry.file_name();
                    let file = file.to_string_lossy();
                    if prefixes.iter().any(|pre| file.starts_with(pre.as_str()))
                        || file.contains(infix.as_str())
                    {
                        remove(&entry.path());
                    }
                }
            }
        }
    }
}

fn remove(path: &Path) {
    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else {
        let _ = fs::remove_file(path);
    }
}

impl ScratchCrate {
    /// Build (and for [`Mode::Run`] / [`Mode::Test`] run) the crate in `dir`,
    /// whose `Cargo.toml` names this crate, as the matching cargo command would.
    ///
    /// Returns the compiler's output when compilation fails, otherwise the
    /// program's output (run and test) or the compiler's (build and check).
    /// Compiles in parallel with other scratch crates; see the module docs.
    pub fn exec(&self, dir: &Path, mode: Mode<'_>) -> std::io::Result<Output> {
        if let Some(template) = self.template(dir, mode) {
            return self.exec_direct(&template, dir, mode);
        }
        self.exec_cargo(dir, mode)
    }

    fn exec_cargo(&self, dir: &Path, mode: Mode<'_>) -> std::io::Result<Output> {
        let mut cmd = self.cargo();
        cmd.current_dir(dir);
        match mode {
            Mode::Run(features) => {
                cmd.arg("run");
                for f in features {
                    cmd.args(["--features", f]);
                }
            }
            Mode::Build => {
                cmd.arg("build");
            }
            Mode::Check => {
                cmd.arg("check");
            }
            Mode::Test(args) => {
                cmd.arg("test").arg("--").args(args);
            }
        }
        cmd.output()
    }

    fn exec_direct(&self, t: &Template, dir: &Path, mode: Mode<'_>) -> std::io::Result<Output> {
        let _slot = Slot::acquire();
        let out_dir = dir.join("out");
        fs::create_dir_all(&out_dir)?;
        let real_dir = fs::canonicalize(dir)?;
        let real_dir = real_dir.to_string_lossy();
        let swap = |s: &str| s.replace(&t.dir, &real_dir).replace(&t.name, &self.name);
        let mut rustc = Command::new(&t.rustc);
        rustc.current_dir(dir);
        let mut args = t.args.iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--out-dir" => {
                    args.next();
                    rustc.arg("--out-dir").arg(&out_dir);
                }
                "-C" => match args.next() {
                    // Symbol/file-name salt only: the output is `out/<name>`.
                    Some(c) if c.starts_with("metadata=") || c.starts_with("extra-filename=") => {}
                    Some(c) => {
                        rustc.arg("-C").arg(swap(c));
                    }
                    None => {
                        rustc.arg("-C");
                    }
                },
                // Cargo renders rustc's JSON diagnostics; rustc's own human
                // output is that same rendered text.
                a if a.starts_with("--error-format=")
                    || a.starts_with("--json=")
                    || a.starts_with("--diagnostic-width=") => {}
                a => {
                    rustc.arg(swap(a));
                }
            }
        }
        for (k, v) in &t.env {
            rustc.env(k, swap(v));
        }
        let built = rustc.output()?;
        if !built.status.success() {
            return Ok(built);
        }
        let run_args: &[&str] = match mode {
            Mode::Run(_) => &[],
            Mode::Test(args) => args,
            Mode::Build | Mode::Check => return Ok(built),
        };
        let mut run = Command::new(out_dir.join(&self.name));
        run.current_dir(dir).args(run_args);
        for (k, v) in &t.env {
            run.env(k, swap(v));
        }
        run.output()
    }

    /// The recorded rustc command for a crate shaped like the one in `dir`,
    /// built once per process and shape. `None` means "use cargo".
    fn template(&self, dir: &Path, mode: Mode<'_>) -> Option<Arc<Template>> {
        if !cfg!(unix) || std::env::var_os("ERGON_SCRATCH_VIA_CARGO").is_some() {
            return None;
        }
        let manifest = fs::read_to_string(dir.join("Cargo.toml")).ok()?;
        let has_main = dir.join("src/main.rs").is_file();
        let has_lib = dir.join("src/lib.rs").is_file();
        // One target only: a lib plus a bin, extra targets or a build script
        // are cargo's to schedule.
        let extra_targets = ["build.rs", "src/bin", "tests", "examples", "benches"];
        if has_main == has_lib || extra_targets.iter().any(|t| dir.join(t).exists()) {
            return None;
        }
        let (cargo_args, wanted): (Vec<String>, Option<&str>) = match mode {
            Mode::Run(features) => {
                if !has_main {
                    return None;
                }
                let mut a = vec!["build".to_owned()];
                for f in features {
                    a.push("--features".to_owned());
                    a.push((*f).to_owned());
                }
                (a, None)
            }
            Mode::Build => (vec!["build".to_owned()], None),
            Mode::Check => (vec!["check".to_owned()], None),
            Mode::Test(_) => (
                vec!["test".to_owned(), "--no-run".to_owned()],
                Some("--test"),
            ),
        };
        let shape = manifest.replace(&self.name, "{name}");
        let key = format!(
            "{shape}\n{cargo_args:?}\nmain={has_main}\nstrip={}",
            self.strip_rustflags
        );
        let slot = {
            static TEMPLATES: OnceLock<Mutex<HashMap<String, Arc<TemplateSlot>>>> = OnceLock::new();
            let map = TEMPLATES.get_or_init(Default::default);
            let mut map = map.lock().unwrap_or_else(PoisonError::into_inner);
            Arc::clone(map.entry(key).or_default())
        };
        slot.get_or_init(|| {
            record_template(&shape, has_main, &cargo_args, wanted, self.strip_rustflags)
                .map(Arc::new)
        })
        .clone()
    }
}

/// A template recorded once per process and crate shape (`None`: use cargo).
type TemplateSlot = OnceLock<Option<Arc<Template>>>;

/// The `rustc` command cargo ran for a template crate.
struct Template {
    /// Template crate name, swapped for each scratch crate's name.
    name: String,
    /// Template source directory, swapped for each scratch crate's directory.
    dir: String,
    rustc: String,
    args: Vec<String>,
    /// `CARGO_*` variables cargo set for rustc (`env!` in the crate reads them).
    env: Vec<(String, String)>,
}

/// Build a stub crate with manifest `shape` (`{name}` = its package name) and
/// record cargo's rustc command for it. `wanted`: pick the invocation that has
/// this argument; otherwise exactly one invocation must have run.
fn record_template(
    shape: &str,
    has_main: bool,
    cargo_args: &[String],
    wanted: Option<&str>,
    strip_rustflags: bool,
) -> Option<Template> {
    let krate = ScratchCrate::new("ergon_scratch_template", strip_rustflags);
    let dir = std::env::temp_dir().join(krate.name());
    let result = (|| {
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).ok()?;
        fs::write(
            dir.join("Cargo.toml"),
            shape.replace("{name}", krate.name()),
        )
        .ok()?;
        let (file, stub) = if has_main {
            ("src/main.rs", "fn main() {}\n")
        } else {
            ("src/lib.rs", "")
        };
        fs::write(dir.join(file), stub).ok()?;
        krate.write_lockfile(&dir);
        let record = dir.join("record");
        fs::create_dir_all(&record).ok()?;
        let out = krate
            .cargo()
            .args(cargo_args)
            .current_dir(&dir)
            .env("RUSTC_WORKSPACE_WRAPPER", recorder()?)
            .env("ERGON_SCRATCH_RECORD", &record)
            .env("ERGON_SCRATCH_TEMPLATE", krate.name())
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let mut calls = Vec::new();
        for entry in fs::read_dir(&record).ok()?.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "args") {
                let raw = fs::read(&path).ok()?;
                let args: Vec<String> = raw
                    .split(|b| *b == 0)
                    .filter(|a| !a.is_empty())
                    .map(|a| String::from_utf8_lossy(a).into_owned())
                    .collect();
                let env = fs::read_to_string(path.with_extension("env")).ok()?;
                calls.push((args, env));
            }
        }
        if let Some(w) = wanted {
            calls.retain(|(args, _)| args.iter().any(|a| a == w));
        }
        if calls.len() != 1 {
            return None;
        }
        let (mut args, env) = calls.pop()?;
        if args.is_empty() {
            return None;
        }
        let rustc = args.remove(0);
        let env = env
            .lines()
            .filter_map(|l| l.split_once('='))
            .filter(|(k, _)| k.starts_with("CARGO_") && *k != "CARGO_MAKEFLAGS")
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        Some(Template {
            name: krate.name().to_owned(),
            // Cargo records the real path (`/private/var/…` on macOS).
            dir: fs::canonicalize(&dir).ok()?.to_string_lossy().into_owned(),
            rustc,
            args,
            env,
        })
    })();
    let _ = fs::remove_dir_all(&dir);
    result
}

/// The `RUSTC_WORKSPACE_WRAPPER` script that records the template's own rustc
/// invocations (NUL-separated arguments and the environment) and then runs them.
fn recorder() -> Option<PathBuf> {
    const SCRIPT: &str = r#"#!/bin/sh
case " $* " in
  *" --crate-name $ERGON_SCRATCH_TEMPLATE "*)
    printf '%s\0' "$@" > "$ERGON_SCRATCH_RECORD/$$.args"
    env > "$ERGON_SCRATCH_RECORD/$$.env"
    ;;
esac
exec "$@"
"#;
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        SCRIPT.hash(&mut h);
        let bin = scratch_root().join("bin");
        fs::create_dir_all(&bin).ok()?;
        let path = bin.join(format!("record-rustc-{:016x}.sh", h.finish()));
        if !path.is_file() {
            // Write, then rename, so no process ever runs a half-written script.
            let tmp = bin.join(format!("record-rustc-{}.tmp", std::process::id()));
            fs::write(&tmp, SCRIPT).ok()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755)).ok()?;
            }
            fs::rename(&tmp, &path).ok()?;
        }
        Some(path)
    })
    .clone()
}

/// One of `available_parallelism` process-wide slots for a direct scratch
/// compile, so parallel tests (and [`parallel_map`] inside a test) never run
/// more compilers at once than the machine has cores.
struct Slot;

static SLOTS: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());

impl Slot {
    fn acquire() -> Self {
        let limit = std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);
        let (lock, cv) = &SLOTS;
        let mut busy = lock.lock().unwrap_or_else(PoisonError::into_inner);
        while *busy >= limit {
            busy = cv.wait(busy).unwrap_or_else(PoisonError::into_inner);
        }
        *busy += 1;
        Self
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        let (lock, cv) = &SLOTS;
        *lock.lock().unwrap_or_else(PoisonError::into_inner) -= 1;
        cv.notify_one();
    }
}

/// `f` over every item on up to `available_parallelism` threads, for a test
/// that checks many independent cases. Results come back in item order. If
/// any call panics, every call still runs and then the panic of the *first*
/// failing item (in item order) is resumed, so the test fails exactly as a
/// one-by-one loop that stops at the first failure would.
pub fn parallel_map<T: Sync, R: Send>(items: &[T], f: impl Fn(usize, &T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<std::thread::Result<R>>>> =
        Mutex::new((0..items.len()).map(|_| None).collect());
    let workers = std::thread::available_parallelism()
        .map_or(4, std::num::NonZeroUsize::get)
        .min(items.len());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(i, item)));
                    results.lock().unwrap_or_else(PoisonError::into_inner)[i] = Some(r);
                }
            });
        }
    });
    results
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner)
        .into_iter()
        .map(|r| match r {
            Some(Ok(v)) => v,
            Some(Err(panic)) => std::panic::resume_unwind(panic),
            None => unreachable!("every item ran"),
        })
        .collect()
}
