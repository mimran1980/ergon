# What I reviewed—and what I did not

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
- whether the public API is clean and usable;
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
