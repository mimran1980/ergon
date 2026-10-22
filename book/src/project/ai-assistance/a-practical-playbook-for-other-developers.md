# A practical playbook for other developers

This project does not provide a universal recipe for AI-generated software.
The conditions were unusually favourable. It does, however, suggest a
repeatable method for projects where the developer already understands the
domain and correctness can be made observable.

## 1. Choose a problem with an oracle

The best agent task is not merely one that can be described. It is one whose
result can be disproved.

For this project, official SBE bytes were the strongest oracle. Compilation,
round trips, generated-source inspection, exact lengths, allocation counts,
and benchmark results supplied additional independent signals. If the only
acceptance criterion had been “the API looks plausible,” I would not trust the
result.

Before delegating a large implementation, ask what will turn a model's mistake
into a concrete failure. If the answer is “a human will eventually notice,”
the feedback loop is too weak.

## 2. Keep domain authority with a person who understands the problem

I did not ask the models to decide what SBE should mean. I knew the ordering
rules, group structure, variable-data behaviour, schema-evolution concerns,
and practical user mistakes before the project started.

An agent can implement a wrong specification extremely well. Tests derived
only from the same wrong specification may all pass. The responsible person
must be able to explain the invariant, recognise a plausible
misinterpretation, and reject an API even when it is polished.

This is why I would not generalise my DeepSeek result to somebody learning SBE
from the model while simultaneously asking it to build the generator. In that
situation, there is no independent domain authority in the loop.

## 3. Define the observable contract before discussing implementation

I obtained better results when I said what generated calling code should look
like, which illegal sequence should fail to compile, or which exact bytes
should be produced. Vague goals such as “make it idiomatic” gave the agent too
much freedom and usually created an ugly API.

A useful task statement includes:

- a representative input;
- the desired public call site;
- the expected output or failure;
- the invariant being protected;
- the tests that must pass; and
- the benchmark or allocation condition, if performance matters.

The model can then explore implementation details without owning the product
decision.

## 4. Review the artefact closest to the real contract

Reviewing every generated line is ideal when it is practical, but enormous
AI-authored diffs can exceed honest human review capacity. That does not
justify pretending they were reviewed.

Instead, identify where your expertise has the highest leverage. I reviewed
emitted Rust, public API shape, stage transitions, bytes, exact lengths, and
performance more deeply than the `syn`/`quote` machinery. In another project
the right surface might be SQL plans, a protocol trace, a rendered page, or a
machine-generated configuration.

Be explicit about what this choice leaves unreviewed. Behavioural evidence
narrows a trust boundary; it does not make the boundary disappear.

## 5. Prefer independent references over self-consistency

A codec that encodes and then decodes its own incorrect format can pass every
round-trip test. A generator and a test written by the same model can share the
same misunderstanding.

The most valuable tests compare against something independent:

- an official implementation;
- a protocol fixture produced by another language;
- a published conformance suite;
- a hand-calculated small example;
- an external parser or validator; or
- a previous production implementation.

For `ergo-sbe`, having both official `sbe-tool` generated Rust and the new
generator encode the same logical message to identical bytes is far stronger
than either codec decoding its own output.

## 6. Use short, mostly sequential loops after shared invariants emerge

Parallel agents are useful for truly independent research or isolated files.
They were counterproductive when several branches changed the same generator
conventions and API model.

The loop that worked was small: test, edit, regenerate, run, inspect, correct.
When an agent moved in the wrong direction, stopping after minutes was better
than reviewing an hour of coherent but unsuitable work.

The unit of delegation should shrink as the codebase becomes more coupled.
Greenfield parallelism is not evidence that late-stage parallelism will work.

## 7. Give one editor ownership of an active change

I had less success when I manually edited files while an agent retained an
older view of them. The agent's context was part of its working state. Silent
external edits made that state stale.

My practical solution was to explain the correction and let the active agent
make it. This is not a rule that humans must never code. It is a coordination
rule: avoid two editors changing the same conceptual unit without an explicit
reload and reconciliation step.

## 8. Treat agent guides as memory, not enforcement

`CLAUDE.md` was valuable for accumulating project preferences and previous
mistakes. It did not reliably stop the agents from reintroducing `unwrap()` or
non-chained examples.

