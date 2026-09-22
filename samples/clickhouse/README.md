# ClickHouse persistence laboratory

A private, HFT-oriented persistence laboratory: public multi-exchange
market data through NautilusTrader, our Ergo SBE recording pipeline,
ClickHouse/S3, Grafana/Prometheus, live diagnostic configuration, and
executable Jupyter notebooks.

**Status (verified 2026-09-21):** every acceptance lane passes on a live
kind cluster — `lint`, `test`, `verify-cluster`, `verify-aeron`,
`verify-follow`, `verify-live` (public feeds, both venues), `verify-local`,
`verify-notebooks` (7/7). All four images (ingester, archive-agent,
market-recorder, archive-driver) build and run in-cluster; the public-live
acceptance run (task 13) passes at the 4-minute window exercised so far
(`just verify-live 4m`; the justfile's own default is 10m, not yet run at
that length); Grafana's dashboards render real
trades and prices from the running pipeline; dynamic config edits (enable a
temporary table with no pod restart) are verified live. See
[Verification status](#verification-status) for the full checklist and
[PLAN.md](PLAN.md) for what each verification actually exercised, including
bugs found and fixed along the way. The ingester now exports a
`ergo_ingester_batch_ack_seconds` Prometheus histogram (send-to-ack latency
for every ClickHouse batch flush), which closed the last gap in the
Prometheus injection checks — including genuinely scaling ClickHouse to
zero and watching the ingester's response, which turned up a real gap of
its own: **the ingester exits rather than retrying when ClickHouse is
unreachable at startup**, currently recovering only because Kubernetes
restarts it until the dependency returns (see below). One known gap
remains, non-blocking and diagnosed to root cause: `jupyter-0` fails to
start on some hosts (a containerd image-unpack race, not in the acceptance
path — notebook verification runs locally instead, see below).

## Architecture

```text
Binance pod ──┐                                    ┌─> Prometheus ──> Grafana
   recorder + ├─ local IPC driver/Archive ── replay ─┤
   archive    │                                      └─> ClickHouse ──> notebooks
   agent ─────┘
Bybit pod ────┘
```

- One exchange StatefulSet pod owns: the Python/Nautilus app with our
  PyO3 recorder extension, a pod-local Aeron Media Driver + Archive
  (shared-memory `emptyDir` + Archive PVC), and the archive-agent
  (registration catalog owner + metrics).
- The central ingester (one active instance) replays the archives,
  resolves durable layout bindings, projects rows, and pushes bounded
  batches to ClickHouse (RowBinary, flush on 8192 rows / 8 MiB / 100 ms).
- Table/column/process names travel **only** in registration records;
  data envelopes carry compact numeric IDs.
- `config/recording.yaml` is edited locally and applied by the watcher —
  no Flux, no second Git repository, no pod restarts.

## Prerequisites

- Rust 1.98 (latest stable; the sample's own MSRV floor is 1.88 — images and
  the container build use the latest) and Temurin JRE 25 (latest) for the
  Aeron archive driver. This sample is excluded from
  the root workspace), Docker (daemon running), kind, kubectl, Python 3.11+,
  `uv` (optional, for the Python bridge), `just`.
- macOS/Apple Silicon supported for the library + tests; container builds
  use linux/arm64 images from pinned registries.

## Layout

```text
crates/persist          ergo-clickhouse-persist: recording library (protocol,
                        registration, recorder, config, ingest)
crates/persist-derive   derives + persist_table!/persist_event!/persist_sbe!/persist_dto!
crates/persist-tracing  optional tracing Layer adapter
crates/market-schema    market-data + diagnostics SBE schemas, DTOs, projections
crates/python-bridge    PyO3 extension (ergo_recorder module)
apps/ingester           central ingester binary
apps/archive-agent      registration/catalog owner + Prometheus metrics
apps/recording-config   recording/v1 validation CLI (shared parser)
apps/market-recorder    Python data-only recording app (fixture + live)
config/                 recording.yaml (edit this!), feeds.yaml, kustomization
deploy/                 kind config, kustomize base + overlays, images
observability/          prometheus + grafana provisioning
notebooks/              7 executable verification notebooks
scripts/                start/stop/status/watch/verify
fixtures/               public-format frames (binance, bybit)
```

