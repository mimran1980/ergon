# ClickHouse recording lab

Record anything a low-latency application sees into ClickHouse, and switch
recording on and off while it runs. The application only publishes to Aeron;
a separate ingester does the ClickHouse work.

```text
recorder app ──try_claim──▶ Aeron IPC ──▶ Aeron Archive (disk) ──replay──▶ ingester ──▶ ClickHouse
(persist-client)                          purged behind the ingester      (persist-server)
```

| Crate | What it is |
|---|---|
| `persist-client` | The small library the application links: `record()` for SBE messages, a `tracing` layer for everything else. |
| `persist-server` | The ingester, as a library and the `ingester` binary: replays the archive into ClickHouse tables, checkpoints, purges. |
| `recorder` | The sample application: public market data from one exchange (Binance, Bybit, OKX, Deribit or Hyperliquid) via NautilusTrader, no API keys. |

## Run it

```sh
cd samples/clickhouse
just up                # kind cluster: ClickHouse, Grafana, JupyterLab, Aeron, the ingester, Binance and Bybit
just exchange deribit  # add an exchange: binance bybit okx deribit hyperliquid
just unexchange okx    # remove one; its data stays
just recorder          # rebuild + redeploy the recorders and ingester after a code or schema change
just aeron             # rebuild + redeploy Aeron, then its clients
just verify            # end-to-end check of the running lab
just logs              # recorder, ingester and Aeron logs
```

You need Docker, kind, kubectl, just and jq.

Every port listens on all interfaces, so the lab is reachable from other
machines on your network as well as from `localhost` (`just urls` prints the
addresses). That includes Grafana (anonymous admin) and JupyterLab (no token,
this checkout mounted read-write): run it only on a network you trust.

| What | Where |
|---|---|
| ClickHouse query UI | <http://localhost:8123/play> (user `lab`, password `lab`) |
| Grafana | <http://localhost:3000>: *Market data* and *ClickHouse tables* (every table, plus ad-hoc SQL) |
| Notebook | <http://localhost:8888/lab/tree/verify.ipynb>, then Run All |
| What is recorded | `config/tables.yaml`: edits apply within a second |

| Pod | Kind | What it runs |
|---|---|---|
| `aeron` | DaemonSet, one per node | The Aeron media driver and archive. Its directory is on the node's tmpfs, so every pod on the node shares it; the recordings are on the node's disk. |
| `ingester` | DaemonSet, beside each Aeron | Archive -> ClickHouse. Its checkpoint is on the node's disk. |
| `recorder-<exchange>` | Deployment per exchange | NautilusTrader + persist-client for one exchange (`deploy/recorder.yaml`). |

The recorders on a node publish on one Aeron stream. Aeron shares one IPC
publication between them, so they make one recording, and adding or removing
an exchange touches nothing else.

If a pause (a laptop sleeping, say) outlasts Aeron's 30 s timeouts, the
archive dies. The `aeron` pod then exits and is restarted, and the ingester
and recorders exit and reconnect to the new driver. The ingester also exits
on any failed archive request, and resumes from its checkpoint.

`just stop` / `just start` pause the cluster and keep the data;
`just destroy` deletes it. `just test` leaves a ClickHouse and an Aeron
driver running for the next run; `just test-stop` frees their memory.

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
without being passed one. With none installed (a unit test, a tool) it does
nothing and never calls the closure. Holding the `Persist` and calling
`persist.record` is the same, minus one load.

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
(`persist-client/schema/events.xml`), and again every 5 s. Each `Row` then
holds the shape's id, the timestamp, a presence bit per field, and the
values, and no names. A shape's id is a hash of the shape, so every
application on the stream agrees on it without coordinating. The ingester
saves the shapes it has seen next to its checkpoint, so rows whose `Shape`
message was purged before a restart still decode. A row whose shape has not
arrived yet (an ingester that starts with none saved) waits up to 30 s for
it, and nothing is checkpointed past it meanwhile; one that still has none
is reported and dropped.

