# The actual working loop

The primary agent interface was **Claude Code CLI**, often connected to custom
models rather than a Claude model. Claude Code was useful because I wanted to
learn the tool used at work and because its endpoint configuration let me run
DeepSeek and GLM through the same agent workflow.

The work ran on a Mac mini at home. I connected with JetBrains **RustRover
Remote Development**, the Claude Code app, and ordinary SSH. I used
[Herdr](https://herdr.dev/), an agent-aware persistent terminal multiplexer,
so the Claude Code session survived disconnects and I could reattach from
another device.

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
