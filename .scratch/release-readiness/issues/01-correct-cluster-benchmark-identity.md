# 01 — Correct Cluster benchmark identity and equal-work enforcement

**What to build:** Make every Cluster codec comparison trustworthy by proving that its fixture and both codec implementations identify the same message and execute equivalent decode or encode work before a timing can influence release evidence.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] Benchmark template and schema identifiers are derived from codec contracts or explicitly asserted against the fixture and both implementations before measurement.
- [ ] The `NewLeaderEvent` reference arm reaches its fixed-field and variable-data decode instead of exiting after the header check.
- [ ] A regression check fails when a fixture, generated codec, reference codec, or benchmark expectation disagrees about message identity.
- [ ] Maintained Cluster cases cannot be silently omitted from the Cluster gate because their estimates are missing.
- [ ] Diagnostic cases remain equal-work even though cold-path parity is not a release requirement.
- [ ] A fresh full Criterion run records the corrected `NewLeaderEvent` comparison and every maintained Cluster ratio.
- [ ] The Cluster benchmark gate passes with no source or fixture identity drift.
