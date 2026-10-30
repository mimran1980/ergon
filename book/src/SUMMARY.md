# Summary

[Introduction](./introduction.md)

# ergo-sbe

- [Getting Started](./sbe/getting-started.md)
  - [Depend on the Generator](./sbe/getting-started/depend.md)
  - [Generate in build.rs](./sbe/getting-started/generate.md)
  - [Include Generated Code](./sbe/getting-started/include.md)
  - [Encode and Decode](./sbe/getting-started/encode-decode.md)
  - [Method Chaining](./sbe/getting-started/method-chaining.md)
  - [Multi-Schema Patterns](./sbe/getting-started/multi-schema.md)
  - [Coming from sbe-tool](./sbe/getting-started/from-sbe-tool.md)
  - [Error Diagnostics](./sbe/getting-started/error-diagnostics.md)
- [Feature Tour](./sbe/feature-tour.md)
  - [Exact Sizing](./sbe/feature-tour/exact-sizing.md)
  - [Bulk Arrays](./sbe/feature-tour/bulk-arrays.md)
  - [Decoder Lanes](./sbe/feature-tour/decode-stages.md)
  - [What Generated Code Looks Like](./sbe/feature-tour/generated-code.md)
  - [Trust Boundaries](./sbe/feature-tour/trust-boundaries.md)
  - [Domain Objects (DTOs)](./sbe/feature-tour/domain-objects.md)
  - [Multi-Template Dispatch](./sbe/feature-tour/multi-template.md)
- [Core Concepts](./sbe/core-concepts.md)
  - [Trust Boundary](./sbe/core-concepts/trust-boundary.md)
  - [Wire Order via Named Stages](./sbe/core-concepts/wire-order-stages.md)
  - [Buffer Sizing](./sbe/core-concepts/buffer-sizing.md)
  - [Flyweight vs Whole-Struct](./sbe/core-concepts/flyweight-vs-struct.md)
  - [Composite Layout & Endianness](./sbe/core-concepts/composite-layout.md)
- [Configuration](./sbe/configuration.md)
  - [Feature Integrations](./sbe/configuration/feature-integrations.md)
  - [with_conversion vs with_domain_type](./sbe/configuration/conversion-vs-domain.md)
  - [GenerationConfig Options](./sbe/configuration/generation-config.md)
  - [Code-Generation Hooks](./sbe/configuration/hooks.md)
- [Recipes](./sbe/recipes.md)
  - [Aeron try_claim](./sbe/recipes/aeron-try-claim.md)
  - [Display / Debug](./sbe/recipes/display-debug.md)
  - [Schema Descriptions → Rustdoc](./sbe/recipes/schema-rustdoc.md)
  - [Domain DTOs](./sbe/recipes/domain-dtos.md)
  - [App Types on Composites](./sbe/recipes/app-types-composites.md)
  - [Timestamp Conversions](./sbe/recipes/timestamps.md)
- [Design Notes](./sbe/design-notes.md)
  - [Type-state and runtime cost](./sbe/design-notes/type-state.md)
  - [API freeze decisions](./sbe/design-notes/api-freeze.md)
  - [Why NullVal Instead of Option](./sbe/design-notes/nullval.md)
  - [Feature Matrix](./sbe/design-notes/feature-matrix.md)
- [Benchmarks](./sbe/benchmarks.md)
  - [Methodology](./sbe/benchmarks/methodology.md)

# Samples

- [Teaching Path](./samples/overview.md)
  - [SBE Feature Tour](./samples/sbe-feature-tour.md)
  - [L3 Order Book](./samples/l3-book.md)
  - [Exchange Example](./samples/exchange-example.md)
  - [Codegen as Library](./samples/codegen-library.md)
  - [Cluster Tutorial](./samples/cluster-tutorial.md)
  - [Cluster HA Orderbook](./samples/cluster-ha-orderbook.md)
  - [Cluster RFQ](./samples/cluster-rfq.md)
- [Build Patterns](./samples/build-patterns.md)
- [Buffer Sizing Guide](./samples/buffer-sizing.md)

# ergo-aeron-cluster

- [Cluster Client](./cluster/overview.md)
  - [SessionBuilder](./cluster/session-builder.md)
  - [Egress Listeners](./cluster/egress-listeners.md)
  - [Chained Message Decoding](./cluster/chained-decoding.md)
  - [Compatibility](./cluster/compatibility.md)

# ergon-io

- [Busy-Spin Network I/O](./io/overview.md)
  - [Backends and Configuration](./io/backends.md)
  - [WebSocket](./io/websocket.md)
  - [HTTP client](./io/http-client.md)
  - [Framed TCP](./io/framed-tcp.md)
  - [UDP](./io/udp.md)
  - [Rate Limits and Errors](./io/rate-limits.md)
  - [Kubernetes](./io/kubernetes.md)

# ergon-tune

- [Host Checks](./tune/overview.md)

# ergon-md-dpdk (experimental)

- [DPDK Transport: Proof of Concept](./md-dpdk/overview.md)
  - [Configuration](./md-dpdk/configuration.md)

# Transports

- [Transport Latency Comparison](./transports/comparison.md)
- [Aeron with Many Streams](./transports/aeron-many-streams.md)

# Deployment

- [Low-Latency Hosts](./deploy/overview.md)
  - [Kubernetes HFT tuning](./deploy/hft.md)
  - [AWS and Azure, x86 and Graviton](./deploy/hosts.md)
  - [Clocks](./deploy/clocks.md)
  - [Kubernetes](./deploy/kubernetes.md)
  - [Shared CPUs for Helper Threads (`k8s/nri-shared-cpus`)](./deploy/shared-cpus.md)
  - [Latency Pods](./deploy/latency-pods.md)
  - [Benchmarks: x86 and Graviton](./deploy/benchmarks.md)
  - [Java for Archive and Cluster](./deploy/java.md)

# Project

- [Contributing](./project/contributing.md)
- [AI Assistance Disclosure](./project/ai-assistance.md)
  - [Cumulative token usage (since 2026-06-28)](./project/ai-assistance/cumulative-token-usage-since-2026-06-28.md)
  - [What-if: all tokens through a single provider](./project/ai-assistance/what-if-all-tokens-through-a-single-provider.md)
  - [Why this page exists](./project/ai-assistance/why-this-page-exists.md)
  - [What I was trying to build](./project/ai-assistance/what-i-was-trying-to-build.md)
  - [Why I chose it as an AI experiment](./project/ai-assistance/why-i-chose-it-as-an-ai-experiment.md)
  - [Authorship: what was mine and what was generated](./project/ai-assistance/authorship-what-was-mine-and-what-was-generated.md)
  - [What I reviewed—and what I did not](./project/ai-assistance/what-i-reviewedand-what-i-did-not.md)
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
- [Verification & Release](./project/verification.md)
- [Road to 1.0](./project/road-to-1.0.md)
- [External Schema Pilot](./project/external-pilot.md)
- [Package Scope](./project/package-scope.md)
