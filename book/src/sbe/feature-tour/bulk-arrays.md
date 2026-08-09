# Bulk Arrays

For repeating groups with **fixed-size entries** (no var-data, no nested
groups), generated encoders offer a `bulk_add(&[Entry])` path that avoids
per-element closure overhead:

```rust,no_run
// Fixed-size entries only — not available when the group has var-data or nested groups.
// See samples/l3-book for a schema with eligible fixed-size entries.
```

The feature-tour Car schema uses groups with var-data tails (`fuelFigures`,
`performanceFigures`), so `bulk_add` isn't applicable there. Use the standard
`add(|e| { ... })` closure pattern for variable-size group entries:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:encode_sample_car}}
```

Constants and `MetaAttribute` expose schema metadata on every generated type:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_fixed_heartbeat}}
```
