# 04 — Restore the Java interoperability harness and supported examples

**What to build:** Restore a runnable end-to-end interoperability seam in which every advertised feature and supported example compiles through the high-level facade and exercises a real Java Aeron Cluster.

**Blocked by:** 03 — Expand the supported Cluster facade and configuration API.

**Status:** ready-for-agent

- [ ] Building and testing with every advertised Cluster feature compiles without stale builder, URI-helper, or C-string calls.
- [ ] The supported connect/echo, controlled-polling, and leader-failover examples compile through the high-level public facade.
- [ ] Examples return useful typed failures instead of relying on avoidable panic-oriented calls.
- [ ] Java harness setup verifies its required Java version and Aeron artifacts with actionable diagnostics.
- [ ] Java-backed connection, authentication, egress fragmentation, UDP transport, and restart baseline tests execute successfully.
- [ ] The harness is usable by repository tests without becoming part of the supported product interface.
- [ ] The default-feature and all-feature Cluster test commands both pass after the migration.