## Start / stop

```sh
just clickhouse-up            # fixture stack (kind) + port-forwards + URLs
just clickhouse-status        # components, config revisions, counters
just clickhouse-watch-config  # foreground watcher for config edits
just clickhouse-down          # stop; retains data
just clickhouse-purge         # stop + delete namespace/volumes (destructive)
```

`start.sh` runs a preflight (tools, docker, kind context, capacity), builds
the binaries, loads images into kind, applies the validated config, applies
the kustomize overlay, waits for readiness, and starts tracked
port-forwards. Live mode: `./scripts/start.sh --mode live` verifies that no
exchange credential variables are present before anything connects. The
readiness wait excludes `jupyter-0` (see the JupyterLab note below) and any
completed Job pod (`clickhouse-init`, `s3-bucket-init` — a Job's pod never
reports `ready=true` even after its container exits 0, which is correct
Kubernetes behaviour, not a stuck workload) so a fully healthy cluster
doesn't time out after 600s waiting for containers that will never flip
ready. **`just clickhouse-up` is verified end to end on this build**
(`13/13` containers ready, reaches the local-URLs banner, exit 0) —
verifying it is what turned up both of the exclusions above and the
`recording-config` build-isolation bug below; before those three fixes it
failed at three different points.

**Disk:** the recorder image compiles Rust *and* Aeron inside the container,
which needs several GB of `target/` and build cache in the Docker VM. That has
filled a default Docker Desktop disk and wedged the daemon several times, so
`build.sh` reports headroom before the build and prunes the build cache after
it. Give Docker Desktop at least ~40 GB, on a disk that actually has that
much free — a full **host** disk wedges the daemon the same way a full VM
disk does, and is much easier to hit by accident (see PLAN.md's disk-recovery
notes if this happens; it is recoverable, not fatal).

**The container images must be built on Linux.** They COPY release binaries
into a Linux image, and the recorder image needs the PyO3 wheel built for
Linux; there is no cross toolchain in this tree. `deploy/images/build.sh`
refuses to run on a non-Linux host rather than producing an image that pulls
and then CrashLoops with "exec format error". On macOS, `start.sh` builds
the two pure-Rust images the normal way (`cargo build --release` runs fine on
the host; only the *image* needs a Linux binary) via a short-lived Linux
container over a bind mount — this is what `just clickhouse-up` actually
does, verified end to end:

```sh
docker run --rm -v "$PWD":/w -w /w/samples/clickhouse rust:1.98-bookworm bash -c '
  apt-get update -qq && apt-get install -y -qq cmake g++ default-jdk-headless \
      curl xz-utils clang libclang-dev llvm-dev libbsd-dev
  CM=3.30.5
  curl -fsSL "https://github.com/Kitware/CMake/releases/download/v${CM}/cmake-${CM}-linux-aarch64.tar.gz" -o /tmp/cmake.tgz
  tar xzf /tmp/cmake.tgz -C /opt
  ln -sf /opt/cmake-${CM}-linux-aarch64/bin/cmake /usr/local/bin/cmake
  cargo build --release -p ingester --features archive -p archive-agent'
ERGO_ALLOW_CROSS_IMAGES=1 ./deploy/images/build.sh ergo/ingester:local
ERGO_ALLOW_CROSS_IMAGES=1 ./deploy/images/build.sh ergo/archive-agent:local
kind load docker-image ergo/ingester:local ergo/archive-agent:local --name clickhouse
```

Each requirement is load-bearing: bookworm's CMake is 3.25 and Aeron needs
≥ 3.30; `bindgen` needs libclang; the Aeron native build links `-lbsd`; and
`rusteron` builds Aeron into the cargo tree rather than installing it, so the
image must carry `libaeron*.so` (the ingester exits 127 without it).

