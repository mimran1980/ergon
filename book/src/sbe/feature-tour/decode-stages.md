# Consuming Decode Stages

Groups and var-data are consumed in schema order. `finish()` hands the next
named stage back to you:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_decode_stages}}
```

Each `into_*_as_str()` returns `(&'a str, NextStage<'a>)` — the `&str` borrows
from the original wire buffer, not from the consumed stage. All three strings
remain valid simultaneously while the stage chain advances.

*(This code comes from the `sbe-feature-tour` sample crate.)*

## `#[must_use]` on stages

Consuming stages (`CarDecoderAfterFuelFigures`, `…AfterManufacturer`,
`CarDecoderComplete`, …) are `#[must_use]`. Dropping a stage without
`into_*` / `finish` / `skip_remaining` **silently skips** remaining wire
tails (groups and var-data). That is easy to miss when a function returns
early — prefer advancing until `Complete` or an explicit skip.

## `finish` vs `skip_remaining`

| Method | Meaning |
|--------|---------|
| `finish()` | Advance past any **remaining entries** of the current group and hand back the next named stage (or complete). |
| `skip_remaining()` | Explicit sequential spelling of the same idea — “I am done with this group; jump to the next tail.” |

Use `skip_remaining` when you want the intent obvious in review; both move the
tail cursor in wire order.

## Full-frame bytes mid-walk

| Need | API |
|------|-----|
| Full frame after finishing the walk | complete stage `as_bytes_with_header()` |
| Full frame without consuming stages | inherent `dec.as_bytes_with_header()?` (rescans tails) |
| Fixed block only (not a full frame) | `dec.get_metadata().as_fixed_region_with_header()?` |

See [Generated code](generated-code.md#metadata-limits-tailed-messages) for the
metadata `limit` vs full-frame table.
