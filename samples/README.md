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
| Copy-paste nested SBE encode | HA [`publish.rs`](cluster-ha-orderbook/src/publish.rs) + [claim-nested-encode guide](../sbe/docs/guide/claim-nested-encode.md) |

**Do not dual-pin** rusteron 0.2.1 and 0.2.4 in one binary — that is why HA and
IPC samples stay separate crates.

### Shared failure modes

| Condition | advanced-bitget / exchange | cluster-ha-orderbook |
|-----------|----------------------------|----------------------|
| ClickHouse down | Live CH tests fail preflight | Offline book tests still pass; live stage fails |
| Claim / offer backpressure | Drop policy | `PublishOutcome::Dropped`; no unbounded retry |
| Leadership change | N/a | `serving=false`; `live_image() == None` until term-valid snapshot |
| Sequence gap / term mismatch | N/a | Resync; no silent merge of old-term levels |

### Stages (always vs live)

| Stage | Needs | Command |
|-------|-------|---------|
| Offline unit / try_claim path | Rust only | `cargo test` in each sample; `just samples-cluster-ha` offline steps |
| Live ClickHouse | Docker CH on `:8123` | `just samples-orderbook` / HA latency stage |
| Kill-leader never-stale | Java 17+ + Aeron jars | `just samples-cluster-ha-kill-leader` |

## Common preflight

```sh
curl -sf http://127.0.0.1:8123/ping || bash persist/tests/run-clickhouse.sh start
```

## ErgoSBE API used here

- Consuming decoder stages: [`sbe/docs/guide/generated-api.md`](../sbe/docs/guide/generated-api.md)
- Claim + nested AppMessage: [`sbe/docs/guide/claim-nested-encode.md`](../sbe/docs/guide/claim-nested-encode.md)
- Group `add` / `bids` (unit or `Result` closures), `payload_with`, framing consts

## Non-goals

- Production exchange matching engines
- Renaming `samples/` or pillar dirs
- Rust Aeron Cluster **service** (Java harness owns v1 clustered service)
