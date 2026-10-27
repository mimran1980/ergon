# Build Patterns

Generated codecs ship their own embedded `sbe_rt` module. Linking the **app**
does not require `ergo-sbe` unless you use its macros or call the generator
library at runtime.

| Pattern | `build-dependencies` | `dependencies` | Typical use |
|---------|----------------------|----------------|-------------|
| **Build only** (**samples default**) | `ergo-sbe` | — | `generate(..)?.write_to("src/generated")` → `src/generated/` (gitignored, unpublished) + `#[path = "generated/….rs"]` |
| **OUT_DIR only** | `ergo-sbe` | — | `generate(..)?.write_to_out_dir()` + `include!(concat!(env!("OUT_DIR"), …))` — the product shape without macros |
| **Build + runtime** (**product default**) | `ergo-sbe` | `ergo-sbe` | `generate(..)?.write_to_out_dir()` + `sbe_mod!` (expands to that `include!`) — what `cluster/build.rs` and the introduction do |
| **Runtime only** | — | `ergo-sbe` | Call `generate`, or `parse` + `Generator`, as a library (no `build.rs`) |

### Seeing generated code (without committing it)

`include!(concat!(env!("OUT_DIR"), …))` and `sbe_mod!` put files under a
hashed path like `target/debug/build/<crate>-<hash>/out/….rs`, which is
awkward to open by hand.

Samples instead write to a **stable, local path**:

```text
samples/<name>/src/generated/*.rs   # created on cargo build, gitignored
```

1. `cargo build --manifest-path samples/sbe-feature-tour/Cargo.toml`
2. Open `samples/sbe-feature-tour/src/generated/feature_tour.rs`
3. From app code, **Go to definition** on `CarEncoder` / etc. should land there

Root `.gitignore` has `**/src/generated/`. Do **not** commit those trees
(Binance alone is multi‑MB). Rebuild after a clean clone.

```rust,ignore
  // build.rs
  let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
  ergo_sbe::generate(ergo_sbe::SchemaSource::file("schemas/messages.xml"), config)?
    .write_to(&out)?;

  // src/lib.rs — real path → IDE go-to-definition works
  #[allow(dead_code, unused_imports, non_camel_case_types, non_snake_case, clippy::all, warnings)]
  #[path = "generated/messages.rs"]
  mod messages;
```
