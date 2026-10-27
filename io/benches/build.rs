//! With `--features fstack`: links F-Stack and its DPDK, both static, from
//! `FSTACK_PREFIX` (fstack/build-fstack.sh's prefix), and hands its `BUILD`
//! file to the arm as `FSTACK_BUILD`. Without the feature it does nothing.
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=FSTACK_PREFIX");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_FSTACK");
    if std::env::var_os("CARGO_FEATURE_FSTACK").is_none() {
        return;
    }
    let Some(prefix) = std::env::var_os("FSTACK_PREFIX").map(PathBuf::from) else {
        fail(
            "--features fstack needs FSTACK_PREFIX: run io/benches/fstack/build-fstack.sh <prefix> first",
        );
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
    // one line for rustc-env: the arm splits it back on ';'
    println!(
        "cargo:rustc-env=FSTACK_BUILD={}",
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
    // F-Stack's examples' link line: the whole library (its symbols are
    // reached through ff_init's constructors and callbacks), then DPDK's
    // static libraries as its pkg-config lists them (drivers whole-archive:
    // they register themselves in constructors), then libc's
    println!("cargo:rustc-link-search=native={}", lib.display());
    // FreeBSD's SYSINITs and DPDK's driver lists are linker sets reached
    // only through __start_/__stop_ symbols: lld (rustc's linker) collects
    // those sections under --gc-sections unless told not to, and ff_init
    // then crashes in FreeBSD's start-up
    println!("cargo:rustc-link-arg=-Wl,-z,nostart-stop-gc");
    println!("cargo:rustc-link-arg=-Wl,--whole-archive,-lfstack,--no-whole-archive");
    for tok in libs.split_whitespace() {
        if let Some(dir) = tok.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={dir}");
        } else {
            println!("cargo:rustc-link-arg={tok}");
        }
    }
    for l in [
        "-lrt", "-lm", "-ldl", "-lcrypto", "-lz", "-pthread", "-lnuma",
    ] {
        println!("cargo:rustc-link-arg={l}");
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("io/benches build.rs: {msg}");
    std::process::exit(1)
}
