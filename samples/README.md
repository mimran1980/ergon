# Samples

Standalone crates that exercise repository APIs. They are **excluded from the
workspace**, set `publish = false`, and are **not** production reference
implementations — they move with experimental APIs on purpose.

## Start here (product teaching path)

| Step | Sample | Why |
|------|--------|-----|
| **1** | [`sbe-feature-tour/`](sbe-feature-tour/) | **Golden path.** Full feature map: stages, EncodedLength, checked constructors + verify, Display, DTO with `DomainVarData::Strings`, all three conversion styles |
| **2a** | [`l3-book/`](l3-book/) | Nested/ragged books; **`with_domain_type` only**; **build-dep only** (plain `include!`) |
| **2b** | [`exchange-example/`](exchange-example/) | Multi-schema; **`with_conversion` only**; IPC + app `TryFromSbe` |
| **3** | [`sbe-codegen-examples/`](sbe-codegen-examples/) | Generator **as a library** (no `build.rs`) |
| Later | [`cluster-tutorial/`](cluster-tutorial/) | Connect, offer, poll, keep-alive, close |
| Later | [`cluster-ha-orderbook/`](cluster-ha-orderbook/) | Claim-based Cluster publishing + HA-shaped book |
| Later | [`cluster-rfq/`](cluster-rfq/) | RFQ / auction codecs over Cluster |
| Lab | [`clickhouse/`](clickhouse/) | Record live Binance/Bybit data (NautilusTrader) into ClickHouse with SBE-defined tables; Grafana, notebook, live config on kind |

```sh
# 1 — always start here
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml

# 2 — pick the conversion style you want in product code
cargo run  --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/exchange-example/Cargo.toml
```

