# ClickHouse recording lab

The application publishes to Aeron. A separate ingester writes ClickHouse.
Recording switches on and off while the process runs, and the application
never waits for the database.

The lab around that is a small trading deployment: one feed handler per
exchange, one media driver per node, one engine per region.

```text
node (one of four, three regions)
  md-<exchange>  --UDP-->  engines
       |  IPC: events, metrics, traces
       v
  aeron: C driver  <--spy--  Java archive  --replay-->  ingester  -->  ClickHouse
  engine-<region>  --orders-->  exch-sim  --fills-->  engine
```

| Crate | Role |
|---|---|
| `ergon-runtime` | What the application links (`../../runtime`), one module per concern: `app` (start a process), `bus` (the Aeron client, run from the application's loop, its identity, the feeds' drop counters), `publication` and `subscription` (UDP feeds, with a persistent one that catches up from the archive), `persist` (`record()`, `record_row` and `record_value` for rows, and the `tracing` bridge for events and spans from any thread), `metrics`, `trace`, `clock`, `idle`, `directory` (feed names to addresses, which the application supplies), `rt` (agents on one loop: live, replay, backtest). |
| `ergon-runtime-server` | The ingester library (`../../runtime-server`). Replays the archive into ClickHouse, checkpoints, purges. It routes frames from schema XML, not from generated codecs. |
| `lab` | The lab's feed registry (`config/streams.yaml`) as the runtime's directory, its node checks, and the backtest's measured route delays. |
| `ingester` | The ingester binary: every archived feed in the registry, followed for new ones, through `ergon-runtime-server`. |
| `schema` | `market.xml`, `trading.xml`, the codecs generated from them, and `AnySchemaMessage` for a buffer that may be either. |
| `md` | One exchange's public market data, via NautilusTrader. No API keys. |
| `engine` | One region's engine, and `exch-sim`, its dummy exchange. |
| `aeron-driver` | Aeron's C media driver (1.52.2), configured from the environment. |

## Run

Recipes in `justfile` work on whichever cluster `LAB_CONTEXT` names (kind's
by default). Making and removing a cluster is per platform: `mac.justfile`
(`just mac …`, kind on this machine) and `azure.justfile` (`just azure …`,
below).

```sh
cd samples/clickhouse
just mac up    # kind cluster (if needed), images, deploy
just deploy    # apply kustomization.yaml after a config or manifest edit
just md        # rebuild the app image and restart publishers and ingesters
just aeron     # rebuild Aeron, then restart its clients
just verify    # check the running lab, including moving a feed between nodes
just logs
just test      # unit and integration tests; leaves ClickHouse and an Aeron driver up
just test-stop
just mac stop  # pause every kind node, keep the data
just mac start
just mac destroy   # delete the cluster and the data
```

Needs Docker, kind, kubectl, just, and jq, and `CLICKHOUSE_PASSWORD` in
`.env` (gitignored; `lab` will do): `just deploy` puts it in the Secret
`clickhouse`, which the pods read. Ports listen on every interface.
Grafana is anonymous admin and Jupyter has no token, so use a network you trust.

| | |
|---|---|
| ClickHouse | <http://localhost:8123/play> user `lab`, password `CLICKHOUSE_PASSWORD` from `.env` |
| Grafana | <http://localhost:3000> |
| Notebook | <http://localhost:8888/lab/tree/verify.ipynb> |
| What is recorded | `config/tables.yaml`. `just config` publishes it and pods see it within seconds. `just watch-config` publishes on every save. |
| Who publishes | `config/streams.yaml`, published the same way. |

| Pod | What |
|---|---|
| `aeron` | DaemonSet, host network. C driver and Java archive. Driver directory on the node's tmpfs; recordings on its disk. |
| `ingester` | DaemonSet. Archive to ClickHouse. Checkpoint on the node's disk. |
| `md-<exchange>` | One feed handler, host network, pinned to its region. |
| `engine-<region>`, `exch-sim-<region>` | The engine and its dummy exchange, host network. |

### Run on Azure

The same k3s nodes as Azure VMs, one per region, in one resource group
(`ergon-lab`). Each region has its own network; the networks are peered, so
k3s, flannel and Aeron's unicast MDC channels use private addresses. Only ssh
(22), the Kubernetes API (6443) and the NodePorts are open, and only to the
address `up` ran from.

```sh
az login                 # once
just azure up            # create everything, provision k3s; prints the .env lines
just azure sync          # copy the checkout; .git goes to the first node only
just azure on up         # on the first node: build, deploy, wait
just azure on verify     # any lab recipe runs there the same way
just azure pause         # stop k3s and every pod, VMs and data kept
just azure resume
just azure stop          # deallocate: no compute charge, cluster kept
just azure start
just azure down          # delete everything; fails unless the subscription is empty
```

