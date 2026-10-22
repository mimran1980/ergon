# Long context, the O(n²) mental model, and caching

Both DeepSeek V4 Flash and V4 Pro exposed a one-million-token context window. I
favoured one long-lived conversation over many short ones, so the window could
hold design decisions, failed benchmarks, and corrections across related work.

That is not the same as “never compact and keep one pure million-token
session.” Whenever I started a **genuinely new task**—something sufficiently
different from what the session had been doing—I compacted at that boundary if
it was a natural handoff point. Continuity within a task was the goal; a single
unbroken transcript across the whole project is not an accurate picture.

Subjectively, DeepSeek became much more useful when it retained the accumulated
history that still mattered for the current line of work: design decisions,
examples, mistakes, failed benchmarks, and my corrections. I did not notice a
significant drop immediately after those task-boundary compactions. In separate
Sonnet usage with a smaller context, I have sometimes noticed forgotten details
after compaction.

Those are personal observations, not controlled experiments. They do,
however, explain the usage shape.

## What I mean by “O(n²)”

My informal developer's mental model became “roughly O(n²) in requests.” Here,
`n` means the number of turns or requests in a growing conversation—not the
number of tokens in one request.

Suppose, only to make the intuition concrete, that each turn adds `d` tokens of
new conversation, source code, tool output, test logs, and model response. If
the complete conversation is sent again on every turn, the prompt at turn `i`
contains approximately `i × d` tokens. Across `k` turns the cumulative input
presented is approximately:

```text
d + 2d + 3d + ... + kd
= d × (1 + 2 + 3 + ... + k)
= d × k × (k + 1) / 2
```

That sum grows quadratically with the number of turns. A simple worked example
shows why the final context-window size is misleading:

- 100 turns each add an average of 10,000 tokens;
- the final prompt is approximately 1,000,000 tokens;
- but the cumulative prompt volume across the 100 turns is approximately
  50,500,000 tokens before counting output separately.

So a one-million-token context window does not mean that a one-million-token
conversation costs only one million input tokens over its lifetime. The route
to that final context has repeatedly carried most of the earlier conversation.
Real coding sessions are messier: turns add different amounts, tool results can
be enormous, agents reread files, benchmarks print logs, and compaction changes
the curve. The formula is a mental model, not an invoice.

It is also not a claim that every provider literally recomputes every
transformer operation from scratch on every request. Model-side KV caching,
provider prompt caching, routing, cache eviction, attention implementations,
and other infrastructure all affect actual compute. I do not know whether a
particular provider keeps a cache in RAM, on disk, or in some other tier, and
the storage detail is not required for the economic point.

The economic point is simpler: the API token ledger still records a very large
repeated prefix, and that repeated prefix normally has a price. This is how a
project using a model with a one-million-token window can plausibly accumulate
billions of billed tokens.

## A cache hit is discounted, not free

In an agent conversation, a request can be thought of as three economically
different categories:

1. **Input cache hits:** the provider recognises a previously processed prefix
   and charges its cache-hit rate.
2. **Input cache misses or cache writes:** new or changed input must be
   processed and, depending on the provider, written into the prompt cache.
3. **Output:** new model tokens, including any billed reasoning tokens under
   the provider's accounting.

The headline “input price” and “output price” therefore do not describe a long
agent session. A useful comparison must ask:

- What does a cache hit cost?
- What does a miss or write cost?
- How long is the cache retained?
- Which prefix changes invalidate it?
- Does the provider apply a long-context multiplier?
- At what threshold does that multiplier begin?
- Is the advertised context size actually available at the standard rate?
- How much output or hidden reasoning does the model generate for equal work?

The last question is why the counterfactual table later in this document holds
token usage constant. It isolates pricing, but it cannot prove what another
model would actually consume.

## Why the cache price changed the project

I originally compared models mainly through benchmark rank and ordinary
input/output price. That missed the largest category in this workload:
repeated context. The selected dashboard samples later in this page show that
approximately **98.66% of token volume** was served as input cache hits.

DeepSeek's cache-hit rate was so low that keeping a large and useful context
alive became affordable. This mattered more to my workflow than a modest
benchmark advantage from a model that I could use only intermittently. I could
keep giving feedback, retain design history, run another test, inspect another
generated file, and try again.

That does not mean cheap cache hits make the entire bill negligible. One
correction is essential: roughly 99% of **token volume** being cache hits does
not mean 99% of **dollar cost** came from cache hits. Hits are heavily
discounted. The much smaller quantities of misses and output can contribute a
large portion of the final dollars. The correct calculation prices all three
categories independently.

## Context was part of the model's effective intelligence

For this project, the long session held decisions that were not easily reduced
to a short prompt: why a particular API had been rejected, how an offset bug
had presented, which benchmark had regressed, which generated style I wanted,
and how official output behaved. Retaining that history made the workhorse
model feel more capable.

Caveat on “one long session”: I did **not** leave one transcript untouched for
the entire project. I compacted whenever I started a task that was sufficiently
different that the old tail was no longer a good default context—i.e. at
genuine task boundaries, not once-or-twice total and not never. Within a task I
kept context long; between dissimilar tasks I compacted and moved on. I did not
notice a material loss immediately after those handoff-style compactions. In my
separate experience with Sonnet and a smaller context window, I have sometimes
noticed forgotten decisions after compaction. That is personal observation, not
a controlled model comparison. It nevertheless changed how I think about model
selection: effective intelligence is a combination of the base model, the
context it can retain for the *current* task, the quality of the feedback, and
whether I can afford enough turns to finish the loop.
