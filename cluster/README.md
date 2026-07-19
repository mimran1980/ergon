# ergo-aeron-cluster (`cluster/`)

Experimental pure-Rust **Aeron Cluster client** on `rusteron-client` **0.2**
(latest 0.2.x via workspace deps), with **ErgoSBE-generated** session (schema
111) and RFQ (schema 101) codecs.

⚠️ **Prototype.** LLM-assisted, less tested than the Java reference. Bugs in
Rusteron pub/sub **or** this client may cause UB, segfaults, or data loss.
Delete or replace when official Aeron Cluster C client bindings are suitable.

## Status

Residual product scope **COMPLETE** (2026-07-18): production codecs ErgoSBE-only,
connect re-offer, log-recovery test, maintained encode+decode benches ≤ 1.00,
RFQ unfrozen. Open items: [`../docs/LIVING_BACKLOG.md`](../docs/LIVING_BACKLOG.md).

## Depends on

- Path: `ergo-sbe` (via `build.rs`)
- `rusteron-client` **0.2** (workspace; latest 0.2.x)
- Optional: `ergo-aeron-cluster-test-support` behind feature `test-harness`
- Aeron submodule **1.52.2** for schemas + Java jars

## Build / test

```sh
# Lib only (no Java)
cargo test -p ergo-aeron-cluster --lib
cargo test -p ergo-aeron-cluster --doc
cargo test -p ergo-aeron-cluster --test codec_golden_bytes
just check-aeron-cluster

# Maintained codec benches (ErgoSBE vs residual sbe-tool, equal work)
just bench-cluster
cargo bench -p ergo-aeron-cluster --bench cluster_codec_bench -- \
  'encode/session_message_header|encode/session_keep_alive|encode/claim_shaped|decode/session_message_header|decode/session_event'

# Integration (Java 17+ + jars)
just build-aeron-jars
just test-aeron-cluster-harness
```

**Gotcha:** never `cargo … --workspace --all-features` without
`--exclude ergo-aeron-cluster` (harness pulls Java).

## Codecs

| Module | Source | Use |
|--------|--------|-----|
| `codecs::session` (`ergo_codecs`) | `build.rs` ← `aeron-cluster-codecs.xml` | **Production** session protocol |
| `codecs::rfq` (`ergo_rfq_codecs`) | `schemas/protocol-codecs.xml` | **Production** RFQ |
| `codecs::ergo_codecs_mark` | mark schema | Mark file codecs |
| `codecs::cluster_codecs` / `rfq_codecs` | residual sbe-tool 1.39.0 | Benches + wire parity **only** |

Prefer **`codecs::session` / `codecs::rfq`** in new code. Prefer
`decode_session_event` / `decode_new_leader_event` for equal-work egress helpers.

## Usage (try_claim hot path)

```rust
use ergo_aeron_cluster::codecs::session::SessionMessageHeaderEncoder;
use ergo_aeron_cluster::{AeronCluster, SessionBuilder};

// SessionBuilder::builder().ingress_channel(...).egress_channel(...)
// let mut client = AeronCluster::connect(&builder, aeron_dir)?;

// Header-inclusive length is generated:
const HDR: usize = SessionMessageHeaderEncoder::ENCODED_LENGTH; // 32

// Claim app payload; session header written into the claim automatically.
let mut claim = client.try_claim(app_len)?;
// claim.payload_mut() — encode AppMessage / nested SBE here
claim.commit()?;

// Egress
// client.poll_egress(&mut adapter, limit)?;
// client.send_keep_alive()?;
```

Nested AppMessage recipe:
[`../sbe/docs/guide/claim-nested-encode.md`](../sbe/docs/guide/claim-nested-encode.md).  
HA sample: [`../samples/cluster-ha-orderbook/`](../samples/cluster-ha-orderbook/).

## Maintained benches (ErgoSBE / sbe-tool ≤ 1.00)

| Scenario | Role |
|----------|------|
| SessionMessageHeader encode | Claim hot path |
| SessionKeepAlive encode | Periodic |
| **claim_shaped** (header + app copy) | Claim-shaped write |
| SessionMessageHeader decode | Egress |
| SessionEvent decode | Egress |

**Diagnostic only (not a ≤1.00 gate):** NewLeaderEvent decode, SessionConnectRequest encode.

Ledger: [`../ergosbe-performance-optimisation-goal.md`](../ergosbe-performance-optimisation-goal.md).

## Layout

```
cluster/
├── build.rs                 # ErgoSBE → OUT_DIR (session + mark + RFQ)
├── schemas/protocol-codecs.xml
├── benches/cluster_codec_bench.rs
├── examples/                # echo, failover, rfq_*, …
├── src/
│   ├── client.rs            # AeronCluster, try_claim, AsyncClusterConnect
│   ├── codecs/              # session/rfq aliases + residual sbe-tool
│   ├── decode.rs            # equal-work frame helpers
│   ├── egress.rs / controlled.rs / poller.rs
│   └── session.rs / state.rs / config.rs / error.rs
└── tests/                   # goldens + test-harness integration
```

## Non-goals

- Rust Aeron Cluster **service** implementation
- Deleting residual sbe-tool trees used for benches
- Promoting connect encode or NewLeader decode into the maintained ≤1.00 gate
- Production-grade quality beyond the experimental banner
