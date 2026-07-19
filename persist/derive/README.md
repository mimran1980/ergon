# ergo-clickhouse-persist-derive

Proc-macro crate for `#[derive(Persist)]` used with `ergo-clickhouse-persist`.

## Status

**Experimental.** Dev-dependency style usage is normal for apps that only
persist on the consumer path.

## Depends on

- Parent crate: [`../README.md`](../README.md) (`ergo-clickhouse-persist`)

## Build / test

```sh
# From repo root — derive is exercised via persist tests
cargo test -p ergo-clickhouse-persist --lib
```

## Usage

```toml
[dependencies]
ergo-clickhouse-persist = { path = ".." }
ergo-clickhouse-persist-derive = { path = "." }
```

```rust
use ergo_clickhouse_persist_derive::Persist;

#[derive(Persist)]
struct Order {
    price: u64,
    qty: u32,
}
```

See the parent README for annotations (`skip`, `flatten`, `array`, `json`, …)
and schema migration behaviour.

## Non-goals

- Hot-path market-data encode/decode (that is `ergo-sbe` + your feed path)
- Standalone use without `ergo-clickhouse-persist`
