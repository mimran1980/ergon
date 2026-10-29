//! The `fstack` feature on Linux with `FSTACK_PREFIX` set (the prefix
//! `io/benches/fstack/build-fstack.sh` built F-Stack and its DPDK under):
//! links both, static, and sets `cfg(ergon_fstack_linked)`. Without the
//! feature, off Linux, or without `FSTACK_PREFIX` it does nothing, and the
//! `fstack` code compiles unlinked (`fstack::init` then refuses).
//!
//! F-Stack's library is linked whole (its FreeBSD start-up is reached
//! through linker sets), and so are DPDK's (drivers register themselves in
//! constructors), as `pkg-config --static --libs libdpdk` lists them. These
//! travel to every binary that links ergon-io. One link argument cannot:
//! `-z nostart-stop-gc`, without which lld (rustc's default linker on
//! `x86_64` Linux) drops the linker sets reached only through
//! `__start_`/`__stop_` symbols and `ff_init` crashes. It is set here for
//! ergon-io's own tests; a binary that uses `fstack-spin` sets it itself
//! (`-C link-arg=-Wl,-z,nostart-stop-gc`, see the crate README).
//!
//! The `smoltcp-dpdk` feature on Linux compiles the DPDK port shim
//! (`native/dpdk/ergon_io_dpdk.c`, the `smoltcp-spin` backend's) against the
//! libdpdk pkg-config finds (>= 23.11, shared, as ergon-md-dpdk's `dpdk`
//! feature): DPDK's burst and mbuf calls are `static inline`, so Rust
//! reaches them through it. Off Linux it compiles nothing.
//! It panics on a failure, which stops the build. Both features at once
//! with F-Stack linked would link two DPDKs: refused.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(ergon_fstack_linked)");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=native/dpdk");
    #[cfg(feature = "smoltcp-dpdk")]
    smoltcp_dpdk::build();
    fstack();
}

fn fstack() {
    println!("cargo:rerun-if-env-changed=FSTACK_PREFIX");
    let linux = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "linux");
    if std::env::var_os("CARGO_FEATURE_FSTACK").is_none() || !linux {
        return;
    }
    let Some(prefix) = std::env::var_os("FSTACK_PREFIX").map(PathBuf::from) else {
        return;
    };
    let lib = prefix.join("lib");
    let build = prefix.join("BUILD");
    println!(
        "cargo:rerun-if-changed={}",
        lib.join("libfstack.a").display()
    );
    println!("cargo:rerun-if-changed={}", build.display());
    let Ok(info) = std::fs::read_to_string(&build) else {
        fail(&format!(
            "{}: not built (build-fstack.sh writes it)",
            build.display()
        ));
    };
    // One line for rustc-env: `fstack::init`'s record splits it on ';'.
    println!(
        "cargo:rustc-env=ERGON_FSTACK_BUILD={}",
        info.trim().replace('\n', ";")
    );
    let pc = prefix.join("dpdk").join("lib").join("pkgconfig");
    let out = Command::new("pkg-config")
        .args(["--static", "--libs", "libdpdk"])
        .env("PKG_CONFIG_PATH", &pc)
        .output();
    let libs = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => fail(&format!("pkg-config libdpdk under {}", pc.display())),
    };
    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static:+whole-archive,-bundle=fstack");
    // DPDK as its pkg-config lists it: the whole-archive block, then the
    // same libraries again by name (already linked: skipped), then the
    // system's.
    let mut whole = false;
    let mut seen = Vec::new();
    for tok in libs.split_whitespace() {
        match tok {
            "-Wl,--whole-archive" => whole = true,
            "-Wl,--no-whole-archive" => whole = false,
            "-pthread" => println!("cargo:rustc-link-lib=dylib=pthread"),
            _ => {
                if let Some(dir) = tok.strip_prefix("-L") {
                    println!("cargo:rustc-link-search=native={dir}");
                } else if let Some(file) = tok.strip_prefix("-l:") {
                    let name = file
                        .strip_prefix("lib")
                        .and_then(|f| f.strip_suffix(".a"))
                        .unwrap_or(file)
                        .to_owned();
                    let kind = if whole {
                        "static:+whole-archive,-bundle"
                    } else {
                        "static:-bundle"
                    };
                    println!("cargo:rustc-link-lib={kind}={name}");
                    seen.push(name);
                } else if let Some(name) = tok.strip_prefix("-l")
                    && !seen.iter().any(|s| s == name)
                {
                    let kind = if name.starts_with("rte_") {
                        "static:-bundle"
                    } else {
                        "dylib"
                    };
                    println!("cargo:rustc-link-lib={kind}={name}");
                    seen.push(name.to_owned());
                }
                // Other linker flags (`--as-needed`, `--export-dynamic`):
                // not needed for a static link.
            }
        }
    }
    // What F-Stack's own objects call.
    for l in ["rt", "m", "dl", "crypto", "z", "numa"] {
        if !seen.iter().any(|s| s == l) {
            println!("cargo:rustc-link-lib=dylib={l}");
        }
    }
    // ergon-io's own tests and examples (see the header).
    println!("cargo:rustc-link-arg=-Wl,-z,nostart-stop-gc");
    println!("cargo:rustc-cfg=ergon_fstack_linked");
}

fn fail(msg: &str) -> ! {
    eprintln!("ergon-io build.rs: {msg}");
    std::process::exit(1)
}

#[cfg(feature = "smoltcp-dpdk")]
#[allow(clippy::panic, clippy::manual_assert)] // a failed build stops here
mod smoltcp_dpdk {
    pub fn build() {
        let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        if os != "linux" {
            // Linux only: the Rust side compiles to `Error::Unsupported`.
            return;
        }
        if std::env::var_os("CARGO_FEATURE_FSTACK").is_some()
            && std::env::var_os("FSTACK_PREFIX").is_some()
        {
            panic!(
                "`smoltcp-dpdk` (the system's libdpdk) and `fstack` with FSTACK_PREFIX (F-Stack's \
                 static DPDK) would link two DPDKs: build one of them"
            );
        }
        if !matches!(arch.as_str(), "x86_64" | "aarch64") {
            panic!(
                "the `smoltcp-dpdk` feature requires Linux x86_64 or aarch64 (target is {os} {arch})"
            );
        }
        let dpdk = pkg_config::Config::new()
            .atleast_version("23.11")
            .probe("libdpdk")
            .unwrap_or_else(|e| {
                panic!(
                    "the `smoltcp-dpdk` feature requires libdpdk >= 23.11 (pkg-config probe failed: {e}). \
                     Install it with e.g. `apt install libdpdk-dev pkg-config` (Ubuntu 24.04) or \
                     `dnf install dpdk-devel` (Amazon Linux 2023)."
                )
            });
        let mut b = cc::Build::new();
        b.std("c11").file("native/dpdk/ergon_io_dpdk.c");
        // rte_memcpy.h uses SSSE3 intrinsics on x86_64 unconditionally;
        // aarch64 needs nothing past Armv8.0.
        if arch == "x86_64" {
            b.flag("-mssse3");
        }
        for inc in &dpdk.include_paths {
            b.include(inc);
        }
        for (key, value) in &dpdk.defines {
            b.define(key, value.as_deref());
        }
        b.compile("ergon_io_dpdk");
        for path in &dpdk.link_paths {
            println!("cargo:rustc-link-search=native={}", path.display());
        }
        for lib in &dpdk.libs {
            println!("cargo:rustc-link-lib={lib}");
        }
    }
}
