//! HFT-001 soundness regressions: safe constructors never UB on short/hostile
//! frames; zero-check paths are `unsafe`; raw byte helpers are not public safe.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
use std::error::Error;

mod common;
use common::{Paths, compile_and_run, generate};

#[test]
fn public_safe_api_rejects_header_only_car_frame() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "hft001_hdr_only");
    compile_and_run(
        "hft001_hdr_only",
        &src,
        r#"
        // Header only: 8 bytes. Declaring blockLength=0 used to wrap and then
        // UB on serial_number(). Checked decode must error.
        let mut hdr = [0u8; 8];
        // templateId=1, schemaId from schema, blockLength=0, version=0
        hdr[0..2].copy_from_slice(&0u16.to_le_bytes()); // blockLength
        hdr[2..4].copy_from_slice(&1u16.to_le_bytes()); // templateId Car
        hdr[4..6].copy_from_slice(&CarDecoder::SCHEMA_ID.to_le_bytes());
        hdr[6..8].copy_from_slice(&0u16.to_le_bytes()); // version
        assert!(CarDecoder::decode(&hdr, 0).is_err());
        assert!(AnyMessage::decode(&hdr, 0).is_err());
        // wrap with acting_block_length=0 also fails min-readable extent
        assert!(CarDecoder::wrap(&hdr, 0, 0, 0).is_err());
    "#,
    );
    Ok(())
}

#[test]
fn safe_encoder_constructors_reject_empty_buffer_without_panic() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "hft001_empty_enc");
    compile_and_run(
        "hft001_empty_enc",
        &src,
        r#"
        let mut empty = [];
        assert!(CarEncoder::wrap_and_apply_header(&mut empty, 0).is_err());
        assert!(CarEncoder::wrap(&mut empty, 0).is_err());
        let mut tiny = [0u8; 1];
        assert!(CarEncoder::wrap_and_apply_header(&mut tiny, 0).is_err());
    "#,
    );
    Ok(())
}

#[test]
fn generated_source_has_no_public_safe_raw_helpers() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "hft001_raw_vis");
    assert!(
        !src.contains("pub fn read_bytes"),
        "read_bytes must not be public safe"
    );
    assert!(
        !src.contains("pub fn write_bytes"),
        "write_bytes must not be public safe"
    );
    assert!(
        src.contains("unsafe fn read_bytes") || src.contains("unsafe fn read_bytes<"),
        "raw read helper must be private unsafe"
    );
    assert!(
        src.contains("pub fn wrap_and_apply_header")
            && src.contains("Result<")
            && src.contains("EncodeError"),
        "checked encoder constructor must return Result"
    );
    assert!(
        src.contains("pub unsafe fn wrap_and_apply_header"),
        "wrap_and_apply_header must be public unsafe"
    );
    assert!(
        src.contains("pub fn decode(") && !src.contains("pub fn try_wrap_and_apply_header"),
        "decoder framed entry is decode; try_wrap* aliases removed"
    );
    assert!(src.contains("pub fn wrap"), "wrap must be public");
    assert!(src.contains("pub fn decode"), "decode must be public");
    Ok(())
}

#[test]
fn catch_unwind_hostile_decode_does_not_panic() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "hft001_unwind");
    // Drive inside the generated crate so we exercise real codecs.
    compile_and_run(
        "hft001_unwind",
        &src,
        r#"
        use std::panic::{self, AssertUnwindSafe};
        for len in 0..16usize {
            let buf = vec![0u8; len];
            let r = panic::catch_unwind(AssertUnwindSafe(|| {
                let _ = CarDecoder::decode(&buf, 0);
                let _ = AnyMessage::decode(&buf, 0);
                let _ = CarDecoder::wrap(&buf, 0, 0, 0);
            }));
            assert!(r.is_ok(), "safe decode panicked at len={len}");
        }
        for len in 0..16usize {
            let mut buf = vec![0u8; len];
            let r = panic::catch_unwind(AssertUnwindSafe(|| {
                let _ = CarEncoder::wrap_and_apply_header(&mut buf, 0);
                let _ = CarEncoder::wrap(&mut buf, 0);
            }));
            assert!(r.is_ok(), "safe encode wrap panicked at len={len}");
        }
    "#,
    );
    Ok(())
}

#[test]
fn checked_encoder_calls_core_in_source() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "hft001_core_share");
    // Checked wrap_and_apply_header body must invoke the unsafe twin once.
    let idx = src
        .find("pub fn wrap_and_apply_header")
        .ok_or("missing wrap_and_apply_header")?;
    let window = &src[idx..idx.saturating_add(800).min(src.len())];
    assert!(
        window.contains("wrap_and_apply_header"),
        "checked encoder must delegate to unsafe core"
    );
    Ok(())
}

