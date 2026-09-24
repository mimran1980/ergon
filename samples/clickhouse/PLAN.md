# ClickHouse persistence lab: plan and status

## Goal

Test ClickHouse persistence of SBE-encoded market data with as little
machinery as possible:

- a **tiny sample app**: NautilusTrader provides public Binance/Bybit data,
  and one actor encodes it as SBE;
- **persistence is the real code**: SBE schema -> ClickHouse tables, with
  dynamic tables that follow the schema and static tables that never change;
- a **file on disk** decides what is recorded, and edits apply live in kind;
- a ClickHouse **query UI**, **Grafana dashboards** over every table, and a
  **notebook** that verifies the data.

## Decisions

| # | Decision | Why |
|---|---|---|
| D1 | Rewrite rather than repair the 2026-09-23 sample (24k lines: Python recorder + PyO3 bridge, SQLite catalogs, Aeron Archive + Java driver, archive-agent, ingester, Prometheus, SeaweedFS, 14 scripts). | On 2026-09-24 its recorder pods had crash-looped 93 times (`aeron_archive_async_connect … connect timeout`). The goal asks for something small and easy to use; almost all of that code was transport and bookkeeping, not persistence. |
| D2 | The recorder is Rust NautilusTrader 0.64 (`LiveNode` + one `DataActor`). Binance spot uses public JSON streams (`BinanceSpotMarketDataMode::Json`) and Bybit linear public streams. No API keys. | That was the stated point of using Nautilus: the app only hands over data. Verified keyless on 2026-09-24. |
| D3 | Persistence is **in-process**: `record()` copies SBE bytes into a buffer, and a writer thread inserts `RowBinary` every second. No Aeron, no Archive, no separate ingester. | This is the smallest thing that exercises SBE-to-ClickHouse. **Cost:** no durable spool. If ClickHouse is down, records are held in memory up to 256 MiB, then dropped and counted. If replay or durability is ever needed, put an Aeron Archive between `record()` and the writer; the writer's input is already whole SBE messages. |
| D4 | Tables come from the **SBE schema IR at runtime** (`ergo_sbe::parse` + `resolve_schema`), not from codegen hooks or derives. | One place decides columns. A field added to `market.xml` is a column, with no second definition to keep in sync. |
| D5 | Timestamps with `semanticType="UTCTimestamp"` become `DateTime64(9, 'UTC')`. | Grafana's `$__timeFilter` works directly. The old UInt64 nanosecond columns needed conversion macros that silently matched nothing when wrong. |
| D6 | `tables.yaml` holds `kind: static\|dynamic` and `enabled`. Every listed table exists; `enabled` only gates writes. Static tables are created if missing and never altered: mismatches log an ERROR with the exact `ALTER` and skip only those columns. Type changes are never applied automatically for either kind. | This is the requested static/dynamic behaviour. Creating disabled tables up front keeps dashboards and notebooks from failing on "unknown table". |
| D7 | kind mounts the sample directory at `/lab` (`extraMounts`), and pods use hostPath for `config/`, `grafana/` and `notebooks/`. Ports are bound to 127.0.0.1. | "A simple filesystem for k8s": edit the file locally and the pod sees it. No ConfigMap watcher, no Flux. |
| D8 | Flush interval 1 s. | Measured 2026-09-24. At 250 ms ClickHouse created 465 parts and ran 369 merges a minute, and the node sat at about 100% CPU. At 1 s it was 159 parts and 103 merges a minute at 48% CPU. Ingest latency is p50 ≈ 0.6 s and p99 ≈ 1.0 s. |
| D9 | Floats (`double`) for prices and sizes. | Simple and Grafana-friendly. Exact decimals would need an SBE decimal composite → `Decimal64(n)` mapping; persist rejects composites today, so that is a deliberate, visible gap. |

## Status (verified 2026-09-24, kind cluster `clickhouse-lab`, macOS arm64, Rust 1.98.1)

- [x] **Recorder**: trades, quotes, 1 s L2 book snapshots (top 10 levels),
  Bybit mark prices and funding rates, for BTC/ETH on both venues.
- [x] **persist**: schema → tables, RowBinary decode, static/dynamic sync,
  live config reload, bounded buffer, error rate-limiting.
- [x] **Tests** (`just test`: 2 unit tests and 10 integration tests against a
  real ClickHouse 25.8): every supported field shape round-trips; a dynamic
  table gains new schema columns; removed fields (a scalar and a group field
  beside its siblings) keep their columns and read defaults, for both kinds;
  a static table is never altered, keeps recording, and picks up a manually
  run `ALTER`; type conflicts are reported and not altered, for both
  kinds; `enabled` follows the file, and an invalid edit keeps the last good
  config; unreachable ClickHouse keeps records until the buffer is full;
  unsupported shapes are rejected; the market schema's DDL.
