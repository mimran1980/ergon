# Cluster HA Orderbook Sample Design

**Status:** **IMPLEMENTED** for residual scope (2026-07-18) in
`samples/cluster-ha-orderbook` + `just samples-cluster-ha`. Design remains the
authority; checkboxes in §9 track shipped proofs.

**Mode for this file:** design authority for the HA sample track.

**Related:**

- Living plan: [`../plans/2026-07-18-ergosbe-experimental-master-plan.md`](../plans/2026-07-18-ergosbe-experimental-master-plan.md)
- Goal prompt: [`../plans/2026-07-18-completion-goal-prompt.md`](../plans/2026-07-18-completion-goal-prompt.md)
- Todo: [`../../../samples/todo/02-cluster-ha-orderbook-latency.md`](../../../samples/todo/02-cluster-ha-orderbook-latency.md)
- IPC baseline design (complete): [`2026-07-10-bitget-aeron-clickhouse-sample-design.md`](2026-07-10-bitget-aeron-clickhouse-sample-design.md)
- Cluster client: `cluster/` crate `ergo-aeron-cluster`
- Canonical SBE rules: [`../../../sbe/design/DECISIONS.md`](../../../sbe/design/DECISIONS.md)

## 1. Purpose

Prove a production-shaped **high-availability** market-data path:

```text
Bitget (or offline fixture) feed
  → normalize L2Book / Trade inside AppMessage (existing schema family)
  → publish via ergo-aeron-cluster try_claim into Cluster ingress
  → Java clustered service (v1) owns / echoes canonical sequenced books
  → Rust client(s) on egress rebuild / follow the book with leadership awareness
  → never serve a stale orderbook across leadership releases
  → record latency spans to ClickHouse via DynamicSchema + DynamicRow
```

This is the **HA successor** to `samples/advanced-bitget` (local Aeron IPC).
The IPC sample remains the non-HA baseline and must stay green
(`just samples-orderbook`).

### Non-goals (first cut)

- Full exchange matching engine or multi-symbol OMS.
- Rust Aeron Cluster **service** implementation (client only; Java harness v1).
- Renaming `samples/`, `cluster/`, or any pillar directory.
- ~~Unfreezing RFQ codecs~~ (done separately on cluster crate, 2026-07-18).
- Replacing typed book tables; latency uses **dynamic** table/row only.
- Claiming “HFT-ready” release quality beyond measured gates.

## 2. Priorities

Apply the repository ladder:

1. Official-SBE wire compatibility.
2. Maintained measured paths ≤ 1.00 vs their baseline (Aeron SBE / sbe-tool as
   applicable).
3. Safer/easier API when zero-cost or off hot path.
4. No free safety tax on benchmarked hot paths.
5. Simplicity only when 1–4 are equal.

Sample-specific constraints:

- Prefer `AeronCluster::try_claim` + SessionMessageHeader (ErgoSBE) for HA
  publish — encode directly into the claim; no temp buffer + copy on success
  path.
- Drop (do not block/retry unbounded) under claim backpressure; count drops.
- Latency recording must not introduce unbounded queues or blocking CH waits
  beyond the existing advanced-bitget foreground batch/drop policy.
- On leadership **release** (NewLeader, session close, forced reconnect): mark
  book **not serving** until a term-valid snapshot is applied.

## 3. Architecture relative to advanced-bitget

| Concern | advanced-bitget (DONE) | HA sample (this design) |
|---------|------------------------|-------------------------|
| Transport | Aeron IPC, local driver | Aeron Cluster ingress/egress |
| Client crate | `rusteron-client` 0.2.1 direct | `ergo-aeron-cluster` (rusteron 0.2.4 today) |
| HA / failover | None | NewLeader + reconnect + stale policy |
| Book authority | Local publisher state | Cluster log / service sequencing |
| Typed books/trades | AppMessage + Persist tables | Same family (extend as needed) |
| Dynamic stream | L2 dynamic book rows | **Plus** latency DynamicSchema/Row |
| Recipe | `just samples-orderbook` | `just samples-cluster-ha` (to add) |

