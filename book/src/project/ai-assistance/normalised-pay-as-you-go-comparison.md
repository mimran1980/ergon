# Normalised pay-as-you-go comparison

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

## At standard or short-context rates

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

## If requests actually use approximately one million tokens of context

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

As a sanity check, scaling the observed DeepSeek charge
($77.39 / 10.5229B tokens) to 14B gives **$102.96**, rounded to **$103**. This
is the estimated 14-billion-token cost of the observed V4 Flash/V4 Pro blend.
It sits between the all-Flash and all-Pro modelled figures because the real
workload mixed both models. The selected cache screenshots are also samples of
three days rather than a billing-complete token-category ledger for every day.
