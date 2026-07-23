# ergon 0.1 Release Specification

Status: ready-for-agent

## Problem Statement

ergon does not currently have a trustworthy path from the repository state to its first crates.io release. The previous release plan marked every task complete even though its baseline, documentation, public interfaces, package contents, CI configuration, and release commands contradicted those claims.

The live audit established that the core crates have substantial working behavior, but release evidence is fragmented:

- `ergo-sbe` has strong tests and benchmarks, but its published payload, public documentation, interface boundary, and release claims still need verification against the actual package.
- `ergo-aeron-cluster` passes default formatting, lint, test, and maintained benchmark gates, but its advertised harness feature does not compile, strict rustdoc fails, implementation modules and generated protocols remain public, several failure paths contradict the intended contract, and its package contains unpublished test and harness material.
- The automated release check does not verify both crates despite claiming that it does.
- GitHub Actions jobs cannot currently start, and some workflow commands reference directories that were renamed.
- The Cluster dry-run cannot resolve `ergo-sbe` until `ergo-sbe` has been published and indexed by crates.io.
- A diagnostic benchmark falsely reported slow `NewLeaderEvent` decoding because the reference arm used a stale template identifier and skipped the decode body.

Without a corrected release boundary and one evidence-producing gate, a maintainer cannot tell whether `0.1.0` is honest, reproducible, installable, or safe to announce even as experimental software.

## Solution

Release `ergo-sbe 0.1.0` and `ergo-aeron-cluster 0.1.0` as explicitly experimental crates from the ergon repository. The release standard is **experimental but reliable within the documented scope**: documented behavior must work, malformed inputs and operational failures must be surfaced consistently, public interfaces must be intentional, packaged examples must compile as external consumers, and every declared release gate must produce fresh evidence.

The crates will be published in dependency order:

1. Verify and publish `ergo-sbe 0.1.0`.
2. Wait until crates.io resolves that compatible version.
3. Verify and publish `ergo-aeron-cluster 0.1.0`.
4. Create the repository release only after both crates are visible and installable.

Persist, its derive crate, benchmarks, samples, Java harness support, and application protocols remain unpublished. They may prove the product crates but must not expand the published surface.

A single release gate will orchestrate the approved testing seams: strict product checks, strict rustdoc, packaged-consumer examples, the Java interoperability harness, public-contract tests, allocation proofs, maintained equal-work benchmarks, package inspection, and dependency-ordered publication preflight. CI must run the same contract and must actually start and pass.

## User Stories

1. As a Rust SBE developer, I want to install `ergo-sbe 0.1.0` from crates.io, so that I can generate codecs without depending on a Git checkout.
2. As an Aeron client developer, I want to install `ergo-aeron-cluster 0.1.0` after `ergo-sbe`, so that Cargo resolves the complete dependency graph from crates.io.
3. As a prospective adopter, I want both crates labelled experimental, so that I understand they do not carry production guarantees.
4. As a prospective adopter, I want documented capabilities to match executable behavior, so that I do not design against planned or removed APIs.
5. As a schema author, I want generated codecs to remain wire-compatible with official SBE, so that messages interoperate with existing systems.
6. As a low-latency developer, I want maintained codec paths to meet declared performance and allocation gates, so that ergonomic APIs do not conceal regressions.
7. As a decoder user, I want malformed headers, fields, text, groups, and variable data to return typed errors, so that corruption is never mistaken for an unknown message.
8. As a text-field user, I want schema-declared UTF-8 and ASCII decoded strictly, so that invalid protocol text is never silently replaced.
9. As a binary-field user, I want credentials and payloads preserved as bytes, so that binary protocol data is not forced through text conversion.
10. As an encoder user, I want required fixed fields and ordered tails enforced by the generated interface, so that incomplete messages are difficult to construct.
11. As a domain-object user, I want conversion failures propagated recursively, so that malformed nested messages cannot become defaults.
12. As a Cluster client user, I want the high-level client to be the supported entry point, so that generated protocol internals do not become accidental contracts.
13. As a Cluster client user, I want malformed egress messages reported as errors, so that protocol failures are observable.
14. As a Cluster listener author, I want callback panics and failures contained at the FFI boundary, so that unwinding cannot cross into Aeron callbacks.
15. As a controlled-polling user, I want decode and callback failures surfaced separately from backpressure actions, so that corruption is not disguised as `Abort` or `Continue`.
16. As a connected client, I want events filtered by the active Cluster session wherever the protocol carries a session identifier, so that another session cannot mutate my state.
17. As a connected client, I want keep-alive failures returned to the polling caller, so that a dying session is not silently treated as healthy.
18. As a failover user, I want a leader transition prepared completely before client state changes, so that a failed reconnect leaves the previous coherent state intact.
19. As a failover user, I want both fragment assemblers reset on a successful transition, so that fragments from different leaders cannot be combined.
20. As a publisher, I want the normal Cluster offer path to avoid a temporary payload allocation, so that the convenient interface does not impose a hot-path copy.
21. As a latency-sensitive publisher, I want the explicit claim path retained, so that I can write directly into Aeron-owned memory.
22. As a Cluster integrator, I want connect, authentication, fragmentation, keep-alive, failover, restart, and controlled polling exercised against Java Aeron Cluster, so that the crate proves real interoperability.
23. As an example reader, I want every published example to compile against the packaged public API, so that examples cannot rely on workspace-only internals.
24. As a crate consumer, I want internal codecs, Java harness code, tests, reference codecs, and application protocols excluded from packages, so that artifacts remain focused.
25. As a docs.rs reader, I want strict rustdoc to build without broken or private links, so that API documentation is navigable.
26. As a maintainer, I want both crates to declare consistent metadata and MSRV, so that crates.io and users receive accurate compatibility information.
27. As a maintainer, I want a captured `0.1.0` public-interface baseline, so that later breaking changes can be identified deliberately.
28. As a contributor, I want one release command that fails on any unsatisfied product gate, so that local and CI readiness cannot drift.
29. As a reviewer, I want package file lists enforced by an allow-list, so that repository-only artifacts cannot silently enter a crate.
30. As a reviewer, I want benchmark comparisons to derive identifiers from the codecs, so that stale literals cannot create unequal work.
31. As a reviewer, I want every maintained benchmark represented in the enforcement script, so that Criterion output and the release verdict cover the same cases.
32. As a release operator, I want publication to stop if `ergo-sbe` is not indexed, so that Cluster cannot be published with an unresolvable dependency.
33. As a release operator, I want the repository release created only after both crates are visible, so that an announcement cannot precede usable artifacts.
34. As a repository owner, I want CI jobs to start and pass on the release commit, so that local success is independently reproduced.
35. As a repository owner, I want laboratories to remain unpublished and non-authoritative, so that exploratory code cannot be mistaken for supported examples.
36. As a future maintainer, I want the release spec to contain unresolved requirements and verified decisions, so that checkboxes cannot substitute for evidence.