- [x] **Live** (`just verify`, all checks passed):
  - `/play` UI served;
  - trades from BINANCE and BYBIT arriving;
  - all 14 Grafana panels query without error and return rows (checked
    through `/api/ds/query`, with a negative control);
  - notebook runs clean in the Jupyter pod;
  - `book_snapshot` switched on and off by editing `config/tables.yaml`, with
    no recorder restart.
- [x] **Schema evolution, live**:
  - Adding `BookSnapshot.levels` produced
    ``applied: ALTER TABLE `market`.`book_snapshot` ADD COLUMN IF NOT EXISTS `levels` UInt8``.
    New rows had `levels = 10`, old rows `0`.
  - Adding `Trade.notional` to the static `trade` table produced
    ``ERROR trade: static table is missing column notional; not writing it. Fix: ALTER TABLE `market`.`trade` ADD COLUMN IF NOT EXISTS `notional` Float64``,
    and trades kept arriving (2,059 in 10 s).
  - Both edits were then reverted. `book_snapshot.levels` stays in the
    table: removed fields keep their column.
- [x] **`just up`** run as one command on the existing cluster (1m48s; the
  first build of the recorder took 16m41s). `just recorder` rebuilds and
  redeploys in about 90 s. On SIGTERM the node stops cleanly, and the writer
  flushes what is queued before exit.
- [x] **Recovery**: the recorder started before ClickHouse was ready,
  queued about 95 s of data, then created the tables and inserted the backlog.
- [x] **Grafana, visual check**: both dashboards render with live data
  (screenshots taken in a headless browser).
- [x] `just lint` (clippy `-D warnings`, rustfmt) and the repository test
  policy (`scripts/check-test-policy.sh`) pass.

## Re-check (2026-09-24, later the same day)

The status above was re-verified rather than trusted:

- **Bug found and fixed**: while a table was in its 5 s sync retry
  back-off, `sync_tables` reported ready and `drain` dropped that table's
  queued records, breaking the D3 promise that records stay queued. The
  readiness check now runs for every enabled table on every tick. Test:
  `table_that_cannot_be_created_keeps_its_records_queued` (a user allowed to
  create the database but not the table) failed before the fix, passes after.
- **`just verify`** now runs the *Columns of $table* and *Latest rows in
  $table* panels against every table, not only `trade` (22 queries).
- **Schema evolution, live, with fresh names**: adding `BookSnapshot.spread`
  applied `ALTER … ADD COLUMN spread Float64`, and new values equal
  `asks[1] - bids[1]`. Adding `Trade.notional` logged the static-table ERROR
  with its `ALTER`, and trades kept flowing. The manual `ALTER` was picked up
  by the next 30 s recheck, with a clean cutover and exact `price × size`
  values. That moment now also logs `trade: fixed, writing every column`
  (added afterwards, not yet seen live). Reverting both
  fields restarted cleanly, and the columns stay with default values.
- This sample's `Cargo.lock` is no longer ignored
  (`!samples/clickhouse/Cargo.lock` in the root `.gitignore`). Committing it
  pins the ~380 Nautilus dependencies.
- `just test` (2 unit + 10 integration), `just lint`, `just verify` and
  `scripts/check-test-policy.sh` all pass.

## Open / possible next steps

- **Durability** (D3): no spool if ClickHouse is down longer than the
  buffer lasts. The upgrade path is an Aeron Archive in front of the writer.
- **Exact decimals** (D9): support an SBE decimal composite as `Decimal64`.
- **Grafana writes as `lab`**, the same user the recorder uses. That is
  acceptable only because every port is bound to 127.0.0.1. Add a read-only
  ClickHouse user if the lab is ever exposed.
- **First load of a Grafana dashboard** takes several seconds while the
  ClickHouse plugin bundle loads; panel queries themselves take about 0.7 s.

## Where the previous implementation went

The 2026-09-23 sample (Aeron Archive pipeline, Python recorder, the previous
1,370-line PLAN.md with its R1–R8 repair list) is in git history (commit
`e62ef6c8`). The working copy, including its uncommitted edits, was moved
aside to `../../../ergon-clickhouse-pre-simplify/`, next to this repository.
Its kind cluster `clickhouse` is stopped, not deleted
(`docker start clickhouse-control-plane clickhouse-worker clickhouse-worker2`).
