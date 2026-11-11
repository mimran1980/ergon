# Configuration

- [Feature integrations](configuration/feature-integrations.md)
- [with_conversion vs with_domain_type](configuration/conversion-vs-domain.md)
- [GenerationConfig options](configuration/generation-config.md)
- [Code-generation hooks](configuration/hooks.md)

## Wire type vs app type

The full comparison is
[with_conversion vs with_domain_type](configuration/conversion-vs-domain.md).

| Name | Role |
|------|------|
| `Decimal` (schema composite) | Wire: generated type / `price_value()`, what is in the buffer |
| `Cents`, `rust_decimal::Decimal`, … | App: what your code wants to use |

```rust,ignore
  app  ──price_from / price(v)──►  wire Decimal on the buffer
  buf  ──price_as / price()──►    app value
```
