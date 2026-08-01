# Cluster HA Orderbook

Claim-based Cluster publishing with an HA-shaped limit order book. Proves
`try_claim` patterns and never-stale book snapshots under leader transitions.

- Offline pipeline (`ha_offline_pipeline`) — claim + encode + verify without a
  live cluster.
- Kill-leader test (`ha_kill_leader`) — validates book integrity across leader
  changes (requires Java harness).

Source: [`samples/cluster-ha-orderbook/src/`](https://github.com/mimran1980/ergon/tree/main/samples/cluster-ha-orderbook/src)

```sh
# Service-free
cargo test --manifest-path samples/cluster-ha-orderbook/Cargo.toml \
  --lib --test ha_offline_pipeline

# Full harness (needs Java)
just build-aeron-jars
cargo test --manifest-path samples/cluster-ha-orderbook/Cargo.toml \
  --features test-harness
```
