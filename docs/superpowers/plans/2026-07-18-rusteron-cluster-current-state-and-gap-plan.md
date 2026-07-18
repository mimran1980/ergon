# Rusteron Cluster — Current State & Gap-Closure (P0)

> **SUPERSEDED (2026-07-19).** Historical session notes only. Do **not** treat the
> residual list below as open work.
>
> **Closed later on `first_cut`:** connect re-offer, log-recovery restart test,
> ErgoSBE production codecs, RFQ **unfrozen**, maintained encode/decode benches,
> HA sample + kill-leader. Living product truth:
> [2026-07-18-ergosbe-experimental-master-plan.md](2026-07-18-ergosbe-experimental-master-plan.md),
> [2026-07-18-completion-goal-prompt.md](2026-07-18-completion-goal-prompt.md),
> verified-open backlog only: [`../../LIVING_BACKLOG.md`](../../LIVING_BACKLOG.md).
>
> **Original framing (stale):** Written when the crate was `rusteron-cluster`;
> now `ergo-aeron-cluster` in `cluster/`. The “residual still active” paragraph
> that once listed re-offer / log-recovery / RFQ freeze is **obsolete**.

**Date:** 2026-07-18
**Branch:** `cluster` (work later landed on `first_cut`)
**Supersedes (for the items below):** `2026-07-17-rusteron-cluster-final-report.md`

This records the reliability-first gap-closure work done on top of the
2026-07-17 report. Every claim here is backed by a command run in this
session (results in "Verified Results").

## P0 tasks — all complete

### P0-1 — `just check-cluster` green (fmt + clippy + lib tests)
- Fixed `clippy::manual_strip` in `rusteron-java-test-support/src/cluster.rs`
  (`strip_prefix`).
- Fixed `clippy::collapsible_if`, `manual_strip`, redundant `as u16`, a dead
  field, and an unused-assignment in `rusteron-cluster/src/{client,config}.rs`.
- Formatted the crate (`cargo fmt`) — the committed cluster + java-test-support
  sources were not rustfmt-clean, so `cargo fmt --check` had never passed.
- Result: `just check-cluster` = clippy `-D warnings` + `cargo fmt --check` +
  **53 lib tests**, all green.

### P0-2 — Portable codec generation + drift check
- `generate-cluster-codecs`: portable across macOS (BSD sed/shasum) and Linux
  (GNU sed/sha256sum) — `sed -i.bak … && rm` and `shasum -a 256`. Fixed a
  malformed `\\*` regex (now `\*`).
- Non-destructive: regenerates only the two cluster subdirs, preserving the
  RFQ schema output (`generated/com_aeroncookbook_cluster_rfq_sbe`).
- **Format-on-generate**: the recipe runs `cargo fmt` after generating, so
  committed codecs are rustfmt-clean AND regeneration reproduces them.
- Centralised the hand-added `impl Writer for WriteBuf` (sbe-tool 1.39.0 omits
  it) into `src/codecs/writer_impls.rs` — generated `mod.rs` files are now
  pure generator output (no hand-edits to drift).
- `check-cluster-codec-drift` now diffs every written dir: `generated/`,
  `cluster_codecs/`, `cluster_codecs_mark/`.
- Result: regeneration is **idempotent**; `just check-cluster-codec-drift` →
  "OK: Generated codecs match committed."

### P0-3 — Deterministic own-driver UDP failover test
- New `tests/failover_own_driver.rs`: connects via the high-level
  `AeronCluster` API over the client's **own** embedded driver + UDP, kills
  the actual elected leader, receives `NewLeaderEvent`, reconnects to the
  **new** leader (resolved by member id), returns to `Connected`, and
  completes a **post-failover round trip**. Every step asserted.
- Root cause found & fixed en route: the handshake's `REDIRECT` handler and
  `poll_egress`'s `NewLeaderEvent` handler both picked the *first* endpoint
  (the dead leader) instead of resolving by `leader_member_id` — a
  reconnect-to-dead-node bug that the print-only `failover_demo` hid.
- `failover_demo` corrected to resolve the leader by member id (it previously
  reconnected to dead node 0 and printed "failover handled").

### P0-4 — Remove hardcoded failover settings; propagate errors
- `poll_egress` no longer hardcodes `stream_id = 101` — `AeronCluster` now
  retains `ingress_stream_id` and the reconnect uses it.
- After a `NewLeaderEvent` reconnect, the client returns to `Connected`
  (previously stuck in `AwaitingNewLeaderConnection`, so `offer` always
  failed post-failover).
