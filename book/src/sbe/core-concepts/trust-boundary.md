# Trust Boundary

Every SBE buffer crossing a process boundary must be validated. In **0.1.10**,
unsuffixed constructors are the **checked** lane: they return `Result`,
validate extents once, then enter a shared private zero-check core.

There is **no** public `try_wrap*` alias. Public constructor `*_unchecked`
twins ship only if HFT-008 records `keep=true` (currently all **keep=false** —
cores stay module-private).

## Checked entry points

| Entry | Role |
|-------|------|
| `Decoder::decode(buf, pos)` / `TryFrom` | Header + template/schema + version-aware fixed extent |
| `Decoder::wrap(buf, pos, acting_block_length, version)` | External metadata path; still returns `Result` and validates the body extent |
| `Encoder::wrap` / `wrap_and_apply_header` | Capacity check for header + fixed block, then private unchecked core |
| `verify(buf)` | Walks the **complete** dynamic tail (groups, var-data); not a header-only peek |

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_try_vs_trusted}}
```
*(From the `sbe-feature-tour` sample.)*

**Do not treat `wrap` as “skip validation.”** A failed extent check is a
`DecodeError` / `EncodeError`, not silent garbage field values. For
already-proven hot paths after `decode` / `verify`, use checked field
accessors (or opt-in field `*_unchecked` only under the documented safety
contract). See [Trust boundaries (feature tour)](../feature-tour/trust-boundaries.md).
