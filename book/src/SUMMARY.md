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
- [Feature Tour](./sbe/feature-tour.md)
  - [Exact Sizing](./sbe/feature-tour/exact-sizing.md)
  - [Bulk Arrays](./sbe/feature-tour/bulk-arrays.md)
  - [Consuming Decode Stages](./sbe/feature-tour/decode-stages.md)
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
  - [Type-state is zero-cost](./sbe/design-notes/type-state.md)
  - [API freeze decisions](./sbe/design-notes/api-freeze.md)
  - [Why NullVal Instead of Option](./sbe/design-notes/nullval.md)
  - [Feature Matrix](./sbe/design-notes/feature-matrix.md)
- [Benchmarks](./sbe/benchmarks.md)

# ergo-aeron-cluster

- [Cluster Client](./cluster/overview.md)
  - [SessionBuilder](./cluster/session-builder.md)
  - [Egress Listeners](./cluster/egress-listeners.md)
  - [Chained Message Decoding](./cluster/chained-decoding.md)

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

# Project

- [Contributing](./project/contributing.md)
- [AI Assistance Disclosure](./project/ai-assistance.md)
- [Verification & Release](./project/verification.md)
- [Road to 1.0](./project/road-to-1.0.md)
- [Package Scope](./project/package-scope.md)
