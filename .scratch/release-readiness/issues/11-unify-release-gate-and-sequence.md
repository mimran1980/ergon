# 11 — Unify the two-crate release gate and publication sequence

**What to build:** Provide one truthful pre-release command and one explicit automation path that verify both product crates, package them in dependency order, and stop before publication whenever evidence is incomplete.

**Blocked by:** 01 — Correct Cluster benchmark identity and equal-work enforcement; 02 — Prove the packaged ergo-sbe 0.1.0 consumer experience; 10 — Contract and package the Cluster 0.1.0 public surface.

**Status:** ready-for-agent

- [ ] The single release command runs strict formatting, lint, tests, rustdoc, package-consumer checks, Java interoperability, allocation proofs, and maintained benchmarks for the relevant product.
- [ ] Package allow-list verification runs automatically for both crates and fails on unexpected files.
- [ ] The command reports individual gate outcomes and cannot claim both products passed after checking only one.
- [ ] Pre-publication verification names the two product crates explicitly and never attempts broad workspace publication.
- [ ] The sequence verifies and prepares `ergo-sbe` before Cluster.
- [ ] Cluster publication preflight waits until crates.io resolves the required `ergo-sbe` version and fails with a clear retryable condition before then.
- [ ] Repository tagging and release creation occur only after both published crates can be fetched.
- [ ] Persist, derive, benchmarks, samples, and harness support remain non-publishable.
- [ ] Local dry runs exercise every non-destructive step without uploading, tagging, or creating a release.
- [ ] Contributor and release documentation describe the same command and dependency order.
