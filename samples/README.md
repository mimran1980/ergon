# Samples laboratory

> **Low-quality unpublished playgrounds. Do not use these crates in production
> and do not copy them as reference implementations.**

The sample crates are deliberately outside the workspace. They exist only to try
ergo-sbe, ergo-clickhouse-persist, and ergo-aeron-cluster interfaces in larger flows.
They may contain experimental structure, unfinished migrations, external-service
assumptions, and code that is less polished than the two product prototypes.

| Sample | Purpose |
|---|---|
| `advanced-bitget` | Exercise Aeron IPC, nested SBE messages, converters, domain objects, and Persist |
| `cluster-ha-orderbook` | Exercise Cluster claims, egress, leader changes, and an HA-shaped data flow |

The current review found interface drift in `advanced-bitget` and generated
warnings in `cluster-ha-orderbook`. Until the implementation-plan compatibility
tasks pass, neither crate should be assumed to compile cleanly.

## Checks

```sh
(cd samples/advanced-bitget && cargo check --all-targets)
(cd samples/cluster-ha-orderbook && cargo check --all-targets)
```

Additional recipes can require Docker, ClickHouse, network access, Java 17+, built
Aeron jars, or a local multi-node Cluster. They are optional laboratory checks,
not release gates for Persist or Samples.

## Rules

- Keep `publish = false` and keep both crates outside the workspace.
- Use the product interfaces being tested; do not create sample-only public
  abstractions and then document them as recommended design.
- Prefer `Result` and `?` over avoidable `unwrap` or `expect`.
- Use domain objects when they make an experiment clearer and flyweights when the
  experiment specifically measures zero-copy behaviour.
- Delete code that no longer exercises a unique ergo-sbe or ergo-aeron-cluster interface.
- Track future work in `.scratch/<feature-slug>/spec.md` and its issue files,
  following [`docs/agents/issue-tracker.md`](../docs/agents/issue-tracker.md),
  not standalone todo Markdown files.
