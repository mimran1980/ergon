# Recipes

Runnable, tested code for every pattern lives in [sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour). See its `src/lib.rs` for the full API map.

- [Aeron try_claim](recipes/aeron-try-claim.md)
- [Display / Debug](recipes/display-debug.md)
- [Schema Descriptions → Rustdoc](recipes/schema-rustdoc.md)
- [Domain DTOs](recipes/domain-dtos.md)
- [App Types on Composites](recipes/app-types-composites.md)
- [Timestamp Conversions](recipes/timestamps.md)

## Quick reference

**Known vs unknown group count:**

```rust,no_run
{{#include ../../../samples/sbe-feature-tour/src/lib.rs:encode_sample_car}}
```
*(From `sbe-feature-tour` — known-count groups with chaining, tested in CI.)*

**Unknown size:** count back-patched after the closure (streaming producers).

```rust,ignore
let unknown_len = CarEncoder::wrap_and_apply_header(&mut buf, 0)
    .fixed(&fields)
    .fuel_figures_unknown_size(|g| {
        for row in rows {
            g.add(|e| {
                e.speed(row.speed).mpg(row.mpg);
                Ok(())
            })?;
        }
        Ok(())
    })?
    .performance_figures(0, |_| Ok(()))?
    .manufacturer(b"Honda")?
    .model(b"Civic")?
    .activation_code(b"active")?
    .encoded_length_with_header()?;

println!("known={known_len} unknown={unknown_len}");
```
