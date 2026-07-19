# Crates.io publish hygiene

ErgoSBE workspace crates and publish order. **Do not publish** experimental
or internal packages until residual acceptance still holds.

## Publishable (default features only)

| Crate | Path | Notes |
|-------|------|--------|
| `ergo-sbe` | `sbe/` | Primary generator; zero-ish runtime deps on generated code |
| `ergo-clickhouse-persist` | `persist/` | Optional features for rust_decimal / chrono / etc. |
| `ergo-aeron-cluster` | `cluster/` | **Experimental.** Publish **without** `test-harness` |

## Never publish

| Crate | Why |
|-------|-----|
| `ergosbe-benchmarks` | Internal head-to-head; `publish = false` |
| `samples/*` | Workspace-excluded demos (Bitget WS / HA) |
| Aeron submodule / jar trees | Upstream artifacts |

## Order

1. `ergo-sbe`
2. `ergo-clickhouse-persist` (depends on path/`crates.io` sbe as configured)
3. `ergo-aeron-cluster` last — depends on sbe via `build.rs`; **omit**
   `test-harness` (Java) from the published feature set users enable by default

## Pre-publish checklist

```sh
just check
cargo test -p ergo-sbe --lib
cargo test -p ergo-clickhouse-persist --lib
cargo test -p ergo-aeron-cluster --lib
cargo test -p ergo-aeron-cluster --doc
cargo package -p ergo-sbe --list
cargo package -p ergo-clickhouse-persist --list
cargo package -p ergo-aeron-cluster --list   # must not require test-harness
```

- No `Box<dyn Error>` on public library APIs (tests/`main` only).
- Cluster channels via `AeronUriStringBuilder` helpers.
- Maintained perf gates still documented in the ledger; re-bench only if hot
  paths change.
- README experimental banners retained for cluster.

## Versioning

Workspace `version = "0.1.0"` until first crates.io release; then bump per
semver for public API breaks (`ClusterError` variants, generator IR, etc.).
