# Cluster HA Orderbook

Claim-based Cluster publishing + HA-shaped book.

May still depend on cluster. Java harness only for leader-kill coverage.

```sh
cargo check --manifest-path samples/cluster-ha-orderbook/Cargo.toml --all-targets
cargo test --manifest-path samples/cluster-ha-orderbook/Cargo.toml \
  --lib --test ha_offline_pipeline
```
