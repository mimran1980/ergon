# ClickHouse recording lab

Record anything a low-latency application sees into ClickHouse, and switch
recording on and off while it runs. The application only publishes to Aeron;
a separate ingester does the ClickHouse work. The lab around it is a small
trading deployment: feed handlers publishing market data over UDP, one
shared media driver per node, and an engine per region trading on it.

```text
             node (one of four, in three regions)
 ┌──────────────────────────────────────────────────────────────────────┐
 │ md-<exchange> ──UDP MDC feeds──▶ other nodes' engines                │
 │      │  (IPC: events, metrics, traces)                                │
 │      ▼                                                                │
 │ aeron pod: C media driver ◀── spy ── Java archive ──replay──▶ ingester ──▶ ClickHouse
 │ engine-<region> ──orders──▶ exch-sim-<region> ──fills──▶ engine        │
 └──────────────────────────────────────────────────────────────────────┘
```

| Crate | What it is |
|---|---|
| `persist-client` | The small library the application links: `record()` for SBE messages, a `tracing` layer for everything else, metrics, traces, a low-latency clock, UDP feeds (`Feed`, `Subscriber`, `Persistent`) and the feed registry. |
| `persist-server` | The ingester, as a library and the `ingester` binary: replays the archive into ClickHouse tables, checkpoints, purges. |
| `market` | The lab's SBE codecs: `schema/market.xml` (market data) and `schema/trading.xml` (EMAs, aggregated books, orders, fills). |
| `md` | A feed handler: public market data from one exchange (Binance, Bybit, OKX, Deribit or Hyperliquid) via NautilusTrader, no API keys, published as SBE on its UDP feeds. |
| `engine` | `engine`, a region's trading engine, and `exch-sim`, its dummy exchange. |
| `aeron-driver` | The C media driver (Aeron 1.52.2, from `rusteron-media-driver`), configured from the environment. |

## Run it

```sh
cd samples/clickhouse
just up                # kind cluster (4 nodes, 3 regions): ClickHouse, Grafana, JupyterLab, Aeron, the ingesters, every exchange, an engine per region
just exchange deribit  # (re)deploy an exchange's feed handler: binance bybit okx deribit hyperliquid
just unexchange okx    # remove one; its data stays
just engines           # (re)deploy each region's engine and dummy exchange
just md                # rebuild + redeploy the feed handlers, engines and ingesters after a code or schema change
just aeron             # rebuild + redeploy Aeron, then its clients
just verify            # end-to-end check of the running lab, including moving a feed handler between nodes
just logs              # feed handler, engine, ingester and Aeron logs
```

You need Docker, kind, kubectl, just and jq.

Every port listens on all interfaces, so the lab is reachable from other
machines on your network as well as from `localhost` (`just urls` prints the
addresses). That includes Grafana (anonymous admin) and JupyterLab (no token,
this checkout mounted read-write): run it only on a network you trust.

| What | Where |
|---|---|
| ClickHouse query UI | <http://localhost:8123/play> (user `lab`, password `lab`) |
| Grafana | <http://localhost:3000>: *Trading* (EMAs, aggregated book, orders, tick-to-trade), *Aeron* (feeds by node, receivers behind senders, MDC destinations, NAKs, re-resolutions), *Market data*, *ClickHouse tables*, *Metrics* and *Traces* |
| Notebook | <http://localhost:8888/lab/tree/verify.ipynb>, then Run All |
| What is recorded | `config/tables.yaml`: edits apply within a second |
| Who publishes what | `config/streams.yaml`: every service's port, region and streams |

| Pod | Kind | What it runs |
|---|---|---|
| `aeron` | DaemonSet, one per node, host network | The C media driver (conductor, sender and receiver threads) and the Java archive as its client. Its directory is on the node's tmpfs, so every pod on the node shares it; the recordings are on the node's disk. |
| `ingester` | DaemonSet, beside each Aeron | Archive -> ClickHouse. Its checkpoint is on the node's disk. |
| `md-<exchange>` | Deployment per exchange, host network, in its region | NautilusTrader + persist-client for one exchange (`deploy/md.yaml`). |
| `engine-<region>`, `exch-sim-<region>` | Deployment per region, host network | The engine and its dummy exchange (`deploy/engine.yaml`). |

`just stop` / `just start` pause the cluster and keep the data;
`just destroy` deletes it. `just test` leaves a ClickHouse and an Aeron
driver running for the next run; `just test-stop` frees their memory.

## Architecture

**Regions.** Each kind node carries `topology.kubernetes.io/region`: `an1`
(Tokyo, two nodes: Binance, Hyperliquid), `as1` (Singapore: Bybit, OKX) and
`ew2` (London: Deribit). A feed handler runs in its exchange's region, on
any node there; each region has one engine and one dummy exchange, which see
only that region's venues. The global view is ClickHouse.

**One registry, no clashes.** `config/streams.yaml` gives every publishing
service its own UDP control port and every stream its own id, so any
services can share a node's driver and a pod can move anywhere without a
clash. `Streams::parse` refuses a duplicate port or id. Adding a feed is one
line.