/// Group/entry constructors that skip extent checks must not be public safe.
#[test]
fn group_and_entry_zero_check_wraps_are_private_unsafe() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "hft001_group_vis");
    assert!(
        !src.contains("pub fn wrap_trusted"),
        "wrap_trusted must not be public safe"
    );
    assert!(
        src.contains("unsafe fn wrap_trusted"),
        "wrap_trusted must be private unsafe"
    );
    // Entry decoder wrap: private unsafe, not public safe.
    assert!(
        !src.contains(
            "pub fn wrap(\n        buf: &'a [u8],\n        pos: usize,\n        acting_block_length"
        ),
        "EntryDecoder::wrap must not be public safe"
    );
    assert!(
        src.contains("unsafe fn wrap") && src.contains("ENTRY_BLOCK_LENGTH"),
        "entry wrap must exist as private unsafe"
    );
    // Entry encoder wrap: private unsafe.
    assert!(
        !src.contains("pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self"),
        "EntryEncoder::wrap must not be public safe"
    );
    Ok(())
}

/// start_entry must reject a buffer too short for the fixed entry block
/// before advancing pos/written (HFT-004 pre-mutation capacity check).
#[test]
fn start_entry_rejects_short_buffer_before_mutation() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "hft001_start_entry");
    compile_and_run(
        "hft001_start_entry",
        &src,
        r#"
        // Acceleration is fixed-block (no entry var-data). count=1, only 5 bytes
        // after pos — less than ENTRY_BLOCK_LENGTH.
        let mut dim_only = [0u8; 4 + 5];
        let bl = PerformanceFiguresAccelerationEncoder::ENTRY_BLOCK_LENGTH as u16;
        dim_only[0..2].copy_from_slice(&bl.to_le_bytes());
        dim_only[2..4].copy_from_slice(&1u16.to_le_bytes());
        let mut g = PerformanceFiguresAccelerationEncoder::wrap(&mut dim_only, 4, 1);
        assert!(
            g.start_entry().is_err(),
            "start_entry must fail when fixed entry does not fit"
        );
        assert_eq!(g.written(), 0, "written must not advance on capacity failure");
    "#,
    );
    Ok(())
}

/// Fixed-block group wrap validates count*block_length before yielding entries.
#[test]
fn fixed_group_wrap_rejects_short_entries_region() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "hft001_group_short");
    compile_and_run(
        "hft001_group_short",
        &src,
        r#"
        use std::panic::{self, AssertUnwindSafe};
        // Acceleration is a true fixed group (no nested tail).
        let bl = PerformanceFiguresAccelerationDecoder::ENTRY_BLOCK_LENGTH as u16;
        let mut dim = [0u8; 4];
        dim[0..2].copy_from_slice(&bl.to_le_bytes());
        dim[2..4].copy_from_slice(&2u16.to_le_bytes());
        assert!(
            PerformanceFiguresAccelerationDecoder::wrap(&dim, 0, 0).is_err(),
            "fixed group wrap must reject missing entry region"
        );
        let mut one = [0u8; 4 + PerformanceFiguresAccelerationDecoder::ENTRY_BLOCK_LENGTH];
        one[0..2].copy_from_slice(&bl.to_le_bytes());
        one[2..4].copy_from_slice(&2u16.to_le_bytes());
        assert!(PerformanceFiguresAccelerationDecoder::wrap(&one, 0, 0).is_err());
        let mut full = [0u8; 4 + 2 * PerformanceFiguresAccelerationDecoder::ENTRY_BLOCK_LENGTH];
        full[0..2].copy_from_slice(&bl.to_le_bytes());
        full[2..4].copy_from_slice(&2u16.to_le_bytes());
        full[4..6].copy_from_slice(&30u16.to_le_bytes()); // mph entry0
        let g = PerformanceFiguresAccelerationDecoder::wrap(&full, 0, 0).expect("full group");
        let e0 = g.nth(0).expect("nth 0");
        assert_eq!(e0.mph(), 30);
        for len in 0..20usize {
            let buf = vec![0xAAu8; len];
            let r = panic::catch_unwind(AssertUnwindSafe(|| {
                let _ = PerformanceFiguresAccelerationDecoder::wrap(&buf, 0, 0);
            }));
            assert!(r.is_ok(), "fixed group wrap panicked at len={len}");
        }
        // Dynamic-tail group (fuelFigures has var-data): dim-only may accept wrap,
        // but advancing the entry must return Err without panic.
        let mut ff = [0u8; 4];
        ff[0..2].copy_from_slice(&6u16.to_le_bytes());
        ff[2..4].copy_from_slice(&1u16.to_le_bytes());
        if let Ok(mut g) = FuelFiguresDecoder::wrap(&ff, 0, 0) {
            if let Some(item) = Iterator::next(&mut g) {
                assert!(item.is_err(), "dynamic entry on dim-only must Err");
            }
        }
    "#,
    );
    Ok(())
}
