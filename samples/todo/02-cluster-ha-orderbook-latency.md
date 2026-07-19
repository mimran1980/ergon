> **HISTORICAL / DONE** — residual sample work is complete. Do not re-open from this file.
> Open items: [`docs/LIVING_BACKLOG.md`](../../docs/LIVING_BACKLOG.md).

# HA cluster orderbook + dynamic latency sample

**Blocked by:** none for offline + CH latency path  
**Severity:** HIGH  
**Status: DONE (2026-07-18)** — crate `samples/cluster-ha-orderbook`; recipe
`just samples-cluster-ha`. Multi-node Java kill-leader never-stale book also
**DONE** (`just samples-cluster-ha-kill-leader` / `ha_kill_leader`, 13s green).

## Authority

[`docs/superpowers/specs/2026-07-18-cluster-ha-orderbook-sample-design.md`](../../docs/superpowers/specs/2026-07-18-cluster-ha-orderbook-sample-design.md).

## Pin decision

- `cluster-ha-orderbook` → `ergo-aeron-cluster` / rusteron **0.2.4**
- `advanced-bitget` IPC → rusteron **0.2.1** (unchanged)

## Acceptance criteria (H1–H8)

- [x] **H1** try_claim-shaped publish with ErgoSBE SessionMessageHeader + AppMessage
      (`publish::ClusterBookPublisher` / `RecordingClaimIngress`; production adapter
      `AeronClusterIngress` wraps `AeronCluster::try_claim`)
- [x] **H2** Leadership release → stale; no silent cross-term apply (`ha_book` + offline tests)
- [x] **H3** After release, resync snapshot matches reference (`failover_sequence_reference_equality`)
- [x] **H4** `LatencyPersistor`: DynamicSchema → DynamicRow → SchemaRegistry
      decode → ClickhouseSink → CH query (`feed_latency_via_latency_persistor_into_clickhouse`)
- [x] **H5** exchange→receive, receive→claim, claim→egress deltas from **decoded**
      DynamicRow (not raw SQL of sample fields)
- [x] **H6** `DynamicRecorder::record` hot path; CH via PersistSender flush
- [x] **H7** `just samples-orderbook` green (2026-07-18)
- [x] **H8** `just samples-cluster-ha` recipe documented and run (2× green)

## Evidence commands

```sh
just samples-cluster-ha
just samples-orderbook
```
