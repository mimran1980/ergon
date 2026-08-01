# Depend on the Generator

**Minimal product path** — codegen only; generated codecs embed their own
`sbe_rt` and do **not** link `ergo-sbe` into the application. Schema parse
errors render a source snippet (line + span) by default:

```toml
[build-dependencies]
ergo-sbe = "0.1"
# no [dependencies] ergo-sbe
```

**Convenience path** — also pull `ergo-sbe` as a normal dependency when you use
`sbe_mod!` / `include_sbe!` (macros expand in the app crate):

```toml
[build-dependencies]
ergo-sbe = "0.1"

[dependencies]
ergo-sbe = "0.1"   # only needed for sbe_mod! / include_sbe!
```

See [Samples](../../samples/overview.md) for monorepo crates that use each pattern.
