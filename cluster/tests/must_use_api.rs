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
