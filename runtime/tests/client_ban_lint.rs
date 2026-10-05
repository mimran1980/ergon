//! The client ban must be able to fail: a gate that cannot fail is no gate.
//!
//! `clippy.toml` bans what shares state across threads, starts one, or
//! resolves a name. Each banned path gets a scratch crate that uses exactly
//! it, linted under this crate's `clippy.toml`: each must be rejected for
//! that path, and a crate using `Rc`, `Cell`, `RefCell` and `thread_local!`
//! must pass. The lab's client crates carry the same entries. No client
//! source implements `Send` or `Sync` unsafely, and `assert_not_send!`
//! rejects a `Send` type.

use std::collections::BTreeSet;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

const CLIPPY_TOML: &str = include_str!("../clippy.toml");

/// `assert_not_send!`, as the library's tests assert with it.
const NOT_SEND: &str = include_str!("../src/not_send.rs");

/// Each banned path, and a crate that uses exactly it.
const FIXTURES: &[(&str, &str)] = &[
    ("std::sync::Arc", "pub fn f(_: &std::sync::Arc<u8>) {}"),
    ("std::sync::Weak", "pub fn f(_: &std::sync::Weak<u8>) {}"),
    ("std::sync::Mutex", "pub fn f(_: &std::sync::Mutex<u8>) {}"),
    (
        "std::sync::RwLock",
        "pub fn f(_: &std::sync::RwLock<u8>) {}",
    ),
    ("std::sync::Condvar", "pub fn f(_: &std::sync::Condvar) {}"),
    ("std::sync::Barrier", "pub fn f(_: &std::sync::Barrier) {}"),
    ("std::sync::Once", "pub fn f(_: &std::sync::Once) {}"),
    (
        "std::sync::OnceLock",
        "pub fn f(_: &std::sync::OnceLock<u8>) {}",
    ),
    (
        "std::sync::LazyLock",
        "pub fn f(_: &std::sync::LazyLock<u8>) {}",
    ),
    (
        "std::sync::mpsc::Sender",
        "pub fn f(_: &std::sync::mpsc::Sender<u8>) {}",
    ),
    (
        "std::sync::mpsc::SyncSender",
        "pub fn f(_: &std::sync::mpsc::SyncSender<u8>) {}",
    ),
    (
        "std::sync::mpsc::Receiver",
        "pub fn f(_: &std::sync::mpsc::Receiver<u8>) {}",
    ),
    (
        "std::sync::atomic::AtomicBool",
        "pub fn f(_: &std::sync::atomic::AtomicBool) {}",
    ),
    (
        "std::sync::atomic::AtomicU8",
        "pub fn f(_: &std::sync::atomic::AtomicU8) {}",
    ),
    (
        "std::sync::atomic::AtomicU16",
        "pub fn f(_: &std::sync::atomic::AtomicU16) {}",
    ),
    (
        "std::sync::atomic::AtomicU32",
        "pub fn f(_: &std::sync::atomic::AtomicU32) {}",
    ),
    (
        "std::sync::atomic::AtomicU64",
        "pub fn f(_: &std::sync::atomic::AtomicU64) {}",
    ),
    (
        "std::sync::atomic::AtomicUsize",
        "pub fn f(_: &std::sync::atomic::AtomicUsize) {}",
    ),
    (
        "std::sync::atomic::AtomicI8",
        "pub fn f(_: &std::sync::atomic::AtomicI8) {}",
    ),
    (
        "std::sync::atomic::AtomicI16",
        "pub fn f(_: &std::sync::atomic::AtomicI16) {}",
    ),
    (
        "std::sync::atomic::AtomicI32",
        "pub fn f(_: &std::sync::atomic::AtomicI32) {}",
    ),
    (
        "std::sync::atomic::AtomicI64",
        "pub fn f(_: &std::sync::atomic::AtomicI64) {}",
    ),
    (
        "std::sync::atomic::AtomicIsize",
        "pub fn f(_: &std::sync::atomic::AtomicIsize) {}",
    ),
    (
        "std::sync::atomic::AtomicPtr",
        "pub fn f(_: &std::sync::atomic::AtomicPtr<u8>) {}",
    ),
    (
        "std::thread::JoinHandle",
        "pub fn f(_: &std::thread::JoinHandle<()>) {}",
    ),
    (
        "std::thread::spawn",
        "pub fn f() { let _ = std::thread::spawn(|| ()); }",
    ),
    (
        "std::thread::scope",
        "pub fn f() { std::thread::scope(|_| ()); }",
    ),
    (
        "std::thread::Builder::spawn",
        "pub fn f() { let _ = std::thread::Builder::new().spawn(|| ()); }",
    ),
    (
        "std::thread::Builder::spawn_unchecked",
        "pub fn f() { let _ = unsafe { std::thread::Builder::new().spawn_unchecked(|| ()) }; }",
    ),
    (
        "std::sync::mpsc::channel",
        "pub fn f() { let _ = std::sync::mpsc::channel::<u8>(); }",
    ),
    (
        "std::sync::mpsc::sync_channel",
        "pub fn f() { let _ = std::sync::mpsc::sync_channel::<u8>(1); }",
    ),
    (
        "std::net::ToSocketAddrs::to_socket_addrs",
        "pub fn f() { use std::net::ToSocketAddrs; let _ = \"localhost:80\".to_socket_addrs(); }",
    ),
    (
        "std::net::TcpStream::connect",
        "pub fn f() { let _ = std::net::TcpStream::connect(\"localhost:80\"); }",
    ),
    (
        "std::net::TcpListener::bind",
        "pub fn f() { let _ = std::net::TcpListener::bind(\"localhost:0\"); }",
    ),
    (
        "std::net::UdpSocket::bind",
        "pub fn f() { let _ = std::net::UdpSocket::bind(\"localhost:0\"); }",
    ),
    (
        "std::net::UdpSocket::connect",
        "pub fn f(s: &std::net::UdpSocket) { let _ = s.connect(\"localhost:80\"); }",
    ),
    (
        "std::net::UdpSocket::send_to",
        "pub fn f(s: &std::net::UdpSocket) { let _ = s.send_to(b\"x\", \"localhost:80\"); }",
    ),
];

