# How the project evolved

Looking back, the work divided into several recognisable phases. This matters
because the workflow that appeared successful in the first phase was not the
workflow that completed the project.

## Phase 1: the spectacular greenfield demo

At the beginning, specifications, issues, subagents, and separate worktrees
looked almost magical. There was no established architecture to collide with,
so several agents could create apparently useful pieces at once. This is the
part of agentic development that makes the best video: a blank repository
becomes a compiling application while multiple streams of activity scroll
past.

It was real progress, but it created the wrong expectation. Parallel output is
not the same thing as an integrated design. As soon as type-state transitions,
generator conventions, error handling, wire offsets, and generated API style
became shared constraints, locally reasonable changes stopped composing.

## Phase 2: shared invariants forced sequential work

The project became productive again when I reduced the unit of work and made
the feedback loop mostly sequential. One agent changed one connected area,
regenerated the codecs, ran the focused tests, showed me the result, and
received a correction. This was less theatrical and much more effective.

At this stage I learned that I needed to inspect the artefact closest to the
contract. For `ergo-sbe`, that was normally the emitted Rust, the calling API,
the encoded bytes, or the benchmark—not the syntax-tree construction that
produced it. I could identify a bad offset or ugly repeating-group API quickly
in generated code. The same mistake was much harder to see by mentally
executing hundreds of lines of `syn` and `quote`.

## Phase 3: performance invalidated an elegant design

The first type-state design used generics (`Encoder<State>`) because I assumed
the abstraction would be free after monomorphisation. It was elegant and enforced
the ordering requirement, but the benchmark showed a meaningful encoding regression.
A design can be type-safe, idiomatic, tested, and still fail the product requirement.

The LLM helped explain why the generated machine code differed and proposed
concrete named stage types with a zero-sized `H: HeaderState` marker. I accepted
the concrete representation because the measured result mattered more than abstract
neatness. All 15 maintained parity comparisons passed at or below the 1.00×
ceiling in the 0.1.8 release (both LTO profiles). From then on, benchmark
parity became part of the definition of done for relevant changes. The gate
as it stands now, including the exceptions and the A/A noise band, is
[`sbe/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md) under "The gate".

## Phase 4: cheap implementation expanded the product

Once the core generator was working, features that would have been too
expensive in a four-to-six-month manual schedule became plausible. Exact
encoded length, DTOs, converters, richer samples, schema-evolution cases, and
more exhaustive generated APIs all grew during this period.

This is one of the genuine advantages of LLM implementation: it lowers the
cost of trying a design. I could ask for a complete version, inspect the
result, reject it, and try a different shape without feeling attached to the
discarded code. The disadvantage is that every cheap feature creates an
expensive verification and review obligation. Generated code is cheap;
confidence is not.

## Phase 5: hardening and disclosure

The final phase was less about visible capability and more about earning
confidence: official fixture decoding, byte-for-byte dual encoding, multiple
official schemas, compile-fail tests, exact-size matrices, allocation checks,
unsafe audits, benchmark gates, documentation, and this disclosure.

The project crossed a line during this phase. It was no longer merely an
exercise in learning agents. It had become a crate I wanted other SBE
developers to evaluate. That raised the standard and is why the work continued
past the holiday.
