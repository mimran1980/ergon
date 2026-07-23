# 02 — Prove the packaged ergo-sbe 0.1.0 consumer experience

**What to build:** Produce an `ergo-sbe 0.1.0` package that an external Rust project can install, understand, and use for a representative generated-code round trip without relying on workspace-only files.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] The crate metadata consistently declares its experimental scope, version, MSRV, license, repository, homepage, documentation target, README, keywords, categories, and docs.rs behavior.
- [ ] Strict rustdoc completes with warnings denied and all public examples and links resolve.
- [ ] The package uses an explicit allow-list and excludes repository-only tests, fixture inventories, benchmarks, plans, and unrelated laboratories while retaining everything needed to build and use the crate.
- [ ] A consumer created from the packaged artifact can generate codecs and complete a representative encode/decode round trip.
- [ ] The documented examples compile against the packaged artifact rather than a workspace path.
- [ ] The intended `0.1.0` public interface is captured as a compatibility baseline.
- [ ] Existing wire, acting-version, error, allocation, and maintained performance gates pass.
- [ ] Package verification and the `ergo-sbe` publication dry run succeed without publishing.
