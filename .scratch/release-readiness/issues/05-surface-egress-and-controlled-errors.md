# 05 — Surface malformed egress and controlled-polling failures

**What to build:** Ensure every regular and controlled egress path distinguishes valid unknown messages, malformed protocol data, listener panics, and backpressure so callers can observe the real failure.

**Blocked by:** 03 — Expand the supported Cluster facade and configuration API; 04 — Restore the Java interoperability harness and supported examples.

**Status:** ready-for-agent

- [ ] A malformed header or body returns a typed protocol error and cannot be reported as an unknown template.
- [ ] Invalid UTF-8 or ASCII remains a field-aware error and is never replaced lossily.
- [ ] Malformed binary variable data cannot become an empty challenge or payload.
- [ ] Valid but unsupported template identifiers remain distinguishable from malformed frames.
- [ ] Regular listener panics are contained and returned through the Cluster error contract.
- [ ] Controlled listener panics and decode failures are observable separately from controlled backpressure actions.
- [ ] Property tests prove arbitrary short and malformed frames do not panic or silently become valid events.
- [ ] Java-backed fragmentation coverage proves reassembled valid messages still reach both listener styles.