**Rule of thumb:** one conversion style per schema type
(`with_domain_type` *or* `with_conversion`, not both for the same selector).
See the main [ergo-sbe README](https://github.com/mimran1980/ergon/blob/main/sbe/README.md)
configuration section.

## `ergo-sbe`: build dependency vs application dependency

Generated codecs ship their own embedded `sbe_rt` module. Linking the **app**
does not require `ergo-sbe` unless you use its macros or call the generator
library at runtime.

| Pattern | `build-dependencies` | `dependencies` | Typical use |
|---------|----------------------|----------------|-------------|
| **Build only** (**product / samples default**) | `ergo-sbe` | — | `generate_to_dir` → `src/generated/` (gitignored) + `#[path = "generated/….rs"]` |
| **OUT_DIR only** | `ergo-sbe` | — | `generate_to_out_dir` + `include!(concat!(env!("OUT_DIR"), …))` — fine for apps; **poor IDE go-to-def** |
| **Build + runtime** | `ergo-sbe` | `ergo-sbe` | Macros such as `sbe_mod!` plus build-time generation |
| **Runtime only** | — | `ergo-sbe` | Call `parse` / `Generator` as a library (no `build.rs`) |

| Sample | Pattern | Purpose | External requirements |
|---|---|---|---|
| [`sbe-feature-tour/`](sbe-feature-tour/) | **Build only** | **Teaching / feature map** — EncodedLength, stages, DTO, AnyMessage, **all three** conversion styles | None |
| [`l3-book/`](l3-book/) | **Build only** | Nested/ragged L3 books; **`with_domain_type` only** | None for local tests |
| [`exchange-example/`](exchange-example/) | **Build only** | Multi-schema + **`with_conversion` only** + Aeron IPC | Network only for live exchange paths |
| [`cluster-rfq/`](cluster-rfq/) | **Build only** | RFQ / auction protocol codecs + cluster examples | Java harness for live examples |
| [`cluster-ha-orderbook/`](cluster-ha-orderbook/) | **Build only** (may still dep cluster) | Claim-based Cluster publishing + HA-shaped book | Java harness only for leader-kill coverage |
| [`sbe-codegen-examples/`](sbe-codegen-examples/) | **Runtime only** | Generator API as a library (no `build.rs`) | None |
| [`cluster-tutorial/`](cluster-tutorial/) | **Neither** (uses `ergo-aeron-cluster`) | Connect, offer, poll, keep-alive, close walkthrough | Java 17+ and built Aeron artifacts |

### Seeing generated code (without committing it)

`include!(concat!(env!("OUT_DIR"), …))` and `sbe_mod!` put files under a
hashed path like `target/debug/build/<crate>-<hash>/out/….rs` — hard to find
and rust-analyzer usually **cannot** jump into them.

Samples instead write to a **stable, local path**:

```text
samples/<name>/src/generated/*.rs   # created on cargo build, gitignored
```

1. `cargo build --manifest-path samples/sbe-feature-tour/Cargo.toml`
2. Open `samples/sbe-feature-tour/src/generated/feature_tour.rs`
3. From app code, **Go to definition** on `CarEncoder` / etc. should land there

Root `.gitignore` has `**/src/generated/`. Do **not** commit those trees
(Binance alone is multi‑MB). Rebuild after a clean clone.

```rust
// build.rs
let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
ergo_sbe::generate_to_dir("schemas/messages.xml", config, &out)?;

// src/lib.rs — real path → IDE go-to-definition works
#[allow(dead_code, unused_imports, non_camel_case_types, non_snake_case, clippy::all, warnings)]
#[path = "generated/messages.rs"]
mod messages;
```

## Buffer sizing (samples & tests)

- **Const-sized messages:** stack `[0u8; MsgEncoder::compute_length()]`
- **Dynamic / ragged:** size with `*EncodedLength` / `compute_length_with_header(…)`,
  then encode into a claim/slot of that exact length — avoid oversize
  `vec![0u8; 4096]` “guess” buffers

See the main README [buffer sizing](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#buffer-sizing)
section and feature-tour `demo_car_size_and_encode`.

## Domain DTOs & var-data

```rust
.with_domain_objects(DomainVarData::Strings) // String; bad UTF-8 → InvalidUtf8 error
.with_domain_objects(DomainVarData::Bytes)   // Vec<u8>; byte-exact
```

`Strings` is strict UTF-8 (no empty-string fallback). Feature-tour uses
`Strings`; l3-book uses `Bytes` where tails are byte-oriented.

## Check each sample

```sh
cargo check --manifest-path samples/sbe-feature-tour/Cargo.toml --all-targets
cargo check --manifest-path samples/exchange-example/Cargo.toml --all-targets
cargo check --manifest-path samples/l3-book/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-ha-orderbook/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-rfq/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-tutorial/Cargo.toml --all-targets
```

Useful service-free tests:

```sh
cargo test --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo test --manifest-path samples/exchange-example/Cargo.toml
cargo test --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/cluster-ha-orderbook/Cargo.toml \
  --lib --test ha_offline_pipeline
```

Java-backed samples require:

```sh
just build-aeron-jars
cargo run --manifest-path samples/cluster-tutorial/Cargo.toml
```

## Conversion: which sample uses what

| Sample | Config | Decode / encode surface |
|--------|--------|-------------------------|
| [`l3-book/`](l3-book/) | **`with_domain_type` only** | `dec.try_price()?` → `Decimal`; `enc.try_price(d)?` |
| [`exchange-example/`](exchange-example/) | **`with_conversion` only** | `dec.price_as::<T>()?`; `enc.price_from(&t)?` (+ app `TryFromSbe`) |
| [`sbe-feature-tour/`](sbe-feature-tour/) | **All three** (different selectors) | bool/timestamp concrete (`Generated`); Decimal generic (`demo_conversion_only`); ManualDecimal concrete + app impl (`demo_domain_type_manual_impl`) |

Rule: **one style per selector**. `with_domain_type` already enables conversion;
do not stack `with_conversion` on the same selector.

```rust
// A — pluggable
.with_conversion(ConversionSelector::named_type("Decimal"))
// B — concrete (implies conversion)
.with_domain_type(ConversionSelector::named_type("Decimal"), "rust_decimal::Decimal")
```

## ClickHouse lab (`clickhouse/`)

Live public Binance and Bybit data, recorded into ClickHouse through SBE.
- **The app is small on purpose:** NautilusTrader supplies the data, and one
  actor (`clickhouse/recorder/src/main.rs`) encodes each callback as an SBE
  message.
- **The interesting code is `clickhouse/persist/`:** it turns the SBE schema
  into ClickHouse tables and keeps them in step with the schema and with
  `config/tables.yaml`.

```sh
cd samples/clickhouse
just up        # kind cluster + ClickHouse + Grafana + JupyterLab + recorder
```

You need Docker, kind, kubectl, just and jq. There are no exchange API keys:
only public streams are used.

| What | Where |
|---|---|
| ClickHouse query UI | <http://localhost:8123/play> (user `lab`, password `lab`) |
| Grafana | <http://localhost:3000>: *Market data* and *ClickHouse tables* (every table, plus ad-hoc SQL) |
| Notebook | <http://localhost:8888/lab/tree/verify.ipynb>, then Run All |
| Recorder log | `just logs` |
| End-to-end check | `just verify` |

- `just stop` and `just start` pause the cluster and keep the data;
  `just destroy` deletes it.
- The cluster mounts `samples/clickhouse/`, so the pods see `config/`,
  `grafana/` and `notebooks/` straight from your checkout.

**The API.** Size a message with its generated length helper; `record` then
hands the encoder a slot of exactly that length (see `on_trade`):

```rust
let (persist, writer) = Persist::start(SCHEMA, Settings::from_env())?;

let len = TradeEncoder::compute_length_with_header(symbol.len(), venue.len(), trade_id.len());
persist.record(TradeEncoder::TEMPLATE_ID, len, |buf| {
    Ok(TradeEncoder::wrap_and_apply_header(buf, 0)
        .fixed(&fields)
        .symbol(symbol)?
        .venue(venue)?
        .trade_id(trade_id)?
        .encoded_length_with_header())
})?;

writer.stop(); // on shutdown: flushes what is queued
```

`record` never allocates, copies or waits on ClickHouse:
- **Disabled table:** one atomic load; the encoder never runs.
- **Enabled table:** an uncontended lock plus the encode.

`just latency` times it on the calling thread while the writer inserts, next
to a timing-only control.

**Tables come from the schema.** Each message is a `MergeTree` table named
in snake_case, and each field is a column:

| SBE | ClickHouse |
|---|---|
| integers, `float`, `double` | `Int8`…`UInt64`, `Float32`, `Float64` |
| `semanticType="UTCTimestamp"` (ns) | `DateTime64(9, 'UTC')` |
| enum | `LowCardinality(String)`, the value's name |
| `char` array | `String` |
| `presence="optional"` | `Nullable(T)` |
| group `bids { price size }` | `bids.price Array(Float64)`, `bids.size Array(Float64)` |
| var-data | `String` |

- Every table also gets `inserted_at DEFAULT now64(3)`.
- Composites, sets, non-`char` arrays, nested groups and big-endian schemas
  are rejected when the schema loads.

**Once a second, the writer thread:**
1. re-reads `tables.yaml`;
2. creates or compares the tables listed there;
3. decodes each message into RowBinary;
4. sends one `INSERT` per table.

If an insert fails, its table is compared again before the retry. A table
altered or dropped while recording is therefore reported, or recreated,
rather than lost.

**Memory:** records wait in a 64 MiB buffer, and as much again can wait for a
retry. Past that, new records are dropped and counted in
`persist.dropped()`. There is no disk spool.

### Things to try

- **Turn recording on and off.** In `config/tables.yaml`, set
  `book_snapshot: { kind: dynamic, enabled: true }` and save.
  - Within a second the recorder logs `recording book_snapshot: on`, and
    Grafana's *Order book* panels fill in. Set it back to `false` and the
    rows stop.
  - No restart is needed. An invalid edit is logged and the last good
    configuration kept.
- **Add a column to a dynamic table.**
  - In `schema/market.xml`, add `<field name="bidLevels" id="12" type="uint8"/>`
    to `BookSnapshot`, after `sequence`.
  - Set `bid_levels: bids as u8,` in `on_book`, then run `just recorder`.
  - The log shows ``applied: ALTER TABLE `market`.`book_snapshot` ADD COLUMN IF NOT EXISTS `bid_levels` UInt8``.
  - Older rows read `0`. Remove the field again and the column stays, with
    new rows getting the default.
- **Change a static table.** `trade` and `quote` are never altered.
  - Add `<field name="quoteQty" id="9" type="double"/>` to `Trade`, after
    `aggressor`.
  - Set `quote_qty: t.price.as_f64() * t.size.as_f64(),` in `on_trade`, then
    run `just recorder`.
  - The recorder logs
    ``ERROR trade: static table is missing column quote_qty; not writing it. Fix: ALTER TABLE `market`.`trade` ADD COLUMN IF NOT EXISTS `quote_qty` Float64``
    and keeps writing every other column.
  - Run that SQL in `/play` and the column fills within 30 seconds.
  - A changed column type is reported the same way for both kinds;
    `MODIFY COLUMN` is never run automatically.

### Checks

```sh
just test     # persist unit + integration tests (starts a throwaway ClickHouse on :18123)
just lint     # clippy -D warnings + rustfmt
just latency  # persist.record() on the app thread, beside a timing-only control
just verify   # the running lab: /play, live data, every Grafana panel, the notebook, a live toggle
```

## Rules

- Keep every sample outside the workspace and unpublished.
- Do not expose sample-only abstractions as product APIs.
- Size SBE buffers from generated encoded-length APIs (prefer stack when const).
- Propagate fallible operations with `Result` and `?`.
- Delete a sample when it no longer exercises a distinct repository behavior.
