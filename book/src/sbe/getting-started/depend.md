# Depend on the generator

The minimal path is codegen only. Generated codecs embed their own
`sbe_rt` and do not link `ergo-sbe` into the application. Schema parse
errors render a source snippet (line and span) by default:

```toml
[build-dependencies]
ergo-sbe = "0.1"
# no [dependencies] ergo-sbe
```

For convenience, also add `ergo-sbe` as a normal dependency when you use
`sbe_mod!` or `include_sbe!` (the macros expand in the app crate):

```toml
[build-dependencies]
ergo-sbe = "0.1"

[dependencies]
ergo-sbe = "0.1"   # only needed for sbe_mod! / include_sbe!
```

See [Samples](../../samples/overview.md) for monorepo crates that use each pattern.
