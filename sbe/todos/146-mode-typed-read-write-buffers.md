# Mode-typed ReadBuf and WriteBuf policy

**Blocked by:** 119-readbuf-writebuf-abstraction, 136-typed-readbuf-writebuf-and-endian-policy
**Severity:** HIGH
**Status: ACTIVE / FAST-PATH POLICY**


## Problem

Generated accessors should not duplicate bounds-check policy, endian selection,
and unsafe pointer details across every field. That makes generated code noisy,
hard to audit, and prone to warning drift when included from user crates.

Stable Rust marker types can move those policies into a deep module: callers see
simple generated accessors while LLVM sees concrete, monomorphised read/write
helpers.

## Design target

Use marker-typed buffer helpers:

```rust
ReadBuf<'a, Checked, LittleEndian>
ReadBuf<'a, Verified, BigEndian>
ReadBuf<'a, Unchecked, LittleEndian>
WriteBuf<'a, Checked, LittleEndian>
```

Mode controls trust:

- `Checked`: normal safe public path.
- `Verified`: constructed from a proof token after structural verification.
- `Unchecked`: explicit unsafe or feature-gated fast path.

Endian controls byte conversion at the type level, not through runtime branches.

## Acceptance criteria

- [x] Generated scalar, enum, set, composite, and array accessors delegate to
      policy buffer methods instead of embedding duplicated read/write logic.
- [x] Unsafe read/write code is localized in the buffer policy implementation.
- [x] Little-endian and big-endian generated schemas use distinct type-level
      endian policies.
- [x] Checked mode stays safe by default and reports contextual errors where
      the public interface requires them.
- [x] Verified mode only exists after todo 147/131 can prove frame extents.
- [x] Unchecked mode requires explicit unsafe API or accepted feature gating.
- [x] Generated modules included by downstream crates do not emit
      `unexpected cfg` warning noise for `bound-check-disabled`.
- [x] Benchmarks show no regression versus current generated direct reads and
      Aeron `ReadBuf`.
