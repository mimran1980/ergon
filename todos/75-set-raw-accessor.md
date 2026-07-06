# `raw()` accessor on Set types — consistent with enums

**Blocked by:** none

Enums already have `.raw()` returning the underlying integer. Sets should too.

## What to add

```rust
impl OptionalExtras {
    pub const fn raw(self) -> u8 {
        self.0
    }
}
```

This is already generated for enums but not for sets. The set's `.raw()` returns
the raw bitmask value, matching the enum's `.raw()` pattern.

## Acceptance criteria

- [ ] `pub const fn raw(self) -> T` on all generated set types
- [ ] Same signature as enum's `.raw()` — consumes self, returns encoding type
- [ ] Tests: `OptionalExtras(0b101).raw() == 0b101`
- [ ] Golden file regenerated

Ref: upstream SBE issue #1086 — "[Java] Add setRaw() method for Set type".
Consistency with existing enum `.raw()` accessor.