/// What the ban allows: a crate of these must pass. It also asserts a
/// `!Send` type with `assert_not_send!`, which must compile.
const CLEAN: &str = "
use std::cell::{Cell, RefCell};
use std::rc::Rc;

thread_local! {
    static COUNT: Cell<u64> = const { Cell::new(0) };
}

pub fn f(shared: &Rc<RefCell<Vec<u64>>>) -> u64 {
    let n = COUNT.with(|count| {
        count.set(count.get() + 1);
        count.get()
    });
    shared.borrow_mut().push(n);
    n
}

assert_not_send!(Rc<RefCell<Vec<u64>>>);
";

/// One entry of a `clippy.toml` ban: `type` (`disallowed-types`) or `method`
/// (`disallowed-methods`), the path, and the reason.
type Entry = (&'static str, String, String);

/// Every entry of `toml`'s `disallowed-types` and `disallowed-methods`. A
/// line in either list that is not an entry, a comment or the list's end is
/// an error, as is another `disallowed-` list: no entry goes unchecked.
fn ban(toml: &str) -> Result<Vec<Entry>, String> {
    let mut list = None;
    let mut entries = Vec::new();
    for line in toml.lines().map(str::trim) {
        let Some(kind) = list else {
            list = match line {
                "disallowed-types = [" => Some("type"),
                "disallowed-methods = [" => Some("method"),
                _ if line.starts_with("disallowed-") => return Err(format!("unread list: {line}")),
                _ => None,
            };
            continue;
        };
        if line == "]" {
            list = None;
        } else if !line.is_empty() && !line.starts_with('#') {
            match line.split('"').collect::<Vec<_>>().as_slice() {
                ["{ path = ", path, ", reason = ", reason, " }," | " }"] => {
                    entries.push((kind, (*path).to_owned(), (*reason).to_owned()));
                }
                _ => return Err(format!("unread entry: {line}")),
            }
        }
    }
    if list.is_some() {
        return Err("a list has no end".into());
    }
    Ok(entries)
}

/// The scratch directory for this test process.
fn scratch() -> PathBuf {
    std::env::temp_dir().join(format!("ergon-client-ban-{}", std::process::id()))
}

/// `cargo clippy` on a scratch crate at `dir` whose `lib.rs` is `body`,
/// under this crate's `clippy.toml`, denying the ban's two lints: whether it
/// passed, and what it printed. Ambient `RUSTFLAGS` are cleared: a caller
/// that caps lints would otherwise make every case pass. Colour is off, as
/// CI forces it on and the diagnostics are compared as text.
fn clippy(dir: &Path, body: &str) -> Result<(bool, String), Box<dyn Error>> {
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"ban-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n",
    )?;
    std::fs::write(dir.join("src/lib.rs"), body)?;
    let out = Command::new(env!("CARGO"))
        .args(["clippy", "--quiet", "--color", "never", "--"])
        .args(["-D", "clippy::disallowed_types"])
        .args(["-D", "clippy::disallowed_methods"])
        .current_dir(dir)
        .env("CLIPPY_CONF_DIR", env!("CARGO_MANIFEST_DIR"))
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_BUILD_RUSTFLAGS")
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .output()?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

#[test]
fn every_banned_path_is_rejected_where_it_is_used() -> TestResult {
    let banned = ban(CLIPPY_TOML)?;
    let listed: BTreeSet<&str> = banned.iter().map(|(_, path, _)| path.as_str()).collect();
    let fixtures: BTreeSet<&str> = FIXTURES.iter().map(|&(path, _)| path).collect();
    assert_eq!(listed.len(), banned.len(), "a path is banned twice");
    assert_eq!(fixtures.len(), FIXTURES.len(), "a path has two fixtures");
    let unmatched: Vec<&&str> = listed.symmetric_difference(&fixtures).collect();
    assert!(
        unmatched.is_empty(),
        "a banned path with no fixture, or a fixture with no banned path: {unmatched:?}"
    );
    for (kind, path, _) in &banned {
        let body = FIXTURES
            .iter()
            .find(|&&(fixture, _)| fixture == path)
            .map(|&(_, body)| body)
            .ok_or("no fixture")?;
        let (ok, stderr) = clippy(&scratch().join(path.replace("::", "-")), body)?;
        assert!(!ok, "{path}: clippy accepted it");
        let expected = format!("error: use of a disallowed {kind} `{path}`");
        let found: Vec<&str> = stderr
            .lines()
            .filter(|line| line.contains("use of a disallowed"))
            .collect();
        assert!(
            !found.is_empty() && found.iter().all(|line| *line == expected),
            "{path}: rejected for another reason:\n{stderr}"
        );
    }
    Ok(())
}

#[test]
fn rc_cell_refcell_and_thread_local_pass() -> TestResult {
    let (ok, stderr) = clippy(&scratch().join("clean"), &format!("{NOT_SEND}{CLEAN}"))?;
    assert!(ok, "the clean crate must pass:\n{stderr}");
    Ok(())
}

#[test]
fn assert_not_send_rejects_a_send_type() -> TestResult {
    let body = format!("{NOT_SEND}\nassert_not_send!(String);\n");
    let (ok, stderr) = clippy(&scratch().join("send"), &body)?;
    assert!(!ok, "assert_not_send! accepted String");
    assert!(
        stderr.contains("E0283") && stderr.contains("AmbiguousIfSend"),
        "rejected for another reason:\n{stderr}"
    );
    Ok(())
}

#[test]
fn the_lab_clients_carry_the_ban() -> TestResult {
    let banned = ban(CLIPPY_TOML)?;
    let lab = Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples/clickhouse");
    for client in ["lab", "md", "engine"] {
        let copy = ban(&std::fs::read_to_string(
            lab.join(client).join("clippy.toml"),
        )?)?;
        let missing: Vec<&Entry> = banned
            .iter()
            .filter(|entry| !copy.contains(entry))
            .collect();
        assert!(
            missing.is_empty(),
            "{client}/clippy.toml lacks runtime/clippy.toml's {missing:?}"
        );
    }
    Ok(())
}

/// The lines of `source` with an `unsafe impl` of `Send` or `Sync` (by its
/// last path segment, after any generic parameters), line comments aside.
fn unsafe_send_sync(source: &str) -> Vec<usize> {
    let code: Vec<&str> = source
        .lines()
        .map(|line| line.split("//").next().unwrap_or_default())
        .collect();
    let code = code.join("\n");
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    let mut lines = Vec::new();
    for (at, _) in code.match_indices("unsafe") {
        if code[..at].chars().next_back().is_some_and(ident) {
            continue;
        }
        let Some(rest) = code[at + "unsafe".len()..]
            .trim_start()
            .strip_prefix("impl")
        else {
            continue;
        };
        if rest.chars().next().is_some_and(ident) {
            continue;
        }
        let mut rest = rest.trim_start();
        if rest.starts_with('<') {
            let (mut depth, mut prev) = (0, ' ');
            for (i, c) in rest.char_indices() {
                match c {
                    '<' => depth += 1,
                    '>' if prev != '-' => {
                        depth -= 1;
                        if depth == 0 {
                            rest = &rest[i + 1..];
                            break;
                        }
                    }
                    _ => {}
                }
                prev = c;
            }
        }
        let rest = rest.trim_start();
        let end = rest
            .find(|c: char| !ident(c) && c != ':')
            .unwrap_or(rest.len());
        let name = rest[..end].rsplit("::").next().unwrap_or_default();
        if name == "Send" || name == "Sync" {
            lines.push(code[..at].matches('\n').count() + 1);
        }
    }
    lines
}

/// Every `.rs` file under `dir`.
fn rust_files(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(rust_files(&path)?);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    Ok(files)
}

#[test]
fn the_unsafe_impl_scan_flags_send_and_sync_and_nothing_else() {
    let fixture = "\
unsafe impl Send for A {}
unsafe impl<T> Sync for B<T> {}
unsafe impl std::marker::Send for C {}
unsafe impl<T: Fn() -> u8>
    ::core::marker::Sync for D<T> {}
unsafe impl GlobalAlloc for E {}
unsafe impl<T: Send> Store for F<T> {}
// unsafe impl Send for G {}
/// No unsafe impl Sync for H.
unsafe fn send() {}
";
    assert_eq!(unsafe_send_sync(fixture), [1, 2, 3, 4]);
}

#[test]
fn no_client_source_implements_send_or_sync_unsafely() -> TestResult {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    for dir in [
        "runtime/src",
        "samples/clickhouse/lab/src",
        "samples/clickhouse/md/src",
        "samples/clickhouse/engine/src",
    ] {
        let files = rust_files(&repo.join(dir))?;
        assert!(!files.is_empty(), "{dir} has no Rust source");
        for file in files {
            for line in unsafe_send_sync(&std::fs::read_to_string(&file)?) {
                found.push(format!("{}:{line}", file.display()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "an unsafe impl of Send or Sync in the client: {found:?}"
    );
    Ok(())
}
