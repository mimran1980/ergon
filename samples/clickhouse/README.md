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
| `persist-client` | What the application links: `record()`, a `tracing` layer for rows and for counters, gauges, and histograms, metric handles, traces, a clock, UDP feeds. |
| `persist-server` | The ingester. Replays the archive into ClickHouse, checkpoints, purges. It routes frames from schema XML, not from generated codecs. |
| `schema` | `market.xml`, `trading.xml`, the codecs generated from them, and `AnySchemaMessage` for a buffer that may be either. |
| `md` | One exchange's public market data, via NautilusTrader. No API keys. |
| `engine` | One region's engine, and `exch-sim`, its dummy exchange. |
| `aeron-driver` | Aeron's C media driver (1.52.2), configured from the environment. |

## Run

```sh
cd samples/clickhouse
just up        # kind cluster, images, deploy
just deploy    # apply kustomization.yaml after a config or manifest edit
just md        # rebuild the app image and restart publishers and ingesters
just aeron     # rebuild Aeron, then restart its clients
just verify    # check the running lab, including moving a feed between nodes
just logs
just test      # unit and integration tests; leaves ClickHouse and an Aeron driver up
just test-stop
just stop      # pause the cluster, keep the data
just destroy   # delete the cluster and the data
```

Needs Docker, kind, kubectl, just, and jq. Ports listen on every interface.
Grafana is anonymous admin and Jupyter has no token, so use a network you trust.

| | |
|---|---|
| ClickHouse | <http://localhost:8123/play> user `lab`, password `lab` |
| Grafana | <http://localhost:3000> |
| Notebook | <http://localhost:8888/lab/tree/verify.ipynb> |
| What is recorded | `config/tables.yaml`. `just config` publishes it; pods see it in about a minute. |
| Who publishes | `config/streams.yaml`, published the same way. |

| Pod | What |
|---|---|
| `aeron` | DaemonSet, host network. C driver and Java archive. Driver directory on the node's tmpfs; recordings on its disk. |
| `ingester` | DaemonSet. Archive to ClickHouse. Checkpoint on the node's disk. |
| `md-<exchange>` | One feed handler, host network, pinned to its region. |
| `engine-<region>`, `exch-sim-<region>` | The engine and its dummy exchange, host network. |

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
`Persist::persistent`. It replays the publisher's archive (port 8010) until it
catches the live stream. If it falls behind, it drops back to the recording.
A restart or a move is a new recording; `Persistent` finds it by name and
replays from the first message, on another thread, so the engine loop does not
wait. The engine drops that venue's books and rebuilds them from the new
session's snapshot. `tob` is a plain best-effort subscription.

`exch-sim` starts at the beginning of its engine's `orders` recording and
ignores orders older than 10 seconds. The engine gives up after 30 seconds
and applies each fill once.

`IDLE` is `spin`, `noop`, `yield`, or `sleep`. The lab sleeps. `just spin` is
the isolated-core setting.

`tables.yaml` and `streams.yaml` are one ConfigMap. Applications re-read them.
A new service is recorded and subscribed with no restart. Changing a running
service's port or stream id needs a restart of that service.

## Engine

One thread, one loop. It keeps each instrument's L2 book as `Decimal9`
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

About 3.4 GiB for the whole lab, measured 2026-09-27. Give Docker Desktop
about 8 GiB, not the whole machine.

| Pod | In use | Limit |
|---|---|---|
| `clickhouse` | 550–700 MiB | 1.25 GiB |
| `aeron` driver | 51–57 MiB | 256 MiB |
| `aeron` archive | 62–72 MiB | 192 MiB |
| `md-<exchange>` | 20–80 MiB | 256 MiB |
| `grafana` | ~130 MiB | 192 MiB |
| `jupyter` | ~75 MiB | 512 MiB |
| `ingester`, `engine`, `exch-sim` | 1–4 MiB | 256, 64, 64 MiB |

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

