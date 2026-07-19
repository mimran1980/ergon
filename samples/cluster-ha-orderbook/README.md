# cluster-ha-orderbook

HA sample: AppMessage L2 books via **`ergo-aeron-cluster` try_claim**,
leadership-aware never-stale book, and `feed_latency` rows through
DynamicSchema → DynamicRow → ClickHouse.

## Status

**Sample — residual H1–H8 DONE.** Multi-node Java kill-leader harness green.

## Depends on

- `ergo-aeron-cluster` → rusteron-client **`0.2`** (workspace-aligned)
- `ergosbe`, `ergo-clickhouse-persist`
- Java + Aeron jars for kill-leader (`test-harness` feature)
- Docker ClickHouse for live latency recipe

## Build / test

```sh
# Offline try_claim path + never-stale book + live feed_latency (needs CH)
just samples-cluster-ha

# Multi-node kill-leader never-stale proof (needs Java jars)
just samples-cluster-ha-kill-leader
# equivalent:
cd samples/cluster-ha-orderbook
cargo test --lib --test ha_offline_pipeline -- --test-threads=1
cargo test --features test-harness --test ha_kill_leader -- --test-threads=1
```

## Layout

| Module | Role |
|--------|------|
| `publish` | try_claim SessionMessageHeader + nested AppMessage/L2Book (**copy this**) |
| `ha_book` | Serving flag; freeze on NewLeader / gap / term mismatch |
| `follower` | Egress apply into leadership-aware book (`into_payload` stages) |
| `latency` | `LatencyPersistor` DynamicRow path |
| `market` | Wire decimals / levels |

Nested SBE recipe: [`../../sbe/docs/guide/claim-nested-encode.md`](../../sbe/docs/guide/claim-nested-encode.md).

## Failure modes

| Event | Expected behaviour |
|-------|-------------------|
| Leadership release / NewLeader | `serving=false`; `live_image()` is `None` |
| Sequence gap / term mismatch | Resync required; no silent merge of stale levels |
| Claim backpressure | Drop + count (no unbounded retry on hot path) |
| CH down | Offline tests still pass; live latency recipe fails preflight |

## Where truth lives

- Design: [`../../docs/superpowers/specs/2026-07-18-cluster-ha-orderbook-sample-design.md`](../../docs/superpowers/specs/2026-07-18-cluster-ha-orderbook-sample-design.md)
- Todo: [`../todo/02-cluster-ha-orderbook-latency.md`](../todo/02-cluster-ha-orderbook-latency.md)
- Samples map: [`../README.md`](../README.md)

## Non-goals

- Rust clustered **service** (Java harness v1)
- Replacing typed book tables with latency-only dynamic rows