**More SBE schemas.** The ingester loads every `.xml` schema in `schema/` at
start-up and tells messages apart by schema id and template id. To persist
another application's messages, put its schema there, list its tables in
`tables.yaml`, and run `just ingester`. It restarts and resumes from its
checkpoint. Keep one version of each schema: the newest decodes records made
with older versions.

## `tables.yaml`

```yaml
tables:
  trade:         { kind: static }                  # an SBE message
  book_snapshot: { kind: dynamic, enabled: false } # an SBE message, off
  spread:        { kind: dynamic }                 # not in the schema: tracing events
```

- **`enabled`** (default `true`): record it now. The application re-reads the
  file every second, so switching a table on or off needs no restart.
- **`kind: dynamic`**: the table follows the data. A field added to the SBE
  message becomes `ALTER TABLE … ADD COLUMN` when the recorder restarts, and a
  new event field is added as soon as it arrives.
- **`kind: static`**: created if missing, then never altered. A column the
  table lacks, or has with another type, is not written. The ingester logs an
  ERROR with the exact `ALTER` that fixes it and keeps writing every other
  column. Run the SQL and it is picked up within 30 seconds.

A changed column type is never altered automatically, for either kind.

**One exchange only.** `config/exchanges/<exchange>.yaml` overrides `enabled`
for that exchange's recorder, and applies within a second, like `tables.yaml`.
It may name only tables that `tables.yaml` lists. `kind` is not overridable:
every exchange writes the same ClickHouse table. Delete the file and
`tables.yaml` decides again.

