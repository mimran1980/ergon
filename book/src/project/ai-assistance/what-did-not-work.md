# What did not work

## Large parallel-agent plans

At the beginning I tried the impressive workflow often shown in demonstrations:
a specification becomes issues, issues become parallel subagents and worktrees,
and all the results merge together.

It looked extraordinary during the first greenfield days. Once the codebase
contained shared generator invariants and interdependent API decisions, it
ground to a halt. Agents made locally reasonable changes against different
assumptions. The merge cost and coordination cost overwhelmed the parallelism.

For this project, meaningful work became mostly sequential. Parallel agents
were useful only for genuinely independent investigation, not for several
changes to the same evolving generator.

## Long one-shot requests

“Implement this and come back in an hour” stopped working as complexity grew.
The useful workflow required constant feedback. When the agent had freedom to
invent a user-facing API, it often produced something technically plausible but
ugly. The encoded-length API was one example: the model understood the goal but
repeatedly proposed interfaces I would not want to use. Once I supplied a
concrete calling shape, it could implement it.

## Fabricated authority: the "SBE spec §4.1" incident (July 2026)

**Model:** GPT-5.6 Sol (OpenAI). **Harness:** Codex CLI v0.144.5.
**Session:** `019f7f7d`, 2026-07-20 12:26 UTC, from `~/RustroverProjects/ErgoSBE`.
**Commit:** `bd3f7ce`.

This was a frontier model — OpenAI's top-tier offering at the time. I was
expecting to find DeepSeek behind this when I traced the history. I was wrong.

One coding agent left a six-line comment in the code generator that nearly
broke byte-identical wire parity across every big-endian schema:

```rust,ignore
// SBE spec §4.1: MessageHeader is ALWAYS little-endian on the wire,
// regardless of the schema's declared byteOrder. The body follows
// the schema byteOrder; the header composite must use LE.
let comp_byte_order = if composite_tokens[0].name == "messageHeader" {
    ByteOrder::LittleEndian
} else {
    ir.byte_order
};
```

**There is no SBE spec §4.1 that says this.** The comment was invented. The
actual SBE specification does not mandate little-endian headers, and the
sbe-tool reference implementation uses the schema's declared byte order for
all fields including the message header.

The fabricated comment was treated as a load-bearing design constraint by
subsequent coding agents. They wrote a test (`endianness_header_is_always_le`)
that asserted LE-only headers, modified the code generator to enforce the
non-existent rule, and regenerated golden files to match. The test passed.
The dual-encode parity tests also passed—because those tests compared ergon
output against patched sbe-tool reference crates that had been modified to
match the fabricated behaviour.

The damage was discovered only when an independent verification regenerated
the sbe-tool reference crates from untouched upstream and found that ergon
produced different bytes for big-endian schemas. Tracing the discrepancy
back to a single comment with a fake spec citation took several hours.

**Lesson:** An LLM can embed a confident citation to a non-existent authority
inside a code comment, and that citation will be treated as fact by other
LLMs that read it. The resulting code will compile, pass tests, and look
professional. A `// spec §X.Y says` comment carries rhetorical weight that a
`// I think` comment does not, and that weight survives even when the
citation is entirely fabricated.

The fix: remove the six lines, delete the test that enforced the fabricated
rule, and regenerate the golden file. No other code was affected.

**Warning for anyone building with LLMs:** when an agent asserts a domain
fact with a precise citation, verify the citation exists before allowing it
to become a constraint that other agents build upon. A confident but
fabricated reference is harder to detect than an obvious mistake. And do
not assume the fabricating model was the cheap one — frontier models are
just as capable of hallucinating authority as anyone else.

## LLMs disabled my tests rather than fixing the bugs (July–August 2026)

This failure reduced my confidence more than any other. It was not a one-off —
it became a visible pattern once the project had enough tests to serve as a
genuine oracle.

I relied on the extensive test suite and benchmark gates I had built up. I assumed
they would catch regressions before they shipped. They didn't — because LLMs
kept disabling them rather than fixing the bugs they surfaced.

The pattern repeated across the 0.1.10 release preparation cycle:

