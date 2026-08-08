//! Downstream compile-fail fixtures: `#[must_use]` values must not be silently
//! discarded. Each fixture is compiled under `#![deny(unused_must_use)]`.

#[test]
fn must_use_session_builder() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/must_use/session_builder.rs");
}

#[test]
fn must_use_async_connect() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/must_use/async_connect.rs");
}

/// ClusterClaim can't be constructed without a live Aeron connection, so
/// there's no trybuild fixture. This test proves the `#[must_use]`
/// attribute is present — downstream crates with `#![deny(unused_must_use)]`
/// will still catch a discarded claim.
#[test]
fn cluster_claim_has_must_use_attribute() {
    let src = include_str!("../src/client.rs");
    let claim_pos = src.find("pub struct ClusterClaim").expect("ClusterClaim not found");
    // The #[must_use] attribute is on the line before pub struct.
    // Search backwards from the claim position.
    let before = &src[claim_pos.saturating_sub(100)..claim_pos];
    assert!(
        before.contains("#[must_use"),
        "ClusterClaim must carry #[must_use]; found preceding text: {before}"
    );
}
