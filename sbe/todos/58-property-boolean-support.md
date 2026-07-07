# Property boolean support — map SBE BooleanType to Rust `bool`

**Blocked by:** none

SBE represents booleans as an enum type (typically `BooleanType` with `F=0, T=1`
or similar). This is correct per the SBE spec, but Rust code should deal with
`true`/`false` directly — not `BooleanType(BooleanTypeKind::T)`.

## What to do

Detect boolean enum types in codegen and generate:

- **`From<bool>` and `Into<bool>` impls** on the boolean enum newtype
- **`const TRUE: Self` and `const FALSE: Self`** constants
- **Optionally: omit the `Kind` enum for boolean types** since `bool` already
  has exactly two values — no unknown discriminant problem
- **Encoder accepts `bool` directly** — `set_foo(&mut self, val: bool)` instead
  of requiring the user to construct the enum

## Acceptance criteria

- [x] Detect boolean enum types: `enum` with exactly two valid values that
  represent true/false (heuristic: presence of `T`/`F` or `True`/`False` or
  `TRUE`/`FALSE` or `Yes`/`No` variants, or `semanticType="Boolean"`)
- [x] `From<bool> for BooleanType` and `From<BooleanType> for bool`
- [x] `pub const TRUE: BooleanType = BooleanType(1);` (or whatever the T discriminant is)
- [x] `pub const FALSE: BooleanType = BooleanType(0);`
- [x] Encoder setter: `pub fn set_foo(&mut self, val: bool)` calls `From<bool>`
- [x] Decoder getter: `pub const fn foo(&self) -> bool` returns `self.raw() != 0`
- [x] Existing `BooleanType`/`Model` enums still work unchanged (backward compat)
- [x] Tests: verify round-trip `true` → encode → decode → `true`

## Design rationale

SBE's `BooleanType` enum pattern is wire-correct but user-hostile. The type
system already guarantees exactly two states for `bool`. Adding a `Kind` enum
for booleans is boilerplate that adds no safety. The generated code should let
users write `true`/`false` while preserving wire compatibility.

Ref: user request. Related: DECISIONS.md §3 (enum handling), §11 (ergonomic API).
