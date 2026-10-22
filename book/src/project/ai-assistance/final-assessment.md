# Final assessment

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
roughly 18 billion tokens (measured by ccusage), repeated cleanup, extensive
tests, and continuous human judgment. The fashionable version—write a specification,
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
