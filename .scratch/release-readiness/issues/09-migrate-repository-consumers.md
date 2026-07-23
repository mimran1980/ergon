# 09 — Migrate repository consumers to the supported Cluster facade

**What to build:** Move all repository examples, tests, and laboratories onto the supported Cluster contract so the legacy modules and application protocols can be removed without breaking real consumers.

**Blocked by:** 05 — Surface malformed egress and controlled-polling failures; 06 — Propagate keep-alive failures and enforce session isolation; 07 — Make leader failover atomic and fragment-safe; 08 — Make normal Cluster offer allocation-free.

**Status:** ready-for-agent

- [ ] Product examples use only the supported high-level client, configuration, listener, error, offer, and claim contracts.
- [ ] Integration and property tests no longer import generated codecs, URI helpers, transport internals, or test-only product exports through the public crate surface.
- [ ] Application RFQ behavior lives only in the unpublished sample that owns that protocol.
- [ ] Persist and sample consumers use public generated SBE/domain contracts without reintroducing product-specific convenience APIs.
- [ ] Removed or renamed APIs have no remaining repository call sites outside their owning internal implementation.
- [ ] Product, laboratory, and sample formatting, strict lint, and offline tests pass.
- [ ] The Java interoperability suite passes through the migrated facade.
- [ ] The repository remains green while the temporary legacy surface is still present.
