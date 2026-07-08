# Flat enum with NullVal — drop the E3 newtype+Kind split

**Blocks:** none (design decision, then codegen)

Switch from the current E3 split-enum pattern to the Aeron-style flat enum
with a `NullVal` catch-all variant. The user prefers the simplicity of a
single match statement over preserving unknown wire values.

**Status: DONE**

## Current (E3 pattern)

```rust
pub struct Model(pub u8);
pub enum ModelKind { A, B, C }
impl Model {
    pub fn kind(self) -> Option<ModelKind> { ... }
    pub fn raw(self) -> u8 { self.0 }
}
// Usage: match val.kind() { Some(ModelKind::A) => ..., _ => ... }
```

## Target (Aeron-style flat enum)

```rust
#[repr(u8)]
pub enum Model { A = b'A', B = b'B', C = b'C', NullVal }
impl Model {
    pub fn raw(self) -> u8 { self as u8 }
}
// Usage: match val { Model::A => ..., _ => ... }
```

## Why this change

- Single match, no `.kind()` unwrapping — cleaner code
- User doesn't need to preserve unknown wire values
- Matches upstream Aeron Rust SBE pattern
- Simpler codegen — one type instead of two
- `#[repr(u8)]` + `NullVal` still provides forward compat

## Acceptance criteria

- [x] Replace `generate_enum` codegen to emit flat enum with NullVal variant
- [x] Remove newtype `Struct` + `Kind` split — single enum per SBE enum
- [x] `pub const fn raw(self) -> T` returns `self as T` (repr discriminant)
- [x] `From<T>` impls for conversion
- [x] `From<&str>` or `FromStr` for parsing (optional, not implemented — YAGNI for now)
- [x] All existing tests pass with updated API
- [x] Golden file regenerated
- [x] Update design record in `design/DECISIONS.md`

## Trade-off

Lose the raw wire value for unknown discriminants. The user accepts this —
they don't route/filter on unknown enum values. If this becomes a requirement
later, we can add a separate `raw_` accessor or switch back.

Ref: user preference — "if its unknown value I don't really care, that would
simplify the code as well."