- New typed errors: `ClusterError::Publication` (offer/claim/keep-alive,
  covers backpressure) and `ClusterError::ReconnectFailed`. The reconnect
  paths no longer swallow errors (`if let Ok(...)` → `?`).

### P0-5 — Real restart/quorum tests
- Rewrote `tests/restart.rs`. Removed the redundant `leader_kill` test
  (superseded by `failover_own_driver`).
- `test_quorum_loss_stops_serving`: kills the 2 non-leader nodes, asserts the
  cluster **stops serving** (no echo after quorum loss) — previously asserted
  nothing (no-op closure).
- `test_cluster_restart_and_reconnect`: connects to cluster A, kills all of
  A, brings up a fresh cluster B, reconnects, round-trip — previously only
  killed and stopped ("restart not implemented").
- Both `#[ignore]`'d (privileged/slow); both verified passing with `--ignored`.

## Verified Results (this session)

```
cargo fmt --all -- --check                 # clean
just check-cluster                          # clippy -D warnings + fmt + 53 lib tests
cargo test -p rusteron-cluster --features test-harness
  → 71 passed, 0 failed (lib 53 + integration 18)
cargo test -p rusteron-java-test-support
  → 8 passed, 0 failed
cargo test -p rusteron-cluster --test failover_own_driver --features test-harness
  → 1 passed (deterministic failover, ~13s)
# privileged (--ignored):
cargo test -p rusteron-cluster --test restart --features test-harness -- --ignored
  → 2 passed (quorum-loss + restart)
just check-cluster-codec-drift
  → OK: Generated codecs match committed.
```

## Adversarial audit (ultracode pass)

A 4-way independent audit (portability, error-propagation, test-quality,
workspace-warnings) confirmed tests + warnings are CLEAN and surfaced real
error-propagation + portability defects, all fixed and re-verified:

- **CRITICAL** `poll_egress` swallowed the `EgressAdapter::on_fragment`
  decode error (`let _ =`). Now buffered and surfaced after the batch
  (after the `NewLeaderEvent` reconnect, so failover isn't blocked by an
  unrelated malformed fragment).
- **HIGH** `send_challenge_response` (sync + async) discarded the offer
  result → silent auth stall. Now returns `ClusterError::Publication` on
  backpressure / not-connected.
- **HIGH** `close()` set `PendingClose` *after* the offer, so a failed
  notify blocked the local state transition. Reordered: local state first,
  advisory `SessionCloseRequest` best-effort (mirrors Java).
- **MEDIUM** `.justfile` `shasum -a 256` needs Perl (absent on slim Linux
  Docker). Both hash sites now detect `sha256sum` (coreutils) first,
  `shasum -a 256` fallback.
- **Deliberately left**: the `Duration::from_secs(5)` transport-creation
  timeout (6 sites) is not a failover setting and its errors already
  propagate; extracting a const would be cosmetic churn the goal's "avoid
  unrelated refactors" rules out.

## Remaining `#[ignore]`d tests

- `restart::test_quorum_loss_stops_serving` (~20s)
- `restart::test_cluster_restart_and_reconnect` (~3s)

Both now carry real assertions; run via `--ignored`. They stay ignored by
default because they are destructive (kill cluster nodes) and unsuited to
the normal CI lane.

## Unresolved Risks / Follow-ups

> **Stamped 2026-07-19:** items 1–2 in the list below are **DONE** (RFQ schema
> vendored + ErgoSBE path; connect re-offer implemented). Remaining bullets are
> historical risk notes only — see [`../../LIVING_BACKLOG.md`](../../LIVING_BACKLOG.md)
> for anything still open.

1. **RFQ codec regeneration has no committed schema.** `rfq_codecs/` and
   `generated/com_aeroncookbook_cluster_rfq_sbe/` are committed and
   compile-tested, but the Aeron-Cookbook RFQ schema XML is not vendored, so
   they cannot be regenerated by `just generate-cluster-codecs` (which
   intentionally preserves them). Vendoring the schema is a follow-up.
2. **Handshake re-send not implemented.** Connect sends the
   `SessionConnectRequest` once. If the first offer lands on a pre-election
   node that neither leads nor redirects, the connect times out. The
   `REDIRECT` path now resolves the leader correctly, so the common case is
   covered; a periodic re-offer would harden the pre-election edge case.
3. **Restart test is a fresh-cluster restart**, not a log-recovery restart
   (the launcher runs `dirDeleteOnStart`, so there is no replicated state to
   recover). A persistence-bearing restart test would need a launcher change.
4. **Single-machine, localhost-only** cluster transport in tests; no
   multi-host / WAN-latency coverage.
