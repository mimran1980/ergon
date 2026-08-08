# Configuration

- [with_conversion vs with_domain_type](configuration/conversion-vs-domain.md)
- [GenerationConfig Options](configuration/generation-config.md)
- [Code-Generation Hooks](configuration/hooks.md)

## Wire type vs app type

| Name | Role |
|------|------|
| **`Decimal`** (schema composite) | **Wire** — generated type / `price_value()` — what is in the buffer |
| **`Cents`**, **`rust_decimal::Decimal`**, … | **App** — what your code wants to use |

```rust,ignore
  app  ──price_from / price()──►  wire Decimal on the buffer
  buf  ──price_as / price()──►   app value
```
