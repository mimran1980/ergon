# ReadBuf / WriteBuf abstraction for dual bounds-check modes

**Ref:** user request. Aeron Rust SBE comparison.

## Problem

Currently `bound-check-disabled` is wired via `#[cfg]` inside every generated
field accessor:

```rust
pub fn serial_number(&self) -> u64 {
    let offset = self.pos + 0;
    #[cfg(not(feature = "bound-check-disabled"))]
    { u64::from_le_bytes(self.buf[offset..][..8].try_into().unwrap()) }
    #[cfg(feature = "bound-check-disabled")]
    unsafe { core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const u64) }
}
```

This is duplicated in every accessor — hundreds of places across generated code.
A `ReadBuf`/`WriteBuf` wrapper (like Aeron's `ReadBuf`/`WriteBuf` in
`aeron_rs_buffers.rs`) would:

1. Centralise the `#[cfg]` gating in ONE place (the `ReadBuf`/`WriteBuf` types)
2. Make generated code simpler — one method call, not two `#[cfg]` arms
3. Make it trivial to audit that bounds checks are correctly gated
4. Reduce generated code bloat significantly

## Design

```rust
// In ergosbe-rt or the generated module:
pub struct ReadBuf<'a> {
    buf: &'a [u8],
}

impl<'a> ReadBuf<'a> {
    #[inline(always)]
    pub fn get_u16(&self, offset: usize) -> u16 {
        #[cfg(not(feature = "bound-check-disabled"))]
        { u16::from_le_bytes(self.buf[offset..][..2].try_into().unwrap()) }
        #[cfg(feature = "bound-check-disabled")]
        unsafe { core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const u16) }
    }
    // ... get_u8, get_i16, get_u32, get_i32, get_u64, get_i64, get_f32, get_f64
}

pub struct WriteBuf<'a> {
    buf: &'a mut [u8],
}

impl<'a> WriteBuf<'a> {
    #[inline(always)]
    pub fn put_u16(&mut self, offset: usize, val: u16) {
        #[cfg(not(feature = "bound-check-disabled"))]
        { self.buf[offset..][..2].copy_from_slice(&val.to_le_bytes()); }
        #[cfg(feature = "bound-check-disabled")]
        unsafe { core::ptr::write_unaligned(self.buf.as_mut_ptr().add(offset) as *mut u16, val); }
    }
}
```

## Acceptance criteria

- [ ] `ReadBuf` struct generated in the output module (or ergosbe-rt)
- [ ] `WriteBuf` struct generated in the output module
- [ ] `#[cfg]` gating removed from all generated field accessors — delegated to ReadBuf methods
- [ ] Byte-by-byte copy loops replaced with slice-based reads (`self.buf[offset..][..N].try_into().unwrap()`)
- [ ] `bound-check-disabled` feature: `ReadBuf`/`WriteBuf` use `unsafe` pointer read/write (zero bounds check)
- [ ] Safe path: `ReadBuf`/`WriteBuf` use `slice[offset..][..N].try_into().unwrap()` (bounds checked by slice indexing)
- [ ] Generated code is DRY — one `self.buf.get_u16(offset)` not two `#[cfg]` arms
- [ ] Golden file regen passes
- [ ] All existing tests pass with both features
- [ ] Performance benchmark shows no regression vs current (should be faster due to no byte-by-byte loops)
