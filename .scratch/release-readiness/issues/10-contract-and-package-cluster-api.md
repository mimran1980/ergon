# 10 — Contract and package the Cluster 0.1.0 public surface

**What to build:** Remove the migrated legacy surface and produce a focused `ergo-aeron-cluster 0.1.0` package whose documented public API and examples work as an external consumer.

**Blocked by:** 04 — Restore the Java interoperability harness and supported examples; 09 — Migrate repository consumers to the supported Cluster facade.

**Status:** ready-for-agent

- [ ] Generated protocol codecs, transport re-exports, implementation modules, URI helpers, and repository test support are no longer public product contracts.
- [ ] RFQ and other application protocol schemas and generated modules are absent from the generic Cluster crate.
- [ ] Java harness implementation is moved behind an unpublished repository boundary.
- [ ] The package uses an explicit allow-list containing only its manifest, README, license, required product source, required Aeron schemas, build support, and supported examples.
- [ ] Repository tests, Java sources, jar hashes, benchmarks, reference codecs, application protocols, and plans are rejected from the package.
- [ ] Metadata consistently declares the crate's experimental scope, version, MSRV, repository, documentation target, categories, keywords, and docs.rs behavior.
- [ ] Strict rustdoc passes with warnings denied and contains no stale branch, RFQ, removed-builder, private-link, or production-readiness claims.
- [ ] Packaged examples compile against only the package's public API.
- [ ] The intended `0.1.0` public interface is captured as a compatibility baseline.
- [ ] Local package verification succeeds using the verified `ergo-sbe` release candidate.
