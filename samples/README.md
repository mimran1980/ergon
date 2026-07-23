# Samples laboratory

> **Low-quality unpublished playgrounds. Do not use these crates in production
> and do not copy them as reference implementations.**

The sample crates are deliberately outside the workspace. They exist only to try
ergo-sbe and ergo-aeron-cluster interfaces in larger flows. They may contain
experimental structure, unfinished migrations, external-service assumptions, and
code that is less polished than the two product prototypes.

| Sample | Purpose |
|---|---|
| `exchange-example` | Exercise Aeron IPC, nested SBE messages, converters, domain objects |
| `cluster-ha-orderbook` | Exercise Cluster claims, egress, leader changes, and an HA-shaped data flow |
| `cluster-rfq` | Historical RFQ/auction protocol experiments (not a reference implementation) |

## Checks

```sh
(cd samples/exchange-example && cargo check --all-targets)
(cd samples/cluster-ha-orderbook && cargo check --all-targets)
(cd samples/cluster-rfq && cargo check --all-targets)
```

Additional recipes can require Java 17+, built Aeron jars, or a local multi-node
Cluster. They are optional laboratory checks, not release gates.

## Rules

- Keep `publish = false` and keep crates outside the workspace.
- Use the product interfaces being tested; do not create sample-only public
  abstractions and then document them as recommended design.
- Prefer `Result` and `?` over avoidable `unwrap` or `expect`.
- Use domain objects when they make an experiment clearer and flyweights when the
  experiment specifically measures zero-copy behaviour.
- Delete code that no longer exercises a unique ergo-sbe or ergo-aeron-cluster interface.
- Track future work in `.scratch/<feature-slug>/spec.md` and its issue files,
  following [`docs/agents/issue-tracker.md`](../docs/agents/issue-tracker.md),
  not standalone todo Markdown files.
