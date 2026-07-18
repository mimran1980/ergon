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
| Dynamic latency rows without a new Persist DTO schema | HA `LatencyPersistor` (or copy its pattern) |

**Do not dual-pin** rusteron 0.2.1 and 0.2.4 in one binary — that is why HA and
IPC samples stay separate crates.

### Shared failure modes

| Condition | advanced-bitget / exchange | cluster-ha-orderbook |
|-----------|----------------------------|----------------------|
| ClickHouse down | Live CH tests fail preflight | Offline book tests still pass; `just samples-cluster-ha` live stage fails |
| Claim / offer backpressure | N/A (IPC) or drop policy | `PublishOutcome::Dropped`; never unbounded retry on hot path |
| Leadership change | N/A | `serving=false`; `live_image() == None` until term-valid snapshot |
| Sequence gap / term mismatch | N/A | Resync; no silent merge of old-term levels |

## Common preflight

```sh
# ClickHouse (Docker) for live latency / book rows
curl -sf http://127.0.0.1:8123/ping || bash persist/tests/run-clickhouse.sh start
```

## Non-goals

- Production exchange matching engines
- Renaming `samples/` or pillar dirs
- Rust Aeron Cluster **service** (Java harness owns v1 clustered service)
