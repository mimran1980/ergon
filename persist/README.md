# Persist laboratory

> **Unpublished internal test bed. Do not use in production and do not treat this
> crate as a reference implementation.**

`ergo-clickhouse-persist` and its derive crate exist to exercise ergo-sbe domain
objects, conversions, annotations, and realistic downstream data shapes. Persist
is not one of the repository's product prototypes and is not planned for
crates.io publication.

The code experiments with:

- derived table and column metadata;
- typed and dynamic ClickHouse rows;
- schema creation and migration checks;
- batching, retry, metrics, and dead-letter behaviour;
- optional timestamp and decimal mappings;
- converting generated SBE/domain values into persistence-oriented values.

These experiments are useful as integration pressure on ergo-sbe. They are not a
stable storage abstraction, supported ClickHouse client, or recommended
application architecture. Interfaces may be simplified or deleted whenever they
stop helping test ergo-sbe.

## Checks

Offline tests:

```sh
cargo test -p ergo-clickhouse-persist --all-features
cargo clippy -p ergo-clickhouse-persist --all-targets --all-features -- -D warnings
```

Live ClickHouse tests use the repository helper:

```sh
bash persist/tests/run-clickhouse.sh start
cargo test -p ergo-clickhouse-persist --all-features -- --ignored
bash persist/tests/run-clickhouse.sh stop
```

Live tests require Docker and a local ClickHouse instance. Their success does not
change the laboratory status.

## Relationship to ergo-sbe

Persist should adopt the final generic conversion and domain-object interfaces
where that makes the tests clearer. It must not force ClickHouse, decimal,
timestamp, serde, or allocation concerns into ergo-sbe's low-level generated hot
path.

Track future Persist work in the relevant `.scratch/<feature-slug>/spec.md` and
issue files using [`docs/agents/issue-tracker.md`](../docs/agents/issue-tracker.md);
do not recreate a Persist-specific todo directory.
