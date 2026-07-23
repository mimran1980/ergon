# 07 — Make leader failover atomic and fragment-safe

**What to build:** Make a leader transition either replace the entire coherent client state or leave the prior state untouched, while preventing fragments from different leader images from being combined.

**Blocked by:** 04 — Restore the Java interoperability harness and supported examples; 05 — Surface malformed egress and controlled-polling failures; 06 — Propagate keep-alive failures and enforce session isolation.

**Status:** ready-for-agent

- [ ] Leader endpoint selection uses the declared leader member identifier and rejects missing or malformed endpoint mappings.
- [ ] Endpoint parsing, publication creation, and both replacement fragment assemblers complete before any live client field changes.
- [ ] Failure at each preparation step preserves the previous term, leader, publication, assemblers, and lifecycle state.
- [ ] A successful transition swaps the term, leader, publication, both assemblers, and connected state as one coherent operation.
- [ ] Partial fragments retained from the old image cannot contaminate messages from the new image.
- [ ] Regular and controlled polling share the same failover transition contract.
- [ ] Fault-injection tests prove rollback for endpoint and publication failures.
- [ ] Java-backed failover and restart tests pass with the previous leader unavailable.
