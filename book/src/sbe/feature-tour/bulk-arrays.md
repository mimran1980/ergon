# Bulk Arrays

For repeating groups with **fixed-size entries** (no var-data, no nested
groups), generated encoders offer a `bulk_add(&[Entry])` path that validates
the destination region once and writes every entry.

Car `fuelFigures` has var-data (`usageDescription`), so it is **not** eligible.
The nested `acceleration` group is — each row is `mph` + `seconds` only:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_bulk_add}}
```
*(From `samples/sbe-feature-tour` — compiled and run in that crate's tests.)*

`bids` / `asks` on the l3-book schema have a nested `orders` group, so those
outer groups are not eligible either. Only a **leaf** group whose entries are
a pure fixed block gets `bulk_add`.

Constants and `MetaAttribute` expose schema metadata on every generated type
(`HeartbeatDecoder::sequence_meta_attribute(MetaAttribute::Presence)` and
friends). See the generated module after `cargo build` of the feature-tour
sample.
