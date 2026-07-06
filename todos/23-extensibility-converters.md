# Extensibility: extension methods + type converters

**Blocked by:** `01-scalar-wire-parity` (need working generated types to extend)

Users need to attach domain behaviour to generated types without forking the
generator. The orphan rule is already in our favour (generated code lives in
the user's crate), but the ergonomics of adding converters should be
frictionless. Classic HFT example: a `semanticType="Price"` int64 field should
trivially become a `rust_decimal::Decimal`.

## User-facing patterns to support

### 1. Inherent extension methods (post-generation injection)

```rust
// In the user's build.rs or config:
Generator::new(config)
    .with_extension("Price", r#"
        pub fn as_decimal(&self) -> rust_decimal::Decimal {
            rust_decimal::Decimal::new(self.raw(), 4)  // 4 decimal places
        }
    "#)
    .generate(&schema);
```

The extension block is appended as an extra `impl Price { ... }` in the
generated module. No trait system needed, just raw code injection.

### 2. Trait impl injection

```rust
.with_trait_impl("Price", "From<Price> for rust_decimal::Decimal", r#"
    fn from(val: Price) -> Self {
        rust_decimal::Decimal::new(val.raw() as i64, 4)
    }
"#)
```

### 3. Semantic-type auto-mapping (config-driven)

```rust
// ergosbe-build config
semantic_types:
  Price:
    rust_type: "rust_decimal::Decimal"
    scale: 4
    impl_from: true
  Qty:
    rust_type: "rust_decimal::Decimal"
    scale: 0

// Generated automatically:
impl From<Price> for rust_decimal::Decimal { ... }
impl From<rust_decimal::Decimal> for Price { ... }
```

This is the full semantic-newtypes path from DECISIONS.md §4 but scoped to
conversions only (not newtype wrappers).

### 4. Raw value passthrough

```rust
// Always generated on every newtype:
impl Price {
    pub const fn raw(self) -> i64 { self.0 }
}
impl From<i64> for Price { fn from(v: i64) -> Self { Self(v) } }
impl From<Price> for i64 { fn from(v: Price) -> Self { v.0 } }
```

This is the unopinionated escape hatch — users can always get the raw int and
do their own conversion. Already spec'd in DECISIONS.md §4.

## Acceptance criteria

- [ ] `with_extension(name, code)` API on `Generator` or `GenerationConfig` —
      appends raw Rust source as an extra `impl` block
- [ ] `with_trait_impl(type_name, trait_bound, code)` — generates a trait impl
- [ ] Semantic-type mapping in config: `semantic_types.Price.convert_to = "rust_decimal::Decimal"`
- [ ] Generated types always expose `raw()` and `From<T>`/`Into<T>` for the
      underlying primitive (zero-cost escape hatch)
- [ ] Example: Car schema with `Price` field → user converts to `Decimal` in 3
      lines of config
- [ ] Works with both `build.rs` driver and the proc-macro frontend (v1.1)
- [ ] Extension code is formatted (rustfmt pass) and appears in generated docs

Ref: `design/DECISIONS.md` §4 semantic newtypes, §10 orphan rule advantage.
