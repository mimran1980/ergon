# AI assistance

This repository was developed **with substantial AI assistance**. Humans set
direction, reviewed design trade-offs, ran verification (tests, benches, wire
parity), and decided what landed in the tree.

This file is the place for an honest account of **how** AI was used — not a
claim that the code is unreviewed or untested.

## What AI was used for

Typical work where AI helped (and will continue to help):

- Exploring SBE / Aeron APIs and comparing API shapes (stages vs flyweights,
  buffer sizing, domain DTOs)
- Drafting and iterating on generated-code design and `build.rs` / sample
  patterns
- Implementing large mechanical surfaces (codegen paths, tests, docs, samples)
- Debugging and refactoring with tight feedback loops (`cargo test`, Criterion,
  golden fixtures)

The list of tools and models used over the life of the project may grow; when
you care about a specific era of the history, check git authorship, PR
discussion, and this file’s updates.

## What humans own

Regardless of how a patch was drafted:

1. **Problem selection** — what problem is worth solving (e.g. safer exact-size
   claims for zero-copy publish).
2. **Acceptance criteria** — wire compatibility with official SBE, maintained
   bench gates vs sbe-tool, test coverage, experimental honesty in docs.
3. **Review and merge** — nothing is “done” solely because a model proposed it.
4. **Verification** — commands actually run; green tests/benches are required
   for hot-path and release-relevant changes (see `CONTRIBUTING.md`,
   `sbe/BENCHMARKS.md`, `just test` / `just bench`).

## How to treat the code

- Treat AI-assisted origin as **process context**, not a quality substitute.
- Prefer the automated suite and benchmarks over narrative claims.
- Report production use and defects the same way you would for any open-source
  library: issues, reproductions, and wire fixtures.

## Updating this note

When the toolchain, policy, or review process changes in a material way, amend
this file in the same PR (or a follow-up) so readers are not relying on stale
process claims.

---

*Maintainer: expand the sections above with the concrete tools, models, and
review habits you want public. Keep it short and accurate.*
