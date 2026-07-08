# ReadBuf / WriteBuf abstraction for dual bounds-check modes

**Ref:** user request. Aeron Rust SBE comparison.

## Current verification status (2026-07-08)

The need for this abstraction is now visible in sample builds. Generated modules
emit many `unexpected cfg condition value: bound-check-disabled` warnings when
included from crates that do not define the same feature. The core
`bound-check-disabled` feature path previously exposed a design conflict:
runtime byte helpers should not be constrained by generated `const fn`
callsites.

This todo should not be treated as cosmetic. It is the route to making bounds
check gating auditable, DRY, and less noisy in generated user crates.

Const policy: `ReadBuf`/`WriteBuf` are runtime hot-path helpers and should not
be `const fn` unless Rust supports the same fast implementation in const
contexts. Constness belongs on pure metadata/constants/no-buffer helpers.

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
pub struct ReadBuf<'a, Mode = Checked, Endian = LittleEndian> {
    buf: &'a [u8],
    _mode: core::marker::PhantomData<Mode>,
    _endian: core::marker::PhantomData<Endian>,
}

impl<'a> ReadBuf<'a, Checked, LittleEndian> {
    #[inline(always)]
    pub fn get_u16(&self, offset: usize) -> u16 {
        #[cfg(not(feature = "bound-check-disabled"))]
        { u16::from_le_bytes(self.buf[offset..][..2].try_into().unwrap()) }
        #[cfg(feature = "bound-check-disabled")]
        unsafe { core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const u16) }
    }
    // ... get_u8, get_i16, get_u32, get_i32, get_u64, get_i64, get_f32, get_f64
}

pub struct WriteBuf<'a, Mode = Checked, Endian = LittleEndian> {
    buf: &'a mut [u8],
    _mode: core::marker::PhantomData<Mode>,
    _endian: core::marker::PhantomData<Endian>,
}

impl<'a> WriteBuf<'a, Checked, LittleEndian> {
    #[inline(always)]
    pub fn put_u16(&mut self, offset: usize, val: u16) {
        #[cfg(not(feature = "bound-check-disabled"))]
        { self.buf[offset..][..2].copy_from_slice(&val.to_le_bytes()); }
        #[cfg(feature = "bound-check-disabled")]
        unsafe { core::ptr::write_unaligned(self.buf.as_mut_ptr().add(offset) as *mut u16, val); }
    }
}
```

Todo 136 expands this into the full typed policy design: checked vs verified vs
unchecked modes and little- vs big-endian marker types.

## Acceptance criteria

- [ ] `ReadBuf` struct generated in the output module (or ergosbe-rt)
- [ ] `WriteBuf` struct generated in the output module
- [ ] Buffer mode and endian policy are type parameters, not runtime booleans
- [ ] `#[cfg]` gating removed from all generated field accessors — delegated to ReadBuf methods
- [ ] Byte-by-byte copy loops replaced with slice-based reads (`self.buf[offset..][..N].try_into().unwrap()`)
- [ ] `bound-check-disabled` feature: `ReadBuf`/`WriteBuf` use `unsafe` pointer read/write (zero bounds check)
- [ ] Safe path: `ReadBuf`/`WriteBuf` use `slice[offset..][..N].try_into().unwrap()` (bounds checked by slice indexing)
- [ ] Generated code is DRY — one `self.buf.get_u16(offset)` not two `#[cfg]` arms
- [ ] Golden file regen passes
- [ ] All existing tests pass with both features
- [ ] Generated modules do not produce `unexpected cfg` warning noise in crates that include them
- [ ] `samples/exchange-orderbook` compiles with generated modules included
- [ ] Performance benchmark shows no regression vs current (should be faster due to no byte-by-byte loops)

Ref: todo 136 for the full typed mode/endian policy.
