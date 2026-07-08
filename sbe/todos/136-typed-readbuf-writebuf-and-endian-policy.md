# Typed ReadBuf/WriteBuf mode and endian policy

**Blocked by:** `119-readbuf-writebuf-abstraction`, `121-endianness-full-type-matrix`, `122-read-write-bytes-unsafe-fastpath`
**Severity:** HIGH
**Status: DESIGN / ROADMAP**
**Status: DESIGN / ROADMAP**


## Problem

Generated accessors currently risk spreading bounds-check policy, endian policy,
and unsafe fast-path details across many generated methods. That makes the code
harder to audit and easier to regress when adding checked/verified decoder modes.

Rust can centralise these policies in monomorphised marker types so LLVM inlines
the same machine code as hand-written accessors while the generated source stays
small and reviewable.

## Design

Use typed policies for read/write mode and byte order:

```rust
pub enum Checked {}
pub enum Verified {}
pub enum Unchecked {}

pub enum LittleEndian {}
pub enum BigEndian {}

pub struct ReadBuf<'a, Mode, Endian> {
    buf: &'a [u8],
    _mode: core::marker::PhantomData<Mode>,
    _endian: core::marker::PhantomData<Endian>,
}

impl<'a, M> ReadBuf<'a, M, LittleEndian> {
    #[inline(always)]
    pub fn get_u64(&self, offset: usize) -> u64 {
        u64::from_le_bytes(self.read_bytes::<8>(offset))
    }
}
```

Mode controls bounds/extent assumptions:

- `Checked`: normal public safe path
- `Verified`: constructed from `VerifiedFrame`, can trust proven structural extents
- `Unchecked`: opt-in unsafe or `bound-check-disabled` path

Endian controls `from_le_bytes` vs `from_be_bytes` without branchy runtime checks.

## Acceptance criteria

- [ ] `ReadBuf<'a, Mode, Endian>` and `WriteBuf<'a, Mode, Endian>` exist in the
      generated runtime or shared runtime
- [ ] Safe checked mode uses slice-indexing/`try_into()` fast paths
- [ ] Verified mode can skip structural checks already proven by todo 131
- [ ] Unchecked mode uses documented unsafe reads/writes only behind the accepted
      feature or explicit unsafe API
- [ ] Endianness is a type-level policy generated from schema byte order
- [ ] Generated field accessors delegate to buffer policy methods instead of
      duplicating `#[cfg]` blocks
- [ ] LE and BE schemas are covered by todo 121 fixtures
- [ ] `cargo test -p ergosbe --features bound-check-disabled` passes
- [ ] Benchmarks show no regression versus direct hand-written reads and Aeron
      `ReadBuf`

Ref: todos 119, 121, 122, 131 and `design/DECISIONS.md` codegen rules.