### Roles

1. **Feed publisher client** — WS or fixture → normalize → `try_claim` publish
   of AppMessage payloads (and optional dynamic book rows) into cluster
   ingress. Tags messages with sequence; includes `sentTs` / receive times.
2. **Clustered service (v1 Java harness)** — existing test jars / launcher
   path used by `cluster` integration tests. First cut may be echo or minimal
   sequenced forwarder; if a real book service is required for snapshot, keep
   it minimal and harness-owned. **Rust clustered service is non-goal for v1.**
3. **Book follower client** — egress poll; leadership-aware apply; exposes
   serving flag; persists books + latency rows to ClickHouse.
4. **Optional:** same process can be publisher+follower for single-binary demo;
   tests should still cover dual-client failover.

## 4. Stale-book policy (“never stale across releases”)

**Release** means any of: `NewLeaderEvent`, session disconnect/reconnect,
explicit `SessionClose` / publisher restart, or detected sequence gap.

```text
state:
  leadership_term_id: Option<i64>
  last_seq: u64
  serving: bool = false
  book: LocalBook (frozen when !serving)

on NewLeaderEvent / reconnect / session release:
  serving = false
  freeze or clear book (implementation may keep last image for UI debug
    but MUST NOT treat it as live)
  await term-valid snapshot (full L2Book for new term, or service snapshot)

on L2Book snapshot with term T, seq S:
  replace book entirely
  leadership_term_id = T
  last_seq = S
  serving = true

on L2Book/Trade increment with term T, seq S:
  if !serving: drop (or buffer only if bounded resync buffer — default: drop)
  else if T != leadership_term_id: serving = false; resync
  else if S != last_seq + 1: serving = false; resync
  else: apply; last_seq = S; emit latency row (best-effort)

on serve/query path:
  if !serving: return Stale / NotReady — never silent last-good as live
```

Harness proof: kill elected leader mid-stream → follower sees NewLeader →
`serving=false` → after snapshot, book equals reference rebuild (no merged
stale levels from old term).

## 5. Latency model (DynamicSchema + DynamicRow → ClickHouse)

Reuse `ergo-clickhouse-persist` dynamic V2 path already proven in
advanced-bitget. **No new Persist derive DTO required** for latency.

### Schema registration (once per process / session)

Table name (suggested): `feed_latency`

| Field | Type | Notes |
|-------|------|--------|
| `instrument` | String | e.g. BTCUSDT |
| `leadership_term_id` | Int64 | cluster term |
| `cluster_session_id` | Int64 | session |
| `sequence` | UInt64 | book/trade seq |
| `exchange_ts_ns` | UInt64 | from exchange / fixture |
| `receive_ts_ns` | UInt64 | feed receive |
| `ingress_claim_ts_ns` | UInt64 | after successful claim commit |
| `egress_decode_ts_ns` | UInt64 | follower decode |
| `book_apply_ts_ns` | UInt64 | after apply when serving |
| `ch_enqueue_ts_ns` | UInt64 | before batch insert |
| `exchange_to_receive_ns` | Int64 | derived |
| `receive_to_claim_ns` | Int64 | derived |
| `claim_to_egress_ns` | Int64 | derived |
| `e2e_ns` | Int64 | exchange → book_apply (or document subset) |

Publish `DynamicSchemaV2` once on the dynamic stream (or dedicated latency
stream if needed for isolation). Emit `DynamicRowV2` per accepted apply
(or sampled — document default: every apply while serving; drop on
backpressure).

### Hot-path policy

- Prebuild `DynamicRecorder` / positional values; avoid per-row heap where the
  existing dynamic path already guarantees that.
- CH insert: foreground batch with flush thresholds **or** drop-on-full —
  match advanced-bitget; never block claim/publish on CH.
- Missing timestamps: use 0 + document; do not panic.

## 6. Threading model

