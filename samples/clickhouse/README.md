# ClickHouse persistence lab

Live public market data from Binance and Bybit, recorded into ClickHouse
through SBE. The application is small on purpose: NautilusTrader supplies
the data, and [`recorder/src/main.rs`](recorder/src/main.rs) encodes each
callback as an SBE message. The interesting code is
[`persist/`](persist/), which turns the SBE schema into ClickHouse tables and
keeps them in step with the schema and with `config/tables.yaml`.

```text
Binance ─┐   NautilusTrader     SBE message      persist                 ClickHouse ──> Grafana
Bybit  ──┴─> LiveNode ──> recorder actor ──> record() ─> writer thread ──> tables    ──> notebook
                                               ▲ config/tables.yaml (live)            ──> /play UI
```

## Run it

Needs Docker, [kind](https://kind.sigs.k8s.io), kubectl, [just](https://just.systems) and jq.
No exchange API keys: only public streams are used.

```sh
cd samples/clickhouse
just up        # first run compiles the recorder in a container (~15-20 min)
```

| What | Where |
|---|---|
| ClickHouse query UI | <http://localhost:8123/play> (user `lab`, password `lab`) |
| Grafana | <http://localhost:3000>: *Market data* and *ClickHouse tables* dashboards |
| Notebook | <http://localhost:8888/lab/tree/verify.ipynb>, then Run All |
| Recorder log | `just logs` |
| Check everything | `just verify` |

`just stop` / `just start` pause the cluster and keep the data; `just destroy`
deletes it. The cluster mounts this directory, so `config/`, `grafana/` and
`notebooks/` in your checkout are what the pods see.

## Things to try

### Turn recording on and off

`book_snapshot` is off. In [`config/tables.yaml`](config/tables.yaml) set

```yaml
  book_snapshot: { kind: dynamic, enabled: true }
```

and save. Within a second the recorder logs `recording book_snapshot: on` and
the *Order book* panels in Grafana fill in. Set it back to `false` and the
rows stop. No restart, no `kubectl`: the file is re-read every second. An
invalid edit is logged and ignored, keeping the last good config. Every table
listed in `tables.yaml` exists from the start; `enabled` only decides whether
rows are written.

### Add a column (dynamic table)

Add a field to `BookSnapshot` in [`schema/market.xml`](schema/market.xml),
among the other fields and before the groups:

```xml
<field name="levels" id="12" type="uint8"/>
```

The recorder no longer compiles until it sets the new field, which is the
point. In `on_book` add `levels: bids.max(asks) as u8,` to
`BookSnapshotFixedFields`, then:

```sh
just recorder   # rebuild, reload, restart
```

The log shows

```text
applied: ALTER TABLE `market`.`book_snapshot` ADD COLUMN IF NOT EXISTS `levels` UInt8
```

and *ClickHouse tables ▸ Schema changes* lists it. Rows written before read the
column's default (`0`). Removing the field again leaves the column in place;
new rows get its default.

### Change a static table

`trade` and `quote` are `static`: persistence creates them once and never
alters them. Add `<field name="notional" id="9" type="double"/>` to `Trade`
after `aggressor`, set `notional: t.price.as_f64() * t.size.as_f64(),` in
`on_trade`, and `just recorder`. The recorder logs

```text
ERROR trade: static table is missing column notional; not writing it. Fix: ALTER TABLE `market`.`trade` ADD COLUMN IF NOT EXISTS `notional` Float64
```

and keeps writing every other column. Run that SQL in the /play UI and the
column starts filling within 30 seconds. A column whose type no longer
matches is handled the same way for both kinds (`MODIFY COLUMN` is never run
automatically, because it can rewrite data).

## How persist works

The SBE schema is the table definition. Each message is a table, named in
snake_case, and each field is a column:

| SBE | ClickHouse |
|---|---|
| integers, `float`, `double` | `Int8`…`UInt64`, `Float32`, `Float64` |
| `semanticType="UTCTimestamp"` (ns) | `DateTime64(9, 'UTC')` |
| enum | `LowCardinality(String)`, the value's name |
| `char` array | `String` |
| `presence="optional"` | `Nullable(T)` |
| group `bids { price size }` | `bids.price Array(Float64)`, `bids.size Array(Float64)` |
| var-data | `String` |

Tables are `MergeTree`, partitioned by day of the first timestamp and ordered
by the var-data columns (`symbol`, `venue`) then that timestamp. Every table
also gets `inserted_at DEFAULT now64(3)`. Composites, sets, non-`char` arrays
and nested groups are rejected when the schema loads.

On the hot path, `persist.enabled(template)` is one atomic load, so a
disabled table costs nothing and is never encoded. `persist.record(bytes)`
copies the encoded message into a shared buffer. Every second the writer thread:

1. re-reads `tables.yaml`;
2. creates or compares the tables listed there;
3. decodes each message with the schema IR into RowBinary;
4. sends one `INSERT … FORMAT RowBinary` per table (ClickHouse wants about
   one insert per table per second; more often just makes parts to merge).

If ClickHouse is unreachable, records stay queued (256 MiB), then are dropped
and counted. There is no disk spool: see [PLAN.md](PLAN.md) for what was left
out and why.

## Layout

```text
schema/market.xml       the SBE messages = the tables
config/tables.yaml      what is recorded, static or dynamic (live)
recorder/               NautilusTrader node + one actor (the sample app)
persist/                the persistence library and its ClickHouse tests
grafana/                datasource, dashboards (edit the JSON, reloads in 10 s)
notebooks/verify.ipynb  asserts and plots what was recorded
deploy/                 kind cluster, manifests, image builds
scripts/verify.sh       end-to-end check of the running lab
```

## Tests

```sh
just test     # unit + integration tests; starts a throwaway ClickHouse on :18123
just lint     # clippy -D warnings + rustfmt
just verify   # against the running lab: UI, live data, every Grafana panel,
              # the notebook, and a live on/off toggle of book_snapshot
```

The integration tests encode messages with the generated SBE encoders,
record them, and read them back from a real ClickHouse. They cover every
field shape, dynamic column adds, static tables left untouched, type
conflicts, live config toggles and an unreachable server. They fail rather
than skip when ClickHouse is missing.
