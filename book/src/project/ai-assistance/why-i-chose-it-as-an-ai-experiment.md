# Why I chose it as an AI experiment

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

## How the scope grew

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
