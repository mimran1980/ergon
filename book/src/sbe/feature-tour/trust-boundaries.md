# Trust Boundaries

Checked entry points validate the message header and fixed block. `verify`
walks the complete dynamic tail before trusted access:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_try_vs_trusted}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

## Checked constructors (0.1.10)

| Entry | When |
|-------|------|
| `Decoder::decode` / `try_from` / `wrap` | Untrusted or network input — validate template/schema ids and version-aware extents; returns `Result` |
| `Encoder::wrap` / `wrap_and_apply_header` | Same: one cold capacity check then shared private zero-check core |

There is **no** public `try_wrap*` alias and **no** public `*_unchecked`
constructor twin unless HFT-008 records `keep=true` (currently all keep=false:
cores are module-private). Offsets are **message start** (not sbe-tool body
offset). See [Coming from sbe-tool](../getting-started/from-sbe-tool.md).

## Trust boundary

The constructor is the single trust checkpoint. Safe constructors (`wrap`,
`wrap_and_apply_header`, `decode`) validate the buffer extent once. After that
proof, all field accessors and setters are branch-free. The zero-check lane is
explicitly `unsafe fn *_unchecked`, with the complete extent precondition in
rustdoc. No per-field bounds checks exist; the single constructor proof
justifies branch-free accessors.
