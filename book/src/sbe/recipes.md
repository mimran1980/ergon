# Recipes

Runnable code: [sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour) `src/lib.rs`.

- [Aeron try_claim](recipes/aeron-try-claim.md)
- [Display / Debug](recipes/display-debug.md)
- [Schema descriptions as rustdoc](recipes/schema-rustdoc.md)
- [Domain DTOs](recipes/domain-dtos.md)
- [App types on composites](recipes/app-types-composites.md)
- [Timestamp conversions](recipes/timestamps.md)

## Quick reference

**Known vs unknown group count:**

```rust,no_run
{{#include ../../../samples/sbe-feature-tour/src/lib.rs:encode_sample_car}}
```
*(From `sbe-feature-tour`: known-count groups with chaining, tested in CI.)*

**Unknown size:** count back-patched after the closure (streaming producers).

```rust,ignore
let unknown_len = CarEncoder::wrap_and_apply_header(&mut buf, 0)
    .fixed(&fields)
    .fuel_figures_unknown_size(|g| {
        for row in rows {
            g.add(|mut e| {
                e.speed(row.speed).mpg(row.mpg);
                e.usage_description_as_str(&row.usage)
            })?;
        }
        Ok(())
    })?
    .performance_figures(0, |_| Ok(()))?
    .manufacturer_as_str("Honda")?
    .model_as_str("Civic")?
    .activation_code_as_str("active")?
    .encoded_length_with_header();

println!("known={known_len} unknown={unknown_len}");
```

## Unknown enum values

A wire discriminant the schema never declared reaches you three different
ways. Pick by what should happen:

| What you want | Use | Behaviour |
|---|---|---|
| Keep decoding; unknown means "not set" | Typed getter, e.g. `dec.code()` | `from_raw` folds every undeclared discriminant into `NullVal`; the decode continues |
| See the byte that arrived | `dec.raw_code()` | The raw discriminant unchanged (`u8`, …); schema null and unknown stay distinguishable (named `*_wire` when a conversion owns the plain name) |
| Refuse the frame | `GenerationConfig::with_validate(true)`, then `dec.validate()?` | `ValidateError::UnknownEnum { field, discriminant }` |

```text
let code = dec.code();   // unknown → EventCode::NullVal; decode continues
dec.raw_code()           // 42u8 — the wire byte, unfolded
dec.validate()?          // with_validate(true): Err(UnknownEnum { discriminant: 42 })
```

The fold is lossy only in the typed view: a `match` on `dec.code()` cannot
tell the schema's null from an unknown value — compare `raw_*` for that, or
make the unknown a hard error with `validate()`. `validate()` is opt-in and
walks every field and group entry, so keep it off the hot path
([Value validation](core-concepts/trust-boundary.md#value-validation-opt-in));
the fold itself is explained in
[Why NullVal instead of Option](design-notes/nullval.md).
