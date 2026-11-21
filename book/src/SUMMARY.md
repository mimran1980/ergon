# Summary

[Introduction](./introduction.md)

# ergo-sbe: start here

- [Getting started](./sbe/getting-started.md)
  - [Depend on the generator](./sbe/getting-started/depend.md)
  - [Generate in build.rs](./sbe/getting-started/generate.md)
  - [Include generated code](./sbe/getting-started/include.md)
  - [Encode and decode](./sbe/getting-started/encode-decode.md)
  - [Method chaining](./sbe/getting-started/method-chaining.md)
  - [Multi-schema patterns](./sbe/getting-started/multi-schema.md)
  - [Coming from sbe-tool](./sbe/getting-started/from-sbe-tool.md)
  - [Error diagnostics](./sbe/getting-started/error-diagnostics.md)
- [Feature tour](./sbe/feature-tour.md)
  - [Exact sizing](./sbe/feature-tour/exact-sizing.md)
  - [Bulk arrays](./sbe/feature-tour/bulk-arrays.md)
  - [Decoder lanes](./sbe/feature-tour/decode-stages.md)
  - [What generated code looks like](./sbe/feature-tour/generated-code.md)
  - [Trust boundaries](./sbe/feature-tour/trust-boundaries.md)
  - [Domain objects (DTOs)](./sbe/feature-tour/domain-objects.md)
  - [Multi-template dispatch](./sbe/feature-tour/multi-template.md)
# ergo-sbe: in depth

- [Core concepts](./sbe/core-concepts.md)
  - [Trust boundary](./sbe/core-concepts/trust-boundary.md)
  - [Wire order via named stages](./sbe/core-concepts/wire-order-stages.md)
  - [Buffer sizing](./sbe/core-concepts/buffer-sizing.md)
  - [Flyweight vs whole-struct](./sbe/core-concepts/flyweight-vs-struct.md)
  - [Composite layout and endianness](./sbe/core-concepts/composite-layout.md)
- [Configuration](./sbe/configuration.md)
  - [Feature integrations](./sbe/configuration/feature-integrations.md)
  - [with_conversion vs with_domain_type](./sbe/configuration/conversion-vs-domain.md)
  - [GenerationConfig options](./sbe/configuration/generation-config.md)
  - [Code-generation hooks](./sbe/configuration/hooks.md)
- [Recipes](./sbe/recipes.md)
  - [Aeron try_claim](./sbe/recipes/aeron-try-claim.md)
  - [Display and Debug](./sbe/recipes/display-debug.md)
  - [Schema descriptions as rustdoc](./sbe/recipes/schema-rustdoc.md)
  - [Domain DTOs](./sbe/recipes/domain-dtos.md)
  - [App types on composites](./sbe/recipes/app-types-composites.md)
  - [Timestamp conversions](./sbe/recipes/timestamps.md)
- [Design notes](./sbe/design-notes.md)
  - [Type-state and runtime cost](./sbe/design-notes/type-state.md)
  - [API freeze decisions](./sbe/design-notes/api-freeze.md)
  - [Why NullVal instead of Option](./sbe/design-notes/nullval.md)
  - [Feature matrix](./sbe/design-notes/feature-matrix.md)
- [Benchmarks](./sbe/benchmarks.md)
  - [Methodology](./sbe/benchmarks/methodology.md)
- [Samples: teaching path](./samples/overview.md)
  - [SBE feature tour](./samples/sbe-feature-tour.md)
  - [L3 order book](./samples/l3-book.md)
  - [Exchange example](./samples/exchange-example.md)
  - [Codegen as a library](./samples/codegen-library.md)
  - [Build patterns](./samples/build-patterns.md)
  - [Buffer sizing guide](./samples/buffer-sizing.md)

# ergo-aeron-cluster: start here

- [Cluster client](./cluster/overview.md)
- [Cluster tutorial](./samples/cluster-tutorial.md)

# ergo-aeron-cluster: in depth

- [SessionBuilder](./cluster/session-builder.md)
- [Egress listeners](./cluster/egress-listeners.md)
- [Chained message decoding](./cluster/chained-decoding.md)
- [Compatibility](./cluster/compatibility.md)
- [Cluster HA order book](./samples/cluster-ha-orderbook.md)
- [Cluster RFQ](./samples/cluster-rfq.md)

