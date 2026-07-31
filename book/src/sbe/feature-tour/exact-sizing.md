# Exact Sizing

Dynamic messages expose schema-aware size APIs. Flat shapes get a direct
checked helper; nested or ragged shapes get a staged `*EncodedLength` builder.
Allocate or claim exactly that many bytes, then write groups and var-data in
wire order:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_size_and_encode}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*
