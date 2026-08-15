//! Downstream compile fixtures: `ClusterError` is `#[non_exhaustive]`.
//! External crates must use a wildcard arm; exhaustive matches fail.

#[test]
fn cluster_error_wildcard_match_compiles() {
    let t = trybuild::TestCases::new();
    t.pass("tests/non_exhaustive/wildcard_ok.rs");
}

#[test]
fn cluster_error_exhaustive_match_fails() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/non_exhaustive/exhaustive_fail.rs");
}
