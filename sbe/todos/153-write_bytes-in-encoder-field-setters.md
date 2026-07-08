# Write_bytes in encoder field setters

**Blocked by:** none
**Severity:** MEDIUM
**Ref:** user request 2026-07-08

## Problem

Encoder field setters (scalar and array, both message and entry) still use the
manual `to_le_bytes()` + `copy_from_slice` pattern instead of `write_bytes::<N>`:

```rust
// Current (bad):
pub fn price(&mut self, val: i64) -> &mut Self {
    let offset = self.entry_start + 8;
    let val_bytes = val.to_le_bytes();
    self.buf[offset..offset + 8].copy_from_slice(&val_bytes);
    self
}

// Desired (good):
pub fn price(&mut self, val: i64) -> &mut Self {
    let offset = self.entry_start + 8;
    write_bytes::<8>(self.buf, offset, &val.to_le_bytes());
    self
}
```

`write_bytes` is the canonical fast path — with `bound-check-disabled` it uses
`ptr::write_unaligned` for zero-overhead writes.

## Locations in codegen

| Path | Line | Type |
|------|------|------|
| Message encoder scalar | ~3957 | `to_le_bytes()` + `copy_from_slice` |
| Message encoder array | ~3943 | `to_le_bytes()` + `copy_from_slice` in while loop |
| Entry encoder scalar | ~4529 | `to_le_bytes()` + `copy_from_slice` |
| Entry encoder array | ~4516 | `to_le_bytes()` + `copy_from_slice` in while loop |
| Message encoder enum | ~3997 | Same pattern |
| Message encoder set | ~4011 | Same pattern |
| Message encoder optional scalar | ~4030 | Same pattern |
| Entry encoder enum | ~4558 | Same pattern |
| Entry encoder set | ~4574 | Same pattern |
| Header template | ~3739 | `to_le_bytes()` + `copy_from_slice` |
| Var-data length prefix | ~4237 | `copy_from_slice(&len_bytes)` |
| Group dim template | ~4383 | `to_le_bytes()` + `copy_from_slice` |

## Acceptance criteria

- [ ] Message encoder scalar setters use `write_bytes::<N>`
- [ ] Message encoder array setters use `write_bytes::<N>` per element
- [ ] Entry encoder scalar setters use `write_bytes::<N>`
- [ ] Entry encoder array setters use `write_bytes::<N>` per element
- [ ] Header template construction uses `write_bytes`
- [ ] Var-data length prefix uses `write_bytes`
- [ ] Group dim template uses `write_bytes`
- [ ] Golden file updated
- [ ] All tests pass
