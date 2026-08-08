# Include Generated Code

**Prefer build-dep only for product crates** (no runtime `ergo-sbe` link).
Generated codecs embed `sbe_rt`; plain `include!` is enough:

```rust,ignore
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:include_build_dep_only}}
```
*(From `sbe-feature-tour/src/lib.rs` — the `#[path]` pattern used by every sample crate. The feature-tour's `cargo build` verifies this compiles.)*

or via `include!`:

```rust,ignore
mod messages {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}
use messages::*;
```

**Optional convenience** — `sbe_mod!` needs `ergo-sbe` as a normal dependency
(macro expansion only; not required for encode/decode):

```rust,ignore
// Cargo.toml: [dependencies] ergo-sbe = "0.1"
ergo_sbe::sbe_mod!(messages);
use messages::*;
// Or only the include: ergo_sbe::include_sbe!("messages");
```

See [Samples](../../samples/overview.md)
for which crates use which pattern.
