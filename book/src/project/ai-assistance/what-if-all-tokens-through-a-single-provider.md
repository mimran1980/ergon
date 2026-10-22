# What-if: all tokens through a single provider

This prices the cumulative table's measured buckets on one Anthropic model.
Rates are the public Claude API card checked on 2026-10-21
([model pricing](https://platform.claude.com/docs/en/about-claude/pricing)).
The result is a counterfactual. It is not an invoice, and it is not the
~$261 out-of-pocket spend.

## Measured mix

The price uses the column sums. Those columns add to **76,098,076,915**
tokens. The published total cell is **76,095,756,856**, which is 2,320,059
lower, because `deepseek-v4-flash`'s Total Tokens cell is short of its own
columns by that amount. Cache writes in this ledger are entirely Claude
Code rows. Every other provider recorded cache reads and left cache create
blank.

| Category | Tokens | Share of 76,098,076,915 |
|---|---:|---:|
| Uncached input | 707,426,440 | 0.93% |
| Output | 200,490,665 | 0.26% |
| Reasoning output | 6,118,100 | 0.01% |
| Cache writes | 711,944,723 | 0.94% |
| Cache reads | 74,472,096,987 | 97.86% |
| **Column sum** | **76,098,076,915** | **100%** |

Cache reads plus cache writes are 98.80% of that sum. Output including
reasoning is 206,608,765 tokens (0.27%).

## One Anthropic model for every measured token

Assumptions, all stated so the arithmetic can be repeated:

- uncached input at the model's input price;
- cache writes at the 5-minute cache-write price (1.25× input), which is the
  write Claude Code records;
- cache reads at the model's cache-hit price;
- output and reasoning output both at the output price;
- standard global API rates: no fast mode, no batch discount, no US-only
  1.1× data-residency multiplier;
- token counts held fixed. Claude 4.7 and later, including Opus 5.5 and
  Sonnet 5.5, use a tokenizer Anthropic says produces about 30% more tokens
  for the same text. That uplift is not applied here;
- Haiku 5.5 bills a higher card when the prompt is over 100,000 tokens,
  counting input, cache writes, and cache reads toward that length. These
  sessions are long, so the over-100k line is the matching card. The
  under-100k line is the short-prompt card only.

Sonnet 5's $2 / $10 per million input/output tokens, announced as
introductory pricing through 31 August 2026, is now the standard price. The
previously scheduled rise to $3 / $15 on 1 September 2026 did not happen.
Opus 5.5 and Sonnet 5.5 cache hits are 0.05× input ($0.20 and $0.10 per
million). Other Claude models on this list use 0.1×.

| Model | Input $/MTok | 5m write $/MTok | Cache hit $/MTok | Output $/MTok | **Total** |
|---|---:|---:|---:|---:|---:|
| Opus 5.5 | $4 | $5 | $0.20 | $20 | **$25,416.02** |
| Sonnet 5.5 | $2 | $2.50 | $0.10 | $10 | **$12,708.01** |
| Opus 5 | $5 | $6.25 | $0.50 | $25 | **$50,388.05** |
| Sonnet 5 | $2 | $2.50 | $0.20 | $10 | **$20,155.22** |
| Haiku 4.5 | $1 | $1.25 | $0.10 | $5 | **$10,077.61** |
| Haiku 5.5, prompts over 100k tokens | $0.50 | $0.625 | $0.05 | $2.50 | **$5,038.81** |
| Haiku 5.5, prompts up to 100k tokens | $0.10 | $0.125 | $0.01 | $0.50 | **$1,007.76** |

Where each dollar comes from, at the 5-minute write price:

| Model | Uncached input | Cache writes | Cache reads | Output incl. reasoning | **Total** |
|---|---:|---:|---:|---:|---:|
| Opus 5.5 | $2,829.71 | $3,559.72 | $14,894.42 | $4,132.18 | **$25,416.02** |
| Sonnet 5.5 | $1,414.85 | $1,779.86 | $7,447.21 | $2,066.09 | **$12,708.01** |
| Opus 5 | $3,537.13 | $4,449.65 | $37,236.05 | $5,165.22 | **$50,388.05** |
| Sonnet 5 | $1,414.85 | $1,779.86 | $14,894.42 | $2,066.09 | **$20,155.22** |
| Haiku 4.5 | $707.43 | $889.93 | $7,447.21 | $1,033.04 | **$10,077.61** |
| Haiku 5.5, prompts over 100k tokens | $353.71 | $444.97 | $3,723.60 | $516.52 | **$5,038.81** |

One-hour cache writes are 2× input. Only the write column changes. Opus 5.5
becomes **$27,551.86** and Sonnet 5.5 becomes **$13,775.93**.

The ledger's own mixed-provider API total is **$19,731.14**. Sonnet 5.5
comes out lower because 97.86% of the measured tokens are cache reads and
its cache hit is $0.10 per million. Opus 5.5 cache hits are $0.20 per
million, so the same reads cost $14,894.42 and the all-Opus bill is
$25,416.02. Opus 5 cache hits are $0.50 per million, which is why that line
is $50,388.05. Out of pocket remains about **$261**.

> **Caveats**
>
> 1. **Cache writes exist only on the Claude rows.** DeepSeek, GLM, Codex,
>    and Grok recorded cache reads and no cache-create column. Pricing those
>    reads as Anthropic hits does not add the write that would have created
>    the cache entry. A real all-Anthropic run would have paid those writes.
>    They are not in this ledger, so they are not in the totals above.
>
> 2. **The cache-read share dominates the bill.** 74.47 billion of the 76.10
>    billion column-sum tokens are cache reads. A project with shorter
>    sessions would rank these models differently, because input and output
>    prices would matter more than the cache-hit price.
>
> 3. **Token counts are held fixed.** Another model can tokenise the same
>    transcript differently, emit different output and reasoning, and keep a
>    different cache-hit ratio. The Claude 4.7+ tokenizer note above is the
>    documented case of that. These figures isolate the rate card.

The [14-billion-token comparison](normalised-pay-as-you-go-comparison.md#normalised-pay-as-you-go-comparison)
further down uses a July 2026 sample and the prices checked on 26 July 2026.
It is not this ledger.

Choose the path that matches why you are here:

- **Evaluating the crate:** read [what I reviewed](what-i-reviewedand-what-i-did-not.md#what-i-reviewedand-what-i-did-not),
  [verification](verification-why-the-tests-matter-so-much.md#verification-why-the-tests-matter-so-much),
  [unsafe code and the trust boundary](unsafe-code-and-the-trust-boundary.md#unsafe-code-and-the-trust-boundary),
  [why it is experimental](why-this-crate-is-still-experimental.md#why-this-crate-is-still-experimental), and the
  [production checklist](what-a-prospective-production-user-should-verify.md#what-a-prospective-production-user-should-verify).
- **Learning from the development method:** read the
  [working loop](the-actual-working-loop.md#the-actual-working-loop),
  [project phases](how-the-project-evolved.md#how-the-project-evolved),
  [Git history](what-the-git-history-shows.md#what-the-git-history-shows),
  [failed approaches](what-did-not-work.md#what-did-not-work), and the
  [practical playbook](a-practical-playbook-for-other-developers.md#a-practical-playbook-for-other-developers).
- **Understanding model economics:** read the
  [cumulative token usage](cumulative-token-usage-since-2026-06-28.md#cumulative-token-usage-since-2026-06-28),
  [single-provider what-if](#what-if-all-tokens-through-a-single-provider),
  [O(n²) and caching explanation](long-context-the-on2-mental-model-and-caching.md),
  [observed usage and spend](observed-usage-and-actual-spend.md#observed-usage-and-actual-spend),
  [cache sample](the-cache-sample-used-for-the-cost-comparison.md#the-cache-sample-used-for-the-cost-comparison), and
  [normalized pay-as-you-go comparison](normalised-pay-as-you-go-comparison.md#normalised-pay-as-you-go-comparison).
- **Contributing:** read the
  [AI-assisted contribution policy](ai-assisted-contributions.md#ai-assisted-contributions).