## Implementation Decisions

- **Vocabulary:** ergon is the repository; `ergo-sbe` is the SBE generator crate; `ergo-aeron-cluster` is the experimental Cluster client crate. Persist and Samples are laboratories.
- **Release scope:** both product crates target `0.1.0`. They share a milestone but publish sequentially because Cluster depends on `ergo-sbe`.
- **Quality bar:** `0.1.0` promises reliable documented behavior within a deliberately experimental scope, not production suitability, support, or long-term stability.
- **MSRV:** both crates declare and continuously check Rust `1.95.0` until a separate compatibility decision changes it.
- **ergo-sbe surface:** expose generator, configuration, schema, diagnostic, and documented wire-model contracts intentionally. Generation internals are not public merely for tests.
- **Generated-code contract:** encoders target the latest schema version; decoders honor the acting version; required fields and ordered tails are structurally enforced; malformed data returns typed errors.
- **Text contract:** variable data remains bytes unless the schema declares supported text. UTF-8 and ASCII views are strict and borrowed.
- **Conversion contract:** configured conversions use static dispatch and propagate failures. Domain mapping cannot replace malformed values with defaults.
- **Cluster surface:** publish a deliberate high-level client API covering configuration, lifecycle, listeners, polling, errors, offer, and claim. Implementation modules and generated protocol codecs remain private.
- **Application protocols:** RFQ, auction, topic routing, order workflows, and other application schemas are removed from the generic Cluster crate.
- **URI construction:** C-string construction helpers are internal. Configuration validates channels and caches the FFI representation required by rusteron.
- **Harness boundary:** Java process management and integration infrastructure move to unpublished support code. Published examples do not require a repository-only harness to compile.
- **Error semantics:** malformed frames, invalid text, callback failures, keep-alive failures, polling failures, and reconnect failures remain distinguishable typed errors. Unknown templates may be ignored only after a valid frame is established.
- **Controlled polling:** protocol errors are observable independently of backpressure actions. Callback panics are contained as in regular polling.
- **Session isolation:** every session-bearing event is checked against the active session before it affects listeners or client state.
- **Atomic failover:** endpoint parsing, publication creation, and both new fragment assemblers are prepared first. Term, leader, publication, assemblers, and state are replaced together only after success.
- **Publishing paths:** normal offer uses scatter/gather or equivalent caller-owned slices without allocating a combined payload. Explicit zero-copy claim remains available.
- **Package policy:** each crate uses an explicit allow-list. Product source, required schemas, build support, README, manifest, license, and supported examples are included; repository tests, Java support, reference implementations, benchmarks, and laboratories are excluded.
- **Metadata policy:** both crates provide version, MSRV, license, repository, homepage, documentation target, README, categories, keywords, and docs.rs settings.
- **Public-interface baseline:** final packaged `0.1.0` APIs are captured before publication. Later checks compare against that baseline while respecting `0.x` semantics.
- **Benchmark equality:** benchmark identifiers come from codec constants or are asserted against them. Both arms perform the same validation and payload work.
- **Maintained performance:** hot-path comparisons are release gates. Connect and leader-change paths are correctness-gated cold paths; diagnostic timings remain equal-work but do not block parity.
- **Single gate:** one release command orchestrates all local evidence and identifies the exact failed product, harness, package, documentation, performance, or publication precondition.
- **CI parity:** required CI jobs execute the same gates. A workflow that exists but cannot start is a failed gate.
- **Release automation:** publication names both crates explicitly, publishes `ergo-sbe`, waits for registry resolution, then verifies and publishes Cluster before tagging.
- **Evidence policy:** completion comes from fresh command output and inspected package artifacts, not historical checkboxes.