Keep an instance with `let persist = Persist::connect(schema, settings)?` and
call `persist.record(...)`. Optional `persist.install()` enables the free functions.
With nothing installed, the free `record` does nothing. `connect` waits up to 10 seconds for a
subscriber (`subscriber_timeout`).

**Tracing event.** For a row with no schema. Slower than `record`, so the
sample does it once a second, not on every quote. The quote path sets a gauge
with the handle below, not with this macro.

```rust
tracing::subscriber::set_global_default(tracing_subscriber::registry().with(persist.layer()))?;
gauge.set(bps);
tracing::info!(table = "spread", instrument = %id, bps);
```

Columns come from the fields. Integers are `Int64`/`UInt64`, floats `Float64`,
bools `Bool`, strings `String`, all `Nullable`. A missing field is NULL.
Every row has `ts DateTime64(9)`. Give a log layer its own filter. A global
level filter hides these events from persist too.

`record_row(table, fields)` is the same table for fields known only at runtime.

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

A new event shape is published before its first row. Every 5 seconds a repeat
of the source message, each shape, and each trace definition is queued, and
each following `record` sends one of them. The ingester saves shapes next to
its checkpoint. A row with no shape waits 30 seconds, and nothing is
checkpointed past it.

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

One writer per counter or histogram handle (`Send`, not `Sync`). Counters use
a relaxed load and store. Histograms lock their own cell so a poll on another
thread takes count, sum, min, and max together. The same series on another
thread is another cell, added at `poll`. A gauge is shared. The last write wins.

```rust
let sent = metrics.counter("orders_sent", &[("venue", "binance")]);
let depth = metrics.gauge("book_depth", &[("side", "bid")]);
let t2t = metrics.histogram("tick_to_trade_ns", &[]);
loop {
    let now = clock.now();
    sent.inc();
    depth.set(12.0);
    t2t.record(850);
    metrics.poll(now); // one compare until the next millisecond or the 5 s boundary
}
```

The same registry accepts `tracing` events when `Persist::layer` is installed.
Use that off the hot path. A counter with no `value` adds 1. An explicit value must be a nonnegative integer;
invalid values are ignored. Other fields are
labels. `poll` still publishes them. These events allocate the label strings
and take the registry lock; the handles above do not. A handle and a tracing
event for the same counter or histogram are two cells, added at `poll`.

```rust
tracing::info!(counter = "orders_sent", venue = "binance");
tracing::info!(gauge = "book_depth", side = "bid", value = 12.0);
tracing::info!(histogram = "tick_to_trade_ns", value = 850u64);
```

The layer owns a clone of the `Persist` instance. No `Persist::install` or
static metric/tracer is needed. A scoped subscriber works too:

```rust
use tracing_subscriber::layer::SubscriberExt;
let persist = Persist::connect(schema, settings)?;
let metrics = persist.metrics();
let subscriber = tracing_subscriber::registry().with(persist.layer());
tracing::subscriber::with_default(subscriber, || {
    tracing::info_span!("send_order", venue = "binance").in_scope(|| {
        tracing::info!(counter = "orders_sent", venue = "binance");
        tracing::info!(histogram = "send_ns", value = 850u64);
    });
});
metrics.poll(clock.now()); // keep polling in the application loop
```

`info_span!` and `#[tracing::instrument]` produce span traces while `otel_traces`
is enabled. Metric events update immediately but publish only when `poll` runs.
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
FROM market.metrics_histogram
WHERE name = 'tick_to_trade_ns' AND ts > now() - INTERVAL 1 HOUR
```

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
let t2t = persist_client::tracer("tick_to_trade", &["wire", "decode", "decide", "send"], &["levels"]);
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
    tick_to_trade: { sample: 1000, slower_than: 50us }
```

The ingester writes `otel_traces` as one span for the whole and one per stage.
The same trace id in two processes is one waterfall. At most 16 stages and 8
numeric attributes. A stage that ends before it began is 0 in the histogram
and counted in `trace_clamped`.