```yaml
md-binance: { port: 40501, region: an1, streams: { md: 2011, tob: 2012 } }
kinds: { md: { reliable: true }, tob: { reliable: false } }
```

**Finding a publisher that moves: Kubernetes DNS is the dynamic DNS.** Every
publisher runs on the host network, so its pod IP is its node's IP, and has a
headless Service of its own name. `md-binance.lab.svc.cluster.local` so
always resolves to the node, and the driver, it runs on; Kubernetes
registers and deregisters it as the pod moves. Publishers bind their
multi-destination-cast (MDC) control socket on their node
(`control=$HOST_IP:40501|control-mode=dynamic`); subscribers name it
(`endpoint=$HOST_IP:0|control=md-binance.lab.svc.cluster.local:40501|control-mode=dynamic`).
The driver resolves the name, and re-resolves it after 5 s without data, so
a subscription follows a moved publisher with no help from the application.
`verify` moves md-binance between the two `an1` nodes: its engine takes data
from the new node within seconds.

What sets the gap is not DNS but the old publisher's liveness: a killed
client's publication keeps heartbeating until the driver times the client
out, and the subscriber does not re-resolve while it hears heartbeats. So
the drivers' client timeout is 10 s, not 30, and every publisher closes its
publications on SIGTERM (`Persist::shutdown`), which ends the old session at
once.

**Reliable and best-effort feeds.** Every feed publishes with `fc=max` (the
fastest subscriber sets the pace, so a slow one never holds a feed handler
back) and `ssc=true` (the archive's spy counts as a subscriber, so a feed
nobody subscribes to is still published and recorded). `md` streams (trades,
book changes and snapshots, bars, mark and index prices, funding) are
reliable: subscribers NAK and are repaired. `tob` streams (quotes) are
`reliable=false|tether=false|group=false`: no NAKs, and a slow subscriber is
dropped rather than slowing anyone. Every feed message fits one UDP frame
(1408-byte MTU): md sizes its `book_deltas` chunks from `Feed::max_payload`.

**Persisting a feed at IPC speed, wherever its publisher runs.** Each node's
archive spy-records every stream of the registry on its own node's address
(`aeron-spy:aeron:udp?control=$HOST_IP:<port>|control-mode=dynamic`). A spy
reads the publication's term buffers in shared memory: no network hop, and no
work for the publisher. Wherever md-binance lands, that node's archive is
already listening, and its ingester replays the new recording; the old
node's recording stops, is ingested and purged. Each feed carries the
`Source` message itself, so its rows name their app, host and pod. A feed's
table switches (`enabled`, `apps`, `until`) are applied when rows are
inserted, not when published: subscribers need every message.

**Persistent subscriptions: nothing lost, nobody waits.** The engine takes
each `md` stream, and its exchange's fills, through Aeron's persistent
subscription (`Persist::persistent`): over the recording that the archive on
the publisher's node makes, reached by the publisher's name on port 8010. It
replays until it catches the live stream, then takes the live stream; an
engine that falls behind drops back to the recording and rejoins, and the
publisher (`fc=max`) never waits for it. A persistent subscription follows
one recording, and a restarted or moved publisher's new session is a new
recording, on whichever node it now runs: Aeron fails the old subscription
once it has replayed it to its end, and `Persistent` finds the new
recording, by name, and replays it from its first message. Its handler is
told, and the engine drops that venue's books and rebuilds them from the
new session's first snapshot (md publishes one per instrument a second,
after the instrument's `InstrumentSpec`). Finding a recording asks the
archive on a thread of its own: the engine's loop never waits.

The dummy exchange starts from the beginning of its engine's `orders`
recording, so after a restart it answers what was sent while it was down;
it ignores orders more than 10 s old, and the engine gives up on an order
unanswered for 30 s. An order answered just before a restart is answered
again, and the engine applies each fill once. Top of book (`tob`) stays a
plain best-effort subscription (`Subscriber`).

**Idle strategies are configuration.** The driver's three threads
(`AERON_{CONDUCTOR,SENDER,RECEIVER}_IDLE_STRATEGY` in `deploy/lab.yaml`) and
every loop the lab owns (`IDLE`: `spin`, `noop`, `yield`, `sleep`) say what
they do when idle. The lab runs the driver's sender and receiver on `yield`
and everything else on `sleep`, so four nodes share one laptop; in
production, spin the driver's sender and receiver and the engine on isolated
cores.

**The embedded (invoker) driver.** An application can run the driver's duty
cycle on its own thread instead. That saves the hand-off to the sender
thread (a cache-line transfer, on the order of 100 ns; not measured here),
but puts everything the conductor does on the
trading thread: mapping a new publication's or image's term buffers
(milliseconds), timeouts, NAKs and retransmits, and blocking name
resolution when a publisher moves. Nobody else can use that driver either,
so there is no node archive to spy-record the feeds. It suits one process
that owns the box, not this design.

