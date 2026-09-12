//! Verify that `check-bench-gate.sh` tolerance constants match the documented
//! policy: zero tolerance, i.e. no slack beyond a comparison's ceiling.
//!
//! Tolerance and ceiling are distinct. The gate fails on
//! `ratio > ceiling + tolerance`; these tests pin `tolerance` at zero. The
//! ceiling is per-comparison and the project target of `1.00` has three
//! documented exceptions — see the table in `book/src/sbe/benchmarks.md` and
//! the allowlist in `sbe/benchmarks/tests/bench_gate_test.rs`.
#![allow(clippy::expect_used, clippy::literal_string_with_formatting_args)]

#[test]
fn sbe_and_cluster_tolerance_is_zero() {
    let script = include_str!("../../scripts/check-bench-gate.sh");
    assert!(
        script.contains("SBE_TOLERANCE=0"),
        "SBE tolerance must be zero — see sbe/BENCHMARKS.md"
    );
    assert!(
        script.contains("CLUSTER_TOLERANCE=0"),
        "Cluster tolerance must be zero — same 1.00 ceiling as SBE"
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
        md.contains("cluster") && md.contains("1.00"),
        "BENCHMARKS.md must state the cluster 1.00 ceiling"
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
}

#[test]
fn cluster_decode_benches_do_not_validate_headers_on_one_arm() {
    let src = include_str!("../../cluster/benches/cluster_codec_bench.rs");
    assert!(
        !src.contains("sbe_tool_header_ok"),
        "sbe-tool decode arms must wrap at the body offset like ergo wrap(); \
         extra template/schema/version checks are unequal work"
    );
    assert!(
        src.contains("dec.detail_slice()"),
        "session_event ergo arm must use detail_slice (equal to sbe-tool), \
         not consuming into_detail which also builds the next stage"
    );
    assert!(
        !src.contains("into_detail().unwrap()"),
        "timed session_event path must not construct the next decoder stage"
    );
}