**Never build `recording-config` in the same `cargo build` invocation as
`ingester --features archive`.** Cargo unifies features across a shared
build graph, so combining them leaks the `archive` feature — and its
`libaeron` link — into the config-validation CLI, which never asked for it
and does not ship with a Linux image to run inside. The symptom on macOS is
`config-watch.py` reporting every edit as `INVALID edit — keeping last
applied revision: … Library not loaded: @rpath/libaeron.dylib`, which reads
as a config problem but is a build-invocation problem. Build it in its own
`cargo build --release -p recording-config` (no other `-p`, no
`--features`), as `start.sh` and `justfile`'s `build-cli` recipe both now
do.

The recorder image (`deploy/images/recorder.Dockerfile`, context is the
repository root) builds the PyO3 wheel on the host over a bind mount — it
needs neither a host binary nor a Linux host, since the wheel build happens
*inside* the image build. With all four images (`ingester`,
`archive-agent`, `market-recorder`, `archive-driver`) built and loaded, the
full pipeline runs in-cluster: recorder pods reach 3/3, the ingester
authenticates against ClickHouse and replays the Archive, and
`just verify-cluster` / `just verify-live` both pass.

**JupyterLab (`jupyter-0`) may fail to start with `exec /usr/bin/tini: exec
format error` or a truncated-stdlib Python crash, non-deterministically,
even with ample disk.** This has been traced to a containerd image-unpack
race specific to the `quay.io/jupyter` base images on some hosts — not
capacity, not architecture, not this repo's Dockerfile (see PLAN.md,
"JupyterLab cannot run on this host" for the full investigation). It is not
in the acceptance path: `just verify-notebooks` executes all 7 notebooks
locally against a venv kernel, independent of this pod, and passes 7/7
regardless of whether `jupyter-0` runs.

## Editable config (temporary tables)

Edit `config/recording.yaml` while the stack runs:

```yaml
rules:
  - process: market-recorder
    instance: "*"
    table: book_debug
    enabled: true          # flip false -> true to start recording
    row_ttl: 24h
    idle_table_ttl: 7d
```

The watcher validates the exact bytes via the shared Rust CLI, constructs
the ConfigMap from the validated snapshot, and applies it. Invalid edits
are rejected whole; the last applied revision is preserved. Rules for
tables absent from the process are inert; a table missing from config is
disabled — an invariant, not a convention.

**Verified live, in-cluster:** flipping `book_debug`'s `enabled` from
`false` to `true` projects into the running recorder pod — confirmed
recording — with the pod's UID unchanged (no restart), then flipping it back
to `false` stops recording again. `just verify-local` exercises this exact
round trip every run (`ok: enabled book_debug via config edit (pod
untouched)`, `ok: pod UID unchanged`, `ok: disabled book_debug again`).

## API examples

Native Rust (strict lazy macros; disabled calls evaluate nothing):

```rust
use ergo_clickhouse_persist_derive::{persist_table, persist_event};

persist_table! {
    static BOOK_DEBUG: temporary "book_debug" {
        instrument: u32,
        update_id: u64,
        buffered_updates: u64,
    }
}

let debug = BOOK_DEBUG.prepare(&mut session)?;
// Event thread: `expensive_count()` runs only when enabled.
persist_event!(writer, debug,
    at = 1_758_000_000_000_000_000,
    instrument = 7u32,
    update_id = book.last_update_id(),
    buffered_updates = expensive_count(),
);
```

(Runnable versions of both macro examples — with the session and handle set
up — are the doctests on `persist_table!` and `persist_event!` in
`crates/persist-derive/src/lib.rs`; `cargo test -p
ergo-clickhouse-persist-derive --doc` compiles them.)

Python (explicit guard before constructing optional state):

```python
# The handle's `enabled()` is checked before building optional state, so the
# expensive computation never runs while the table is disabled.
if self.debug.enabled():
    self.debug.record_debug(
        self.writer,
        instrument_id,
        update_id,
        self.compute_optional_diagnostic(),
        validity,
        event_time_ns,
    )
self.trades.record_trade(self.writer, trade)  # exact raw values, no floats
```