**Production.** One Kubernetes cluster per region. Git holds
`streams.yaml` and `tables.yaml`, and GitOps (Argo CD or Flux) syncs them
into every cluster as ConfigMaps; names resolve across regions with
multi-cluster DNS (`*.clusterset.local`), and every region's ingesters write
to one ClickHouse. Cross-node tick-to-trade stages need PTP-synchronised
clocks: kind's nodes share one kernel clock, a real cluster does not.

## The engine

`engine-<region>` is one thread in one loop, the HFT model. From every feed
handler in its region it keeps each instrument's L2 book (Decimal9
mantissas, never rounded), and per asset:

- **the aggregated book**, sizes normalised to base quantity from each
  instrument's `InstrumentSpec`: linear `size × multiplier` (OKX's contracts
  are 0.01 BTC or 0.1 ETH), inverse `size × multiplier / price` (Deribit's
  perpetuals are sized in USD). USDT, USDC and USD are taken as one quote
  currency; venues' perpetuals trade at a basis of a basis point or two, so
  the aggregate is often crossed by that much;
- **time-decayed EMAs** of the aggregated mid over 5m, 30m, 1h, 4h, 12h and
  1d (`α = 1 − e^(−Δt/τ)`, so each moves on every change);
- **a strategy**: the mid crossing its 5m EMA, taken with the trend (5m
  against 30m) or when it reduces the position, at most one order per 30 s
  per asset and never past 0.01, as a marketable limit order at the
  aggregated best price.

Once a second it publishes each asset's `ema` and `agg_book` on its
`signals` feed, and orders on `orders`; `exch-sim` fills each at its price
and answers `New` then `Filled` on `exec`. The node's archive records all of
it, so each is a ClickHouse table with no more code.

**Tick-to-trade** is the checkpoint trace `tick_to_trade`, from the feed
handler's receive time through `feed`, `decode`, `book`, `signal`, `decide`
and `send`. Every tick counts in its stage histograms; one that led to an
order is kept (`Trace::keep`) under the order's id (`Trace::set_id`), and
`exch-sim`'s `order_ack` trace of the same order shares it, so Grafana's
trace view shows both applications in one waterfall. On the lab (loops
sleeping 1 ms when idle, four nodes on a laptop), medians: decode 3–7 µs,
book 13–45 µs, EMAs and decision about 1 µs, publishing the order 10–22 µs;
`feed` dominates at a few ms, most of it the sleeping loops.

## Memory

