# Trust Boundaries

Checked entry points validate the message header and fixed block. `verify`
walks the complete dynamic tail before trusted access:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_try_vs_trusted}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*