If a rule is important, promote it from prose to something executable:

- a compiler error;
- a lint;
- a compile-fail test;
- a source scan;
- a formatter;
- a golden file; or
- a CI check.

Instructions influence probability. Automation changes the acceptance
boundary.

## 9. Put performance in the definition of done

Performance-sensitive abstractions should be benchmarked from the beginning,
not after the API has hardened. The generic type-state design taught me this
the expensive way.

For every relevant change, the agent had to rerun benchmarks and investigate a
regression rather than merely report it. This increased token use, but it
prevented performance debt from accumulating invisibly.

Benchmark equal work. A comparison is meaningless if one side encodes fewer
fields, omits a group, performs less validation, or uses a different buffer
lifecycle. Generated APIs make accidental unequal work especially easy to
hide.

## 10. Experiment with unsafe code; do not assume it is faster

I asked the agents to try unsafe variants because a checked boundary can make
some repeated bounds checks redundant. Some experiments improved performance;
others made no meaningful difference and were removed.

Every retained unsafe block needs:

- a stated invariant;
- a place where that invariant is established;
- tests around the boundary;
- a measured reason to keep it, unless borrowing genuinely requires it; and
- a clear distinction between checked and trusted public entry points.

“The LLM wrote it” is neither a safety proof nor a reason to reject it. The
proof must stand independently of its author.

## 11. Preserve context deliberately and compact with a handoff

Long context helped because it contained rejected designs, benchmark history,
style corrections, and domain explanations. Throwing that away mid-task made
the agent repeat old mistakes. Compacting at a **real task boundary**—when the
next work was sufficiently different—was usually the right trade.

At the same time, unlimited history increases cost and can bury the current
task. A good compaction or handoff should preserve:

- decisions and their reasons;
- invariants;
- known failure modes;
- commands that prove completion;
- current benchmark baselines; and
- unfinished work.

Do not retain a million tokens merely because the window exists. Retain them
when the accumulated decisions improve the next turn enough to justify their
cache cost.

## 12. Price the workload, not the marketing number

For agentic coding, compare at least cache hits, cache writes or misses, output,
context thresholds, and rate limits. A model with cheaper ordinary input may
still be expensive for a repeated-prefix workload. A subscription that looks
cheap may stop an intensive day halfway through.

Use your own dashboard split when possible. Then calculate:

```text
hit tokens × hit price
+ miss/write tokens × miss/write price
+ output tokens × output price
```

Apply any long-context multiplier separately. Keep clear whether you are
comparing equal token volume, equal elapsed time, or equal completed work.
Those answer different questions.

## 13. Separate the agent harness from the model

Most implementation happened through Claude Code, but most implementation was
not performed by a Claude model. The harness provided file access, tool use,
session management, and interaction conventions. DeepSeek or GLM supplied the
model behind it.

This distinction matters when reporting results and when reproducing them. A
good model in a poor harness may be frustrating; a good harness can make a
less expensive model highly productive. “I used Claude Code” is not a complete
model-provenance statement.

## 14. Use frontier-model reviews as another opinion, not proof

Claude, OpenAI, and Grok reviews were useful. They sometimes expressed a
problem better or found a slightly different angle. They were not an
independent conformance oracle, and none rescued a task that DeepSeek could not
complete in this project.

Models can share the same training-derived assumptions. Several agreeing that
code looks correct is weaker than one official byte comparison demonstrating
that it is correct for a specific case.

## 15. Stop if you cannot explain the result

The human responsible for merging a change should be able to explain:

- what requirement changed;
- why the implementation satisfies it;
- what evidence would fail if it were wrong;
- which surfaces remain unreviewed; and
- what operational risk remains.

If the explanation is “the agent seemed confident and all of its own tests
passed,” the work is not ready.

## 16. Publish the uncomfortable facts

AI-assisted projects need more provenance, not less. State how much code was
generated, where human review concentrated, which models and harnesses were
used, which evidence is independent, what the work cost, and why the maturity
label remains.

This disclosure is long because “AI-assisted” covers everything from
autocomplete to a project in which an agent wrote nearly every implementation
line. Users deciding whether to put a codec near money deserve the meaningful
version.