## Verification

```sh
just test             # workspace tests (all features) + producer variant
just lint             # clippy + fmt
just verify-local     # fixture queries, config enable/disable, dup-free views
just verify-aeron     # publish the fixture export into a real Archive, replay it, verify rows
just verify-notebooks # execute all 7 notebooks against the latest run
just verify-cluster   # in-cluster: recorder pod -> pod-local Archive -> ingester -> ClickHouse
just verify-live      # public-only live run (credentials must be absent)
```

All eight lanes above pass on a live kind cluster (2026-09-21) — see
PLAN.md for the exact runs, including three real bugs the process of
verifying them turned up and fixed rather than papered over: a hardcoded
producer `run_id` that made ClickHouse duplicate-identity checks fail on any
second run against the same database, `kind load docker-image` silently
leaving a node's old image tagged despite reporting success, and the
`recording-config`/feature-unification build bug above.

`verify-local` records `outstanding` (rather than silently passing) for any
criterion it could not exercise — for example the config-projection check when
no recorder pod is deployed. Read the run's `manifest.jsonl` before treating a
green run as complete coverage.

`verify-notebooks` starts the archive-agent's `/metrics` surface for the
duration of the run, so notebook 06 scrapes a real Prometheus-format endpoint.
Point `ERGO_METRICS_URL` at a running Prometheus to scrape that instead.

Service tests (`ingest_clickhouse`) boot a pinned ClickHouse container
automatically; Docker must be running.

**Grafana dashboards:** verified by actually running every panel query
through Grafana's own `/api/ds/query`, not by reading the dashboard JSON —
two of the three checked panels had real query bugs (one hard SQL error,
one silently-wrong-but-plausible-looking number from a `now()` unit
mistake) and are now fixed and confirmed showing live trades and prices;
see PLAN.md for the exact queries and the fix.

## SQL (query the recorded data)

```sql
-- Duplicate-free access always goes through views (FINAL)
SELECT * FROM market.trades FINAL WHERE venue = 'binance' ORDER BY exchange_event_time_ns;

-- Raw capture payloads are byte-exact
SELECT venue, receive_sequence, payload FROM market.raw_exchange_messages;
```

## Storage tiers

Target placement (PLAN §5, task 5). **Not implemented yet** — treat this as the
design, not as current behaviour:

| Age | Placement | Initial codec |
|---|---|---|
| 0–24 h | Fast local storage | LZ4 |
| 24 h–30 d | Local warm storage | ZSTD(3) |
| > 30 d | S3-compatible cold volume | ZSTD(15) |

What exists today: `create_table_ddl` emits the two `TO VOLUME` TTL clauses
only when the table's ordering prefix carries the `__storage_tiers` marker —
and nothing sets that marker. No `CODEC(...)` clause is emitted for any
column, and `deploy/base/clickhouse.yaml` defines no `storage_configuration`
with `warm`/`cold` disks, so the volumes the TTL clauses name do not exist.
Tier movement, recompression, cold reads and the capacity-pressure contract
are all unverified (`PLAN.md` task 5). Permanent tables do have no delete TTL,
which is the part of the contract that is in force.

## Toolchain (Python/Nautilus bridge)

Pinned after task 0's integration proof:

| Component | Pin |
|---|---|
| Python | 3.12 |
| `nautilus_trader` | **1.231.0** (stable wheel; v1 Actor API) |
| Binance public data | `api_key=None` — JSON public is the default; this pin has no `spot_market_data_mode` |
| Live recording | public Nautilus `Actor` callbacks only (no adapter patches, no private-handler wraps) |
| Raw JSON table | fixture-owned bytes only. Live writes typed rows + our generated SBE, not original venue frames |

