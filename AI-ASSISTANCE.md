# AI Assistance Disclosure

`ergo-sbe` was developed with heavy AI assistance. Most of the implementation,
tests, benchmarks, samples, and documentation were written by coding agents.
The maintainer did very little direct coding. If you do not want software
written substantially by LLMs, stop here.

## What the AI did

Coding agents wrote nearly all of the `syn`/`quote` generator, most tests,
benchmarks, samples, and documentation. The harness was Claude Code CLI, often
pointed at non-Claude models. DeepSeek V4 Flash and V4 Pro did most of the
implementation work. GLM-5.2 and GLM-4.7 were used substantially early on.
Claude, OpenAI, and Grok were used mainly for extra reviews. The maintainer
supplied more than a decade of SBE experience, the requirements, API judgement,
domain corrections, acceptance criteria, and release decisions. The main design
ideas — compile-time wire ordering, exact encoded length, closure-based groups,
checked constructors — came from that experience.

## What was reviewed, and what was not

Human review concentrated on the generated Rust API and source, encoded bytes,
test failures, and benchmark results. The maintainer did not perform an
exhaustive line-by-line audit of the generator internals. Do not describe this
as "human-reviewed code". A more accurate description is: human-directed,
generated-output reviewed, and behaviourally verified, with generator internals
substantially AI-authored and not exhaustively line-reviewed by a human.

## Independent verification

The observable output is constrained by live byte-for-byte comparisons against
official `sbe-tool` Rust codecs, Java-produced official fixtures, upstream
schemas, compile-fail proofs for illegal stage ordering, property tests, exact
encoded-length matrices, malformed-buffer tests, zero-allocation checks, and
performance gates at or below the 1.00x sbe-tool ceiling. That is strong
evidence. It is not a substitute for independent production use.

## Known failure modes

**Fabricated authority.** A coding agent invented a citation ("SBE spec §4.1")
and changed header endianness behaviour to match it. Later agents treated the
comment as fact, wrote a passing test, and regenerated golden files. The error
was found only when the sbe-tool reference crates were regenerated from
untouched upstream. Verify a model's domain citations before letting them
constrain other work.

**Tests suppressed rather than fixed.** Across the 0.1.10 cycle, LLMs removed
benchmark-gate entries, added `#[ignore]` to tests, filtered failing tests from
the justfile, skipped schemas with `continue`, and edited parity assertions to
match broken output. The policy tooling here (`just policy`,
`check-test-policy.sh`, fail-closed verification) exists because of this
pattern. A removed, skipped, or weakened test or gate is a blocking defect.

## Trust boundary, cost, and status

Checked entry points (`wrap`, `decode`) return `Result`, validate extents once,
then enter a private zero-check core. Public `*_unchecked` twins are not
currently shipped. Unsafe is kept only where Rust borrowing requires it or a
repeatable hot-path benchmark justifies it. The initial 0.1.0 release used
approximately 14 billion tokens (provider dashboards). Out-of-pocket spend for
the intensive development period was about **$261** (DeepSeek pay-as-you-go
$77.39, GLM plan $114, subscriptions $70). This is experimental 0.x software,
not production-proven. Audit the internal unsafe assumptions before using the
crate near money.

## What to verify before production

Generate codecs for your actual schemas. Encode the same messages with official
SBE tooling and `ergo-sbe`; compare exact bytes and cross-decode both ways.
Cover empty, maximum, nested, ragged, and variable-data cases, and every acting
version you expect. Test malformed and truncated frames at the trust boundary.
Verify exact buffer lengths before using `try_claim`. Benchmark your real hot
fields. Soak-test under real traffic and deployment conditions.

## AI-assisted contributions

AI-assisted pull requests are not banned, but a contributor must understand the
relevant SBE behaviour and be able to explain it. A focused issue with a real
schema, a minimal reproduction, expected official bytes, and a failing
behavioural or parity test is often more valuable than a large patch. AI
assistance is not disqualifying. Lack of domain understanding is.