```yaml
# config/exchanges/binance.yaml: Binance stops recording book changes
tables:
  book_deltas: { enabled: false }
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
`Decimal9` (mantissa × 10⁻⁹). The recorder's `d9()` is ergo-sbe's generated
`rust_decimal` conversion (`with_domain_type` in `recorder/build.rs`): it
converts exactly, and returns an error rather than rounding.

Other composites, sets, non-`char` arrays, nested groups and big-endian
schemas are rejected when the schema loads.

**Changing the schema.** Add fields at the end of the message block, and new
groups or var-data with `sinceVersion`. The archive may still hold records
from before the change. The ingester decodes each record for its own version,
so fields it doesn't carry are written as their defaults.
Change a schema in `schema/` and run `just ingester` (and `just recorder`
for the recorder's own `market.xml`).

## Durability

- **The archive is the buffer.** If ClickHouse is down or slow, the ingester
  stops replaying and the archive holds the data on disk. Nothing is dropped.
- **Checkpoint and purge.** After each insert the ingester saves its position
  (`recording position`), then deletes the archive segments behind it. When
  the last recorder on the node exits, the recording stops. Once all of it is
  in ClickHouse, the recording is deleted.
- **At least once.** A crash between an insert and the checkpoint write
  replays those records, so they can appear twice.
- **Sessions.** The recorders on a node share one publication, so they make
  one recording. It stops only when none is left; the next recorder starts a
  new one. The ingester takes recordings oldest first.

## Latency

`just latency` times recording on the application thread at 200k records/s,
with the ingester running beside it (Apple M-series; the timer's resolution is
42 ns). The results below had no dropped records:

| Arm | p50 | p99 | p99.9 |
|---|---|---|---|
| empty loop (the floor) | 0 ns | 42 ns | 42 ns |
| `record()`, one SBE message | 83 ns | 167 ns | 291 ns |
| `persist_client::record()`, installed handle | 42 ns | 167 ns | 334–667 ns |
| `persist_client::record()`, none installed | 0 ns | 42 ns | 42 ns |
| `tracing` event, table on | 84 ns | 250–333 ns | 1.3–3.1 µs |
| `tracing` event, table off | 41 ns | 83 ns | 375–542 ns |
| `record_value`, a struct of the event's three fields | 125 ns | 375 ns | 1.8–2.8 µs |
| `record_value`, a nested struct and five levels in a `Vec` | 333 ns | 875 ns | 5.3–7.8 µs |
| `trace!` without a `table` field | 0 ns | 42 ns | 42–83 ns |

The installed-handle and `tracing` rows are from later runs (2026-09-26). In
the first, `record()` on a held handle measured 42 / 167 / 417 ns: the one
extra load is below the timer's resolution. The `tracing` rows are for the
shape-and-row format above, over three runs. The format before it, which
wrote every field's name into every row, measured 125 / 375–417 ns / 2.9–3.3 µs
with the table on and 42 / 125 ns / 0.9–1.1 µs with it off. The last row shows
that the layer's own filter leaves the rest of the application's `tracing`
calls disabled.

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
  and save. Within a second every recorder logs `recording book_snapshot: off`
  and Grafana's *Order book* panels stop moving; set it back to `true` and
  they resume. For one exchange only, use `config/exchanges/<exchange>.yaml`.
- **Add a column to a dynamic table.** Add
  `<field name="bidLevels" id="12" type="uint8"/>` to `BookSnapshot` after
  `sequence`. Set `bid_levels: bids.len() as u8` in `on_book`, then run
  `just recorder`. The ingester logs
  ``applied: ALTER TABLE `market`.`book_snapshot` ADD COLUMN IF NOT EXISTS `bid_levels` UInt8``.
- **Change a static table.** Add `<field name="isMaker" id="9" type="uint8"/>`
  to `Trade` after `aggressor`, set `is_maker: 0` in `on_trade`, then run
  `just recorder`. The ingester logs the `ALTER` that `trade` needs, and every
  other column keeps flowing.
- **Record a new signal.** Add `tracing::info!(table = "my_signal", value = x)`
  anywhere in the recorder, plus `my_signal: { kind: dynamic }` in
  `tables.yaml`.

## Checks

```sh
just test     # unit + integration tests (a throwaway ClickHouse on :18123 and an Aeron archive driver)
just lint     # clippy -D warnings + rustfmt
just latency  # the table above
just verify   # the running lab: /play, live data, every Grafana panel, the notebook, a live toggle
```

## Limits

- `Decimal9` holds ±9.2 billion with nine decimals. A value outside that is an
  error in `d9()`, and that record is not written.
- `record()` never waits. A record Aeron cannot take is dropped and counted in
  `persist.dropped()`, and logged every second while the count grows. That
  covers no archive recording yet, back pressure, or a record over 64 KiB. The
  16 MiB terms leave 8 MiB of headroom for the archive.
- One table that never inserts (say, a static table ClickHouse refuses) holds
  the checkpoint back. The archive then grows until it is fixed. Nothing is
  lost, but it uses disk.
- The recorders and the ingester reconnect by exiting and being restarted,
  so a restarted `aeron` pod costs every client a restart and the records
  published in between. A client notices within the 30 s driver timeout.
- An event table's column types come from the first shape that has the
  column. A later shape with another type for it (a field that is sometimes
  an integer, sometimes text) has its values converted when that loses
  nothing, and otherwise written as NULL and reported.
- A nested struct recorded through `tracing` is its `?debug` text; record
  it with `record_value` to get its fields as columns.
- `record_value` is slower than a `tracing` event of the same fields (125
  against 84 ns): serde walks the value generically. For the hottest data,
  an SBE message and `record()` stay the fastest.
- In `record_row`, a JSON null leaves the field out, so each pattern of
  nulls in the data is its own shape.
- `just verify` counts container restarts, so after `just stop` / `just start`
  its last check fails: the node restart restarts every container.
- Applications that publish on one stream must use the same channel.
  Aeron refuses a second IPC publication whose parameters (a `session-id`, say)
  differ from the one already open.
- Only venue data NautilusTrader subscribes to is recorded. Options (greeks,
  chains) need live option instruments picked by expiry, and are not
  recorded.