The whole lab runs in about 3.4 GiB, a third of it Kubernetes itself
(API server, etcd, and each node's CNI and proxy). Every container has a
limit; measured in use, 2026-09-27:

| Pod | In use | Limit | How |
|---|---|---|---|
| `clickhouse` | 550–700 MiB | 1 GiB | thread pools cut from over 700 threads to about 140 (each thread's allocator cache is memory ClickHouse does not count), a cap at 75% of the limit, small caches, system logs other than the query and part logs off (`deploy/clickhouse-lab.xml`) |
| `aeron`: driver | 51–57 MiB | 256 MiB | mostly its shared-memory term buffers: feeds use 1 MiB terms |
| `aeron`: archive | 62–72 MiB | 192 MiB | JVM: 64 MiB heap, 48 MiB direct, C1 only, 16 MiB code cache |
| `md-<exchange>` | 20–80 MiB | 256 MiB | `MALLOC_ARENA_MAX=2` |
| `grafana` | ~130 MiB | 192 MiB | `GOMEMLIMIT=96MiB` |
| `jupyter` | ~75 MiB | 512 MiB | pandas frames when the notebook runs |
| `ingester`, `engine`, `exch-sim` | 1–4 MiB | 256, 64, 64 MiB | |

Docker Desktop's own VM limit is what protects the Mac: give it what the
clusters need (8 GiB is plenty for this lab), not all of the machine's
memory, or macOS swaps.

## Record a table

**From an SBE message.** Add the message to `schema/market.xml`, list it in
`config/tables.yaml`, and record it where the data arrives:

```rust
let len = TradeEncoder::compute_length_with_header(symbol.len(), venue.len(), trade_id.len());
persist_client::record(TradeEncoder::TEMPLATE_ID, len, |buf| {
    Ok(TradeEncoder::wrap_and_apply_header(buf, 0)
        .fixed(&TradeFixedFields { ts_event, ts_init, price: d9(price)?, size: d9(size)?, aggressor })
        .symbol(symbol)?
        .venue(venue)?
        .trade_id(trade_id)?
        .encoded_length_with_header())
})?;
```

`persist_client::record` uses the handle installed once at start-up
(`Persist::connect(schema, settings)?.install()`), so any code can record
without being passed one. `connect` waits up to 10 seconds for a subscriber
to be recording the stream (`subscriber_timeout`); until one is, or with a
zero timeout, a record Aeron cannot take is dropped. With none installed (a
unit test, a tool) `record` does nothing and never calls the closure.
Holding the `Persist` and calling `persist.record` is the same, minus one load.

The table is the message in snake_case (`Trade` → `trade`), and every field
is a column. `record` encodes straight into the Aeron term buffer. It skips
`encode` entirely when the table is off. It never allocates or waits for
ClickHouse.

**From a `tracing` event.** For anything without a schema, such as signals,
diagnostics or model output, list a table name in `tables.yaml` and emit
events that name it:

```rust
tracing::info!(table = "spread", instrument = %q.instrument_id, bps = spread_bps);
```

The table's columns are the events' fields, typed by the first value seen:
integers are `Int64`/`UInt64`, floats `Float64`, bools `Bool`, and strings,
`%display` and `?debug` values `String`, all `Nullable`: a field an event
does not carry (an `Option` that is `None`, say) is NULL. Every row also gets
`ts DateTime64(9)`. Install the layer once, beside your own log layer:

```rust
tracing::subscriber::set_global_default(tracing_subscriber::registry().with(persist.layer()))?;
```

Give your log output its own per-layer filter (`fmt::layer().with_filter(...)`).
A global level filter would hide these events from persist too.

Data whose fields are known only at run time, such as JSON from a venue, goes
through `persist_client::record_row(table, fields)`, into the same kind of
table.

**From any Rust value.** `persist_client::record_value(table, &value)` records
a `Serialize` value, nested as deep as it goes, with no schema to write:

```rust
#[derive(Serialize)]
struct BookView<'a> { instrument: &'a str, spread: Spread, bids: &'a [Level], imbalance: Option<f64>, regime: Regime }
persist_client::record_value("book_view", &view);
```

| Rust | ClickHouse column |
|---|---|
| `bool`, integers, floats | `Nullable(Bool)`, `Nullable(Int64)` / `Nullable(UInt64)`, `Nullable(Float64)` |
| `str`, `String`, `char`, `i128`, `u128` | `Nullable(String)` |
| `Option<T>`: `None` | NULL |
| nested struct, tuple | its fields, dotted: `spread.bps`, `pair.0` |
| `Vec`, slice, set | a group: `bids.price Array(Nullable(Float64))` |
| a list in a list | `bids.orders Array(Array(Nullable(UInt64)))`, to any depth |
| map | `tags.key`, `tags.value` arrays |
| enum | the variant's name in `regime`, its fields in `regime.Wide.bps` |

A list inside a struct or enum field is named with underscores up to it
(`stats.bids` → `stats_bids`, `side.Busy.trades` → `side_Busy_trades`).
ClickHouse treats array columns sharing a first name segment as one Nested
structure, whose arrays must be equally long, and only one list's own fields
always are. `#[serde(flatten)]` fields sit beside the others, and an
internally tagged enum (`#[serde(tag = "type")]`) is its tag and its fields.
A fixed-size array serializes as a tuple (`x.0`, `x.1`, …); use a slice or
`Vec` for an `Array` column. Raw bytes are not recorded, and a value nested
deeper than 8 lists or 32 structs (a recursive type) is refused, with an
error naming the field.

The first value of a type makes its shape. Each later one is written in
one pass, compiled for the type, into a reused buffer, then copied into
Aeron. A value the shape does not cover (an `Option` now `Some`, another
enum variant) grows the shape, keeping its fields in declaration order, and
the table gains the columns.

**How event rows travel.** Like an SBE message, a row carries values only.
The first time a call site records, its layout (the table, and its fields'
names, kinds and nesting in order) is published once as a `Shape` message
(`persist-client/schema/events.xml`). Each `Row` then
holds the shape's id, the timestamp, a presence bit per field, and the
values, and no names. A shape's id is a hash of the shape, so every
application on the stream agrees on it without coordinating. Every 5 s the
next record publishes each shape again, on the same publication as the rows.
The ingester saves the shapes it has seen next to its checkpoint, so rows
whose `Shape` message was purged before a restart still decode. A row whose
shape has not arrived yet (an ingester that starts with none saved) waits
up to 30 s for it, and nothing is checkpointed past it meanwhile; one that
still has none is reported and dropped. An application that records nothing
does not publish those repeats until its next record.

**More SBE schemas.** The ingester loads every `.xml` schema in `schema/` at
start-up and tells messages apart by schema id and template id. To persist
another application's messages, put its schema there, list its tables in
`tables.yaml`, and run `just ingester`. It restarts and resumes from its
checkpoint. Keep one version of each schema: the newest decodes records made
with older versions.

**Who recorded it.** Every row of every table ends with `host`, `pod` and
`app` (`LowCardinality(String)`). They cost the application nothing per
record: each Aeron frame carries its application's source id in the frame
header's 64-bit reserved value, which the archive keeps through replay, and
a `Source` message names the id once (and every 5 s). The names come from
`Settings`: `from_env` reads `PERSIST_APP`, `NODE_NAME` (the node, from
Kubernetes' downward API; else the machine's name) and `HOSTNAME` (the pod).
A static table created before these columns existed logs the `ALTER` that
adds them, like any other missing column.

## Metrics

Counters, gauges and histograms with labels, made once and updated from the
hot path with plain loads and stores: no lock, no allocation, no system call.

```rust
let metrics = persist_client::metrics(); // or persist.metrics()
let sent = metrics.counter("orders_sent", &[("venue", "binance")]);
let depth = metrics.gauge("book_depth", &[("side", "bid")]);
let t2t = metrics.histogram("tick_to_trade_ns", &[]);
let clock = Clock::new();
loop {
    let now = clock.now();       // the loop's one clock read
    sent.inc();                  // a load and a store
    depth.set(12.0);             // a store
    t2t.record(850);             // a bucket index, three load/stores, two compares
    metrics.poll(now);           // one compare until the interval ends
}
```

- A **counter** or **histogram** handle has one writer at a time: it is
  `Send`, not `Sync`, so an update needs no locked instruction. Ask for the
  same series on another thread and you get another cell; `poll` adds them
  up. A **gauge** is shared: every handle of a series is one cell, and the
  last value set wins. `counter_fn` samples an atomic kept elsewhere at each
  poll (persist's own `persist_dropped{reason}` counters are these).
- A **histogram** keeps log-linear buckets, as HdrHistogram does: each power
  of two is split into 32, so every value is known to within 3.1%, from 0 to
  `u64::MAX`. Buckets add up exactly across intervals, threads and
  applications; percentiles never do. That is why histograms are not
  per-millisecond min/max/sum/count summaries: from those, a p99 cannot be
  recovered (1000 values of 1 µs and 10 of 500 µs in one millisecond give
  min 1 µs, max 500 µs, mean 6 µs, and no way to tell whether the tail was
  one value or ten).
- **`poll(now)`** publishes every interval (`PERSIST_METRICS_INTERVAL`,
  default `5s`), at whole multiples of it in UNIX time, so applications line
  up. Each series' name and labels go out once per interval as a
  `MetricDef`, keyed by a hash of them; the values carry only that key. A
  call publishes at most one message and the next call continues, so no
  call costs more than one publish, and polling allocates nothing once every
  series has been seen. Call it from the thread's loop with the time it
  already has; any thread may call it.

Two tables, created by the ingester:

| Table | A row |
|---|---|
| `metrics` | `ts`, `name`, `kind`, `series`, `labels Map(…)`, `value` (a counter's total, a gauge's value), `delta` (what the interval added to a counter) |
| `metrics_histogram` | `ts`, `name`, `labels`, `count`, `sum`, `min`, `max`, `p50` … `p9999`, and the non-empty buckets `buckets.le` / `buckets.count` |

The row's percentiles are its own interval's. Over any other span, merge
the buckets:

```sql
SELECT quantileExactWeighted(0.99)(le, c) / 1e3 AS p99_us
FROM market.metrics_histogram ARRAY JOIN buckets.le AS le, buckets.count AS c
WHERE name = 'record_ns' AND ts > now() - INTERVAL 1 HOUR
```

## Traces

**Checkpoint traces**, for hot paths: a trace names its stages once, and
each event stamps a timestamp per stage into a record on the stack.

```rust
let t2t = persist_client::tracer("tick_to_trade", &["wire", "decode", "decide", "send"], &["levels"]);
const ORDERS: u64 = TraceId::namespace("order"); // hashed at compile time

let mut t = t2t.start(Nanos::from_epoch(ts_event), TraceId::new(ORDERS, order_id));
t.mark(clock.now()); // wire: the venue's timestamp to ours
t.mark(clock.now()); // decode
t.mark(clock.now()); // decide
t.attr(0, levels);
t.mark(clock.now()); // send
t.finish();
```

`finish` always records each stage, and the whole, into the histogram
`trace_ns{trace, stage}`, so every event is counted. It publishes the trace
itself only when `otel_traces` is on for this app and the trace is one in
`sample`, or slower than `slower_than`:

```yaml
otel_traces:
  kind: static
  enabled: false                                  # off for every app
  apps: { binance: { until: 2026-09-27T18:00:00Z } }  # Binance, for an hour
  traces:
    tick_to_trade: { sample: 1000, slower_than: 50us }  # 1 in 1000, and every one over 50 µs
```

Changes apply within a second. The ingester writes each trace to
`otel_traces`, the OpenTelemetry ClickHouse exporter's table, as a span for
the whole and one per stage, which Grafana's trace view shows as they are
(*Traces* dashboard). A business id as the trace id (an order id in the
`"order"` namespace) puts every application's trace of that order into one
trace, with nothing passed between them; `tracer.next_id()` makes one when
there is none. A stage that ends before it began (a venue's clock ahead of
ours) is recorded as 0 in its histogram and counted in `trace_clamped`.

**`tracing` spans** (`#[instrument]`, `info_span!`) at INFO and above are
recorded too while `otel_traces` is on, for code off the hot path: each
costs what the `tracing` registry costs (see *Latency*). While it is off,
the registry never stores them. Their fields are the span's attributes and
their parent the span they were entered in.

## Clock

```rust
let clock = Clock::new();   // one per thread
let now = clock.now();      // read the clock, and cache it
let t = clock.cached();     // the last read: a plain load
let wall = now.epoch_ns();  // UNIX ns: the process's anchor plus the offset, no system call
let venue = Nanos::from_epoch(ts_event); // comparable with `now`
```

Times are `Nanos`, signed nanoseconds since one anchor per process (so a
venue's timestamp before it, or ahead of us, never wraps). Where the CPU's
time-stamp counter is invariant (x86-64 Linux), a read is `rdtsc` through
`minstant`; elsewhere `minstant` would read the wall clock, which can step
backwards, so the clock reads `std::time::Instant`. Read it once per
iteration of the loop and pass the time around, as Agrona's
`CachedNanoClock` does.

## Aeron's own statistics

Every 5 s the ingester samples the media driver's CnC file, as `AeronStat`,
`ErrorStat` and `LossStat` print it:

| Table | A row |
|---|---|
| `aeron_counters` | every counter: `value`, `delta` since the last sample, and its label taken apart: `type` (`pub-pos`, `sub-pos`, `rec-pos`, …), `session_id`, `stream_id`, `channel`, `recording_id`, and `client_name` |
| `aeron_errors` | each distinct error the driver logged, when first seen or seen again |
| `aeron_loss` | each stream's data loss, when it grows |

Counters join on those columns rather than on text. `client_name` is a
Java client's own name, or, for this lab's C clients, the `app` of the
`Source` whose Aeron client owns the counter. The *Aeron* dashboard shows
how far each subscriber and the archive are behind each publisher, and how
much room each publisher has before back pressure.

## `tables.yaml`

```yaml
tables:
  trade:         { kind: static }                  # an SBE message
  book_snapshot: { kind: dynamic, enabled: false } # an SBE message, off
  spread:        { kind: dynamic }                 # not in the schema: tracing events
```

- **`enabled`** (default `true`): record it now: `true`, `false`, or
  `{ until: 2026-09-27T18:00:00Z }`, on until that time (UTC) and off after
  it, with no further edit. The application re-reads the file every second,
  so switching a table needs no restart.
- **`apps`**: the same values per app, by name (`PERSIST_APP`; each
  feed handler is its exchange). An app not listed takes `enabled`.
- **`kind: dynamic`**: the table follows the data. A field added to the SBE
  message becomes `ALTER TABLE … ADD COLUMN` when its publisher restarts, and a
  new event field is added as soon as it arrives.
- **`kind: static`**: created if missing, then never altered. A column the
  table lacks, or has with another type, is not written. The ingester logs an
  ERROR with the exact `ALTER` that fixes it and keeps writing every other
  column. Run the SQL and it is picked up within 30 seconds.

A changed column type is never altered automatically, for either kind.
`kind` is the same for every app: they all write one ClickHouse table.

```yaml
tables:
  # Book changes from Binance only, and from Deribit for the next hour.
  book_deltas:
    kind: dynamic
    enabled: false
    apps: { binance: true, deribit: { until: 2026-09-27T18:00:00Z } }
  # Every app until 18:00, except OKX.
  spread: { kind: dynamic, enabled: { until: 2026-09-27T18:00:00Z }, apps: { okx: false } }
```

## Types

| SBE | ClickHouse |
|---|---|
| integers, `float`, `double` | `Int8`…`UInt64`, `Float32`, `Float64` |
| decimal composite: `mantissa` + constant `exponent` | `Decimal(18, S)`, exact |
| `semanticType="UTCTimestamp"` integer (`timeUnit`, default ns) | `DateTime64(9, 'UTC')` |
| timestamp composite: `time` + constant `unit` (FIX `TimeUnit` 0/3/6/9) | `DateTime64(0/3/6/9, 'UTC')` |
| enum | `LowCardinality(String)`, the value's name |
| `char` array | `String` |
| `presence="optional"` | `Nullable(T)` |
| group `bids { price size }` | `bids.price Array(…)`, `bids.size Array(…)` |
| var-data | `String` |

Decimals and timestamps travel as their integer, and ClickHouse stores that
integer, so nothing is rounded. Prices and sizes in `market.xml` are
`Decimal9` (mantissa × 10⁻⁹). md's `d9()` is ergo-sbe's generated
`rust_decimal` conversion (`with_domain_type` in `market/build.rs`): it
converts exactly, and returns an error rather than rounding.

Other composites, sets, non-`char` arrays, nested groups and big-endian
schemas are rejected when the schema loads.

**Changing the schema.** Add fields at the end of the message block, and new
groups or var-data with `sinceVersion`. The archive may still hold records
from before the change. The ingester decodes each record for its own version,
so fields it doesn't carry are written as their defaults.
Change a schema in `schema/` and run `just md` (it restarts the ingesters
first, then every publisher).

## Durability

- **The archive is the buffer.** If ClickHouse is down or slow, the ingester
  stops replaying and the archive holds the data on disk. Nothing is dropped.
- **Checkpoint and purge.** After each insert the ingester saves its position
  (`recording position`), then deletes the archive segments behind it. When
  its last publisher on the node exits, the recording stops. Once all of it is
  in ClickHouse, the recording is deleted.
- **At least once.** A crash between an insert and the checkpoint write
  replays those records, so they can appear twice.
- **Sessions.** The applications on a node share one IPC publication, so
  they make one recording; each feed is a recording of its own, and a new
  session (a restart, a move) a new one. The ingester replays them all at
  once, each with its own checkpoint.

## Latency

`just latency` times each operation on the application thread, 200k times a
second for 8 s, with an ingester replaying the archive beside it. The run
below: 2026-09-27, Apple M4, rustc 1.98.1, with the kind lab (ClickHouse,
the ingester, five feed handlers) running on the same machine, which widens the
tails. The timer's resolution is 42 ns, so the metric and clock arms time
100 operations per sample; their rows are divided back to one operation.
Nothing was dropped.

| Arm | p50 | p99 | p99.9 |
|---|---|---|---|
| empty loop (the floor) | 41 ns | 42 ns | 125 ns |
| `record()`, one SBE message | 83 ns | 292 ns | 1.2 µs |
| `persist_client::record()`, installed handle | 83 ns | 292 ns | 2.4 µs |
| `persist_client::record()`, none installed | 41 ns | 42 ns | 125 ns |
| `tracing` event, table on | 125 ns | 792 ns | 7.6 µs |
| `tracing` event, table off | 42 ns | 291 ns | 2.0 µs |
| `record_value`, a struct of the event's three fields | 166 ns | 458 ns | 5.1 µs |
| `record_value`, a nested struct and five levels in a `Vec` | 375 ns | 1.5 µs | 13 µs |
| `trace!` without a `table` field | 41 ns | 42 ns | 84 ns |
| counter `inc`, per operation | 1.3 ns | 2.1 ns | 4.6 ns |
| gauge `set`, per operation | 1.3 ns | 3.3 ns | 4.2 ns |
| histogram `record`, per operation | 1.7 ns | 5.0 ns | 7.9 ns |
| `Clock::cached`, per operation | 1.3 ns | 2.9 ns | 5.4 ns |
| `Clock::now`, per operation (`std::time::Instant`: no TSC here) | 21 ns | 90 ns | 1.1 µs |
| `SystemTime::now`, per operation | 15 ns | 24 ns | 403 ns |
| `Metrics::poll`, between intervals | 41 ns | 42 ns | 458 ns |
| `Metrics::poll`, every 1 ms interval publishing (10 counters, a histogram) | 42 ns | 250 ns | 3.7 µs |
| 4-stage checkpoint trace, `otel_traces` off | 42 ns | 84 ns | 625 ns |
| the same, on but not sampled | 41 ns | 84 ns | 500 ns |
| the same, every one published | 83 ns | 333 ns | 1.5 µs |
| `tracing` span, `otel_traces` off | 83 ns | 209 ns | 1.8 µs |
| `tracing` span, on | 250 ns | 667 ns | 5.9 µs |

A checkpoint trace's cost includes its five stage histograms, and a clock
read per mark is the caller's. Every record stamps its source id into the
Aeron frame header: one 8-byte store, next to what the claim already
touches. `Clock::now` on this machine is `std::time::Instant`
(`mach_absolute_time`); on x86-64 Linux it reads the TSC through `minstant`,
which was not measured here. Earlier runs on a quieter machine measured
`record()` at 83 / 167 / 291 ns and a `tracing` event at 84 / 250–333 ns /
1.3–3.1 µs.

## What each exchange records

Everything NautilusTrader offers for two instruments (BTC and ETH) per venue:

| Table | From | Binance (spot) | Bybit, OKX | Deribit | Hyperliquid |
|---|---|---|---|---|---|
| `trade`, `quote`, `book_snapshot`, `book_deltas`, `bar` | SBE | yes | yes | yes | yes |
| `mark_price`, `index_price`, `funding_rate` | SBE | | yes | yes | yes |
| `ticker`, `instrument`, `instrument_status`, `spread` | events | yes | yes | yes | yes |
| `deribit_volatility_index` (DVOL) | events | | | yes | |
| `hyperliquid_open_interest`, `hyperliquid_public_trade` (with buyer and seller addresses) | events | | | | yes |

`ticker` is one row per instrument a second, with the columns the venue has.
Binance spot has no mark or index price, so those columns are NULL in its
rows. Deribit adds `volatility_index` and Hyperliquid
`open_interest`.

Venue-specific data is recorded field for field as the venue sends it.
Decimals stay text, so they are exact; use `toDecimal64(x, 9)` in a query.

## Things to try

- **Deploy an exchange with different columns.** `just exchange deribit`,
  then in ClickHouse:
  `SELECT name, type FROM system.columns WHERE table = 'ticker'`. Within a
  few seconds of Deribit's first row, the ingester logs
  ``applied: ALTER TABLE `market`.`ticker` ADD COLUMN IF NOT EXISTS `volatility_index` Float64``,
  and `deribit_volatility_index` appears as a new table. `just exchange hyperliquid`
  does the same for `open_interest`. Nothing else is redeployed.

- **Turn recording off and on.** Set `book_snapshot: { kind: dynamic, enabled: false }`
  and save. Within a second every feed handler's ingester stops inserting it
  and Grafana's *Order book* panels stop moving; set it back to `true` and
  they resume. For one exchange only, give the table `apps: { <exchange>: false }`.
- **Add a column to a dynamic table.** Add
  `<field name="bidLevels" id="12" type="uint8"/>` to `BookSnapshot` after
  `sequence`. Set `bid_levels: bids.len() as u8` in `on_book`, then run
  `just md`. The ingester logs
  ``applied: ALTER TABLE `market`.`book_snapshot` ADD COLUMN IF NOT EXISTS `bid_levels` UInt8``.
- **Change a static table.** Add `<field name="isMaker" id="9" type="uint8"/>`
  to `Trade` after `aggressor`, set `is_maker: 0` in `on_trade`, then run
  `just md`. The ingester logs the `ALTER` that `trade` needs, and every
  other column keeps flowing.
- **Trace one exchange for ten minutes.** Give `otel_traces` in
  `tables.yaml` `enabled: false` and `apps: { okx: { until: <ten minutes from now, UTC> } }`,
  and save. Within a second only OKX publishes `book_update` traces (open
  *Traces*, pick one in `trace_id`); at that time it stops by itself, while
  the stage latency panels keep counting every update.
- **Record a new signal.** Add `tracing::info!(table = "my_signal", value = x)`
  anywhere in md, plus `my_signal: { kind: dynamic }` in
  `tables.yaml`.

## Checks

```sh
just test     # unit + integration tests (a throwaway ClickHouse on :18123 and an Aeron archive driver)
just lint     # clippy -D warnings + rustfmt
just latency  # the table above
just verify   # the running lab: /play, live data, the engines' orders and traces, every Grafana panel, the notebook, a live toggle, a feed handler moved between nodes
```

## Limits

- `Decimal9` holds ±9.2 billion with nine decimals. A value outside that is an
  error in `d9()`, and that record is not written.
- `record()` never waits. A record Aeron cannot take is dropped and counted in
  `persist.drops()` (`not_connected`, `back_pressure`, `too_large`, `other`),
  and logged once a second while the total grows. `dropped()` is that total.
  A term rotation is retried eight times and then counted in `other`, so the
  call cannot spin. That covers no archive recording yet, back pressure, or a
  record over 64 KiB. The 16 MiB terms leave 8 MiB of headroom for the archive.
- One table that never inserts (say, a static table ClickHouse refuses) holds
  the checkpoint back. The archive then grows until it is fixed. Nothing is
  lost, but it uses disk.
- The applications and the ingester reconnect by exiting and being restarted,
  so a restarted `aeron` pod costs every client a restart and the records
  published in between. A client notices within the 30 s driver timeout.
- An event table's column types come from the first shape that has the
  column. A later shape with another type for it (a field that is sometimes
  an integer, sometimes text) has its values converted when that loses
  nothing, and otherwise written as NULL and reported.
- A nested struct recorded through `tracing` is its `?debug` text; record
  it with `record_value` to get its fields as columns.
- `record_value` is slower than a `tracing` event of the same fields (125
  against 84 ns): serde walks the value once into a reused buffer and that
  buffer is copied into Aeron. Walking the value a second time, to write
  straight into the claim, measured slower. For the hottest data, an SBE
  message and `record()` stay the fastest.
- In `record_row`, a JSON null leaves the field out, so each pattern of
  nulls in the data is its own shape.
- `just verify` counts container restarts, so after `just stop` / `just start`
  its last check fails: the node restart restarts every container.
- Applications that publish on one stream must use the same channel.
  Aeron refuses a second IPC publication whose parameters (a `session-id`, say)
  differ from the one already open.
- Aeron's statistics are sampled from shared memory, not recorded through
  the archive: while the ingester is down, nothing is sampled.
- Metrics are published every interval from `poll`: those of an
  application that exits before its next interval are lost. A counter's
  `delta` is exact across restarts; its `value` restarts from 0.
- A histogram's `sum` and count can differ by the values recorded while a
  poll on another thread was reading the cell. Polling from the recording
  thread, as a busy-spinning application does, is exact.
- Every 5 s the next `record` (or event, or `poll`) also publishes the
  heartbeat: the `Source` message, every event shape and every trace
  definition, one after another. That one call costs a publish per
  message, which `poll`'s one-message bound does not cover. A cursor over
  the heartbeat, one message per call, would bound it too.
- A persistent subscription replays only what the archive still holds:
  the ingester purges a recording's segments once they are in ClickHouse,
  so an engine or exchange down for longer than that catches up from the
  oldest segment left.
- Checkpoint traces take at most 16 stages and 8 numeric attributes;
  `tracing` spans may carry text.
- In this lab, the kind node's clock can lag the venues' (Docker Desktop's
  VM drifts, by 100–400 ms here): `venue_to_local_ns` then reads 0 and
  `trace_clamped` counts the negative stages.
- Only venue data NautilusTrader subscribes to is recorded. Options (greeks,
  chains) need live option instruments picked by expiry, and are not
  recorded.
