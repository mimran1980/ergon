# 03 — Expand the supported Cluster facade and configuration API

**What to build:** Introduce the complete supported high-level Cluster client and configuration experience alongside the legacy surface, so callers can migrate without breaking the repository before the legacy surface is contracted.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] The supported facade covers configuration, synchronous and poll-driven connection, lifecycle state, listeners, regular and controlled polling, typed errors, offer, and claim.
- [ ] Configuration construction has one coherent documented entry point used by all new examples.
- [ ] Invalid channel and endpoint configuration preserves the underlying validation reason as a typed error instead of becoming a generic missing-value failure.
- [ ] Dynamic C-string construction remains an internal FFI concern while callers can provide ordinary configuration values.
- [ ] The high-level quick-connect example compiles against only supported exports.
- [ ] The legacy public surface remains temporarily available so independent migration tickets can keep the repository green.
- [ ] Focused tests prove the supported facade can configure and initiate both synchronous and poll-driven connections.
- [ ] Public documentation clearly distinguishes the supported facade from temporary compatibility surface.
