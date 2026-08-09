//! Verify that `check-bench-gate.sh` tolerance constants match the
//! documented policy: SBE = zero, cluster = 0.005.
#![allow(clippy::expect_used, clippy::literal_string_with_formatting_args)]

#[test]
fn sbe_tolerance_is_zero() {
    let script = include_str!("../../scripts/check-bench-gate.sh");
    assert!(
        script.contains("SBE_TOLERANCE=0"),
        "SBE tolerance must be zero — see sbe/BENCHMARKS.md"
    );
    assert!(
        script.contains("NOISE_TOLERANCE=\"${2:-0.005}\""),
        "Cluster default tolerance must be 0.005 — see sbe/BENCHMARKS.md"
    );
}

#[test]
fn bencharks_md_states_zero_tolerance() {
    let md = include_str!("../BENCHMARKS.md");
    assert!(
        md.contains("zero tolerance"),
        "BENCHMARKS.md must state zero tolerance for SBE"
    );
    assert!(
        md.contains("0.005"),
        "BENCHMARKS.md must state 0.005 tolerance for cluster"
    );
}

#[test]
fn book_bencharks_md_states_policy() {
    let md_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../book/src/sbe/benchmarks.md");
    let md = std::fs::read_to_string(&md_path).expect("book benchmarks.md must exist");
    assert!(
        md.contains("zero tolerance"),
        "book benchmarks.md must state zero SBE tolerance"
    );
    assert!(
        md.contains("0.005"),
        "book benchmarks.md must state cluster tolerance"
    );
}
