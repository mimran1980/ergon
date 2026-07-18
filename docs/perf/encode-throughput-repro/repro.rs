//! Minimal standalone reproducer for the encode/throughput_10k codegen divergence.
//!
//! Compile and run (timings):
//!   rustc -O --edition 2021 repro.rs -o repro && ./repro
//!
//! Emit assembly (aarch64) for each variant:
//!   rustc -O --emit asm --edition 2021 repro.rs
//!
//! Context: ErgoSBE's generated encoder (struct holds `&mut [u8]`, setters write
//! via `self.buf[off..].copy_from_slice`) compiles to a scalar, un-vectorised,
//! un-unrolled loop. Aeron's encoder (WriteBuf + header/parent value-move chain)
//! compiles to an 8x-unrolled, SIMD-vectorised loop for the SAME logical writes
//! when the loop body is near-constant (only `serial_number` varies). This file
//! isolates three shapes so the codegen difference can be inspected without the
//! full SBE/Aeron dependency tree.

#![allow(clippy::all)]

use std::hint::black_box;

/// 8-byte constant message header template (blockLength/templateId/schemaId/version).
const HEADER_TEMPLATE: [u8; 8] = [18, 0, 1, 0, 1, 0, 0, 0];
const SLOT: usize = 64; // bytes per message slot (matches the parity bench)
const N: usize = 10_000;

// ── Variant A: ErgoSBE shape — struct borrows `&mut [u8]`, setters index it ─

struct IndexEnc<'a> {
    buf: &'a mut [u8],
}
impl<'a> IndexEnc<'a> {
    #[inline(always)]
    fn wrap_and_apply_header(buf: &'a mut [u8]) -> Self {
        buf[0..8].copy_from_slice(&HEADER_TEMPLATE);
        Self { buf }
    }
    #[inline(always)]
    fn serial_number(&mut self, val: u64) {
        self.buf[8..16].copy_from_slice(&val.to_le_bytes());
    }
    #[inline(always)]
    fn model_year(&mut self, val: u16) {
        self.buf[16..18].copy_from_slice(&val.to_le_bytes());
    }
}

#[inline(never)]
fn encode_index(buf: &mut [u8]) {
    for i in 0..N {
        let off = i * SLOT;
        let mut e = IndexEnc::wrap_and_apply_header(&mut buf[off..off + SLOT]);
        e.serial_number(i as u64);
        e.model_year(2013);
    }
}

// ── Variant B: Aeron shape — WriteBuf value-move + header/parent chain ──────
// WriteBuf carries only a pointer; setters do raw pointer writes (no length
// carried, no bounds branch), mirroring Aeron's `put_u64`/`put_u16`.

struct WriteBuf {
    ptr: *mut u8,
}
impl WriteBuf {
    #[inline(always)]
    fn put_u64(&mut self, offset: usize, val: u64) {
        unsafe {
            core::ptr::copy_nonoverlapping(val.to_le_bytes().as_ptr(), self.ptr.add(offset), 8);
        }
    }
    #[inline(always)]
    fn put_u16(&mut self, offset: usize, val: u16) {
        unsafe {
            core::ptr::copy_nonoverlapping(val.to_le_bytes().as_ptr(), self.ptr.add(offset), 2);
        }
    }
    #[inline(always)]
    fn put_template(&mut self, offset: usize, tpl: &[u8; 8]) {
        unsafe {
            core::ptr::copy_nonoverlapping(tpl.as_ptr(), self.ptr.add(offset), 8);
        }
    }
}

#[inline(never)]
fn encode_pointer(buf: &mut [u8]) {
    for i in 0..N {
        let off = i * SLOT;
        let mut wb = WriteBuf { ptr: buf[off..].as_mut_ptr() };
        wb.put_template(0, &HEADER_TEMPLATE);
        wb.put_u64(8, i as u64);
        wb.put_u16(16, 2013);
    }
}

// ── Variant C: bare stores, no encoder abstraction (control) ────────────────

#[inline(never)]
fn encode_bare(buf: &mut [u8]) {
    for i in 0..N {
        let off = i * SLOT;
        buf[off..off + 8].copy_from_slice(&HEADER_TEMPLATE);
        buf[off + 8..off + 16].copy_from_slice(&(i as u64).to_le_bytes());
        buf[off + 16..off + 18].copy_from_slice(&2013u16.to_le_bytes());
    }
}

// ── Variant D: faithful ErgoSBE shape — Result return + extra struct fields
//    (message_start, pos) + a bounds branch, exactly as the generated encoder.

#[derive(Debug)]
enum EncodeError {
    BufferTooShort { needed: usize, available: usize },
}

struct FaithfulEnc<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
impl<'a> FaithfulEnc<'a> {
    const BLOCK_LENGTH: usize = 18;

    #[inline(always)]
    fn wrap_and_apply_header(buf: &'a mut [u8], p: usize) -> Result<Self, EncodeError> {
        let needed = 8 + Self::BLOCK_LENGTH;
        if buf.len().saturating_sub(p) < needed {
            return Err(EncodeError::BufferTooShort { needed, available: buf.len().saturating_sub(p) });
        }
        buf[p..p + 8].copy_from_slice(&HEADER_TEMPLATE);
        Ok(Self { buf: &mut buf[p..], message_start: 0, pos: needed })
    }
    #[inline(always)]
    fn serial_number(&mut self, val: u64) {
        self.buf[8..16].copy_from_slice(&val.to_le_bytes());
    }
    #[inline(always)]
    fn model_year(&mut self, val: u16) {
        self.buf[16..18].copy_from_slice(&val.to_le_bytes());
    }
}

#[inline(never)]
fn encode_faithful(buf: &mut [u8]) {
    for i in 0..N {
        let off = i * SLOT;
        let mut e = FaithfulEnc::wrap_and_apply_header(&mut buf[off..off + SLOT], 0).unwrap();
        e.serial_number(i as u64);
        e.model_year(2013);
    }
}

fn time_it<F: FnMut(&mut [u8])>(mut f: F) -> u128 {
    // Pre-allocate once OUTSIDE the timed region so the measurement is encode-only.
    let mut buf = vec![0u8; N * SLOT];
    for _ in 0..5 {
        f(&mut buf);
    }
    let mut best = u128::MAX;
    for _ in 0..50 {
        for b in buf.iter_mut() {
            *b = 0;
        }
        let t0 = std::time::Instant::now();
        f(&mut buf);
        let dt = t0.elapsed().as_nanos();
        black_box(buf.as_ptr());
        best = best.min(dt);
    }
    best
}

fn main() {
    let ti = time_it(encode_index);
    let tp = time_it(encode_pointer);
    let tb = time_it(encode_bare);
    let tf = time_it(encode_faithful);
    println!("encode_index    (ErgoSBE shape)         : {ti} ns  ({:.0} ps/msg)", ti as f64 / N as f64);
    println!("encode_pointer  (Aeron shape)           : {tp} ns  ({:.0} ps/msg)", tp as f64 / N as f64);
    println!("encode_bare     (control)               : {tb} ns  ({:.0} ps/msg)", tb as f64 / N as f64);
    println!("encode_faithful (Result+fields+bounds)  : {tf} ns  ({:.0} ps/msg)", tf as f64 / N as f64);
    println!("index/pointer ratio                      : {:.3}", ti as f64 / tp as f64);
    println!("faithful/index ratio                     : {:.3}", tf as f64 / ti as f64);
}
