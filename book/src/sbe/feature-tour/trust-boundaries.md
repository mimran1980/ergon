# Trust Boundaries

Checked entry points validate the message header and fixed block. `verify`
walks the complete dynamic tail before trusted access:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_try_vs_trusted}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

## Checked constructors (0.2)

| Entry | When |
|-------|------|
| `Decoder::decode` / `try_from` / `wrap` | Untrusted or network input — validate template/schema ids and version-aware extents; returns `Result` |
| `Encoder::wrap` / `wrap_and_apply_header` | Same: one cold capacity check then shared private zero-check core |

There is **no** public `try_wrap*` alias and **no** public `*_unchecked`
constructor twin unless HFT-008 records `keep=true` (currently all keep=false:
cores are module-private). Offsets are **message start** (not sbe-tool body
offset). See [Coming from sbe-tool](../getting-started/from-sbe-tool.md).

## `_unchecked` field accessors (supported opt-in)

With `GenerationConfig::with_unchecked_companions(true)`, each field getter
gains a `*_unchecked` companion that skips a *redundant* per-field bounds
check **after** a checked constructor / `verify` has accepted the buffer.

**Intended use:** production HFT hot loops only on a proven extent.

**Contract (caller’s responsibility):**

1. Validate with `decode` / `try_from` / `wrap` / `verify` before any
   field-level `_unchecked` call.
2. Do not carry unchecked access across a consuming stage transition
   (`into_fuel_figures()`, etc.) — position advances and the prior guard
   no longer applies.
3. Calling field `_unchecked` without a proven extent is a programmer bug:
   out-of-bounds raw reads are **undefined behaviour**, not “garbage but
   safe”. Prefer checked accessors at every untrusted seam.

Checked accessors remain the default surface. Full wording:
`GenerationConfig::with_unchecked_companions` rustdoc.
