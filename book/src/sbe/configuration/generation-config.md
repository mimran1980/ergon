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
| `with_encode_version(version)` | — | Encoder writes `version` and omits members above it; the decoder still reads every version in the schema |
| `with_hook(fn)` | — | Register a code-generation hook (serde, custom traits, …) |

Turn off `with_display_debug`, `with_meta_attributes`, and `with_dispatch` to
reduce generated-code size (~6,100 lines/message with all on). Text fields
stay bytes unless the schema declares a character encoding (then strict
UTF-8/ASCII helpers apply).

## Decoder lanes (no configuration needed)

Tail-offset memoization used to be a generation-time knob
(`with_memoized_tail_offsets`), paired with a storage-width knob
(`with_compact_tail_offsets`). **Both are gone.** A knob forced the choice at
code-generation time, for the whole module, when the right answer depends on
how each individual call site reads the message.

It is now a runtime lane. Every generated decoder gives you:

```rust,ignore
let decoder = CarDecoder::try_from(bytes)?;  // small, Sync, recalculates tails
let decoder = decoder.memoized();            // lazy cache, no allocation
```

`Decoder::memoized(self)` consumes the base decoder and returns
`{Name}MemoizedDecoder`, which has the same getter names and a progressive
cache of discovered dynamic-tail ends. `into_inner()` goes back.

See [Decoder Lanes](../feature-tour/decode-stages.md) for the decision table
and the cases where each lane wins. Two things worth repeating here:

- Build the memoized decoder **once** and share `&`-references. Calling
  `.memoized()` in every function creates a separate empty cache each time.
- The base decoder is `Sync`; the memoized one is `Send` but not `Sync`
  (`Cell` interior mutability).

Compact `u32` tail-offset storage was removed with the knob. It saved a little
decoder memory but cost more instructions on both cache primitives and was
materially slower under LTO, so it was never a defensible default and is not
worth a second public surface.

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
