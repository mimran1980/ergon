# samples/

End-to-end **test harnesses** for the ErgoSBE umbrella (not production apps).
**Excluded** from the workspace members set — each sample is a standalone crate.

## Map (two samples)

| Sample | Transport | Recipe | What it exercises |
|--------|-----------|--------|-------------------|
| [`advanced-bitget`](advanced-bitget/) | Aeron **IPC** | `just samples-orderbook` / `just test-ipc` | Nested AppMessage, claims, multi-schema roundtrips, Persist DTO, typed+dynamic CH |
| [`cluster-ha-orderbook`](cluster-ha-orderbook/) | Aeron **Cluster** | `just samples-cluster-ha` | `ergo-aeron-cluster` try_claim, NewLeader/never-stale book, kill-leader, feed_latency CH |

`exchange-orderbook` was **merged into advanced-bitget** (LocalBook + `OrderbookSnapshot` Persist + multi-schema roundtrips). Historical sample todos under `samples/todo/` were removed (all DONE).

### When to use which

| Need | Sample |
|------|--------|
| IPC claims + nested SBE + ClickHouse | `advanced-bitget` |
| Cluster session header, leadership freeze, HA | `cluster-ha-orderbook` |
| Nested encode recipe to copy | HA [`publish.rs`](cluster-ha-orderbook/src/publish.rs) or IPC [`publication.rs`](advanced-bitget/src/publication.rs) |

Both use **`rusteron-* = "0.2"`** (latest 0.2.x).

### Stages

| Stage | Needs | Command |
|-------|-------|---------|
| Offline unit / IPC (skip live CH) | Rust only | `just test-ipc` / `just check` sample steps |
| Live ClickHouse | Docker CH on `:8123` | `just samples-orderbook` |
| HA offline + latency CH | Docker CH | `just samples-cluster-ha` |
| Kill-leader never-stale | Java + Aeron jars | `just samples-cluster-ha-kill-leader` |

## Common preflight

```sh
curl -sf http://127.0.0.1:8123/ping || bash persist/tests/run-clickhouse.sh start
```

## ErgoSBE guides

- [claim-nested-encode.md](../sbe/docs/guide/claim-nested-encode.md)
- [generated-api.md](../sbe/docs/guide/generated-api.md)

## Non-goals

- Production matching engines or exchange connectors
- Rust Aeron Cluster **service** (Java harness owns v1)
- A third sample crate (IPC + Cluster is enough)
