# Trust Boundaries

Checked entry points validate the message header and fixed block. `verify`
walks the complete dynamic tail before trusted access:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_try_vs_trusted}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

## `try_*` vs trusted `wrap`

| Entry | When |
|-------|------|
| `try_from` / `try_wrap` / `try_wrap_and_apply_header` | Untrusted or network input — validate template_id, schema_id, bounds |
| `wrap` / `wrap_and_apply_header` | After a trust boundary (you already validated, or you own the buffer) |

Both families take **message start** as the offset (not sbe-tool’s body
offset). See [Coming from sbe-tool](../getting-started/from-sbe-tool.md).

## `_unchecked` field accessors (supported opt-in)

With `GenerationConfig::with_unchecked_companions(true)`, each field getter
gains a `*_unchecked` companion that skips the per-field bounds check.

**Intended use:** production HFT hot loops **after** `try_from` / `try_wrap`
/ `verify` (or an equivalent application check) has accepted the buffer.
This is not a bench-only secret API.

**Contract (caller’s responsibility):**

1. Validate before any `_unchecked` call.
2. Do not carry unchecked access across a consuming stage transition
   (`into_fuel_figures()`, etc.) — position advances and the prior guard
   no longer applies.
3. Malformed buffers yield garbage values, not undefined behaviour (still a
   valid `&[u8]`).

Checked accessors remain the default surface. Full wording:
`GenerationConfig::with_unchecked_companions` rustdoc.