## Testing Decisions

- Tests assert external behavior at the highest available seam. Internal structure is tested only when no stable public or package seam can reproduce the contract.
- The primary acceptance seam is the single release command. It fails on any strict product check, documentation build, package consumer, interoperability suite, maintained performance gate, or publication precondition.
- A packaged-consumer test builds documented examples against exact package artifacts. Workspace path resolution is not sufficient.
- Strict rustdoc runs with warnings denied for both crates and validates public examples and intra-doc links.
- `ergo-sbe` contract tests cover wire parity, acting versions, required-field phases, ordered tails, text validation, conversion failures, recursive domain mapping, short buffers, allocation behavior, and shared runtime generation.
- Cluster unit and property tests cover codec parity, malformed frames, session filtering, listener panic containment, controlled errors, keep-alive propagation, offer status, and reconnect rollback.
- Cluster integration tests use Java Aeron for connect, authentication, fragmentation, UDP transport, keep-alive, failover, restart, and controlled polling. All advertised features compile before these run.
- Atomic failover tests inject endpoint and publication failures, assert rollback, and prove both assemblers are replaced on success.
- Allocation tests prove maintained generated hot paths, Cluster offer, and Cluster claim meet their contracts.
- Maintained Criterion cases perform byte-identical work with equivalent validation and are enforced by the benchmark gate. Diagnostic cases remain equal-work.
- A benchmark guard fails if fixture, generated codec, reference codec, or expected identifiers disagree, preventing the diagnosed skipped decode.
- Package tests enforce allow-lists and reject Java sources, harnesses, reference codecs, benchmarks, application protocols, and plans.
- CI verifies the declared MSRV and current stable toolchain where dependencies permit. Linux Java interoperability is required.
- The release preflight verifies `ergo-sbe` independently. Cluster's final crates.io dry run occurs only after the registry resolves `ergo-sbe`.
- Release completes only after both crates can be fetched from crates.io and the repository tag points at the verified commit.

## Out of Scope

- Production-readiness, safety certification, formal support, service-level objectives, or compatibility guarantees.
- A Rust Cluster service, consensus module, archive replacement, or replacement for official Aeron Cluster bindings.
- Tokio or another async runtime abstraction; Cluster connection remains poll-driven.
- Application-level RFQ, auction, topic, order, exchange, or ClickHouse APIs.
- Publication of Persist, its derive crate, benchmarks, samples, Java infrastructure, or reference codecs.
- Stabilizing every experimental interface before `0.1.0`.
- Performance parity for cold-path connection, authentication, and leader-change operations; these remain correctness-gated.
- Exhaustive operating-system, architecture, network-topology, or Aeron-version certification beyond the tested matrix.
- A formal security audit or claim that unsafe behavior in rusteron/Aeron has been eliminated.
- Lowering the current MSRV as part of this effort.

## Further Notes

- Default Cluster formatting, strict Clippy, 28 library tests, 45 default-feature all-target tests, benchmark compilation, and all five maintained Cluster ratios currently pass.
- Strict Cluster rustdoc currently fails on fourteen broken or private links.
- The advertised Cluster harness currently fails to compile because examples and integration tests use removed or changed APIs.
- The current Cluster package contains forty-nine files, including Java harness, tests, benchmark, and test-support material.
- Cluster's publication dry run fails because `ergo-sbe 0.1.0` is not in the crates.io index.
- GitHub reports that CI cannot start because of an account billing/spending-limit condition. Resolving it is a release prerequisite.
- The `NewLeaderEvent` slowdown was a benchmark expecting template ID `3` while both codecs declared `6`. The reference arm skipped decoding; changing that one value moved the median ratio from roughly `2.23` to `0.85`.
- The dirty `simple-binary-encoding` submodule is unrelated and must remain untouched.
- This spec authorizes implementation and verification, but not publishing, tagging, or announcing a release without explicit release-time approval.
