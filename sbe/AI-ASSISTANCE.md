# AI assistance: how `ergo-sbe` was built

`ergo-sbe` was developed with **heavy AI assistance**. Most of the implementation,
tests, benchmarks, samples, and documentation were written by coding agents. I
did very little direct coding.

I am saying that at the beginning because this is not a conventional project
with a little autocomplete around the edges. If you do not want to use software
written substantially by LLMs, stop here. This is probably not the right
project for you.

That warning is not an apology, and this page is not marketing. It is a
disclosure of:

- why I chose this project;
- what I knew before the models became involved;
- what I asked the models to do;
- what I did and did not review;
- how the generated codecs were verified;
- where the workflow worked and where it failed;
- which tools and models were used;
- what the work actually cost;
- why the crate is still labelled experimental; and
- what I would expect before using it in a financial system.

The short version is:

> I supplied the SBE domain knowledge, requirements, API judgment, corrections,
> acceptance criteria, and release decisions. Coding agents supplied nearly all
> of the implementation. I reviewed the generated Rust API and output far more
> closely than I reviewed the `syn`/`quote` generator implementation. The
> observable output is constrained by extensive tests, live byte-for-byte
> comparisons with official `sbe-tool` Rust codecs, official fixtures,
> compile-fail proofs, allocation checks, and performance gates. That is strong
> evidence, but it is not a substitute for independent production use.

This account describes the initial intensive development period ending in July
2026. Prices, model names, repository state, and my process will change. Treat
dated figures as a historical record, not a permanent promise.

## Contents

