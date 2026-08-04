# Trust Boundaries

Checked entry points validate the message header and fixed block. `verify`
walks the complete dynamic tail before bulk access:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_try_vs_trusted}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

## Three-tier constructors (0.1.12+)

| Tier | Entry | Bad buffer |
|------|-------|------------|
| **Checked** | `try_wrap` / `try_wrap_and_apply_header` / `try_decode` | `Result::Err` |
| **Trusted** | bare `wrap` / `wrap_and_apply_header` / `decode` | **panic** after the same extent proof |
| **Unchecked** | `unsafe fn *_unchecked` | **UB** — prove extent first |

Offsets are **message start** (not sbe-tool body offset). See
[Trust Boundary (core concepts)](../core-concepts/trust-boundary.md) for the
full table and migration notes, and [Coming from sbe-tool](../getting-started/from-sbe-tool.md).

## Trust boundary

The constructor is the single trust checkpoint. After a safe constructor
proves header + version-aware fixed extent, field accessors and setters are
branch-free (unchecked loads/stores justified by that proof). Dynamic
group/var-data tails still check on consume. Use `*_unchecked` only when you
have proven the fixed extent independently (benchmarks, pre-validated buffers).
