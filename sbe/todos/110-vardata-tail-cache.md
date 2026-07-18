# Var-data tail offset caching (avoid re-walking)

**Status: WON'T-DO (2026-07-19)**

Equal-work fairness check on maintained decode scenarios (cluster SessionMessageHeader /
SessionEvent smoke ratios **0.873 / 0.849**) shows **no fairness failure** that would
justify a cache. Re-opening only if a maintained decode ratio exceeds 1.00 after an
equal-work audit.

Original re-open was driven by a full_message ratio that has since been closed on the
Car/Aeron matrix (see perf goal 2026-07-17/18).
