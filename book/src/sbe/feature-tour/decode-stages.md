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
