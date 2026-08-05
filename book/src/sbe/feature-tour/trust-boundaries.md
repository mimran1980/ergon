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
| **Checked** | `try_wrap` / `try_wrap_and_apply_header` / `try_decode` | `Result::Err` (all failures) |
| **Trusted** | bare `wrap` / `wrap_and_apply_header` | **panic** after the same extent proof |
| **Trusted hybrid** | bare `decode` | **panic** if short; **`Err`** on wrong template/schema only |
| **Unchecked** | `unsafe wrap_unchecked` / `wrap_and_apply_header_unchecked` | **UB** — prove extent first |
| **Unchecked hybrid** | `unsafe decode_unchecked` | **UB** on OOB extent; **`Err`** on wrong template/schema |

Bare `decode` looks like `try_decode` (`Result`) but short buffers still panic
— prefer `try_decode` when every failure must be a `Result`. Full table:
[Trust Boundary (core concepts)](../core-concepts/trust-boundary.md). sbe-tool
offsets: [Coming from sbe-tool](../getting-started/from-sbe-tool.md).

## Trust boundary

The constructor is the single trust checkpoint. After a safe constructor
proves header + version-aware fixed extent, field accessors and setters are
branch-free (unchecked loads/stores justified by that proof). Dynamic
group/var-data tails still check on consume. Use `*_unchecked` only when you
have proven the fixed extent independently (benchmarks, pre-validated buffers).
