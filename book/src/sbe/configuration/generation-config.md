# GenerationConfig Options

Every option except `new("module_name")` is a chained builder method. Boolean
flags default to the value shown.

| Option | Default | Purpose |
|--------|---------|---------|
| `with_conversion(selector)` | — | Generic `*_as::<T>()` / `*_from(&t)` per selected field |
| `with_domain_type(selector, path)` | — | One canonical app type per field (implies conversion); Generated impl |
| `with_manual_domain_type(selector, path)` | — | Same signatures, caller supplies `TryFromSbe`/`TryToSbe` |
| `with_domain_objects(var_data)` | off | Owned `*Domain` + `encode`; `Strings` → `String` (bad UTF-8 → `InvalidUtf8`), `Bytes` → `Vec<u8>` |
| `with_display_debug(enable: bool)` | `true` | Emit `Debug`/`Display` impls on generated types |
| `with_meta_attributes(enable: bool)` | `true` | Emit `*_ENCODING_OFFSET`, `*_ID`, `*_META_ATTRIBUTE` etc. |
| `with_dispatch(enable: bool)` | `true` | Emit `AnyMessage`/`FrameCursor`/`MessageVisitor` dispatch |
| `with_bool_domain_type(enable: bool)` | `false` | Auto-register `bool` converters for every boolean enum (name, `semanticType`, or `{0,1}` value pair) |
| `with_null_as_option(selector)` | — | `NullVal` → `None` for matching enum fields; getter returns `Option<Enum>` |
| `with_all_enums_as_option()` | `false` | All enums → `Option<Enum>`; blanket form of `with_null_as_option` |
| `profile(GenerationProfile)` | `Full` | Preset: `Full` (default conveniences) or `Lean` (off: Display/Debug, meta attrs, dispatch; domains stay off unless re-enabled). Individual `with_*` overrides still apply after `profile`. |
| `with_deprecated_attrs(enable: bool)` | `false` | `#[deprecated]` on schema-deprecated items |
| `with_shared_module(name)` | — | Multi-schema shared types module |
| `with_external_sbe_rt(path)` | — | Share one `sbe_rt` runtime module instead of inlining |
| `with_error_from_impls(path)` | — | Deprecated: `From<EncodeError> for YourError` via `String`; prefer a typed `From` (see below) |
| `with_keyword_append_token(token)` | `"_"` | Schema `type` → Rust `type_` |
| `with_memoized_tail_offsets(enable: bool)` | `true` | Random-access decoders cache discovered dynamic-tail boundaries. Off restores the pre-memoization decoder: smaller, `Sync`, but every tail access re-walks |
| `with_compact_tail_offsets(enable: bool)` | `false` | Store cached tail ends as `u32` relative to the decoder base instead of absolute `usize`. Smaller decoder, more instructions per cache operation |
| `with_encode_version(version)` | — | Encoder writes `version` and omits members above it; the decoder still reads every version in the schema |
| `with_hook(fn)` | — | Register a code-generation hook (serde, custom traits, …) |

Turn off `with_display_debug`, `with_meta_attributes`, and `with_dispatch` to
reduce generated-code size (~6,100 lines/message with all on). Text fields
stay bytes unless the schema declares a character encoding (then strict
UTF-8/ASCII helpers apply).

## Tail-offset memoization

Random-access decoders can memoize dynamic-tail boundaries, so reading tails
out of order — or reading one twice — walks the wire at most once. The cache
uses `Cell`, which is what makes a tail-bearing decoder `Send` but **not
`Sync`**: use one decoder instance per thread over shareable immutable bytes.

`with_memoized_tail_offsets(true)` turns it on. It is **off by default**: the
cache is constructed on every decoder, so a decoder that reads only fixed
fields pays for it and gets nothing back, and no cost on a benchmarked hot path
is added without an explicit opt-in.

| | default | memoized |
|---|---|---|
| Repeated / out-of-order tail reads | re-walk every time | walk once, then cached |
| Single pass in wire order | no cache to pay for | pays, gains nothing |
| Decoder size | smaller | larger (one slot per dynamic tail) |
| `Sync` | yes | no |
| `decode_cache_stats` (debug builds) | not generated | generated |

Decoded values and wire bytes are identical either way; only tail discovery
differs. Turn it on when you read tails out of order or more than once per
message — a random-access consumer, a view that jumps to the last var-data
field, a decoder reused across several readers of the same buffer.

Measure your own access pattern before deciding; `just bench-diagnostics` runs
`versioned_l3_bench`, whose `vl3/memoization` group covers the cold, warm,
single-pass and repeated-read shapes in both LTO profiles.

### Offset representation

`with_compact_tail_offsets(true)` stores each cached tail end as a `u32`
relative to the decoder base rather than an absolute `usize` (native `usize` on
32-bit targets). It only means anything alongside `with_memoized_tail_offsets`
— without a cache there are no slots to store. A span that cannot be
represented is **not** an error: the representable prefix stays cached and the
suffix is walked uncached.

The default is `usize`, chosen on measurement rather than taste. Compact makes
every tailed decoder and entry decoder smaller, and its wall-clock advantage
comes from moving a smaller struct — but it costs more instructions on both
cache primitives: a `checked_sub` plus a `u32::try_from` range check on
publish, and a checked `base + relative` on read. The adoption rule is
conjunctive (less memory **and** no slower **and** no more instructions), and
compact fails the instruction leg. Turn it on when decoder footprint matters
more than per-operation instruction count; the `vl3/offsets` benchmark group
compares the two on identical traversals.

## Typed error conversions

`GenerationConfig::with_error_from_impls` is deprecated and scheduled for
removal in 1.0. It formats through `String`, so `needed`/`available` are
lost. Prefer a typed `From` on the generated `sbe_rt` types:

```rust,no_run
enum AppError {
    Encode(sbe_rt::EncodeError),
    Decode(sbe_rt::DecodeError),
}

impl From<sbe_rt::EncodeError> for AppError {
    fn from(error: sbe_rt::EncodeError) -> Self {
        match error {
            sbe_rt::EncodeError::BufferTooShort {
                field,
                needed,
                available,
            } => {
                let _ = (field, needed, available);
                Self::Encode(error)
            }
            other => Self::Encode(other),
        }
    }
}

impl From<sbe_rt::DecodeError> for AppError {
    fn from(error: sbe_rt::DecodeError) -> Self {
        Self::Decode(error)
    }
}
```
