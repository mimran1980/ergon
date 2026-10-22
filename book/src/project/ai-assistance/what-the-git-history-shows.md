# What the Git history shows

The commit history provides a useful independent record of how the project
actually developed. It should not be treated as a measure of human effort:
agents commit much more frequently than I would when working manually, and
some commits are tiny formatting, test, or repair steps. It does, however,
show the shape and sequence of the work.

The history snapshot ending at commit `85442ed` on 26 July 2026 contains
**976 commits after the initial `main` scaffold**. The active codegen work
began on 5 July and the history records:

- 14 commits on 5 July;
- 157 on 6 July;
- 128 on 7 July;
- 89 on 8 July;
- 46 on 9 July;
- 74 on 10 July; and
- 41 on 11 July.

That is consistent with the intensity of the holiday period, but the more
interesting evidence is structural. The history contains ten explicitly named
`worktree-agent` or equivalent worktree merge commits. Every one of those
occurred on 6 or 7 July. After that early experiment, the branch becomes
overwhelmingly linear; the only much later merge was an ordinary remote
tracking merge on 25 July. This is visible evidence of the transition I
described: parallel subagents were exciting in the greenfield phase, then
shared invariants pushed the development into a sequential loop.

The subjects also expose the sequence of technical lessons:

- **5–6 July:** scaffold the generator, create the roadmap, add baseline
  wire tests, CI, golden generation, upstream regression schemas, benchmarks,
  unsafe experiments, and early exact-length support.
- **7–8 July:** migrate more generator code to `syn`/`quote`, expand group and
  variable-data support, add property and conformance tests, and repeatedly
  repair the interactions between those features.
- **9 July:** benchmark-driven redesign from generic encoder states to
  non-generic concrete structs, followed by generated-shape regression tests.
- **10 July:** introduce concrete consuming decoder stages, compile-fail
  ordering proofs, zero-allocation checks, and broad generator coverage.
- **11 July:** restrict bytes and encoded length to complete stages, add nested
  message and converter workflows, and prove callback and stage ownership
  constraints.
- **17–21 July:** rerun multi-run benchmark matrices, deepen DTO/converter
  support, migrate the Aeron Cluster client to `ergo-sbe`, and harden Cluster
  behaviour and samples.
- **22–24 July:** rename the workspace to `ergon`, expand the L3 examples,
  build the staged exact-length API for uniform, ragged, nested, and
  variable-data shapes, and revisit checked versus unchecked performance.
- **25 July:** fix DTO conversion and ragged-length defects, complete Cluster
  reliability work, run Java interoperability tests, repair benchmark gates,
  and add broader generated-code safety tests.
- **26 July:** clarify documentation and packaging, add live byte-for-byte
  comparisons against checked-in official `sbe-tool` Rust codecs, close the
  multi-schema parity gaps, and publish this disclosure.

This is a messier and more credible history than a one-shot generation story.
It contains reverts, merge repairs, performance regressions, benchmark
redesigns, API replacements, small cleanup passes, and tests added after
failures. It shows that the project was not produced by one prompt. It was
produced through hundreds of short, observable corrections.
