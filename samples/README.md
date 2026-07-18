# samples/

End-to-end demos for the ErgoSBE umbrella. **Excluded** from the workspace
members set — each sample is a standalone crate.

## Map

| Sample | Transport | rusteron pin | Recipe | Purpose |
|--------|-----------|--------------|--------|---------|
| [`advanced-bitget`](advanced-bitget/) | Aeron **IPC** (local) | **0.2.1** | `just samples-orderbook` | WS → AppMessage + dynamic V2 → ClickHouse |
| [`exchange-orderbook`](exchange-orderbook/) | In-process / optional CH | n/a | `just samples-orderbook` | Multi-exchange normalize → local book → CH |
| [`cluster-ha-orderbook`](cluster-ha-orderbook/) | Aeron **Cluster** | **0.2.4** (via `ergo-aeron-cluster`) | `just samples-cluster-ha` | try_claim publish, never-stale book, feed_latency CH |

### When to use which

| Need | Sample |
|------|--------|
| Single-process feed + CH exact rows, no leadership | `advanced-bitget` |
| Multi-venue schema roundtrip / local book | `exchange-orderbook` |
| Leadership-aware book + cluster publish + failover | `cluster-ha-orderbook` |

**Do not dual-pin** rusteron 0.2.1 and 0.2.4 in one binary — that is why HA and
IPC samples stay separate crates.

## Common preflight

```sh
# ClickHouse (Docker) for live latency / book rows
curl -sf http://127.0.0.1:8123/ping || bash persist/tests/run-clickhouse.sh start
```

## Non-goals

- Production exchange matching engines
- Renaming `samples/` or pillar dirs
- Rust Aeron Cluster **service** (Java harness owns v1 clustered service)
