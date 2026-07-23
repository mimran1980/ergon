# 12 — Restore CI parity and produce a verified release candidate

**What to build:** Make the release commit independently reproducible in GitHub Actions and produce a complete evidence report showing that both experimental `0.1.0` crates are ready for an explicitly authorized publication.

**Blocked by:** 11 — Unify the two-crate release gate and publication sequence.

**Status:** ready-for-agent

- [ ] Workflow commands reference the current crate and sample names and contain no stale renamed directories.
- [ ] Required jobs cover strict product lint and rustdoc, MSRV, default and all-feature tests, Java interoperability, package allow-lists, packaged consumers, and maintained benchmarks.
- [ ] CI invokes the same release contract used locally rather than maintaining a divergent command list.
- [ ] The repository owner's Actions billing or spending-limit prerequisite is reported explicitly and resolved before live CI acceptance.
- [ ] Every required job starts and passes on the exact release-candidate commit.
- [ ] The evidence report records toolchain, commit, package contents, test counts, interoperability results, allocation results, benchmark ratios, and dry-run outcomes.
- [ ] The report distinguishes the successful `ergo-sbe` preflight from the Cluster registry-wait condition until indexing completes.
- [ ] The release candidate can be rebuilt from a clean checkout with initialized submodules.
- [ ] The unrelated dirty upstream submodule state is not modified by ticket implementation.
- [ ] No crate is uploaded, tag created, or release announced without separate explicit authorization.