Align with advanced-bitget three-thread ownership where possible:

1. **Ingest / publish thread** — feed, normalize, cluster `try_claim`, keep-alive
   / poll ingress as required by client API.
2. **Media / driver** — reuse SHARED or cluster test topology; document exact
   driver ownership (may be Java cluster nodes + client embedded driver).
3. **Follow / persist thread** — egress poll, stale-book state machine, CH
   batch.

Cluster client is poll-driven (`poll_egress` / async connect). Do not add
unbounded internal queues between claim and persist.

## 7. Dependency pins

| Crate | advanced-bitget today | cluster today | HA sample decision |
|-------|----------------------|---------------|-------------------|
| rusteron-client | `=0.2.1` | `0.2.4` | **Decide one pin before coding**; prefer align to cluster’s 0.2.4 if measured OK; ask human before bump |
| rusteron-media-driver | `=0.2.1` | (via tests) | Same decision |
| ergo-aeron-cluster | unused | path dep | **required** |
| ergosbe / persist | path | path | path |

Do not silently dual-pin two rusteron versions in one binary.

## 8. Test plan

| Layer | What | Command shape |
|-------|------|----------------|
| Unit | stale-book state machine (term change, gap, snapshot) | `cargo test` in sample |
| Codec | AppMessage + latency DynamicRow roundtrip | sample tests |
| Harness | 3-node cluster, kill leader, book consistency | `test-harness` feature + jars |
| CH | exact rows for latency table + book tables | Docker ClickHouse, `#[ignore]` live OK if recipe runs them |
| Regression | IPC baseline unchanged | `just samples-orderbook` |
| Recipe | one-shot HA demo | `just samples-cluster-ha` |

Allocation: warmed claim + encode + latency record should match existing
sample allocation discipline; prove or document bounds.

## 9. Acceptance checklist (H1–H8)

- [x] **H1** try_claim-shaped publish: `SessionMessageHeader` (ErgoSBE) +
      AppMessage via `ClusterBookPublisher` / `AeronClusterIngress` (wraps
      `AeronCluster::try_claim`) — `publish` tests + offline pipeline.
- [x] **H2** Leadership release → book stale; no silent cross-term apply —
      `ha_book` + offline pipeline.
- [x] **H3** After release, resync snapshot matches reference —
      `failover_sequence_reference_equality` (offline) + multi-node Java
      kill-leader `ha_kill_leader` / `just samples-cluster-ha-kill-leader`.
- [x] **H4** Latency via shipped `LatencyPersistor`: DynamicSchema once →
      DynamicRow encode → SchemaRegistry/RowDecoder → ClickhouseSink → CH query
      (`feed_latency_via_latency_persistor_into_clickhouse`).
- [x] **H5** exchange→receive, receive→claim, claim→egress, e2e deltas in
      DynamicRow and CH exact-row query.
- [x] **H6** Hot-path encode uses `DynamicRecorder::record` (reuse buffer);
      CH is sink flush, not blocking encode.
- [x] **H7** `just samples-orderbook` still green.
- [x] **H8** `just samples-cluster-ha` recipe documented and green.

Samples todo `02-cluster-ha-orderbook-latency.md` is DONE with the same evidence.

## 10. Implementation sequencing (after this design freeze)

1. Cluster residual reliability (connect re-offer, log-recovery) green enough.
2. rusteron pin decision recorded in master plan.
3. Stale-book module + unit tests (no cluster required).
4. Wire publisher to `AeronCluster::try_claim`.
5. Follower egress + CH books.
6. Latency DynamicSchema/Row.
7. Failover harness + CH exact rows.
8. `just samples-cluster-ha` + docs.

## 11. Open decisions (ask human if blocking)

- Exact clustered service behaviour (pure echo vs snapshot API).
- rusteron 0.2.1 → 0.2.4 bump for any shared sample code.
- Whether latency shares the dynamic book stream or a third stream id.
- Sampling vs every-message latency rows under load.