1. [Why this page exists](#why-this-page-exists)
2. [What I was trying to build](#what-i-was-trying-to-build)
3. [Why I chose it as an AI experiment](#why-i-chose-it-as-an-ai-experiment)
4. [Authorship](#authorship-what-was-mine-and-what-was-generated)
5. [What I reviewed—and what I did not](#what-i-reviewedand-what-i-did-not)
6. [The actual working loop](#the-actual-working-loop)
7. [What did not work](#what-did-not-work)
8. [Verification](#verification-why-the-tests-matter-so-much)
9. [Performance](#performance-was-part-of-correctness)
10. [Unsafe code and the trust boundary](#unsafe-code-and-the-trust-boundary)
11. [Tools and models](#tools-models-and-what-each-contributed)
12. [Long context and caching](#long-context-and-why-caching-changed-the-economics)
13. [Observed usage and spend](#observed-usage-and-actual-spend)
14. [Pay-as-you-go comparison](#normalised-pay-as-you-go-comparison)
15. [The personal experience](#the-personal-experience-pride-enjoyment-and-review-fatigue)
16. [Why the crate is experimental](#why-this-crate-is-still-experimental)
17. [What production users should verify](#what-a-prospective-production-user-should-verify)
18. [AI-assisted contributions](#ai-assisted-contributions)
19. [Final assessment](#final-assessment)

## Why this page exists

SBE is widely used in financial systems. In that environment, code quality is
not an aesthetic preference. A plausible-looking logic error can corrupt a
message, misread a price or quantity, or fail only when a particular schema
version or repeating-group shape appears in production.

AI has made software provenance much harder to judge. Two people can publish
similar-looking libraries:

1. a domain expert who has encountered the protocol's failure modes for years;
2. somebody with no practical knowledge of the domain who asked an LLM to
   generate a library and was impressed that it compiled.

The second person may honestly believe the result is excellent because they do
not know what the model misunderstood. A reader should not have to reverse
engineer which situation applies.

LLM-generated defects also tend to feel different from ordinary human defects.
A human implementation often contains a typo, a missed branch, or a
copy-and-paste error inside a design the author understands. An LLM can produce
a coherent implementation of the wrong mental model. The code may be clean,
consistent, documented, and comprehensively wrong about one domain invariant.
That changes how it should be reviewed.

This page therefore exists so that an engineer—especially one responsible for
money—can make an informed decision. Do not infer quality from the amount of
code, the fluency of the documentation, or the fact that tests are green. Look
at the verification evidence, the remaining trust boundaries, and the
experimental status.

## What I was trying to build

I have used SBE for more than a decade, including Java and Rust systems. The
project idea predates this AI experiment.

Over those years I repeatedly saw the same classes of problem:

- A new SBE user reads or writes positional data in the wrong order.
- Repeating groups and variable-length data make buffer sizing easy to get
  wrong.
- Java-style flyweights and parent hopping become awkward under Rust's borrow
  checker.
- A low-latency wire representation is valuable on a hot path, but it is
  unnecessarily unpleasant when application code simply wants a Boolean,
  decimal, owned DTO, or database record.
- A partially completed encoder can be mistaken for a complete message unless
  the API makes that misuse impossible.

My desired API followed from those experiences:

- wire order enforced at compile time;
- no final byte slice or encoded length until all required tail fields have
  been written;
- closure-based nested groups that work naturally with Rust borrowing;
- exact buffer sizing for messages containing groups and variable data;
- a fast flyweight path for latency-sensitive code;
- optional conversions and domain DTOs for code that does not need to operate
  directly on the wire representation; and
- official SBE wire compatibility as the non-negotiable contract.

Without coding agents, I estimated that the generator would take me roughly
four to six months. I had previously written code generation by hand in another
open-source project and knew how fiddly a large `syn`/`quote` implementation
could become. My original plan was to wait until a long garden-leave period and
build it manually.

## Why I chose it as an AI experiment

I had a two-and-a-half-week holiday and wanted to improve from being a regular
LLM user to understanding agentic coding properly: which models were useful for
which jobs, how context affected them, how quickly subscription limits became
the bottleneck, and whether the impressive multi-agent workflows shown in demos
survived contact with a non-trivial codebase.

This project seemed unusually well suited to that experiment:

- I already knew the domain and the behaviour I wanted.
- The requirements were concrete; I did not need an LLM to teach me SBE.
- Much of the implementation was mechanical but cumbersome code generation.
- Official SBE has extensive schemas, fixtures, and tests.
- The product of the generator is Rust source that I can inspect directly.
- Wire bytes and benchmark results provide objective feedback.
- If the model misunderstood an API, I could normally identify the problem
  quickly and show it the shape I wanted.

I had also seen a Jon Gjengset demonstration of an AI-assisted Rust porting
exercise. It made this kind of project look worth trying.

The holiday work was intense. It was effectively all I was doing: at least
eight hours and sometimes closer to fourteen hours on a day. An agent might run
for 10, 20, or occasionally 30–40 minutes, during which I could do something
else, but I stayed available to inspect its progress and intervene. The project
was not finished in two and a half weeks. It expanded to roughly a month of
heavy work, including subsequent evenings and weekends, because I added useful
features and raised the quality threshold as the crate became closer to
something I might genuinely use.

### How the scope grew

Some capabilities were firm requirements from the beginning; others became
practical only after I saw how quickly the generator could evolve.

- Exact encoded length was something I had wanted for years, but I did not
  initially assume it would fit into the holiday scope. Once the agent could
  generate the repetitive machinery quickly, I decided it was worth doing
  properly.
- Domain DTOs began as a nice-to-have and expanded when it became clear that
  they could make SBE useful outside the hottest latency-sensitive path.
- Schema evolution was part of the intended protocol support.
- The Aeron Cluster client began as a realistic consumer of the codecs rather
  than the main product. Focused samples later became a better way to exercise
  difficult API shapes.

The ease of generating experiments encouraged scope growth, but it did not
make the verification free. Every added feature created more generated
surfaces, tests, and benchmark work.

## Authorship: what was mine and what was generated

I did very little manual implementation. Even when I saw the required code
change, I usually described it to the active agent and let the agent edit the
files.

That was partly deliberate. I found that mixing my own live edits with an
agent's existing context often made the agent less reliable. It continued from
the version of a file it had already read unless I explicitly told it to reload
and reconcile everything. The most successful workflow was to let the agent
drive the edits while I drove the design and feedback.

The division was approximately:

| Area | My role | Coding-agent role |
|---|---|---|
| Problem and product | Chose the problem from practical SBE experience | None |
| Domain behaviour | Defined the required SBE behaviour and corrected misunderstandings | Implemented the behaviour described |
| API design | Specified the desired properties, inspected generated APIs, rejected awkward designs | Proposed Rust representations and iterated on them |
| Generator | Specified and reviewed generated output; did not deeply review every generator path | Wrote nearly all `syn`/`quote` implementation |
| Tests | Chose important scenarios, inspected tests, ported the verification strategy from official SBE | Wrote and ran most test code |
| Performance | Required equal-work parity, personally ran and inspected benchmarks, rejected regressions | Wrote harnesses, ran them repeatedly, diagnosed and revised regressions |
| Documentation and samples | Supplied the intent and judged whether examples represented a usable API | Drafted most prose and sample code |
| Release decisions | Decided what was acceptable and retained the experimental warning | Provided recommendations, not authority |

### Design ideas that came from my SBE experience

The principal goals were not invented by a model:

- **Compile-time wire ordering.** I wanted it because ordering mistakes were a
  routine real-world SBE support problem.
- **Exact encoded length.** I wanted it because humans repeatedly miscalculate
  nested groups and variable data.
- **Closure-based groups.** I had already used this pattern manually to avoid
  borrow-checker pain.
- **Converters and DTOs.** I wanted application code to use types such as Rust
  `bool` and decimal values where direct wire access was unnecessary.
- **Checked and trusted wrapping.** I wanted a checked boundary for untrusted
  data and an official-codec-style fast path when the caller already knows the
  buffer is valid.
- **Unsafe only when justified.** I asked the agents to test particular unsafe
  optimisations, measure them, and remove them when they did not matter.

The models helped explore implementations. For example, I asked how Rust could
enforce ordering and discussed traits and type-state representations. A model
proposed concrete named stage structs. That was a good implementation of my
requirement and ultimately avoided a performance problem with generic stages.

For converters, I knew the user-facing capability I wanted but had not fully
designed the API. The models proposed several shapes. Some were wrong or ugly;
I showed the kind of calling code I wanted and refined the result.

This is the fairest summary: **the domain requirements and product judgment
were human-authored; the implementation and much of the Rust design exploration
were AI-assisted.**

## What I reviewed—and what I did not

This distinction is essential.

I did **not** perform a comprehensive, line-by-line human review of the
generator implementation. In particular, I did not deeply audit every
`syn`/`quote` path that constructs the emitted source.

I had written this sort of generator manually before. It is fiddly, but it is
also a task at which I had already seen LLMs perform well: provide a concrete
Rust shape and ask the model to emit that shape for every schema case. My
expected failure mode was not usually “the model cannot call `quote!`.” It was
“the model has misunderstood what the generated API or wire behaviour should
be.”

I therefore concentrated review on the generator's observable product:

- the generated Rust source;
- the public API and how it feels to use;
- whether staged types make illegal orderings unrepresentable;
- actual encoded bytes;
- official SBE fixtures and official `sbe-tool` output;
- exact-length results;
- error behaviour on malformed or incomplete buffers;
- allocations on intended zero-allocation paths; and
- performance relative to official generated Rust codecs.

I could often inspect generated Rust and immediately see an incorrect offset,
an awkward repeating-group API, an invalid stage transition, or unnecessary
complexity. That feedback was much faster and more useful than mentally
simulating a large code generator.

This means the project should **not** be described simply as “human-reviewed
code.” A more accurate description is:

> Human-directed, generated-output reviewed, and behaviourally verified, with
> generator internals substantially AI-authored and not exhaustively
> line-reviewed by a human.

The automated evidence reduces risk. It does not prove that an untested schema
shape cannot expose a generator defect.

## The actual working loop

The primary agent interface was **Claude Code CLI**, often connected to custom
models rather than a Claude model. Claude Code was useful because I wanted to
learn the tool used at work and because its endpoint configuration let me run
DeepSeek and GLM through the same agent workflow.

The work ran on a Mac mini at home. I connected with JetBrains **RustRover
Remote Development**, the Claude Code app, ordinary SSH, and sometimes an SSH
terminal from my phone. I used [Herdr](https://herdr.dev/), an agent-aware
persistent terminal multiplexer, so the Claude Code session survived
disconnects and I could reattach from another device.

This made long-running work convenient, but not unattended. There were two
review modes:

1. **Output review.** Let the agent finish a small change, generate codecs, run
   checks, and then inspect the resulting API and behaviour.
2. **Live intervention.** Watch the edits and reasoning, stop the agent when the
   approach was visibly wrong, explain the correction, and let it continue.

A typical successful loop was:

1. I describe one concrete behaviour or API improvement.
2. The agent adds or updates focused tests.
3. The agent changes the generator.
4. It regenerates and compiles the Rust output.
5. It runs relevant unit and parity tests.
6. For hot-path changes, it reruns the benchmark gate.
7. I inspect the generated API, bytes, failures, and benchmark result.
8. I give a specific correction and repeat.

The short feedback loop mattered more than one-shot model intelligence.

## What did not work

### Large parallel-agent plans

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

### Long one-shot requests

“Implement this and come back in an hour” stopped working as complexity grew.
The useful workflow required constant feedback. When the agent had freedom to
invent a user-facing API, it often produced something technically plausible but
ugly. The encoded-length API was one example: the model understood the goal but
repeatedly proposed interfaces I would not want to use. Once I supplied a
concrete calling shape, it could implement it.

### Assuming `CLAUDE.md` would enforce everything

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

### Human and agent editing at the same time

Manual edits made while the agent was working frequently damaged continuity.
The agent had already formed a model from older file contents. I had more
success telling it exactly what was wrong and letting it perform the edit than
silently changing the same code underneath it.

## Verification: why the tests matter so much

I will be blunt: if somebody publishes a substantial LLM-generated library with
very little meaningful test coverage, I assume the code is slop until shown
otherwise.

LLMs respond extremely well to objective verification. A failing test gives the
agent a bounded problem with an observable correct outcome. Without that
oracle, it can produce a confident implementation of a misunderstanding.

Early in development I sometimes asked for a numerical code-coverage target.
Later I cared less about the percentage than the breadth and independence of
the evidence.

The checked-in suite includes:

- schemas and cases ported from the upstream SBE project;
- decoding Java-produced official fixtures;
- exact comparisons of headers, fixed blocks, composites, groups, and variable
  data;
- checked-in Rust codecs generated by official `sbe-tool`;
- live dual encoding where `ergo-sbe` and official Rust codecs must produce
  byte-identical messages;
- a multi-schema official parity matrix;
- property-based round trips over scalars, arrays, groups, nested structures,
  and variable data;
- compile-fail proofs for illegal stage ordering and use-after-consume;
- exact encoded-length matrices compared with actual completed messages;
- malformed and truncated buffer tests;
- schema-version and acting-version tests;
- deterministic generated-source golden tests;
- zero-allocation checks using a counting allocator;
- upstream issue-regression schemas; and
- sample applications that exercise more complicated API combinations.

See:

- [`sbe_tool_wire_parity_test.rs`](tests/sbe_tool_wire_parity_test.rs)
- [`sbe_tool_multi_schema_wire_parity_test.rs`](tests/sbe_tool_multi_schema_wire_parity_test.rs)
- [`baseline_test.rs`](tests/baseline_test.rs)
- [`proptest_roundtrip.rs`](tests/proptest_roundtrip.rs)
- [`allocation_count_test.rs`](tests/allocation_count_test.rs)
- [`ordered_decoder_stages_test.rs`](tests/ordered_decoder_stages_test.rs)
- [`l3_consuming_stages_test.rs`](tests/l3_consuming_stages_test.rs)
- [`encoded_length_api_test.rs`](tests/encoded_length_api_test.rs)
- [`sbe_tool_reference/README.md`](tests/sbe_tool_reference/README.md)

The strongest compatibility test is differential, not self-referential. For
the same schema and logical values, the suite encodes with both `ergo-sbe` and
the official `sbe-tool` Rust generator and requires identical bytes. A library
that only decodes its own encoded output can be consistently wrong on both
sides; an independent reference makes that much harder.

One concrete AI failure involved offsets around variable data. A generator
change broke an existing test. The agent observed the failure, diagnosed it,
and fixed the implementation without me having to identify the exact line.
That was impressive, but the important fact is that the test existed. Without
it, the wrong offset could have compiled and looked plausible.

Tests are still not enough to remove the experimental warning. They cover the
cases we and upstream authors thought to encode. Production traffic eventually
finds assumptions that a controlled suite did not.

## Performance was part of correctness

For this library, an ergonomic abstraction that makes a maintained hot path
meaningfully slower than official generated code is a failed design.

I learned that painfully. The first compile-time ordering design used generic
type-state stages. I assumed it would be a zero-cost abstraction. Benchmarks
showed some encode paths were roughly **17% slower**. The model helped explain
that the generated generic chain was not being optimised as effectively as the
plain monomorphic code. We replaced it with concrete named stage structs,
retaining compile-time ordering without the measured generic tax.

After that, benchmark regression became part of the definition of done:

- run benchmarks after every material generated hot-path change;
- compare against checked-in official `sbe-tool` Rust output;
- ensure both arms do equal work;
- keep allocations and setup out of only one timed arm;
- rerun suspicious or borderline results;
- diagnose regressions before accepting a feature; and
- remove an abstraction if it cannot meet the gate.

The benchmark harness itself needed review. Earlier comparisons accidentally
included asymmetric allocation or different buffer traversal. One official
encode arm even risked overlapping header and body work. These were corrected
so the maintained comparison uses the same input or byte-identical output and
equivalent field work. The current methodology is documented in
[`BENCHMARKS.md`](BENCHMARKS.md).

Repeated benchmark execution likely explains part of the enormous token count.
An agent would implement a change, benchmark it, discover a regression, revise
the generator, and benchmark again.

## Unsafe code and the trust boundary

The unsafe strategy came from me, not from an LLM spontaneously “optimising”
the project.

Official-style codecs often separate checked setup from a trusted hot path. I
wanted:

- `try_wrap` for untrusted data, where buffer bounds and framing are checked;
- `wrap` for data whose contract has already been established, analogous to the
  official fast path; and
- no repeated dynamic bounds check for every constant schema offset after the
  required block length has already been proved.

I asked the agents to try several unsafe optimisations and measure them. Many
did not materially help, so I removed them. Unsafe is retained only where it is
required by the borrowing model or where a repeatable hot-path benchmark
justifies the additional audit burden.

One retained example came from the Cluster codec benchmark. Generated setters
using checked slice ranges produced regressions around **1.19×** and **1.28×**
relative to the reference path. After the wrapping boundary had already proved
the fixed block was present, using `get_unchecked_mut` for compile-time offsets
restored parity.

The policy is not “unsafe is fast.” It is:

> If the invariant can be stated and established, and the benchmark shows a
> meaningful need—or Rust borrowing requires the internal operation—unsafe may
> be justified. Otherwise use safe Rust.

The existence of a safe public API does not remove the need to audit internal
unsafe invariants. Users evaluating the crate should include those trust
boundaries in their review.

## Tools, models, and what each contributed

### Agent harness

The main harness was Claude Code CLI. This does not mean Claude wrote most of
the code. Claude Code was the interface; custom endpoints supplied other
models.

### DeepSeek

DeepSeek V4 Flash and V4 Pro performed most of the implementation work. I
started using DeepSeek because I repeatedly exhausted other subscription or
coding-plan limits. I expected it to be a fallback. It became the workhorse.

There was no task where DeepSeek failed and I then had to escalate the same
problem to Opus, ChatGPT, or Grok to rescue the implementation. That surprised
me. It was possible because:

- I knew the domain;
- I normally knew what the correct result should look like;
- I could provide concrete API examples;
- the output was easy for me to inspect; and
- tests and benchmarks supplied tight feedback.

That is not evidence that DeepSeek is universally equivalent to a frontier
model. It is evidence that, for a tightly specified mechanical implementation
task under constant domain-expert supervision, a cheaper workhorse can be more
useful than buying maximum intelligence for every token.

My practical description is:

> DeepSeek V4 Pro delivered roughly Sonnet-class usefulness for this workflow.
> I would not treat that as proof that it replaces Opus for ambiguous,
> autonomous, long-horizon work where the developer does not know the answer.

DeepSeek's own model information is available from its
[official V4 documentation and reports](https://huggingface.co/deepseek-ai/DeepSeek-V4-Pro).
Cross-vendor benchmark numbers use different harnesses and should not be read
as controlled equivalence.

### GLM

I used GLM-5.2 and GLM-4.7 substantially, particularly early in the project.
The 30-day screenshot below records approximately 3.28 billion GLM tokens,
including 2.65 billion on GLM-5.2.

![GLM model usage over 30 days](https://raw.githubusercontent.com/mimran1980/ergon/first_cut/assets/ai-assistance/glm-30-day-model-usage.jpg)

I bought the highest coding-plan tier I was using—about **$114**—and still hit
limits during the intensive development schedule.

### Claude, OpenAI, and Grok

I also paid for Claude, OpenAI, and Grok access. I used them mainly for
additional reviews and experiments, not as indispensable implementation
engines. Their reviews were sometimes a little stronger, but generally raised
the same categories of issue that DeepSeek reviews found. I cannot point to a
frontier-model finding without which this project could not have been
completed.

Cross-model review is useful, but agreement among models is not independent
proof. They can share training patterns and repeat the same plausible
misunderstanding. Official bytes and behavioural tests are stronger evidence.

## Long context and why caching changed the economics

Both DeepSeek V4 Flash and V4 Pro exposed a one-million-token context window. I
usually kept one long session and reused it, compacting only once or twice.

Subjectively, DeepSeek became much more useful when it retained the accumulated
project history: design decisions, examples, mistakes, failed benchmarks, and
my corrections. I did not notice a significant drop immediately after its
occasional compactions. In separate Sonnet usage with a smaller context, I have
sometimes noticed forgotten details after compaction.

Those are personal observations, not controlled experiments. They do,
however, explain the usage shape.

My informal developer's mental model was “roughly N-squared in requests”: as a
session grows, every new exchange carries an ever larger history, so reaching a
very large context requires paying for all the preceding turns along the way.
That is not a literal description of every provider's inference
implementation. KV caching, prompt caching, compaction, and model architecture
change the compute, and I do not know whether a provider stores a particular
cache in RAM, on disk, or elsewhere. It was nevertheless a useful intuition
for understanding why repeated context, rather than only the latest answer,
dominated the token ledger.

In an agent conversation, each new turn includes a large amount of prior
context. Providers can serve repeated prefixes from a prompt cache, but cache
hits are still billable tokens. The headline uncached input and output prices
therefore do not describe the economics of a long coding session. Cache-hit
price, cache-write policy, context thresholds, and long-context multipliers
matter enormously.

I originally thought about models mainly through benchmark rank and ordinary
input/output price. This project changed that. DeepSeek's cache-hit price was
so low that I could keep a large, useful context alive without repeatedly
hitting a subscription ceiling.

One correction is important: the sampled screenshots show roughly **99% of
token volume** as cache hits. That does **not** mean 99% of dollar cost came
from cache hits. Because cache hits are heavily discounted, uncached input and
output contribute a much larger share of cost than their token percentages.

## Observed usage and actual spend

The DeepSeek dashboard for the displayed 30-day window shows:

- **10,522,859,893 tokens**
- **47,668 API requests**
- **$77.39**

![DeepSeek 30-day usage summary](https://raw.githubusercontent.com/mimran1980/ergon/first_cut/assets/ai-assistance/deepseek-30-day-summary.jpg)

The model split shown by the dashboard was:

- DeepSeek V4 Flash: 3,392,304,915 tokens across 29,015 requests
- DeepSeek V4 Pro: 7,130,554,978 tokens across 18,653 requests

![DeepSeek V4 Flash and V4 Pro usage, upper dashboard](https://raw.githubusercontent.com/mimran1980/ergon/first_cut/assets/ai-assistance/deepseek-v4-model-usage-upper.jpg)

![DeepSeek V4 Flash and V4 Pro usage, lower dashboard](https://raw.githubusercontent.com/mimran1980/ergon/first_cut/assets/ai-assistance/deepseek-v4-model-usage-lower.jpg)

My rounded estimate for the full project period is approximately **14 billion
tokens** across providers:

- about 10.5 billion shown in the DeepSeek 30-day window;
- about 3.28 billion shown in the GLM 30-day window; and
- smaller usage on Claude, OpenAI, and Grok that I did not attempt to reconcile
  into a billing-grade ledger.

The 14-billion figure is therefore an honest order-of-magnitude estimate, not
an audited total.

My approximate cash spend was:

| Provider | Approximate spend |
|---|---:|
| DeepSeek pay as you go | $90 |
| GLM coding plan | $114 |
| OpenAI subscription | $20 |
| Claude subscription | $20 |
| Grok subscription | $30 |
| **Approximate total** | **$274** |

The DeepSeek dashboard's $77.39 is for its displayed 30-day window. My $90
figure is the rounded project-period spend. They describe different scopes and
should not be forced to match.

Subscription spend is also not directly comparable with enterprise API
pay-as-you-go pricing. The next section normalises the observed workload for
that comparison.

## The cache sample used for the cost comparison

The dashboard screenshots expose the input-cache-hit, input-cache-miss, and
output split for three selected days:

![DeepSeek V4 Flash cache split on 7 July 2026](https://raw.githubusercontent.com/mimran1980/ergon/first_cut/assets/ai-assistance/deepseek-v4-flash-cache-2026-07-07.jpg)

![DeepSeek V4 Pro cache split on 9 July 2026](https://raw.githubusercontent.com/mimran1980/ergon/first_cut/assets/ai-assistance/deepseek-v4-pro-cache-2026-07-09.jpg)

![DeepSeek V4 Pro cache split on 24 July 2026](https://raw.githubusercontent.com/mimran1980/ergon/first_cut/assets/ai-assistance/deepseek-v4-pro-cache-2026-07-24.jpg)

Combined, those samples contain:

| Token category | Sample tokens | Sample share |
|---|---:|---:|
| Input cache hits | 2,225,207,680 | 98.6602% |
| Input cache misses | 23,305,848 | 1.0333% |
| Output | 6,911,504 | 0.3064% |
| **Total** | **2,255,425,032** | **100%** |

Scaling that exact mix to 14 billion tokens gives:

| Token category | Normalised millions of tokens |
|---|---:|
| Input cache hits | 13,812.433168 MTok |
| Input cache misses / cache writes | 144.665359 MTok |
| Output | 42.901473 MTok |
| **Total** | **14,000 MTok** |

The cost formula is:

```text
cost =
    cache-hit MTok × cache-hit price
  + cache-miss MTok × miss/write price
  + output MTok × output price
```

This calculation includes all three categories. It does not price 14 billion
tokens as though they were all cheap cache hits.

## Normalised pay-as-you-go comparison

The following is a historical estimate using public prices checked on **26 July
2026**:

- [DeepSeek API pricing](https://api-docs.deepseek.com/quick_start/pricing/)
- [Z.AI / GLM pricing](https://docs.z.ai/guides/overview/pricing)
- [Claude API pricing](https://platform.claude.com/docs/en/about-claude/pricing)
- [OpenAI GPT-5.5 pricing](https://developers.openai.com/api/docs/models/gpt-5.5)
- [OpenAI GPT-5.6 Sol pricing](https://developers.openai.com/api/docs/models/gpt-5.6-sol)
- [xAI / Grok pricing](https://docs.x.ai/developers/pricing)

Assumptions:

- exactly 14 billion billed tokens for every model;
- exactly the sampled cache-hit/miss/output mix above;
- zero model-specific increase or reduction in token usage;
- no 30% tokenizer adjustment for newer Claude models, even though Anthropic
  documents that their newer tokenizer may produce approximately 30% more
  tokens for the same text;
- standard synchronous API rates, not batch, fast mode, regional residency, or
  negotiated enterprise discounts;
- Claude cache misses treated as five-minute cache writes;
- GPT-5.6 Sol cache misses treated as cache writes at its documented 1.25×
  input rate;
- GPT-5.5 misses treated as normal uncached input;
- no separate tool-call, hosted-agent, storage, tax, or network charges; and
- “medium” reasoning effort does not alter the per-token rate; this table holds
  token counts constant rather than guessing different reasoning-token usage.

### At standard or short-context rates

| Model | Cache-hit $/MTok | Miss/write $/MTok | Output $/MTok | Estimated total |
|---|---:|---:|---:|---:|
| DeepSeek V4 Flash | $0.0028 | $0.14 | $0.28 | **$70.94** |
| DeepSeek V4 Pro | $0.003625 | $0.435 | $0.87 | **$150.32** |
| Claude Sonnet 5 promotional price through 31 Aug 2026 | $0.20 | $2.50 | $10.00 | **$3,553.16** |
| GLM-5.2 | $0.26 | $1.40 | $4.40 | **$3,982.53** |
| Grok 4.5 below 200K context | $0.30 | $2.00 | $6.00 | **$4,690.47** |
| Claude Sonnet 4.6 | $0.30 | $3.75 | $15.00 | **$5,329.75** |
| Claude Sonnet 5 standard price from 1 Sep 2026 | $0.30 | $3.75 | $15.00 | **$5,329.75** |
| Claude Opus 4.8 | $0.50 | $6.25 | $25.00 | **$8,882.91** |
| Claude Opus 5 | $0.50 | $6.25 | $25.00 | **$8,882.91** |
| OpenAI GPT-5.5 medium | $0.50 | $5.00 | $30.00 | **$8,916.59** |
| OpenAI GPT-5.6 Sol medium | $0.50 | $6.25 | $30.00 | **$9,097.42** |

Sonnet 4.6 and Sonnet 5's post-promotion line are equal because that is what the
official first-party Claude pricing page publishes. Opus 4.8 and Opus 5 are
also equal on that page. The calculation does not invent a quality or
token-volume premium where the requested assumption is a 0% usage increase.

### If requests actually use approximately one million tokens of context

Long-context pricing changes the comparison:

| Model | Published long-context treatment | Estimated total |
|---|---|---:|
| DeepSeek V4 Flash | 1M context; same published token rates | **$70.94** |
| DeepSeek V4 Pro | 1M context; same published token rates | **$150.32** |
| Claude Sonnet 5 promotional | Full 1M at standard rates | **$3,553.16** |
| GLM-5.2 | 1M context; same published token rates | **$3,982.53** |
| Claude Sonnet 4.6 | Full 1M at standard rates | **$5,329.75** |
| Claude Sonnet 5 standard | Full 1M at standard rates | **$5,329.75** |
| Claude Opus 4.8 | Full 1M at standard rates | **$8,882.91** |
| Claude Opus 5 | Full 1M at standard rates | **$8,882.91** |
| Grok 4.5 | Cannot accept 1M; maximum 500K. At ≥200K all rates double | **$9,380.94 at 500K** |
| OpenAI GPT-5.5 medium | Above 272K: 2× input and 1.5× output | **$17,189.65** |
| OpenAI GPT-5.6 Sol medium | Above 272K: 2× input, 1.5× output, 1.25× cache writes | **$17,551.32** |

Claude's official page states that Claude 4.6 and later include the full
one-million-token context window at standard rates. OpenAI documents the
greater-than-272K multiplier for GPT-5.5 and GPT-5.6 Sol. Grok 4.5 has a 500K
maximum and doubles all token rates from 200K.

These are counterfactual estimates, not invoices. Different models can
tokenise the same conversation differently, emit different amounts of
reasoning, call different tools, finish in different numbers of turns, and
achieve different cache-hit ratios. Holding usage fixed is useful for
isolating price; it does not predict the total cost of rerunning the project
with another model.

As a sanity check, scaling the observed DeepSeek dashboard window
($77.39 / 10.5229B tokens) to 14B gives roughly **$103**. Scaling my rounded
project-period estimate ($90 / 10.5B) gives roughly **$120**. Those sit between
the all-Flash and all-Pro modelled figures because the real workload mixed
models and the three selected cache screenshots are not a billing-complete
sample of every day.

## What I learned about model “intelligence”

Before this project I placed more weight on using the best-ranked model. My
conclusion is now more conditional.

If I do not understand a problem, model intelligence matters greatly. If I am
the domain expert, know the required output, can inspect it quickly, and have
objective tests, I may get more value from an affordable model that I can use
continuously.

For this project, intelligence was not the scarce resource. The scarce
resources were:

- enough context to retain the evolving design;
- enough affordable requests to sustain constant feedback;
- tests that made correctness observable;
- benchmark loops that made performance regressions observable; and
- my attention for API and domain review.

That is why DeepSeek was so effective here. It does not imply that the same
choice is correct for a developer who is asking the model to discover SBE
semantics on their behalf.

For somebody learning agentic coding on personal projects, my practical advice
would now be to start with a small amount of inexpensive pay-as-you-go credit.
It is difficult to learn sustained feedback loops when a premium subscription
repeatedly stops the session at its usage limit.

## The personal experience: pride, enjoyment, and review fatigue

I have written another open-source project almost entirely by hand. I felt a
different connection to that code. Writing software is enjoyable; learning the
small implementation details and keyboard shortcuts is part of the craft.

I do not feel the same ownership of this generator's individual lines because
I did not write them. I am concerned that heavy LLM use can cause skill
atrophy. It also changes the job from implementation to review.

That change was harder than I expected. I have historically been a very fast
developer and often spent more time writing new code than reviewing other
people's work. Reading code quickly to understand an API is not the same skill
as auditing a huge generated diff line by line. Careful review is slower, more
tedious, and easier to overwhelm. LLMs can generate code far faster than a
person can honestly review it.

This project remained enjoyable because I was still solving a problem I care
about. I spent a great deal of time thinking about SBE, Rust API design,
verification, and performance. The LLM did much of the repetitive generator
work that I already knew how to do and did not particularly want to spend six
months repeating.

So the pride is attached differently:

- less attachment to the implementation;
- more satisfaction in the API and product;
- genuine pleasure that something I wanted throughout my working life now
  exists; and
- less reluctance to throw away experiments that do not work.

That last point helped with the Aeron Cluster code. The Cluster crate was
initially a realistic consumer and test bed for `ergo-sbe`. Samples later
became a better way to exercise complicated API shapes. The Cluster work
remains less mature and more disposable. My priority is to make `ergo-sbe`
right before treating the Cluster client as a finished product.

## Why this crate is still experimental

The experimental label is deliberate.

I would be angry if a developer introduced an unproven codec into a financial
system merely because it had many unit tests. If I apply that standard to
somebody else's library, I must apply it to mine.

One thing I valued in the Java ecosystem was that mature projects often had
visible institutional signals—long histories, broad production use, or
governance under organisations such as Apache. The quality of an arbitrary Rust
crate can be much harder to infer from its presentation. That makes explicit
evidence and an honest maturity label more important, not less.

The suite is extensive. The official byte parity is meaningful. The benchmark
gate is meaningful. None of that is the same as sustained production use across
independent firms, schemas, traffic patterns, deployment environments, and
upgrade cycles.

I will become comfortable removing the warning when there is a sufficient base
of real users who tell me:

- they are using it in production;
- which features and schema shapes they use;
- how they compared it with their existing codecs;
- what volumes and environments it has survived;
- how schema evolution behaved; and
- what defects or operational surprises they found.

The reports I most want are already listed in the
[`ergo-sbe` README](README.md): multi-schema streams, DTO use, exact sizing with
Aeron/IPC claims, nested or ragged books, and mixed acting versions.

Until that evidence exists, treat the crate as 0.x experimental software. Pin
versions and perform your own migration testing.

## What a prospective production user should verify

Do not rely on this narrative alone.

At minimum:

1. Generate codecs for your actual schemas.
2. Encode the same logical messages with your existing official SBE tooling and
   `ergo-sbe`; compare exact bytes.
3. Cross-decode in both directions.
4. Include empty, maximum-sized, nested, ragged, and variable-data cases.
5. Exercise every acting version and schema-evolution path you expect to see.
6. Test malformed and truncated frames at the trust boundary.
7. Verify exact buffer lengths before integrating with `try_claim` or another
   zero-copy publication API.
8. Benchmark your real hot fields and message shapes, not only the Car example.
9. Audit the internal unsafe assumptions relevant to your use of checked and
   trusted wrapping.
10. Run soak tests under real traffic and deployment conditions.

If you are not already comfortable explaining SBE block lengths, group
dimensions, variable-data prefixes, acting versions, and positional ordering,
do not use an LLM-generated explanation as your only review.

## AI-assisted contributions

I am not applying a simplistic ban on AI-assisted pull requests, but I will not
blindly accept them.

Large AI-generated PRs are difficult to review. More importantly, a contributor
can ask a model to “fix” a protocol bug without understanding why the result is
correct. Passing compilation is not enough.

For a generator or wire-format change, the contributor must understand the
relevant SBE behaviour and be able to explain it. A focused issue containing:

- a real schema;
- a minimal reproduction;
- expected official bytes;
- a failing behavioural or parity test; and
- an explanation of the protocol invariant

may be more valuable than a large implementation patch. In many cases I would
rather take that evidence and implement the change through the controlled
workflow used for this repository.

AI assistance is not disqualifying. Lack of domain understanding is.

## Final assessment

This project was a particularly favourable case for AI-assisted development:

- the maintainer was a domain expert;
- the desired behaviour was unusually concrete;
- the tedious part was mechanical code generation;
- the generated product was directly inspectable;
- an independent official implementation existed;
- wire bytes supplied a hard oracle;
- benchmarks constrained abstraction cost; and
- constant feedback was possible.

Even in that favourable case, it consumed about a month of intense work,
roughly 14 billion estimated tokens, repeated cleanup, extensive tests, and
continuous human judgment. The fashionable version—write a specification,
dispatch many agents, and return to a finished library—did not survive beyond
the early greenfield stage.

I would use this development method again for a personal project with similar
properties, and I would likely use DeepSeek again as the workhorse. I would not
generalise the result into “LLMs can safely build any library” or “the cheapest
model is always enough.”

The result is a crate whose API I genuinely wish I had during the last decade
of SBE work. I am proud of that result. I am also being explicit that most of
its implementation was written by models, that the generator internals did not
receive exhaustive human line review, and that production maturity still has
to be earned.

If you use `ergo-sbe` seriously, please report the schema shapes and features
you exercise, including failures. Real-world evidence is what will make this
project trustworthy—not another paragraph claiming that AI-generated code is
either magically perfect or automatically worthless.

## Maintaining this disclosure

Update this file when any of the following materially changes:

- the primary models or agent harnesses;
- the human review boundary;
- the official parity or benchmark methodology;
- the unsafe trust boundary;
- production adoption;
- the experimental status; or
- a pricing table that is intentionally refreshed for a later historical
  snapshot.

Do not silently rewrite old usage or cost figures to current prices. Add a new
dated comparison so that the historical record remains intelligible.
