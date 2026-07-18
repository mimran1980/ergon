# ergo-aeron-cluster (`cluster/`)

Experimental pure-Rust **Aeron Cluster client** on `rusteron-client` **0.2.4**,
with **ErgoSBE-generated** session (schema 111) and RFQ (schema 101) codecs.

⚠️ **Prototype.** LLM-assisted, less tested than the Java reference. Bugs in
Rusteron pub/sub **or** this client may cause UB, segfaults, or data loss.
Delete or replace when official Aeron Cluster C client bindings are suitable.

## Status

Residual product scope **COMPLETE** (2026-07-18): production codecs ErgoSBE-only,
connect re-offer, log-recovery test, maintained encode+decode benches ≤ 1.00,
RFQ unfrozen. See living completion prompt under `docs/superpowers/plans/`.

## Depends on

- Path: `ergosbe` (via `build.rs`)
- `rusteron-client` 0.2.4
- Optional: `ergo-aeron-cluster-test-support` behind feature `test-harness`
- Aeron submodule **1.52.2** for schemas + Java jars

## Build / test

```sh
# Lib only (no Java)
cargo test -p ergo-aeron-cluster --lib
cargo test -p ergo-aeron-cluster --test codec_golden_bytes
just check-aeron-cluster

# Maintained codec benches (ErgoSBE vs residual sbe-tool, equal work)
just bench-cluster
# filter example:
cargo bench -p ergo-aeron-cluster --bench cluster_codec_bench -- \
  'encode/session_message_header|encode/session_keep_alive|decode/session_message_header|decode/session_event'

# Integration (Java 17+ + jars)
just build-aeron-jars
just test-aeron-cluster-harness
```

**Gotcha:** never `cargo … --workspace --all-features` without
`--exclude ergo-aeron-cluster` (harness pulls Java).

## Codecs

| Module | Source | Use |
|--------|--------|-----|
| `codecs::ergo_codecs` | `build.rs` ← `aeron-cluster-codecs.xml` | **Production** session protocol |
| `codecs::ergo_codecs_mark` | mark schema | Mark file codecs |
| `codecs::ergo_rfq_codecs` | `schemas/protocol-codecs.xml` (cookbook 101) | **Production** RFQ examples |
| `codecs::cluster_codecs` / `rfq_codecs` | residual sbe-tool 1.39.0 trees | Head-to-head benches + wire parity only |

Prefer `SessionBuilder` / `AeronCluster` over hand-rolled publications.

## Usage

```rust
use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
// SessionBuilder: ingress/egress channels, stream ids, timeout
// AeronCluster::connect / connect_async
// AeronCluster::try_claim(payload_len) — SessionMessageHeader via ErgoSBE in claim
// AeronCluster::poll_egress(adapter, limit)
```

RFQ: `codecs::ergo_rfq_codecs` + examples `rfq_client` / `rfq_roundtrip`.

## Maintained benches (ErgoSBE / sbe-tool ≤ 1.00)

| Scenario | Role |
|----------|------|
| SessionMessageHeader encode | Claim hot path |
| SessionKeepAlive encode | Periodic |
| SessionMessageHeader decode | Egress |
| SessionEvent decode | Egress |

`SessionConnectRequest` encode is **demoted** (cold path). Ledger:
[`../ergosbe-performance-optimisation-goal.md`](../ergosbe-performance-optimisation-goal.md).

## Layout

```
cluster/
├── build.rs                 # ErgoSBE → OUT_DIR (session + mark + RFQ)
├── schemas/protocol-codecs.xml
├── benches/cluster_codec_bench.rs
├── examples/                # echo, failover, rfq_*, …
├── src/
│   ├── client.rs            # AeronCluster, try_claim, AsyncClusterConnect
│   ├── codecs/              # ergo_* production + residual sbe-tool
│   ├── connect.rs           # connect re-offer helpers
│   ├── egress.rs / controlled.rs / poller.rs
│   ├── session.rs / state.rs / config.rs / error.rs
└── tests/                   # goldens + test-harness integration
```

## HA sample

Leadership-aware orderbook + latency → CH:
[`../samples/cluster-ha-orderbook/`](../samples/cluster-ha-orderbook/)  
(`just samples-cluster-ha`, `just samples-cluster-ha-kill-leader`).

## Non-goals

- Rust Aeron Cluster **service** implementation
- Deleting residual sbe-tool trees used for benches
- Promoting connect encode into the maintained ≤1.00 gate
