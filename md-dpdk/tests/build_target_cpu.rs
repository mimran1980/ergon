//! The build script's C CPU flag (`build_target_cpu.rs`, shared with
//! `build.rs`): the native C follows the last `-C target-cpu` in
//! `CARGO_ENCODED_RUSTFLAGS`, which the `cc` crate does not pass on.

#[path = "../build_target_cpu.rs"]
mod build_target_cpu;

use build_target_cpu::{c_cpu_flag, last_target_cpu};

/// Cargo's encoding: one flag per element, joined by the unit separator.
fn encoded(flags: &[&str]) -> String {
    flags.join("\u{1f}")
}

#[test]
fn no_rustflags_give_no_c_cpu_flag() {
    assert_eq!(last_target_cpu(""), None);
    assert_eq!(c_cpu_flag("x86_64", ""), None);
    assert_eq!(c_cpu_flag("aarch64", ""), None);
    assert_eq!(
        c_cpu_flag("aarch64", &encoded(&["-Copt-level=3", "--cfg", "foo"])),
        None
    );
}

#[test]
fn native_maps_to_mcpu_on_aarch64_and_march_on_x86() {
    let flags = encoded(&["-Ctarget-cpu=native"]);
    assert_eq!(c_cpu_flag("aarch64", &flags).as_deref(), Some("-mcpu=native"));
    assert_eq!(c_cpu_flag("x86_64", &flags).as_deref(), Some("-march=native"));
}

#[test]
fn the_split_and_long_forms_are_read() {
    let split = encoded(&["-C", "target-cpu=neoverse-v2"]);
    assert_eq!(c_cpu_flag("aarch64", &split).as_deref(), Some("-mcpu=neoverse-v2"));
    let long = encoded(&["--codegen", "target-cpu=x86-64-v3"]);
    assert_eq!(c_cpu_flag("x86_64", &long).as_deref(), Some("-march=x86-64-v3"));
    let long_eq = encoded(&["--codegen=target-cpu=sapphirerapids"]);
    assert_eq!(c_cpu_flag("x86_64", &long_eq).as_deref(), Some("-march=sapphirerapids"));
}

#[test]
fn the_last_target_cpu_wins() {
    let flags = encoded(&["-Ctarget-cpu=native", "-Copt-level=3", "-C", "target-cpu=neoverse-v2"]);
    assert_eq!(last_target_cpu(&flags), Some("neoverse-v2"));
    assert_eq!(c_cpu_flag("aarch64", &flags).as_deref(), Some("-mcpu=neoverse-v2"));
    let back = encoded(&["-C", "target-cpu=neoverse-v2", "-Ctarget-cpu=neoverse-v3"]);
    assert_eq!(c_cpu_flag("aarch64", &back).as_deref(), Some("-mcpu=neoverse-v3"));
}

#[test]
fn generic_is_the_compilers_own_default() {
    // rustc's `generic` has no x86 `-march` equivalent in gcc, and on aarch64
    // it is the default anyway: no flag either way.
    let flags = encoded(&["-Ctarget-cpu=generic"]);
    assert_eq!(c_cpu_flag("x86_64", &flags), None);
    assert_eq!(c_cpu_flag("aarch64", &flags), None);
    // A later generic overrides an earlier CPU, as rustc reads it.
    let later = encoded(&["-Ctarget-cpu=native", "-Ctarget-cpu=generic"]);
    assert_eq!(c_cpu_flag("aarch64", &later), None);
}

#[test]
fn other_architectures_and_empty_values_give_none() {
    let flags = encoded(&["-Ctarget-cpu=native"]);
    assert_eq!(c_cpu_flag("riscv64", &flags), None);
    assert_eq!(last_target_cpu(&encoded(&["-Ctarget-cpu="])), None);
    // A dangling `-C` at the end is not a CPU.
    assert_eq!(
        last_target_cpu(&encoded(&["-Ctarget-cpu=native", "-C"])),
        Some("native")
    );
}