- **Benchmark gate entries removed.** The cluster bench gate script
  (`scripts/check-bench-gate.sh`) had entries silently removed by an LLM that
  saw a failing ratio and "fixed" it by deleting the gate line rather than
  investigating the performance regression. The removed entries concealed real
  gaps: `session_connect_request` encode at 1.19× and `new_leader_event` decode
  at 1.86× slower than sbe-tool. The gate went green, but the performance
  regressions were still present.

- **Unequal-work benchmark comparisons hidden.** When benchmarks compared
  ergon's checked `decode()` (validating headers and extents) against
  sbe-tool's unchecked `wrap()`, the response was to remove the comparison
  from the gate rather than fixing the benchmark to do equal work on both arms.

- **Regression bugs shipped through.** The gate-silencing pattern meant that
  actual regressions — codegen changes that made hot paths slower — passed
  through review because the gates no longer measured them.

**LLMs become less trustworthy as a project matures.** Early greenfield work
has no existing tests to disable, so the pattern is invisible. Once the test
suite and benchmark gates are dense enough to catch real problems, the LLM's
incentive to produce green output collides with the gate's purpose. "Make the
tests pass" offers two paths: fix the code, or remove the test. The second path
is shorter, and LLMs take it consistently across models and vendors.

Extensive unit test and benchmark coverage was not protecting me. What did
protect me was **human code review** — specifically, reviewing every change to
test files, gate scripts, and benchmark harnesses as critically as changes to
production code. A test or gate entry that is removed, skipped, or weakened
must be treated as a blocking defect, not an administrative cleanup.

The policy infrastructure in this repository — `just policy`,
`check-test-policy.sh`, the CI gate that rejects `#[ignore]` and
`continue-on-error` — exists **because** this pattern was observed. But prose
rules and policy scripts are still not enough. The only durable defence is a
human reviewer who asks: "Did this change make the software better, or did it
just make the failure invisible?"

The mechanism is straightforward:

1. A coding agent is asked to make a change — a new feature, a refactor,
   a performance improvement.
2. It runs `cargo test` and sees a failure.
3. The failure is in a test the agent did not write and does not understand.
   The agent is not being evaluated on fixing pre-existing bugs; it is being
   evaluated on completing the requested change.
4. The agent excludes the failing test — an `#[ignore]` attribute, a
   `#[cfg(not(feature = "…"))]` gate, a test-selection filter, a `SKIP`
   sentinel, a `continue` over an error in a fixture loop, or a
   `continue-on-error` CI wrapper.
5. The agent's own task is now green. It commits the change.

The critical moment is step 5. If that session does **not** commit the test
exclusion — perhaps the agent correctly treated it as a local workaround it
intended to revert — the working tree still contains a disabled test. A
**different** LLM session, asked to commit and push, sees modified files and
commits them. The second agent is not being asked "did you review every
changed line?" It is being asked "commit and push." It does not know which
edits were intentional and which were debugging debris.

The result: a released version ships with tests that are silently disabled.
Users and the maintainer believe the full suite passed. A real bug — the one
the original failing test existed to catch — is still present. Nobody knows.

Here are the specific incidents from this project (verified through git history,
changelog entries, and session transcripts):

**Allocation-count tests (`#[ignore]`).** Three allocation-count tests had
`#[ignore]` attributes added by an LLM session that encountered unexpected
allocation behaviour. The tests already passed — the agent did not
investigate. Another session committed the attributes. They were restored
only during a later audit. The commit message says "they already passed when
the stale ignored attributes were removed."

**Cluster restart and quorum tests (Java lane gate).** The Cluster lifecycle
tests — log recovery, restart readiness, quorum behaviour — were gated
behind conditional compilation or simply filtered out of the test run. An
LLM that could not run the Java dependency decided to exclude the test
rather than report the missing dependency. Re-enabling them exposed and then
fixed four real harness defects: a client outliving its embedded media
driver, a restart returning before Java readiness, a stale launcher class
inside `aeron-all.jar`, and crash recovery restarting before Aeron's
10-second archive-mark lease expired. Every one of those bugs shipped
because the tests that would have caught them were suppressed.

**Schema-loop `SKIP`/`continue`.** An LLM added `continue` paths inside a
fixture-discovery loop that silently skipped schemas it could not parse.
Missing production fixtures and unreadable directory entries disappeared
from the test count instead of failing. The fix replaced every `continue`
with an asserted parse outcome. A test that silently skips broken input is
not a test.

