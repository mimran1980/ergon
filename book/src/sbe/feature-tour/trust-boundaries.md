# Trust boundaries

Checked entry points validate the message header and fixed block. `verify`
walks the complete dynamic tail before bulk access:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_try_vs_trusted}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

## Where the tiers are specified

Checked `try_*`, trusted bare `wrap` / `decode`, and `unsafe *_unchecked`
are specified once, with the encoder and decoder tables, in
[Trust boundary](../core-concepts/trust-boundary.md). Prefer `try_decode`
when every failure, including a short buffer, must be a `Result`. sbe-tool
offsets: [Coming from sbe-tool](../getting-started/from-sbe-tool.md).

The constructor is the trust checkpoint. After a safe constructor proves
the header and the version-aware fixed extent, field accessors do not
re-check that extent. Dynamic tails still check when they are read. Use
`*_unchecked` only after an independent extent proof.