The first node builds the images and runs the lab's recipes (the build
container mounts its checkout), so `just azure on <recipe>` runs them there.
The UIs are on the NodePorts of any node: `:30123/play`, `:30300`, `:30888`.

`.env` (gitignored) holds `LAB_CONTEXT=lab-vms`,
`KUBECONFIG=~/.kube/lab-vms.yaml`, `LAB_AZ_REGIONS`, `CLICKHOUSE_PASSWORD` and
the `LAB_VM_*` lines `up` prints. **Run `just azure down` when a session ends**: deallocated VMs
still pay for disks and addresses, and only a deleted group costs nothing.
`down` also removes the `NetworkWatcherRG` Azure creates on its own.

| Lab region | Azure region | VM |
|---|---|---|
| `an1` | Japan East (Tokyo) | Standard_D4s_v5 |
| `as1` | East Asia (Hong Kong) | Standard_D4s_v5 |
| `ew2` | Sweden Central (Stockholm) | Standard_D4s_v5 |
| `us1` | East US 2 (Virginia) | Standard_D4s_v6 |

A free-trial subscription allows 4 vCPUs and 3 public addresses per region,
so one 4-vCPU node per region; more quota needs a pay-as-you-go upgrade,
which also lifts the spending limit. It also may not get every size in every
region: in October 2026 Southeast Asia, UK South/West, North Europe and France
offered it no D-series size, East US only v4, and West Europe accepted no new
customers. `az vm list-skus -l <region>` shows what a subscription can use;
`LAB_AZ_REGIONS` (`azure-region:lab-region[:size]`) picks them. A node costs
about $0.19–0.26 an hour, four about $1. Cross-region transfer is
$0.02–0.08/GB, and engines subscribe to every feed, so market data crosses
regions continuously. Round trips from Tokyo measured 54 ms (Hong Kong),
159 ms (Virginia) and 254 ms (Stockholm).

One ClickHouse, wherever Kubernetes places it, takes every region's inserts,
so a far region's rows land seconds later. With it in Stockholm (2026-10-03),
the ingest lag (`inserted_at - ts_init`) of trades was p99 about 2 s for
Stockholm's feeds, 5.5 s for Hong Kong's and 9 s for Tokyo's; inserting each
tick's tables concurrently did not change that. The verification notebook
asserts p99 under 5 s, which this layout does not meet; a ClickHouse per
region would.

Changing the running lab, from `samples/clickhouse` on the Mac:

| Changed | Run |
|---|---|
| Rust code (md, engine, ingester, exch-sim) | `just azure sync`, then `just azure on md` |
| The Aeron driver or `docker/aeron.Dockerfile` | `just azure sync`, then `just azure on aeron` |
| Manifests under `deploy/` | `just deploy` |
| `config/tables.yaml`, `config/streams.yaml` | `just config` (no restart) |
| Grafana dashboards | `just azure sync` (reloaded within 10 s) |
| One pod, restarted | `kubectl --context lab-vms -n lab rollout restart deploy/<name>`, or k9s |

`just azure sync` copies by content, so cargo on the first node rebuilds exactly what
changed. Watch the cluster with
`k9s --kubeconfig ~/.kube/lab-vms.yaml --context lab-vms -n lab`. Grafana's
**Quant**, **SRE** and **Developer** dashboards are the starting points; each
links to the detailed ones.

## Deploy

`kubectl apply -k .` applies `kustomization.yaml`. Image tags are its `images:` block.

```text
deploy/kind.yaml            4 nodes, 3 regions
deploy/infra/               ClickHouse, Aeron, ingester, Grafana, Jupyter
deploy/md/base/md.yaml      the feed handler, once
deploy/md/<exchange>/       name, label, region
deploy/trading/base/        the engine and dummy exchange, once
deploy/trading/<region>/    name and region
deploy/spin/                patches for `just spin`; `just up` does not apply them
```

Add an exchange in four places, then `just deploy`: a line in `config/streams.yaml`,
a venue in md's `VENUES`, a copy of `deploy/md/binance/` with the name and region
changed, and a line in `kustomization.yaml`. The region word must match.
`just verify` fails if the pod's node is in a different region. Removing the
kustomization line and deleting the Deployment stops it; its data stays.

`just spin` then applies `deploy/spin/`: `IDLE=spin`, and the driver's three
threads spinning, one core each. `just up` sleeps, so four nodes fit on one machine.

Schemas ship in the app image (`just md` rebuilds it). Grafana and Jupyter read
this checkout through the kind mount at `/lab`.

## Layout

Regions are node labels: `an1` (Tokyo: Binance, Hyperliquid), `as1` (Singapore:
Bybit, OKX), `ew2` (London: Deribit, Kraken Futures). An engine sees only its
region's venues. ClickHouse is the view across regions.

