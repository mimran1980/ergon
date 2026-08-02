//! Stale 0.1 interface names must not appear in generated sources (HFT-010).

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
use std::error::Error;

mod common;
use common::{Paths, generate};

const STALE: &[&str] = &[
    "pub fn try_wrap(",
    "pub fn try_wrap_and_apply_header(",
    "pub fn read_bytes_unchecked",
    "pub fn write_bytes_unchecked",
];

#[test]
fn car_generated_source_rejects_stale_0_1_names() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "stale_iface");
    for needle in STALE {
        assert!(
            !src.contains(needle),
            "stale interface still generated: {needle}"
        );
    }
    assert!(src.contains("pub fn wrap("), "checked wrap missing");
    assert!(
        src.contains("pub fn wrap_and_apply_header("),
        "checked wah missing"
    );
    assert!(src.contains("pub fn decode("), "decode missing");
    assert!(
        src.contains("pub unsafe fn wrap_and_apply_header_unchecked")
            || src.contains("unsafe fn wrap_and_apply_header_unchecked"),
        "unchecked twin missing"
    );
    Ok(())
}
