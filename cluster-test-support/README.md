# ergo-aeron-cluster-test-support

Java test harness helpers for `ergo-aeron-cluster` integration tests.

## Status

**Harness-only.** Not a production dependency. Excluded from the workspace
members set; built as a path dependency when `test-harness` is enabled.

## Depends on

- Java **17+**
- Aeron submodule jars (`just build-aeron-jars` from repo root)
- `ClusterLauncher` compiled into the harness classpath

## Build / test

```sh
# From repo root
just build-aeron-jars
just test-aeron-cluster-harness

# Or targeted
cargo test -p ergo-aeron-cluster --features test-harness -- --test-threads=1
```

Jar integrity: `test-jars.sha256` records expected digests after
`just build-aeron-jars`. Re-run and update the hash file when Aeron pin or
Gradle outputs change.

## Layout

| Path | Role |
|------|------|
| `src/cluster.rs` | `TestCluster` multi-node spawn / `kill_node` |
| `src/jar.rs` | Locate Aeron jars under `aeron/` build outputs |
| `src/java/ClusterLauncher.java` | Node process launcher |
| `tests/` | Spawn / harness failure smoke |
| `test-jars.sha256` | Optional jar content digests |

## Public entry points

- `TestCluster::single_node` / `three_node` / `restart_keep_dirs`
- `TestCluster::kill_node` — used by failover + HA kill-leader tests

## Spawn failure modes

| Symptom | Fix |
|---------|-----|
| `java` not found | Install JDK 17+ and put it on `PATH` |
| Jar not found | `just build-aeron-jars` (Aeron submodule init first) |
| No `CLUSTER_READY` | Free ports; kill stale cluster Java processes; re-run |
| Flaky connect after kill | Use own-driver UDP tests; wait for election; check `kill_node` index |

## Safety notes

- Processes are killed on drop (best-effort).
- Do not commit `aeron-cluster-[0-9]/` runtime directories created by launches.
- Slow / destructive tests may be `#[ignore]` — run with `--ignored` only when intended.

## Non-goals

- Production cluster deployment tooling
- Replacing Aeron’s own media driver configuration surface