`config/streams.yaml` gives every publisher its own UDP port and every stream
its own id. A duplicate is refused.

```yaml
md-binance: { port: 40501, region: an1, streams: { md: 2011, tob: 2012 } }
kinds: { md: { reliable: true }, tob: { reliable: false } }
```

A publisher runs with `hostNetwork`, so its headless Service name resolves to
the node, which is where that node's driver listens. It refuses to start if
`POD_IP` is not `HOST_IP`. Subscribers use the name
(`md-binance.lab.svc.cluster.local`). The driver re-resolves it after 5 seconds
without data. On SIGTERM the publisher closes its publications immediately;
the driver client timeout is 10 seconds, so a killed publisher does not keep
the old session alive on heartbeats.

Every application runs its Aeron client's conductor in its own loop, never
on a thread of the client's, so a client nothing drives for those 10 seconds
is closed: `md` builds its runtime in the Nautilus actor's `on_start`, once
Nautilus has connected its venue clients. SIGTERM's handler writes a byte to
a pipe that the runtime reads every 10 ms; the runtime then stops the loop
and closes what the application publishes, its feeds and persist's stream,
at once.

`md` streams are reliable (trades, book changes, snapshots, bars, mark and
index prices, funding). `tob` (quotes) is best effort: no NAKs, and a slow
subscriber is dropped. Every publication uses `fc=max` and `ssc=true`, so a
slow subscriber and the archive spy never slow the publisher. Each message
fits one UDP frame.

Each node's archive spies the feeds published on that node. The spy reads
shared memory. Table switches (`enabled`, `apps`, `until`) apply when the
ingester inserts, not when the feed publishes, because subscribers need every
message.

The engine follows each `md` stream and its exchange's fills with
`Bus::subscribe`. It replays the publisher's archive (port 8010) until it
catches the live stream. If it falls behind, it drops back to the recording.
A restart or a move is a new recording; `PersistentSubscription` finds it by name and
replays from the first message. Finding it (the archive connect, the
recording list) is a state machine that each `poll` advances one step, so the
engine loop never waits on another region. The media driver resolves the
publisher's name off its own conductor, and resolves it again when the old
address goes quiet after a move. The engine drops that venue's books and
rebuilds them from the new session's snapshot.
`tob` is a plain best-effort subscription.

`exch-sim` starts at the beginning of its engine's `orders` recording and
ignores orders older than 10 seconds. The engine gives up after 30 seconds
and applies each fill once.

`IDLE` is one of:

- `spin`, `noop` or `yield`;
- `sleep[:<period>]`, 1 ms by default;
- `backoff[:<spins>,<yields>,<min park>,<max park>]`, Agrona's
  `BackoffIdleStrategy` with its defaults `10,5,1us,1ms`.

The lab backs off: no parking while messages flow, and an idle loop parks
for at most 1 ms. `TIMER_SLACK` (Linux) lowers the 50 µs the kernel may add
to each park. `just spin` is the isolated-core setting.

`tables.yaml` and `streams.yaml` are one ConfigMap, under a fixed name so a
change restarts nothing. Applications re-read them every second:
`tables.yaml` from their own loop (`Persist::poll`), `streams.yaml` from a
once-a-second timer (`lab::Watch`): one `stat`, and a read only when the file
changed. The kubelet
updates the mounted files at its next pod sync, about a minute later.
`just config` cuts that to seconds by annotating the pods. A GitOps tool such
as Flux can apply the same kustomization; it waits for the kubelet's sync,
and must supply the Secret `clickhouse` (key `password`) itself, which `just
deploy` makes from `CLICKHOUSE_PASSWORD` in `.env`.
A new service is recorded and subscribed with no restart. Changing a running
service's port or stream id needs a restart of that service and of every
subscriber of it (engines in every region; `exch-sim` for its engine's
orders): a feed already open keeps its address. The ingesters follow the
change on their own.

## Engine

One loop on one thread, which handles every message and timer and drives
the Aeron client's conductor. It keeps each instrument's L2 book as `Decimal9`
mantissas, and per asset an aggregated book (linear `size × multiplier`,
inverse `size × multiplier / price`), time-decayed EMAs of the mid, and a
strategy: the mid crossing its 5 minute EMA, at most one order per 30 seconds,
never past 0.01. Once a second it publishes `ema` and `agg_book` on `signals`,
and orders on `orders`. The archive records them.

Tick-to-trade is the checkpoint trace `tick_to_trade`: `feed`, `decode`,
`book`, `signal`, `decide`, `send`. Every tick updates the stage histograms.
A tick that sends an order is kept, under the order's id, and `exch-sim`'s
`order_ack` uses the same id. A replayed message rebuilds the book and does
nothing else. A stage that crosses a process boundary uses `Clock::wall`;
stages inside one process stay monotonic.

