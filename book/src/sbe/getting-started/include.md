# Include generated code

Product crates should depend on `ergo-sbe` at build time only, with no runtime link.
Generated codecs embed `sbe_rt`, so plain `include!` is enough:

```rust,ignore
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:include_build_dep_only}}
```
*(From `sbe-feature-tour/src/lib.rs`: the `#[path]` pattern used by every sample crate. The feature tour's `cargo build` checks that it compiles.)*

or via `include!`:

```rust,ignore
mod messages {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}
use messages::*;
```

`sbe_mod!` is optional and needs `ergo-sbe` as a normal dependency
(for macro expansion only, not for encode or decode):

```rust,ignore
// Cargo.toml: [dependencies] ergo-sbe = "0.1"
ergo_sbe::sbe_mod!(messages);
use messages::*;
// Or only the include: ergo_sbe::include_sbe!("messages");
```

See [Samples](../../samples/overview.md)
for which crates use which pattern.
