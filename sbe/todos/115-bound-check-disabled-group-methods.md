# Gate group decoder bounds checks behind `bound-check-disabled` feature

**Blocked by:** none
**Ref:** user request, existing `#[cfg(not(feature = "bound-check-disabled"))]` pattern in `wrap_and_apply_header`

## Problem

`wrap_and_apply_header` already gates its bounds check behind the feature flag:
```rust
#[cfg(not(feature = "bound-check-disabled"))]
let header_bytes: [u8; 8] = buf.get(pos..pos + 8).ok_or_else(|| { ... })?;

#[cfg(feature = "bound-check-disabled")]
let header_bytes: [u8; 8] = unsafe { ptr::read_unaligned(...) };
```

But group decoder methods still have unconditional bounds checks:

### `nth()`
```rust
pub fn nth(&self, idx: usize) -> Result<EntryDecoder, DecodeError> {
    if idx >= self.total {                                    // ← not gated
        return Err(DecodeError::BufferTooShort { ... });
    }
    let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
    if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {   // ← not gated
        return Err(DecodeError::BufferTooShort { ... });
    }
    Ok(EntryDecoder::wrap(self.buf, offset, self.acting_version))
}
```

### `skip_n()` (both paths)
```rust
pub fn skip_n(&mut self, n: usize) -> Result<(), DecodeError> {
    if n > self.count {                                       // ← not gated
        return Err(DecodeError::BufferTooShort { ... });
    }
    // ...
}
```

### Entry field accessor bounds checks
The `offset + size > self.buf.len()` check on every field accessor in entry
decoders should also respect the feature flag.

## Design

Wrap each bounds check in `#[cfg(not(feature = "bound-check-disabled"))]` and
provide an unsafe fast path with `#[cfg(feature = "bound-check-disabled")]`:

```rust
pub fn nth(&self, idx: usize) -> Result<EntryDecoder, DecodeError> {
    #[cfg(not(feature = "bound-check-disabled"))]
    {
        if idx >= self.total {
            return Err(DecodeError::BufferTooShort { ... });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(DecodeError::BufferTooShort { ... });
        }
    }
    #[cfg(feature = "bound-check-disabled")]
    let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
    Ok(EntryDecoder::wrap(self.buf, offset, self.acting_version))
}
```

Consistent with the existing pattern: safe path is the default, unsafe path
requires the feature flag. No API change — the return type stays `Result` but
with `bound-check-disabled`, the `Err` path is never taken.

## Scope

| Method | Location | Has gate? |
|--------|----------|-----------|
| `wrap_and_apply_header` | message decoder (~2011) | ✅ Already |
| `nth()` | group decoder (~3194) | ❌ |
| `skip_n()` | group decoder (~3110, ~3128) | ❌ |
| entry field accessor bounds | group decoder (~3250+) | ❌ |
| `next()` in Iterator | group decoder (~3240, ~3260) | ❌ |

## Acceptance criteria

- [ ] `nth()` bounds checks — WONT DO (per user: nth takes user input, trust boundary)
- [x] `skip_n()` bounds checks gated behind `bound-check-disabled`
- [x] Entry field accessor bounds checks gated
- [x] Iterator `next()` bounds checks gated
- [x] `#[cfg(feature = "bound-check-disabled")]` fast paths use `unsafe` where needed
- [x] Golden file regenerated and stability test passes
- [x] Baseline tests pass with default features
- [x] `cargo test --features bound-check-disabled` passes