On this lab, with loops sleeping up to 1 ms, medians are roughly: feed 1–3 ms,
decode 3–7 µs, book 13–45 µs, decision about 1 µs, send 10–22 µs.

## Memory

The whole lab used about 3.4 GiB when measured on 2026-09-27, before the
cache reductions below. That includes Kubernetes; pod limits are ceilings,
not reserved memory. On a 16 GiB Mac, set Docker Desktop → Settings → Resources
→ Memory to 8 GiB and apply/restart, leaving the other 8 GiB for macOS and
host builds. A 16 GiB Docker allowance on that machine leaves no headroom
when the lab and builds peak together. The repository does not change this
host setting automatically.

| Pod | In use | Limit |
|---|---|---|
| `clickhouse` | 550–700 MiB | 1.25 GiB |
| `aeron` driver | 51–57 MiB | 256 MiB |
| `aeron` archive | 62–72 MiB | 192 MiB |
| `md-<exchange>` | 180–205 MiB | 256 MiB |
| `grafana` | ~130 MiB | 192 MiB |
| `jupyter` | ~75 MiB | 512 MiB |
| `ingester`, `engine`, `exch-sim` | engine heap ~17 MiB | 256, 128, 128 MiB |

A process is charged the pages of its own publications' log buffers as it
first writes them (the driver's directory is tmpfs): its persist stream's
48 MiB (three 16 MiB terms) and 3 MiB per feed, on top of its heap. An
engine reaches about 70 MiB once it has written its whole persist log; at
64 MiB it was killed for memory within minutes of a busy start.

`deploy/infra/clickhouse.xml` gives each mark, index-mark, primary-index and
query-condition cache 16 MiB, and bounds the optional query-result cache to
16 MiB. ClickHouse's tracked server memory has a 70% allowance inside its
1.25 GiB container; the remaining room is for native allocations. It uses
eight merge threads and 32 task slots, preserving MergeTree's default
free-slot thresholds. `clickhouse-users.xml` limits a query to two threads
and 384 MiB; oversized queries fail instead of exhausting the container.
See the [ClickHouse 25.8 server defaults](https://github.com/ClickHouse/ClickHouse/blob/v25.8.1.5101-lts/programs/server/config.xml)
and [MergeTree threshold checks](https://github.com/ClickHouse/ClickHouse/blob/v25.8.1.5101-lts/src/Storages/MergeTree/MergeTreeSettings.cpp).
Apply these changes with `just deploy`; the hashed settings ConfigMap
restarts ClickHouse. Aeron's driver uses an explicit `16m` IPC default.

`just test` and `just latency` use the same ClickHouse settings in their
separate `persist-test-clickhouse` container, capped at 1.25 GiB with no extra
swap allowance and two CPUs. The next run recreates that disposable container
if its settings changed; its test databases are disposable. The host test
Aeron JVM has a 128 MiB heap and 64 MiB direct-memory cap (mapped logs and
other JVM memory are additional); an old uncapped test driver is restarted.
Tests run with two test threads; `just test_threads=4 test` overrides this.
`just build` defaults to one Rust and native compiler job; use
`just build_jobs=2 build` when memory permits. Run `just test-stop` after
testing to release the test services. For benchmarks, `just mac stop` (kind) or
`just azure pause` (Azure) pauses the lab and `just mac start` / `just azure
resume` resumes it afterward, preserving recordings.

Check actual usage with `docker stats --no-stream` and
`kubectl --context kind-clickhouse-lab -n lab top pods --containers` (the
latter needs metrics-server). Check pod terminations with
`kubectl --context kind-clickhouse-lab -n lab get pods -o json`; an
`OOMKilled` termination confirms a container memory failure. These settings
still need a live workload check after deployment; the table above is the
historical measurement, not a measurement of the new settings.

## Record

**SBE.** Add the message to `schema/market.xml` or `schema/trading.xml`, list it in `tables.yaml`,
and encode into the Aeron claim. The table name is the message name in
snake_case. `encode` is not called when the table is off. `record` returns
`Ok` when Aeron drops the message; the drop is counted.

```rust
let len = TradeEncoder::compute_length_with_header(symbol.len(), venue.len(), trade_id.len());
persist.record(TradeEncoder::TEMPLATE_ID, len, |buf| {
    Ok(TradeEncoder::wrap_and_apply_header(buf, 0)
        .fixed(&TradeFixedFields { ts_event, ts_init, price: d9(price)?, size: d9(size)?, aggressor })
        .symbol(symbol)?
        .venue(venue)?
        .trade_id(trade_id)?
        .encoded_length_with_header())
})?;
```

Keep an instance with `let persist = Persist::connect(schema, &bus, settings)?`, where `bus = Bus::connect(&settings)?` is the application's Aeron client (feeds are opened on the bus, which the runtime owns and whose conductor its loop runs; `persist.drops()` counts its own records' drops plus the feeds', which `bus.drops()` counts), and
call `persist.record(...)` from the loop; a runtime agent has it as `ctx.persist()`.
There is no process-wide handle. `connect` waits up to 10 seconds for a
subscriber (`subscriber_timeout`).

**A row with no schema.** `record_row(table, fields)` records fields known
only at run time. Slower than `record`, so the sample records `spread` once a
second with the book, not on every quote. The quote path sets a gauge with the
handle below.

```rust
gauge.set(bps);
persist.record_row("spread", [("instrument", Value::Str(&id)), ("bps", Value::F64(bps))]);
```

Columns come from the fields. Integers are `Int64`/`UInt64`, floats `Float64`,
bools `Bool`, strings `String`, all `Nullable`. A missing field is NULL.
Every row has `ts DateTime64(9)`.

**Tracing event.** The same row from any thread, through the `tracing`
bridge. `Persist::layer` makes it once, on the loop thread at start-up, on a
concurrent publication of its own. It switches nothing: the ingester applies
`enabled`, `apps` and `until` to its rows per app, at each row's time. Give a
log layer its own filter. A global level filter hides these events from the
bridge too.

```rust
tracing::subscriber::set_global_default(tracing_subscriber::registry().with(persist.layer()?))?;
tracing::info!(table = "signal", instrument = %id, edge = 0.25);
```

**Any Rust value.** `record_value(table, &value)` records a `Serialize` value.
The first value of a type fixes the shape. A later value that does not fit
grows it, in declaration order, and the table gains the columns.

| Rust | Column |
|---|---|
| `bool`, integers, floats | `Nullable(Bool)`, `Nullable(Int64)` / `UInt64`, `Nullable(Float64)` |
| `str`, `String`, `char`, `i128`, `u128` | `Nullable(String)` |
| `Option::None` | NULL |
| struct, tuple | dotted fields: `spread.bps`, `pair.0` |
| `Vec`, slice, set | `bids.price Array(Nullable(Float64))` |
| list in a list | `Array(Array(...))` |
| map | `tags.key`, `tags.value` |
| enum | variant name in `regime`, fields in `regime.Wide.bps` |

A list nested in a struct is named with underscores (`stats.bids` becomes
`stats_bids`), because ClickHouse treats `a.b` columns as one Nested structure
whose arrays must be the same length. A fixed-size array serializes as a tuple.
Raw bytes are not recorded. Deeper than 8 lists or 32 structs is refused.

A new event shape is published before its first row. Every 5 seconds
`Persist::poll` sends the source message, each shape, and each trace
definition again, up to 8 of them a call; one too long for a claim is passed
over. The bridge sends each thread's again before that thread's next row or
span after 5 seconds. The ingester
saves shapes next to its checkpoint. A row with no shape waits 30 seconds,
and nothing is checkpointed past it.

Put another application's schema in `schema/`, list its tables, and run
`just md`. Keep one version of each schema. The newest decodes older records.

## Several schemas

`market::AnyMessage` is every message of `market.xml`. `trading::AnyMessage`
is every message of `trading.xml`. Template ids start at 1 in each file, so
they do not identify a message on their own. A buffer that might be either
is `schema::AnySchemaMessage`. The build generates it with
`Generator::generate_schema_dispatch` over every configured codec module. It reads
the header schema id and decodes with that module's enum. A schema id that is neither (persist events, metrics, traces) is
`Other`, not an error.

```rust
match schema::AnySchemaMessage::decode(frame, 0)? {
    schema::AnySchemaMessage::Market(schema::market::AnyMessage::Trade(trade), _) => {
        trade.symbol_as_str()?;
    }
    schema::AnySchemaMessage::Trading(schema::trading::AnyMessage::Ema(ema), _) => {
        ema.asset_as_str()?;
    }
    schema::AnySchemaMessage::Other { schema_id, template_id, .. } => {
        let _ = (schema_id, template_id);
    }
    _ => {}
}
```

Bound `frame` to one transport message; unknown schemas and templates retain
that complete byte range. `message.as_bytes()` borrows the SBE header and body,
so the persistence writer accepts the combined enum without re-encoding:

```rust
let message = schema::AnySchemaMessage::decode(frame, 0)?;
writer.push(message.as_bytes(), source_id);
writer.tick(); // deliver all available live frames before flushing elapsed windows
```

The archive ingester loads every `.xml` it is given and routes a frame by that
same schema id and template id. A schema that was not
compiled into the `schema` crate still persists. The engine's book stream is
already one schema, so it keeps calling `market::AnyMessage` directly.

Every row ends with `host`, `pod`, and `app`. The Aeron frame header carries
the source id. A `Source` message names it.

## Metrics

Handles live on the loop's thread (`Rc`'d cells, neither `Send` nor `Sync`),
and nothing locks. A counter or histogram handle is a cell of its own: a
counter adds with a load and a store, a histogram updates count, sum, min, and
max with loads and stores. The same series asked for again is another cell,
added at `poll`. A gauge is shared. The last write wins.

```rust
let sent = metrics.counter("orders_sent", &[("venue", "binance")]);
let depth = metrics.gauge("book_depth", &[("side", "bid")]);
let t2t = metrics.histogram("tick_to_trade_ns", &[]);
loop {
    let now = clock.now();
    sent.inc();
    depth.set(12.0);
    t2t.record(850);
    persist.poll(now); // three compares until a millisecond, the 5 s boundary or a config re-read is due
}
```

Metrics are handles only: `tracing` events with a `counter`, `gauge` or
`histogram` field are not recorded. Make handles from the instance the loop
owns; no static metric or tracer exists. A scoped subscriber with the bridge
works too, for spans:

```rust
use tracing_subscriber::layer::SubscriberExt;
let persist = Persist::connect(schema, &bus, settings)?;
let sent = persist.metrics().counter("orders_sent", &[("venue", "binance")]);
let subscriber = tracing_subscriber::registry().with(persist.layer()?);
tracing::subscriber::with_default(subscriber, || {
    tracing::info_span!("send_order", venue = "binance").in_scope(|| sent.inc());
});
persist.poll(clock.now()); // keep polling in the application loop
```

`info_span!` and `#[tracing::instrument]` produce span traces through the
bridge; the ingester keeps them while `otel_traces` is on for the app.
Handles update immediately but publish only when `poll` runs.
`Persist::poll` also re-reads `tables.yaml` once a second; nothing else does,
so an application that never polls never sees an edit.
Use `persist.tracer(...)` for instance-owned checkpoint traces on the hot path.

A histogram records the count, the sum, the minimum, and the maximum. Every
millisecond that had samples, `poll` publishes that summary. An empty
millisecond publishes nothing. Counters and gauges stay on the 5 s interval.

The ingester keeps an HdrHistogram of 3 significant figures for each series
and folds each summary into the 5 s window: the minimum, the maximum, and
the mean of the rest. A busy millisecond pulls the percentiles toward that
mean. The row stores `count`, `sum`, `min`, `max`, `avg`, `p50`, `p75`,
`p90`, `p99`, `p999`, `p9999`, and `p99999`. These percentiles estimate the
reconstructed distribution; the summaries cannot recover the original one.
For example, a millisecond containing values 1 through 1000 reconstructs most
values near 500, even though the original p99 is near 990.
`avg` is the exact sum divided
by the count. `min` and `max` stay exact. `interval_ns` is 5 s.

Percentiles do not merge across rows. Query each stored window's estimates
separately. A count-weighted average of p99 values is an average of window
estimates, not the p99 of all samples; the dashboards label it accordingly.
Counts, sums, minima, and maxima do combine across windows:

```sql
SELECT sum(count) AS sample_count, sum(sum) / sum(count) AS mean,
       min(min) AS minimum, max(max) AS maximum
FROM metrics.metrics_histogram
WHERE name = 'tick_to_trade_ns' AND ts > now() - INTERVAL 1 HOUR
```

With a database of their own (`metrics` in the lab), every metric name is
also a view there that reads only its rows: `SELECT * FROM
metrics.tick_to_trade_ns`. A metric named like a table gets no view.

`metrics` is `ts`, `name`, `kind`, `series`, `labels`, `value`, `delta`.
`metrics_histogram` is `ts`, `name`, `series`, `labels`, `interval_ns`,
`count`, `sum`, `min`, `max`, `avg`, then the percentiles above. An existing
`metrics_histogram` does not gain a column until you run the `ALTER` the
ingester logs, or drop the table. A missing column is skipped.

## Traces

A checkpoint trace stamps stages on the stack. `finish` always updates
`trace_ns{trace, stage}`. It publishes the trace only when `otel_traces` is
on and the trace is sampled, slower than the threshold, or kept.

```rust
let t2t = ctx.tracer("tick_to_trade", &["wire", "decode", "decide", "send"], &["levels"]);
let mut t = t2t.start(Nanos::from_epoch(ts_event), TraceId::new(ORDERS, order_id));
t.mark(clock.now());
t.attr(0, levels);
t.finish();
```

```yaml
otel_traces:
  kind: static
  enabled: false
  apps: { binance: { until: 2026-09-27T18:00:00Z } }
  traces:
    tick_to_trade: { sample: 1000, slower_than: 5s }
```

`slower_than` judges the whole trace, from `start`. A trace started at the
venue's timestamp, as above, also counts the venue's lag and the network.
Set the threshold above their usual total, or nearly every trace publishes
as slow. The lab's is 5 s; a trace started at the receive can use 50 µs.

The ingester writes `otel_traces` as one span for the whole and one per stage.
The same trace id in two processes is one waterfall. At most 16 stages and 8
numeric attributes. A stage that ends before it began is 0 in the histogram
and counted in `trace_clamped`.

`tracing` spans (`info_span!`, `#[instrument]`) are the same kind of trace
through the bridge (`Persist::layer`), from any thread; the ingester keeps
them only while `otel_traces` is on for the app. A span costs the `tracing`
registry, an allocation and a claim. Leave spans off the book loop. The
checkpoint tracer is the path that stays on the stack.

## Clock

```rust
let clock = Clock::new();
let now = clock.now();     // one read per loop
let t = clock.cached();    // last read
```

`Nanos` is signed UNIX-epoch nanoseconds: the wall clock paired with the
monotonic clock when the `Clock` is made, plus the monotonic time since. On
Linux a read is `minstant` (the time-stamp counter when it is available).
Elsewhere it is `Instant`.

## Aeron counters

Every 5 seconds the ingester reads the driver's CnC file.

| Table | Rows |
|---|---|
| `aeron_counters` | `value`, `delta`, and the label split into `type`, `session_id`, `stream_id`, `channel`, `recording_id`, `client_name` |
| `aeron_errors` | each distinct driver error |
| `aeron_loss` | data loss, when it grows |

These are sampled from shared memory. While the ingester is down, nothing is sampled.

## tables.yaml

```yaml
tables:
  trade:         { kind: static }
  book_snapshot: { kind: dynamic, enabled: false }
  spread:        { kind: dynamic }
  book_deltas:
    kind: dynamic
    enabled: false
    apps: { binance: true, deribit: { until: 2026-09-27T18:00:00Z } }
```

`enabled` is `true`, `false`, or `{ until: <UTC time> }`. `apps` overrides it
per `PERSIST_APP`. Applications re-read the file every second, in
`Persist::poll`.

`dynamic` adds a column when a new field arrives. `static` never alters the
table. A missing or wrong column is skipped, and the log prints the `ALTER`.
A changed type is never altered for you. `kind` is one value for every app.

`database` names the ClickHouse database a table is kept in. Without one, a
table is kept in the ingester's `CLICKHOUSE_DATABASE` (`md`). The lab keeps
market data in `md`, the engines' tables in `engine`, orders in `orders`,
traces in `tracing`, and metrics and Aeron counters in `metrics`. One query
joins across them:

```sql
SELECT minute, orders, trades
FROM (SELECT toStartOfMinute(ts) AS minute, count() AS orders FROM orders.new_order GROUP BY minute) AS o
JOIN (SELECT toStartOfMinute(ts_event) AS minute, count() AS trades FROM md.trade GROUP BY minute) AS t
USING minute
ORDER BY minute
```

Moving a table makes a new one in the new database. Rows already written
stay where they were (before this, in `market`).

## Types

| SBE | ClickHouse |
|---|---|
| integers, float, double | `Int8`…`UInt64`, `Float32`, `Float64` |
| decimal: mantissa + constant exponent | `Decimal(18, S)` |
| `UTCTimestamp`, or time + constant unit | `DateTime64` |
| enum | `LowCardinality(String)`, the name |
| `char` array, var-data | `String` |
| optional | `Nullable(T)` |
| group `bids { price }` | `bids.price Array(...)` |

The mantissa or tick count is stored as-is. `Decimal9` is nine decimal places,
±9.2 billion. `d9()` errors instead of rounding. Other composites, sets,
non-`char` arrays, nested groups, and big-endian schemas are rejected.

Add fields at the end of the block with their `sinceVersion`. New groups and
var-data need `sinceVersion` too. A newer fixed field is absent in old records
even when its bytes fit in the old block's padding. Then `just md`.

## Durability

If ClickHouse is down, the ingester stops and the archive holds the data.
After a successful insert the ingester saves `recording position` and deletes
the segments behind it. A crash before that save replays the same batch.
The insert's token is those positions, and ClickHouse drops the repeat.
Recovery collects the entire pending batch before inserting, even when replay
delivery takes several ticks. A prefix cannot consume the full batch's token.
Each table remembers the last 1000 inserts. An older retry can land twice.

One failed table holds that recording's checkpoint, so the archive grows until
the table is fixed. A new session (restart or move) is a new recording.

## Latency

`just latency`, 2026-09-27, Apple M4, rustc 1.98.1, lab running on the same
machine. Timer resolution is 42 ns. Metric and clock rows below are amortized
costs of 100-operation batches; their divided quantiles do not describe
individual-operation tail latency. Nothing was dropped in that historical run.

| Call | p50 | p99 | p99.9 |
|---|---|---|---|
| empty loop | 41 ns | 42 ns | 125 ns |
| `record()`, SBE | 83 ns | 292 ns | 1.2 µs |
| `tracing` event, on / off | 125 ns / 42 ns | 792 ns / 291 ns | 7.6 µs / 2.0 µs |
| `record_value`, flat / nested | 166 ns / 375 ns | 458 ns / 1.5 µs | 5.1 µs / 13 µs |
| counter, gauge, histogram | 1.3 / 1.3 / 1.7 ns | 2–5 ns | 4–8 ns |
| `Clock::cached` / `Clock::now` | 1.3 ns / 21 ns | 2.9 ns / 90 ns | 5.4 ns / 1.1 µs |
| `poll`, 5 s / 1 ms metrics intervals (mixed idle/due calls) | 41 ns / 42 ns | 42 ns / 250 ns | 458 ns / 3.7 µs |
| checkpoint trace, off / unsampled / published | 42 / 41 / 83 ns | 84 / 84 / 333 ns | 625 ns / 500 ns / 1.5 µs |
| `tracing` span, off / on | 83 ns / 250 ns | 209 ns / 667 ns | 1.8 µs / 5.9 µs |

The histogram figure of 1.7 ns includes the old per-value bucket update.
`Histogram::record` now updates count, sum, min, and max in its own cell, with
loads and stores and no lock. The histogram and checkpoint-trace rows have not
been remeasured since. With a histogram registered, `poll` is due every 1 ms.
The idle poll figure was measured before that. Due counter/gauge and histogram
cycles run in deadline order so neither can starve the other.

The current harness selects the operation before timing and makes handle,
value, input and output observations opaque to the optimizer. It reports both
raw batch quantiles and amortized batch costs. `control-x100` measures the
amplified loop floor. The arms for the installed handle and for `tracing`
metric events are gone with them. `event`, `event-off`, `span-on` and
`span-off` go through the `tracing` bridge, which switches nothing: an off arm
costs what its on arm does, and the ingester leaves its rows out.
`poll-idle` and `poll-due` both include mostly idle polls at the 5 µs cadence;
`poll-due` uses a 1 ms metrics interval and includes counter deadlines too.
Drops during measurement are reported separately from total drops, which
include warm-up. These harness changes have not yet been measured through the
live archive and ClickHouse path; the table above remains historical.

`Clock::now` here is `mach_absolute_time`. On x86-64 Linux it is `rdtsc`,
which this run did not measure.

## What each exchange records

Two instruments, BTC and ETH. Kraken Futures has no bars. XBT is published as BTC.

| Table | Binance | Bybit, OKX | Deribit | Hyperliquid | Kraken |
|---|---|---|---|---|---|
| `trade`, `quote`, books, `instrument_spec` | yes | yes | yes | yes | yes |
| `bar` | yes | yes | yes | yes | |
| mark, index, funding | | yes | yes | yes | yes |
| `ticker`, `instrument`, `instrument_status`, `spread` | yes | yes | yes | yes | yes |

Deribit also records `deribit_volatility_index`. Hyperliquid records open
interest and public trades. Venue-specific decimals stay text.
`ticker` is one row per instrument per second.

## Limits

- `record()` does not wait. A drop is `not_connected`, `back_pressure`, `too_large`, or `other`, logged once a second while the total grows. A term rotation is tried eight times, then counted as `other`. One claim holds up to 64 KiB.
- A table ClickHouse refuses holds that recording's checkpoint. The archive grows. The rows are not lost.
- A replay older than the last 1000 inserts of that table can land twice.
- Restarting the `aeron` pod restarts its clients. Records in that gap are dropped.
- An event column's type is fixed by the first value. A later value of another type becomes NULL when it cannot convert.
- A `tracing` field that is a nested struct is stored as debug text. Use `record_value` for columns.
- In `record_row`, a JSON null omits the field, so each pattern of nulls is its own shape.
- `just verify` counts restarts. After pausing and resuming the lab its last check fails.
- Two publications on one stream must use the same channel parameters.
- Metrics of a process that exits before the next `poll` are lost. A counter's `delta` survives a restart. Its `value` starts again at 0.
- A crash can drop the histogram window still open in the ingester, at most 5 s.
- A heartbeat round, every 5 s, sends up to 8 dictionary messages per `Persist::poll` call until it is done.
- A persistent subscription can replay only the segments the ingester has not yet purged.
- The kind VM clock can lag the venues by 100–400 ms. `venue_to_local_ns` then reads 0.
- Options are not recorded. Nautilus needs a live instrument picked by expiry.
