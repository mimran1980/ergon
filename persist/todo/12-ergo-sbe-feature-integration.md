# ergo_sbe feature flag integration

**Blocked by:** 11

Wire `ergo-clickhouse-persist` into ErgoSBE so users can optionally depend on it
via a feature flag.

## What to build

In `ergo_sbe/Cargo.toml`:
```toml
[features]
persist = ["dep:ergo-clickhouse-persist"]
```

In `ergo_sbe/src/lib.rs`:
```rust
#[cfg(feature = "persist")]
pub use ergo_clickhouse_persist;
```

That's it. ErgoSBE doesn't need to know about the persist crate's internals.
Users opt in:

```toml
# User's Cargo.toml
ergo_sbe = { features = ["persist"] }
```

And get access to:
```rust
use ergo_sbe::persist::{Persist, ClickhouseSink, ...};
use ergo_sbe::persist_derive::Persist;  // if we re-export the derive too
```

## Acceptance criteria

- [x] `ergo_sbe` with `features = ["persist"]` compiles
- [x] `ergo_sbe` without the feature does not pull in `clickhouse` or `ergo_clickhouse_persist`
- [x] User can `use ergo_sbe::persist::Persist` when feature is on
- [ ] Example in `samples/` showing a DTO with `#[derive(SbeMessage)]` + `#[derive(Persist)]`
- [x] `cargo test --workspace` passes with and without the persist feature
