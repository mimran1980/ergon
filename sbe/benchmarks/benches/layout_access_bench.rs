//! Flyweight vs wire-image vs `repr(C, packed)` single-field access.
//!
//! Question: on little-endian, is reading one field from a large composite a
//! true no-op whether you use the flyweight or materialise the value struct?
//! And does a hand-written `#[repr(C, packed)]` overlay beat either?
//!
//! Fixture: `BigBlock` = 32 x u64 = **256 bytes**. Target field `f15` sits in
//! the middle (offset 120). Message `Wide` = seq(u64) + BigBlock.
//!
//! Arms (nothing allocates on the timed path; buffers/values pre-sized):
//!
//! | Arm | What is timed |
//! |-----|----------------|
//! | `flyweight_f15` | preheld decoder: `dec.block().f15()` only |
//! | `value_preheld_f15` | preheld `BigBlock` wire image: `.f15()` only |
//! | `packed_preheld_f15` | `#[repr(C, packed)]` overlay: unaligned load of f15 |
//! | `value_copy_then_f15` | `block_value()` (256 B copy) then `.f15()`; copy forced via `black_box` |
//! | `wrap_plus_flyweight_f15` | wrap decoder + flyweight field (full entry cost) |
//!
//! Expectation on LE hosts:
//! - `flyweight_f15` ~ `value_preheld_f15` ~ `packed_preheld_f15` (one load)
//! - `value_copy_then_f15` slower (must pay for the 256-byte materialisation)
//! - packing does **not** beat the wire-image accessor

#![allow(
    unsafe_code,
    missing_docs,
    unused_variables,
    dead_code,
    unused_mut,
    unused_must_use,
    clippy::all
)]

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use ergo_sbe_benchmarks::large_comp::*;

/// Mid-block field under test (schema name `f15`, offset 15 * 8 = 120).
const TARGET_FIELD_VALUE: u64 = 0x0123_4567_89AB_CDEF;

/// Hand-written LE packed overlay of the same 256-byte BigBlock layout.
/// Field order and sizes must match the schema exactly (32 x u64).
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct PackedBigBlock {
    f0: u64,
    f1: u64,
    f2: u64,
    f3: u64,
    f4: u64,
    f5: u64,
    f6: u64,
    f7: u64,
    f8: u64,
    f9: u64,
    f10: u64,
    f11: u64,
    f12: u64,
    f13: u64,
    f14: u64,
    f15: u64,
    f16: u64,
    f17: u64,
    f18: u64,
    f19: u64,
    f20: u64,
    f21: u64,
    f22: u64,
    f23: u64,
    f24: u64,
    f25: u64,
    f26: u64,
    f27: u64,
    f28: u64,
    f29: u64,
    f30: u64,
    f31: u64,
}

const _: () = assert!(core::mem::size_of::<PackedBigBlock>() == 256);
const _: () = assert!(core::mem::size_of::<BigBlock>() == 256);

fn encode_wide_fixture() -> Vec<u8> {
    let mut buf = vec![0u8; WideEncoder::ENCODED_LENGTH];
    let mut enc = WideEncoder::wrap_and_apply_header(&mut buf, 0);
    enc.seq(42);
    let block = BigBlock::new(
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        TARGET_FIELD_VALUE,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        26,
        27,
        28,
        29,
        30,
        31,
    );
    enc.block(block);
    let _ = enc;
    let dec = WideDecoder::try_wrap_and_apply_header(&buf, 0).unwrap();
    assert_eq!(dec.block().f15(), TARGET_FIELD_VALUE);
    assert_eq!(dec.block_value().f15(), TARGET_FIELD_VALUE);
    buf
}

fn block_bytes_from_fixture(buf: &[u8]) -> [u8; 256] {
    let off = 8 + 8; // header + seq
    buf[off..off + 256].try_into().unwrap()
}

fn bench_layout_access(c: &mut Criterion) {
    let fixture = encode_wide_fixture();
    let block_bytes = block_bytes_from_fixture(&fixture);
    let preheld_value = BigBlock(block_bytes);
    let preheld_packed: PackedBigBlock =
        unsafe { core::ptr::read_unaligned(block_bytes.as_ptr() as *const PackedBigBlock) };
    let preheld_dec = WideDecoder::wrap(fixture.as_slice(), 8, WideDecoder::BLOCK_LENGTH, 0);

    let packed_f15 =
        unsafe { core::ptr::read_unaligned(core::ptr::addr_of!(preheld_packed.f15)) };
    assert_eq!(packed_f15, TARGET_FIELD_VALUE);
    assert_eq!(preheld_value.f15(), TARGET_FIELD_VALUE);
    assert_eq!(preheld_dec.block().f15(), TARGET_FIELD_VALUE);

    let mut group = c.benchmark_group("layout/large_composite_single_field");
    group.throughput(Throughput::Elements(1));

    // --- Fair field-only arms (no wrap / no materialise in the timed path) ---

    group.bench_function("flyweight_f15", |b| {
        b.iter(|| black_box(preheld_dec.block().f15()));
    });

    group.bench_function("value_preheld_f15", |b| {
        b.iter(|| black_box(preheld_value.f15()));
    });

    group.bench_function("packed_preheld_f15", |b| {
        b.iter(|| {
            let v = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!(preheld_packed.f15)) };
            black_box(v)
        });
    });

    // --- Cost of materialising the 256-byte value just to read one field ---

    group.bench_function("value_copy_then_f15", |b| {
        b.iter(|| {
            let owned = preheld_dec.block_value();
            // Force the copy to remain observable (otherwise LLVM can fold
            // block_value().f15() into a single mid-block load from the buffer).
            black_box(owned.0);
            black_box(owned.f15())
        });
    });

    // --- Full entry: wrap + flyweight field (diagnostic, not vs packed) ---

    group.bench_function("wrap_plus_flyweight_f15", |b| {
        b.iter(|| {
            let dec = WideDecoder::wrap(
                black_box(fixture.as_slice()),
                8,
                WideDecoder::BLOCK_LENGTH,
                0,
            );
            black_box(dec.block().f15())
        });
    });

    group.finish();
}

criterion_group!(benches, bench_layout_access);
criterion_main!(benches);
