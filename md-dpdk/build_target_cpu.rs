//! The C compiler's CPU flag for the native DPDK C, from Rust's target CPU.
//!
//! The `cc` crate passes some of rustc's codegen flags on to the C compiler
//! (x86 target features, for one) but not `-C target-cpu`, so without this the
//! C hot path (poller, packet, transport) stays generic while the Rust is
//! built for the core: on Graviton, outline-atomics helper calls in the C
//! against single LSE instructions in the Rust. `build.rs` includes this file
//! (`#[path]`), and `tests/build_target_cpu.rs` tests it.
//!
//! Pure functions over `CARGO_ENCODED_RUSTFLAGS` (flags joined by `0x1f`).

/// The value of the last `-C target-cpu=<cpu>` in Cargo's encoded rustflags,
/// as rustc reads repeated codegen options (the last one wins). Takes the
/// `-Ctarget-cpu=<cpu>`, `-C target-cpu=<cpu>`, `--codegen target-cpu=<cpu>`
/// and `--codegen=target-cpu=<cpu>` spellings; `None` without one or for an
/// empty value.
pub fn last_target_cpu(encoded_rustflags: &str) -> Option<&str> {
    let mut last = None;
    let mut flags = encoded_rustflags.split('\u{1f}');
    while let Some(flag) = flags.next() {
        let option = match flag {
            "-C" | "--codegen" => flags.next(),
            _ => flag.strip_prefix("-C").or_else(|| flag.strip_prefix("--codegen=")),
        };
        if let Some(cpu) = option.and_then(|o| o.strip_prefix("target-cpu=")) {
            last = Some(cpu);
        }
    }
    last.filter(|cpu| !cpu.is_empty())
}

/// The flag that gives the C compiler the same CPU as rustc's target CPU:
/// `-mcpu=<cpu>` on aarch64 (which sets both the architecture and the tuning,
/// as `-Ctarget-cpu` does), `-march=<cpu>` on `x86_64`. `None` without a target
/// CPU, for `generic` (rustc's name for the compiler's own default, which gcc
/// takes for no x86 `-march`) and on any other architecture. The name is
/// rustc's (LLVM's); `build.rs` checks that the C compiler accepts it.
pub fn c_cpu_flag(target_arch: &str, encoded_rustflags: &str) -> Option<String> {
    let cpu = last_target_cpu(encoded_rustflags).filter(|cpu| *cpu != "generic")?;
    match target_arch {
        "aarch64" => Some(format!("-mcpu={cpu}")),
        "x86_64" => Some(format!("-march={cpu}")),
        _ => None,
    }
}