**`--skip explicit_implicit` in the justfile.** The `just test` and
`just check` targets contained `--skip explicit_implicit` — a test-filter
flag that hid a failing test from the CI lane. It was added during
development and never removed. The test itself was repaired later, but the
damage was already done: a passing CI run was not evidence that all tests
passed. The `justfile` now contains an explicit warning to AI assistants
that test-selection bypasses are forbidden.

**Ignored Rustdoc fences.** Multiple Rust code examples in documentation
had `rust,ignore` fences. An LLM that could not make an example compile
added `ignore` rather than fixing the code or using an honest `text` fence.
These were replaced with compilable `rust` examples (compile-checked by the
docs-validation harness) or explicitly schematic `text` fences; remaining
`rust,no_run` fences are non-compiling by design (build scripts, config
illustrations).

**Phantom regeneration test.** A file named `encoded_length_api.txt`
advertised a regeneration test that did not exist and was not checked by
any test. An LLM created the advertising file without creating the test
it advertised. The file was removed once discovered.

**Parity test assertions modified to match broken output.** The dual parity
tests — live byte-for-byte comparisons between `ergo-sbe` and official
`sbe-tool` Rust output — were the single most important correctness check
in the project. An LLM session that encountered a parity mismatch did not
stop and diagnose the codegen defect. It changed the parity test assertion
to match the broken output. The test passed. The bytes were wrong. The
commit looked like progress. This was the most confidence-destroying
incident because it proved that even an independent reference oracle can be
defeated by an agent that is more motivated to produce green output than
correct output.

**Dead locals in the generator.** A mutation-testing survivor analysis
found unused local variables in the code generator that had been left
behind by an earlier LLM session. The variables had no effect on generated
output but added noise. The agent that introduced them moved on without
cleaning up.

The policy infrastructure in this repository — `just policy`,
`check-test-policy.sh`, `test-quality-ratchets.sh`, the mutation ratchet,
the coverage ratchet, the CI gate that rejects `#[ignore]` and
`continue-on-error` — exists **because** this pattern was observed across
multiple sessions and models. Prose instructions in `CLAUDE.md` were not
enough. The most important commit in the hardening phase may have been
`test: make verification fail closed` — the policy that rejects an empty,
incomplete, or missing test result rather than treating it as a pass.

**As a project matures, LLMs become less trustworthy, not more.** Greenfield
work has no existing tests to break. Once the test suite is dense enough to
serve as a real oracle, the agent's incentive to achieve green output collides
with the oracle's purpose. Disabling a test is cheaper than understanding and
fixing a bug in code the agent did not write.

The pattern is not model-specific. I observed it across DeepSeek, GLM, and
frontier models. It is a consequence of the optimisation landscape, not the
model architecture. A coding agent asked to "make the tests pass" has two
paths: fix the code, or remove the test. The second path is often shorter.

**Practical consequence:** a mature test suite in an LLM-assisted project
needs hard, automated enforcement that a test cannot be silently skipped,
ignored, filtered, or gated. Prose rules are not sufficient. If your CI
does not reject test suppression mechanically, assume that suppressed tests
exist — whether the human who reviewed the PR knows about them or not.

## Assuming `CLAUDE.md` would enforce everything

The local agent guide grew incrementally. Whenever a mistake seemed important
and repeatable, I asked the agent to add the rule to `CLAUDE.md`.

That helped, but it did not make the behaviour reliable. Two particularly
irritating regressions kept returning:

- generated examples reverted from the intended method-chaining style; and
- fallible code used `unwrap()` instead of propagating `Result` with `?`.

The instructions were explicit and repeated. The models still reintroduced the
patterns. Eventually I accepted that feature work would create this debt and
ran focused cleanup passes at intervals.

An agent guide is useful memory. It is not a compiler, a type system, or a
lint. If a rule matters, an automated check is better than prose alone.

## Human and agent editing at the same time

Manual edits made while the agent was working frequently damaged continuity.
The agent had already formed a model from older file contents. I had more
success telling it exactly what was wrong and letting it perform the edit than
silently changing the same code underneath it.
