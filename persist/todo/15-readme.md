# Good README for ergo-clickhouse-persist

**Blocked by:** None (docs only)

The `persist/` crate has no `README.md`. The only user-facing doc is
`persist/docs/plan.md`, which is a grilled design plan — not a proper README.

## What needs to happen

Replace `persist/docs/plan.md` with a proper `persist/README.md` that
includes:

1. **Badge bar** — crates.io version, docs.rs, license, MSRV
2. **One-liner** — "Debugging persistence: auto-persist annotated Rust structs
   to ClickHouse with automatic schema management"
3. **Quick start** — minimal `#[derive(Persist)]` example with `cargo add`
   instructions
4. **Architecture diagram** — Producer (SBE bytes) → Consumer (decode → DTO →
   ClickHouse)
5. **Annotations reference** — `skip`, `flatten`, `json`, `array`, `name`,
   `type`, `order_by`
6. **Type mapping table** — Rust → ClickHouse (the `default_column_type` table)
7. **Feature flags** — `rust_decimal`, `chrono`, `serde`, `duration`
8. **Dynamic tables** — `DynamicRecorder` quick example
9. **Sink setup** — `ClickhouseSinkBuilder`, sender, batching
10. **Schema migration** — auto-add columns, widen types, never drop
11. **Running tests** — `just test` with docker dependency
12. **Link to** `sbe/todos/108-samples-e2e-orderbook-persist.md` for the
    end-to-end sample

## Acceptance criteria

- [ ] `persist/README.md` exists and is the canonical entry point
- [ ] `persist/docs/plan.md` deleted or merged into README
- [ ] README renders correctly on GitHub (check headings, code blocks, links)
- [ ] README referenced from workspace `README.md`
- [ ] `cargo doc` link points to the right docs.rs path
