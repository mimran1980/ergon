# HA cluster orderbook + dynamic latency sample

**Blocked by:** rusteron pin decision for full HA binary (0.2.1 IPC vs 0.2.4
cluster); Java failover harness wiring for H3/H8
**Severity:** HIGH  
**Status: IN PROGRESS (2026-07-18)** — pure `ha_book` + `latency` modules landed
in `samples/advanced-bitget` with unit tests; cluster connect re-offer +
log-recovery residual reliability are DONE on the client side.

## Authority

Implement
[`docs/superpowers/specs/2026-07-18-cluster-ha-orderbook-sample-design.md`](../../docs/superpowers/specs/2026-07-18-cluster-ha-orderbook-sample-design.md).

Canonical SBE / performance rules remain in `sbe/design/DECISIONS.md`.
Residual umbrella order:
[`docs/superpowers/plans/2026-07-18-completion-goal-prompt.md`](../../docs/superpowers/plans/2026-07-18-completion-goal-prompt.md).

This is the **HA successor** to the completed IPC sample in
`01-bitget-aeron-app-message.md`. Do not reinterpret that DONE record as HA
evidence. Keep dirs under `samples/`; never rename pillars.

## Problem

`samples/advanced-bitget` proves Bitget → AppMessage → Aeron **IPC** →
ClickHouse. It does not use `ergo-aeron-cluster`, so leadership releases
(NewLeader, session close, reconnect) are out of scope and a follower can
theoretically keep a **stale orderbook** across failover. There is also no
**latency** observability table on the dynamic path for that HA path.

## Goal

```text
feed → AppMessage(L2Book|Trade)
  → ergo-aeron-cluster try_claim (HA ingress)
  → Java clustered service (v1 harness)
  → egress follower with leadership-aware book (never serve across release
    without term-valid snapshot)
  → latency DynamicSchema + DynamicRow → ClickHouse
```

## Sub-tasks

1. **Stale-book state machine** — DONE: `advanced-bitget/src/ha_book.rs` (7 unit tests).
2. **Publisher** — `AeronCluster::try_claim` for AppMessage (and optional
   dynamic book rows); drop-on-backpressure counters. **OPEN** (needs pin).
3. **Follower** — egress poll; apply policy; CH typed books as needed. **OPEN**.
4. **Latency** — DONE offline: `advanced-bitget/src/latency.rs` builds
   DynamicSchema fields + encodes DynamicRow into caller buffer (2 unit tests).
   CH insert + schema publish on live path still OPEN.
5. **Failover harness** — kill leader mid-stream; assert no stale serve;
   book == reference after resync. **OPEN**.
6. **Recipe** — `just samples-cluster-ha` (jars + CH + sample). **OPEN**.
7. **Regression** — `just samples-orderbook` still green (re-verify after wire-up).
8. **Pin decision** — record rusteron 0.2.1 vs 0.2.4 before shared binary. **OPEN**.

## Acceptance criteria (H1–H8)

- [ ] **H1** Publisher uses `ergo-aeron-cluster` `try_claim` (or documented equal)
- [x] **H2** Leadership change / session release → book stale; no silent cross-term apply
      (unit: `ha_book::tests::*`, 2026-07-18)
- [ ] **H3** After failover, consistent book before serving (assert vs reference)
- [ ] **H4** Latency DynamicSchema once + DynamicRow rows visible in ClickHouse
- [x] **H5** At least exchange→receive, receive→claim, claim→egress deltas encoded
      in DynamicRow (unit: `latency::tests::*`; live CH still open under H4)
- [x] **H6** Hot-path latency uses `record_into` into caller buffer (no CH block in helper)
- [ ] **H7** IPC baseline `just samples-orderbook` still green
- [ ] **H8** Documented runnable HA recipe

## Done means

All H1–H8 checked with **fresh command output** recorded in this file or the
master plan ledger. Design-only or compile-only does **not** count.
