# Aeron Cluster Client Hardening Design

**Created:** 2026-07-20
**Status:** In progress
**Supersedes:** `2026-07-17-rusteron-cluster-client-design.md` (adds egress hardening; original design remains canonical for state-machine basics)

## 1. Codec Ownership and Module Seams

### Before (current state)

```
codecs::ergo_codecs        ← OUT_DIR (ErgoSBE, schema 111)
codecs::ergo_codecs_mark   ← OUT_DIR (ErgoSBE, mark schema)
codecs::ergo_rfq_codecs    ← OUT_DIR (ErgoSBE, schema 101)
codecs::cluster_codecs     ← checked-in sbe-tool 1.39.0
codecs::cluster_codecs_mark← checked-in sbe-tool 1.39.0
codecs::rfq_codecs         ← checked-in sbe-tool 1.39.0
codecs::session            ← pub use ergo_codecs as session
codecs::rfq                ← pub use ergo_rfq_codecs as rfq
```

Callers use `codecs::session::*` via the alias; sbe-tool trees exist for benchmarks only.

### After

```
codecs::session            ← OUT_DIR (ErgoSBE, schema 111)
codecs::mark               ← OUT_DIR (ErgoSBE, mark schema)
codecs::rfq                ← OUT_DIR (ErgoSBE, schema 101)
```

Direct public modules, no aliases. sbe-tool trees deleted from `cluster/src/codecs/`.
Reference sbe-tool runtime exists only at `cluster/benches/reference_sbe/` (Criterion-private).

### build.rs changes

Generate into modules named `session`, `mark`, `rfq` instead of `aeron_cluster_codecs`, `aeron_cluster_codecs_mark`, `aeron_rfq_codecs`.

## 2. Regular and Controlled Egress Flow

### Fragment reassembly

`AeronFragmentClosureAssembler` and `AeronControlledFragmentClosureAssembler` (from rusteron-client) are stored inside `AeronCluster`. Created fallibly during `connect()` and `AsyncClusterConnect::finish()`.

**Flow (regular egress):**
```
Aeron poll → fragments → AeronFragmentClosureAssembler::on_fragment
  → complete message assembled? → decode via AnyMessage
    → filter by cluster_session_id
      → dispatch to EgressListener
```

**Flow (controlled egress):**
```
Aeron controlled_poll → fragments → AeronControlledFragmentClosureAssembler::on_fragment
  → complete message assembled? → decode via AnyMessage
    → filter by cluster_session_id
      → dispatch to ControlledEgressListener
        → map listener action to Aeron C values
```

### Assembler lifecycle

- Created on connect (synchronous + async finish).
- Recreated on `NewLeaderEvent` to prevent old-image message contamination.
- Always enabled; no configuration toggle.

### Error ordering

Current behaviour preserved: complete any detected leader transition before returning the first buffered decode error.

## 3. C++ Client Adoption/Rejection Matrix

Source: [reverb-sys/aeron-cluster-client-cpp@d67e5ac](https://github.com/reverb-sys/aeron-cluster-client-cpp/tree/d67e5acf057825950e48401d040f1d665945a428)

| Feature | Status | Notes |
|---------|--------|-------|
| Fragment reassembly via `AeronFragmentAssembler` | **Adopted** | Always-on inside `AeronCluster` |
| Controlled fragment reassembly | **Adopted** | Via `AeronControlledFragmentClosureAssembler` |
| Session-ID filtering before dispatch | **Adopted** | Filter by `cluster_session_id` |
| Assembler reset on new leader | **Adopted** | Prevents cross-image contamination |
| Background polling thread | **Rejected** | Poll-driven API only; no hidden threads |
| Process-global signal handlers (SIGINT) | **Rejected** | Caller controls signal handling |
| Custom topic/order/commit protocol | **Rejected** | Not a service implementation |
| Automatic hot-path retries | **Rejected** | Callers retain backpressure policy |
| Placeholder acknowledgements | **Rejected** | Not applicable |
| Unverified statistics | **Rejected** | Not applicable |

## 4. Public Interface Changes

### New method

```rust
pub fn poll_egress_controlled<L: ControlledEgressListener>(
    &mut self,
    adapter: &mut ControlledEgressAdapter<L>,
    fragment_limit: usize,
) -> ClusterResult<i32>;
```

### ControlledPollAction

```rust
pub enum ControlledPollAction {
    Continue,  // maps to Aeron C value 4
    Abort,     // maps to Aeron C value 1
    Break,     // maps to Aeron C value 2
    Commit,    // maps to Aeron C value 3
}
```

No `#[repr(i32)]`. Values are explicitly mapped to Aeron C constants at the FFI boundary.

### ControlledEgressListener

Only `on_message` returns `ControlledPollAction`. All other callbacks (lifecycle, challenge, admin) become default no-op methods returning `()`:

```rust
pub trait ControlledEgressListener {
    fn on_message(&mut self, cluster_session_id: i64, timestamp: i64, buffer: &[u8]) -> ControlledPollAction;

    // Default no-op methods:
    fn on_session_event(&mut self, ...) {}
    fn on_new_leader(&mut self, ...) {}
    fn on_challenge(&mut self, ...) {}
    fn on_admin_response(&mut self, ...) {}
}
```

### Removed

- `codecs::ergo_codecs`, `codecs::ergo_codecs_mark`, `codecs::ergo_rfq_codecs`
- `codecs::cluster_codecs`, `codecs::cluster_codecs_mark`, `codecs::rfq_codecs`
- `unchecked-companions` Cargo feature
- `codecs::writer_impls`

## 5. Failure Handling

- Assembler creation failures → `ClusterError` during connect.
- Decode failures after reassembly → buffered, surfaced on next `poll()` return.
- Session-ID mismatch → message silently dropped (not an error).
- Assembler reset failure on NewLeaderEvent → logged, old assembler retained (graceful degradation).
- Fragment handler panics → caught by rusteron C boundary; converted to error.

## 6. Acceptance Criteria

1. All existing tests pass without modification to test intent.
2. `just bench-cluster` retains all maintained ratios ≤ 1.00.
3. No `codecs::(ergo_codecs|cluster_codecs|rfq_codecs)` imports remain outside `cluster/benches/reference_sbe/`.
4. `poll_egress_controlled` delivers reassembled messages through `ControlledEgressListener`.
5. `egress_fragmentation.rs` verifies 16 KiB payload round-trip through Java Echo with `mtu=1408`.
6. Controlled unit tests cover all four `ControlledPollAction` mappings.
7. Leader-kill tests pass with assembler resets.
