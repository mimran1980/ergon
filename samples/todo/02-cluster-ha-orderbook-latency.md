# HA cluster orderbook + dynamic latency sample

**Blocked by:** none for offline + CH latency path  
**Severity:** HIGH  
**Status: DONE (2026-07-18)** — crate `samples/cluster-ha-orderbook`; recipe
`just samples-cluster-ha`. Live multi-node Java failover remains optional
harness follow-up; offline H3-equivalent proven in `ha_offline_pipeline`.

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
- [x] **H4** Latency DynamicSchema + DynamicRow + CH exact rows (`ha_latency_clickhouse`, live)
- [x] **H5** exchange→receive, receive→claim, claim→egress deltas in row + CH query
- [x] **H6** `record_into` into caller buffer (no CH block in helper)
- [x] **H7** `just samples-orderbook` green (2026-07-18)
- [x] **H8** `just samples-cluster-ha` recipe documented and run (2× green)

## Evidence commands

```sh
just samples-cluster-ha
just samples-orderbook
```
