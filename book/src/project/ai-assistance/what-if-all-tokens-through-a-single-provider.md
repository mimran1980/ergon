# What-if: all tokens through a single provider

This prices the cumulative table's measured buckets on one Anthropic model.
Rates are the public Claude API card checked on 2026-10-21
([model pricing](https://platform.claude.com/docs/en/about-claude/pricing)).
The result is a counterfactual. It is not an invoice, and it is not the
~$261 out-of-pocket spend.

## Measured mix

The price uses the column sums. Those columns add to 86,982,101,679
tokens. The published total cell is 86,979,781,620, which is 2,320,059
lower, because `deepseek-v4-flash`'s Total Tokens cell is short of its own
columns by that amount. Cache writes in this ledger are entirely Claude
Code rows. Every other provider recorded cache reads and left cache create
blank.

| Category | Tokens | Share of 86,982,101,679 |
|---|---:|---:|
| Uncached input | 768,664,792 | 0.88% |
| Output | 241,670,698 | 0.28% |
| Reasoning output | 6,118,100 | 0.01% |
| Cache writes | 1,021,205,651 | 1.17% |
| Cache reads | 84,944,442,438 | 97.66% |
| **Column sum** | 86,982,101,679 | 100% |

Cache reads plus cache writes are 98.83% of that sum. Output including
reasoning is 247,788,798 tokens (0.28%).

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

| Model | Input $/MTok | 5m write $/MTok | Cache hit $/MTok | Output $/MTok | Total |
|---|---:|---:|---:|---:|---:|
| Opus 5.5 | $4 | $5 | $0.20 | $20 | $30,125.35 |
| Sonnet 5.5 | $2 | $2.50 | $0.10 | $10 | $15,062.68 |
| Opus 5 | $5 | $6.25 | $0.50 | $25 | $58,892.80 |
| Sonnet 5 | $2 | $2.50 | $0.20 | $10 | $23,557.12 |
| Haiku 4.5 | $1 | $1.25 | $0.10 | $5 | $11,778.56 |
| Haiku 5.5, prompts over 100k tokens | $0.50 | $0.625 | $0.05 | $2.50 | $5,889.28 |
| Haiku 5.5, prompts up to 100k tokens | $0.10 | $0.125 | $0.01 | $0.50 | $1,177.86 |

Where each dollar comes from, at the 5-minute write price:

| Model | Uncached input | Cache writes | Cache reads | Output incl. reasoning | Total |
|---|---:|---:|---:|---:|---:|
| Opus 5.5 | $3,074.66 | $5,106.03 | $16,988.89 | $4,955.78 | $30,125.35 |
| Sonnet 5.5 | $1,537.33 | $2,553.01 | $8,494.44 | $2,477.89 | $15,062.68 |
| Opus 5 | $3,843.32 | $6,382.54 | $42,472.22 | $6,194.72 | $58,892.80 |
| Sonnet 5 | $1,537.33 | $2,553.01 | $16,988.89 | $2,477.89 | $23,557.12 |
| Haiku 4.5 | $768.66 | $1,276.51 | $8,494.44 | $1,238.94 | $11,778.56 |
| Haiku 5.5, prompts over 100k tokens | $384.33 | $638.25 | $4,247.22 | $619.47 | $5,889.28 |

One-hour cache writes are 2× input. Only the write column changes. Opus 5.5
becomes $33,188.97 and Sonnet 5.5 becomes $16,594.48.

The ledger's own mixed-provider API total is $24,273.94. Sonnet 5.5
comes out lower because 97.66% of the measured tokens are cache reads and
its cache hit is $0.10 per million. Opus 5.5 cache hits are $0.20 per
million, so the same reads cost $16,988.89 and the all-Opus bill is
$30,125.35. Opus 5 cache hits are $0.50 per million, which is why that line
is $58,892.80. Out of pocket remains about $261.

> **Caveats**
>
> 1. Cache writes exist only on the Claude rows. DeepSeek, GLM, Codex,
>    and Grok recorded cache reads and no cache-create column. Pricing those
>    reads as Anthropic hits does not add the write that would have created
>    the cache entry. A real all-Anthropic run would have paid those writes.
>    They are not in this ledger, so they are not in the totals above.
>
> 2. The cache-read share dominates the bill. 84.94 billion of the 86.98
>    billion column-sum tokens are cache reads. A project with shorter
>    sessions would rank these models differently, because input and output
>    prices would matter more than the cache-hit price.
>
> 3. Token counts are held fixed. Another model can tokenise the same
>    transcript differently, emit different output and reasoning, and keep a
>    different cache-hit ratio. The Claude 4.7+ tokenizer note above is the
>    documented case of that. These figures isolate the rate card.

The [14-billion-token comparison](normalised-pay-as-you-go-comparison.md#normalised-pay-as-you-go-comparison)
further down uses a July 2026 sample and the prices checked on 26 July 2026.
It is not this ledger.

Choose the path that matches why you are here:

- Evaluating the crate: read [what I reviewed](what-i-reviewedand-what-i-did-not.md#what-i-reviewed-and-what-i-did-not),
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