```sh
uv venv .venv --python 3.12
uv pip install --python .venv/bin/python nautilus-trader==1.231.0 maturin pytest
env -u CONDA_PREFIX VIRTUAL_ENV=$PWD/.venv .venv/bin/maturin develop \
    --manifest-path crates/python-bridge/Cargo.toml --features pyo3/extension-module
PYTHONPATH=apps/market-recorder/src .venv/bin/pytest apps/market-recorder/tests tests/test_public_clients.py
```

`market-recorder --exchange binance --instance lab-0 --mode fixture` replays `fixtures/{binance,bybit}` through the same actor as live. `--mode live` builds a data-only `TradingNode` (no exec clients) and stays up until SIGTERM.

## Troubleshooting

- `config-watch` reports `API server unavailable` — the kind context is
  down; it retries the latest valid revision automatically.
- `BlockLengthTooShort` from a parity schema — a codegen offset resolution
  changed; fix the generator, never the fixture (root repo policy).
- Port-forwards die after sleep/wake — rerun `just clickhouse-up`
  (idempotent; it restarts only tracked processes).
- ClickHouse `AUTHENTICATION_FAILED` in local tests — the harness sets
  `CLICKHOUSE_PASSWORD=ergo_test`; clients must send X-ClickHouse-User/Key.
- `config-watch.py` reports `INVALID edit` with `Library not loaded:
  @rpath/libaeron.dylib` in the message — `recording-config` was built in
  the same `cargo` invocation as `ingester --features archive`; see "Never
  build `recording-config` in the same `cargo build` invocation…" above.
- `kubectl exec`/logs show a rebuilt image's old behaviour after `kind load
  docker-image` reported success — the CRI plugin's cached image view can go
  stale on some hosts. `docker exec <node> crictl rmi <image>` on every node,
  then `kind load` again, and confirm with `docker exec <node> crictl
  images` before trusting it.
- `jupyter-0` is `CrashLoopBackOff`/`Error` with `exec format error` or a
  Python stdlib crash — see the JupyterLab note above; not in the
  acceptance path, `just verify-notebooks` is unaffected.
- `ingester-0` is `CrashLoopBackOff` with `Error: Transport("... Dns Failed:
  resolve dns name 'clickhouse:8123' ... No address associated with
  hostname")` — ClickHouse is unreachable (scaled to 0, mid-restart, or the
  Service has no ready endpoints yet). The ingester does not retry this; it
  exits, and only Kubernetes' ordinary restart backoff recovers it once
  ClickHouse is resolvable again — which can take a few restart cycles. Not
  a hang: check `kubectl get pod clickhouse-0 -n clickhouse` and wait for it
  to be ready rather than intervening.
- A manually-run `kubectl port-forward` (as opposed to `just clickhouse-up`'s
  tracked ones) dies silently if the pod on the other end is deleted and
  recreated — a scale-down/up or pod restart leaves it forwarding to
  nothing. Symptom: `curl` exit 52 (empty reply) against a port that was
  working a moment ago. Re-run the port-forward, or use
  `just clickhouse-up`, which tracks and restarts its own.
- `ingester-0` crash-looping with `Error: Disconnected("server 500: ...
  MEMORY_LIMIT_EXCEEDED ...")` after the stack has been left running in live
  mode for many hours unattended — check `SELECT database, formatReadableSize(sum(bytes_on_disk))
  FROM system.parts WHERE active GROUP BY database` first. If `system` (not
  `market`) is the large one, it's ClickHouse's own diagnostic tables
  (`text_log` especially — the stock image logs at `trace` level with no
  retention); this is now fixed at the source (`log-retention.xml`: logger
  down to `information`, 1-day TTL on every system log table), but a run
  left up far longer than a day is still worth a memory check, not an
  assumption.

## Verification status

- [x] Recording protocol, registration, prepared producer (56 tests)
- [x] Zero-allocation gate: 1M enabled records / 1M disabled calls
- [x] recording/v1 config validation + shared CLI
- [x] Live ClickHouse: DDL, RowBinary (exact decimals, nullables, arrays),
      duplicate-free views, TTL expiry, out-of-band drift detection