# ergon-tune

- [Host checks](./tune/overview.md)

# Kubernetes: start here

- [Shared CPUs for helper threads (`k8s/nri-shared-cpus`)](./deploy/shared-cpus.md)
- [Latency pods](./deploy/latency-pods.md)

# Kubernetes: in depth

- [Low-latency hosts](./deploy/overview.md)
  - [Busy-spin loop](./deploy/busy-spin-loop.md)
  - [Kubernetes HFT tuning](./deploy/hft.md)
  - [AWS and Azure, x86 and Graviton](./deploy/hosts.md)
  - [Clocks](./deploy/clocks.md)
  - [Kubernetes](./deploy/kubernetes.md)
  - [Java for Archive and Cluster](./deploy/java.md)
  - [Troubleshooting](./deploy/troubleshooting.md)

# Project

- [Contributing](./project/contributing.md)
- [AI assistance disclosure](./project/ai-assistance.md)
  - [Cumulative token usage (since 2026-06-28)](./project/ai-assistance/cumulative-token-usage-since-2026-06-28.md)
  - [What-if: all tokens through a single provider](./project/ai-assistance/what-if-all-tokens-through-a-single-provider.md)
  - [Why this page exists](./project/ai-assistance/why-this-page-exists.md)
  - [What I was trying to build](./project/ai-assistance/what-i-was-trying-to-build.md)
  - [Why I chose it as an AI experiment](./project/ai-assistance/why-i-chose-it-as-an-ai-experiment.md)
  - [Authorship: what was mine and what was generated](./project/ai-assistance/authorship-what-was-mine-and-what-was-generated.md)
  - [What I reviewed, and what I did not](./project/ai-assistance/what-i-reviewedand-what-i-did-not.md)
  - [The actual working loop](./project/ai-assistance/the-actual-working-loop.md)
  - [How the project evolved](./project/ai-assistance/how-the-project-evolved.md)
  - [What the Git history shows](./project/ai-assistance/what-the-git-history-shows.md)
  - [What did not work](./project/ai-assistance/what-did-not-work.md)
  - [Verification: why the tests matter so much](./project/ai-assistance/verification-why-the-tests-matter-so-much.md)
  - [Performance was part of correctness](./project/ai-assistance/performance-was-part-of-correctness.md)
  - [Unsafe code and the trust boundary](./project/ai-assistance/unsafe-code-and-the-trust-boundary.md)
  - [Tools, models, and what each contributed](./project/ai-assistance/tools-models-and-what-each-contributed.md)
  - [Long context, the O(n²) mental model, and caching](./project/ai-assistance/long-context-the-on2-mental-model-and-caching.md)
  - [Observed usage and actual spend](./project/ai-assistance/observed-usage-and-actual-spend.md)
  - [The cache sample used for the cost comparison](./project/ai-assistance/the-cache-sample-used-for-the-cost-comparison.md)
  - [Normalised pay-as-you-go comparison](./project/ai-assistance/normalised-pay-as-you-go-comparison.md)
  - [What I learned about model “intelligence”](./project/ai-assistance/what-i-learned-about-model-intelligence.md)
  - [A practical playbook for other developers](./project/ai-assistance/a-practical-playbook-for-other-developers.md)
  - [The personal experience: pride, enjoyment, and review fatigue](./project/ai-assistance/the-personal-experience-pride-enjoyment-and-review-fatigue.md)
  - [Why this crate is still experimental](./project/ai-assistance/why-this-crate-is-still-experimental.md)
  - [What a prospective production user should verify](./project/ai-assistance/what-a-prospective-production-user-should-verify.md)
  - [AI-assisted contributions](./project/ai-assistance/ai-assisted-contributions.md)
  - [Final assessment](./project/ai-assistance/final-assessment.md)
- [Verification and release](./project/verification.md)
- [Performance at a glance](./project/performance.md)
- [Road to 1.0](./project/road-to-1.0.md)
- [External schema pilot](./project/external-pilot.md)
- [Package scope](./project/package-scope.md)