`tracing` spans (`info_span!`, `#[instrument]`) are the same kind of trace
through `Persist::layer`, recorded only while `otel_traces` is on. A span
costs the `tracing` registry and an allocation. Leave both spans and the
metric events above off the book loop. The checkpoint tracer is the path
that stays on the stack.

## Clock

```rust
let clock = Clock::new();
let now = clock.now();     // one read per loop
let t = clock.cached();    // last read
```

`Nanos` is signed nanoseconds from one anchor per process. On Linux a read is
`minstant` (the time-stamp counter when it is available). Elsewhere it is
`Instant`.

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
per `PERSIST_APP`. Applications re-read the file every second.

`dynamic` adds a column when a new field arrives. `static` never alters the
table. A missing or wrong column is skipped, and the log prints the `ALTER`.
A changed type is never altered for you. `kind` is one value for every app.

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

Add fields at the end of the block. New groups and var-data need `sinceVersion`.
Old records still decode. Then `just md`.

## Durability

If ClickHouse is down, the ingester stops and the archive holds the data.
After a successful insert the ingester saves `recording position` and deletes
the segments behind it. A crash before that save replays the same batch.
The insert's token is those positions, and ClickHouse drops the repeat.
Each table remembers the last 1000 inserts. An older retry can land twice.

One failed table holds that recording's checkpoint, so the archive grows until
the table is fixed. A new session (restart or move) is a new recording.

## Latency

`just latency`, 2026-09-27, Apple M4, rustc 1.98.1, lab running on the same
machine. Timer resolution is 42 ns. Metric and clock rows are 100 operations
per sample, divided back to one. Nothing was dropped.

| Call | p50 | p99 | p99.9 |
|---|---|---|---|
| empty loop | 41 ns | 42 ns | 125 ns |
| `record()`, SBE | 83 ns | 292 ns | 1.2 µs |
| `record()`, installed / not installed | 83 ns / 41 ns | 292 ns / 42 ns | 2.4 µs / 125 ns |
| `tracing` event, on / off | 125 ns / 42 ns | 792 ns / 291 ns | 7.6 µs / 2.0 µs |
| `record_value`, flat / nested | 166 ns / 375 ns | 458 ns / 1.5 µs | 5.1 µs / 13 µs |
| counter, gauge, histogram | 1.3 / 1.3 / 1.7 ns | 2–5 ns | 4–8 ns |
| `Clock::cached` / `Clock::now` | 1.3 ns / 21 ns | 2.9 ns / 90 ns | 5.4 ns / 1.1 µs |
| `poll`, idle / publishing | 41 ns / 42 ns | 42 ns / 250 ns | 458 ns / 3.7 µs |
| checkpoint trace, off / unsampled / published | 42 / 41 / 83 ns | 84 / 84 / 333 ns | 625 ns / 500 ns / 1.5 µs |
| `tracing` span, off / on | 83 ns / 250 ns | 209 ns / 667 ns | 1.8 µs / 5.9 µs |

The histogram figure of 1.7 ns includes the old per-value bucket update.
`Histogram::record` now locks its cell and updates count, sum, min, and max.
The histogram and checkpoint-trace rows have not been remeasured with that
lock. With a histogram registered, `poll` is due every 1 ms. The idle poll
figure was measured before that. Due counter/gauge and histogram cycles run in
deadline order so neither can starve the other.

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
- `just verify` counts restarts. After `just stop` / `just start` its last check fails.
- Two publications on one stream must use the same channel parameters.
- Metrics of a process that exits before the next `poll` are lost. A counter's `delta` survives a restart. Its `value` starts again at 0.
- A histogram read from another thread while it is being updated can disagree on `sum` and count. Poll from the recording thread.
- A crash can drop the histogram window still open in the ingester, at most 5 s.
- A heartbeat round sends one dictionary message per `record` until it is done. A quiet process finishes it only as fast as it records.
- A persistent subscription can replay only the segments the ingester has not yet purged.
- The kind VM clock can lag the venues by 100–400 ms. `venue_to_local_ns` then reads 0.
- Options are not recorded. Nautilus needs a live instrument picked by expiry.
