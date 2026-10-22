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

## Read this first

This article is long because “AI-assisted” is too vague to be useful on its
own. If you only have a minute, these are the facts that should determine
whether you continue evaluating the crate:

| Question | Short answer |
|---|---|
| How much was written by AI? | Most of the implementation, tests, benchmarks, samples, and documentation. I did very little direct coding. |
| What was the human contribution? | More than a decade of SBE experience, the product requirements, API judgment, domain corrections, acceptance criteria, and release decisions. |
| What received human review? | Primarily the generated Rust API and source, encoded bytes, test failures, and benchmark results—not an exhaustive line-by-line audit of the `syn`/`quote` generator internals. |
| What independently constrains the output? | Official `sbe-tool` byte-for-byte comparisons, Java-produced fixtures, upstream schemas, compile-fail proofs, property tests, exact-length checks, allocation tests, and performance gates. |
| Is it production-proven? | No. It remains experimental 0.x software. Production users should validate their own schemas, versions, message shapes, and traffic. |
| What did development consume? | Roughly one month of intensive work. The initial `0.1.0` release used approximately **14 billion tokens** (estimated from provider dashboards). The combined ledger on 2026-10-21 is **76.10 billion tokens** (76,095,756,856): the other machine's `ccusage` high-water mark plus this MacBook's local `ccusage`. A run on either machine alone will not match that total (see [cumulative token usage](ai-assistance/cumulative-token-usage-since-2026-06-28.md#cumulative-token-usage-since-2026-06-28)). |
| What did it cost? | **~$261 actual out-of-pocket** (DeepSeek PAYG $77.39 + GLM plan $114 + subscriptions $70). The combined ledger's own API-rate total is **$19,731.14**. The same measured mix, priced on one Anthropic model at the public card checked on 2026-10-21 with 5-minute cache writes, is **$12,708.01 on Sonnet 5.5** and **$25,416.02 on Opus 5.5**. With my work Claude Enterprise subscription the Claude portion would be covered by the seat licence rather than per-token billing, so the real cost at work would be lower still. The [single-provider what-if](ai-assistance/what-if-all-tokens-through-a-single-provider.md#what-if-all-tokens-through-a-single-provider) shows the input, output, and cache arithmetic. |
| Which model did most of the work? | DeepSeek: V4 Flash handled much of the early UltraMode/subagent work; the later sequential development stayed primarily on V4 Pro. |


## Contents

1. [Cumulative token usage (since 2026-06-28)](ai-assistance/cumulative-token-usage-since-2026-06-28.md#cumulative-token-usage-since-2026-06-28)
2. [What-if: all tokens through a single provider](ai-assistance/what-if-all-tokens-through-a-single-provider.md#what-if-all-tokens-through-a-single-provider)
3. [Why this page exists](ai-assistance/why-this-page-exists.md#why-this-page-exists)
4. [What I was trying to build](ai-assistance/what-i-was-trying-to-build.md#what-i-was-trying-to-build)
5. [Why I chose it as an AI experiment](ai-assistance/why-i-chose-it-as-an-ai-experiment.md#why-i-chose-it-as-an-ai-experiment)
6. [Authorship](ai-assistance/authorship-what-was-mine-and-what-was-generated.md#authorship-what-was-mine-and-what-was-generated)
7. [What I reviewed—and what I did not](ai-assistance/what-i-reviewedand-what-i-did-not.md#what-i-reviewedand-what-i-did-not)
8. [The actual working loop](ai-assistance/the-actual-working-loop.md#the-actual-working-loop)
9. [How the project evolved](ai-assistance/how-the-project-evolved.md#how-the-project-evolved)
10. [What the Git history shows](ai-assistance/what-the-git-history-shows.md#what-the-git-history-shows)
11. [What did not work](ai-assistance/what-did-not-work.md#what-did-not-work)
12. [Verification](ai-assistance/verification-why-the-tests-matter-so-much.md#verification-why-the-tests-matter-so-much)
13. [Performance](ai-assistance/performance-was-part-of-correctness.md#performance-was-part-of-correctness)
14. [Unsafe code and the trust boundary](ai-assistance/unsafe-code-and-the-trust-boundary.md#unsafe-code-and-the-trust-boundary)
15. [Tools and models](ai-assistance/tools-models-and-what-each-contributed.md#tools-models-and-what-each-contributed)
16. [Long context, O(n²), and caching](ai-assistance/long-context-the-on2-mental-model-and-caching.md)
17. [Observed usage and spend](ai-assistance/observed-usage-and-actual-spend.md#observed-usage-and-actual-spend)
18. [Pay-as-you-go comparison](ai-assistance/normalised-pay-as-you-go-comparison.md#normalised-pay-as-you-go-comparison)
19. [A practical playbook](ai-assistance/a-practical-playbook-for-other-developers.md#a-practical-playbook-for-other-developers)
20. [The personal experience](ai-assistance/the-personal-experience-pride-enjoyment-and-review-fatigue.md#the-personal-experience-pride-enjoyment-and-review-fatigue)
21. [Why the crate is experimental](ai-assistance/why-this-crate-is-still-experimental.md#why-this-crate-is-still-experimental)
22. [What production users should verify](ai-assistance/what-a-prospective-production-user-should-verify.md#what-a-prospective-production-user-should-verify)
23. [AI-assisted contributions](ai-assistance/ai-assisted-contributions.md#ai-assisted-contributions)
24. [Final assessment](ai-assistance/final-assessment.md#final-assessment)