- [x] Deployment manifests + scripts + observability provisioning
- [x] Aeron transport leg: fixture export → ArchivingMediaDriver →
      bounded replay → ClickHouse rows, run-id and metric-sample verified
      (`just verify-aeron`)
- [x] Notebooks execute cleanly end-to-end (7/7, exit 0) against an
      Aeron-ingested run; assertions negative-tested against a dead exporter,
      a wrong database, and a wrong run id
- [x] Nautilus 1.231.0 pin, public no-key config, live Actor callbacks, fixture-owned raw JSON (no Nautilus patch)
- [~] Two-worker reschedule run with portable test storage — **partial, infra
      level only**: proves shared-mount reattachment and headless DNS
      resolving the replacement pod on one host. Does not prove locking (no
      lock, no SQLite WAL, none of the recording stack involved), the pod-IP
      assertion is conditional (a run can pass with an unchanged IP), and
      cross-host failover is out of scope. See PLAN.md's Task 13 row for the
      full caveat.
- [x] Public-live acceptance run (task 13) — `just verify-live 4m`, public
      feeds, no exchange credentials present, both venues advancing
      (binance +2,559 / bybit +2,768 trades over that window). Not yet run
      at the justfile's own 10m default.
- [x] Config enable/disable projected into a running recorder pod without a
      restart, with the pod UID unchanged — verified live in the kind cluster
- [x] Prometheus/Grafana provisioning verified against the running kind
      cluster: rules loaded, pod service discovery and its RBAC working,
      both datasources answering real queries, all five dashboards loaded
      (`ergo-aeron`, `ergo-latency`, `ergo-market-data`, `ergo-pipeline`,
      `ergo-storage`) — "loaded" means present and query-error-free, not that
      every panel shows every metric the spec asks for: the Latency
      dashboard's own text panel still says recording-call and
      callback-to-publication histograms are unmet (batch-ack is now
      exported; see below).
- [x] Prometheus/Grafana *panel data*: every panel query run through
      Grafana's own `/api/ds/query` against live data. Two real bugs found
      and fixed in the process — "Best bid/ask" was a hard SQL error
      (`NOT_AN_AGGREGATE`) on every load, and "Recorder freshness"/"Ingest
      lag" silently computed nonsense from a `now()` function that returns
      seconds, not nanoseconds, masked by a coincidentally plausible-looking
      number until checked against a table with real timestamps. Confirmed
      live: real BTCUSDT/ETHUSDT prices, both venues' trade counts climbing.
- [x] Prometheus delay/drop-injection visibility — the ingester now exports
      `ergo_ingester_batch_ack_seconds` (send-to-ack latency per ClickHouse
      batch flush, port 9102), confirmed scraped and queryable
      (`histogram_quantile(0.99, ...)` ≈ 9ms under live traffic). Restart
      leg: deleted a recorder pod, watched Prometheus service discovery
      re-resolve the new pod IP and the target return to `up`. Drop leg:
      scaled ClickHouse to 0 for real — both ingester metrics targets
      correctly flipped to `down`, and this surfaced a genuine resilience
      gap (see the Troubleshooting note below): the ingester exits on a
      ClickHouse DNS failure rather than retrying, currently recovering
      only via Kubernetes' ordinary restart backoff.
- [x] Linux build of all four images verified: the ingester, archive-agent,
      market-recorder, and archive-driver images build and load into kind,
      and the full pipeline runs in-cluster end to end
- [x] Recorder pods run 3/3 in the kind cluster (Nautilus app + PyO3 bridge, a
      live ArchivingMediaDriver, the archive-agent), the Archive writes to its
      PVC, Prometheus scrapes both venues, and config edits project into the
      running pod without a restart
- [x] In-cluster produce → Archive → replay: the recorder pod publishes over
      pod-local Aeron IPC, its Archive records it, and the ingester replays it
      into ClickHouse — `just verify-cluster` passes end to end
- [ ] JupyterLab (`jupyter-0`) pod — fails to start on hosts hit by a
      containerd image-unpack race (see the note above); not in the
      acceptance path, notebook verification runs locally instead
