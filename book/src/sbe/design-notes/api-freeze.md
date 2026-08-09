# API freeze decisions (pre-1.0)

Deliberate decisions on the generated public surface. Changing any of these
after 1.0 is a major version. Golden file
[`sbe/tests/golden/car_example.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/golden/car_example.rs)
is the artifact for API shape review.

## 1. `wrap` takes message start (not body offset)

| | ergo-sbe | sbe-tool Rust |
|---|----------|---------------|
| `wrap` offset | **Message start** (first byte of header) | **Body** offset (usually `message_start + 8`) |
| Field bytes | at `message_offset + HEADER_LENGTH + field_offset` | at `body_offset + field_offset` |

**Decision: keep ergon semantics.** One offset works for encode wrap,
`wrap_and_apply_header`, and claim buffers. sbe-tool refugees who pass `8`
for a frame at zero will mis-align every field — that is the #1 migration
trap. Loud rustdoc on every generated `wrap` / `wrap_and_apply_header` /
`decode` documents this; the book chapter
[Coming from sbe-tool](../getting-started/from-sbe-tool.md) is the full mapping.

## 2. `*FixedFields` is intentionally exhaustive

Generated `CarFixedFields` (and peers) are **not** `#[non_exhaustive]`.

**Decision: exhaustive is a feature.** When the schema adds a fixed field,
every `fixed(&…)` call site must update. Silent `Default` / ignored new
fields would hide schema drift. Do not “fix” this with `#[non_exhaustive]`
without a major-version design review.

## 3. Stage struct naming

Pattern: `{Message}After{GroupPascal}` for intermediate stages,
`{Message}Complete` for the terminal encoder stage; decoder stages use
`{Message}DecoderAfter{…}` similarly. Multi-word group names go through the
same PascalCase path as other types (`fuelFigures` → `FuelFigures` →
`CarAfterFuelFigures`). Reserved-name clash coverage:
`sbe/tests/reserved_name_clash_test.rs`.

**Decision: keep named monomorphic stages** (not `Encoder<State = AfterBids>`).
Rationale: [Type-state design note](type-state.md).

## 4. `_unchecked` companions are a supported opt-in

**Decision: supported production opt-in after a proven trust boundary** — not
“benchmarking only” framing.

- Default generation exposes the three-tier constructor boundary: `try_*`
  (Result), bare names (panic after extent proof), and `unsafe fn *_unchecked`.
- Safety contract: validate with `try_decode` / `try_from` / `try_wrap` /
  `verify` at trust edges; bare constructors also prove fixed extent before
  returning; only `*_unchecked` may skip that proof (UB if wrong).
- Hot loops after validation are an **intended** use case for the
  unchecked lane. Checked constructors remain the default for untrusted input.

See [Trust boundaries](../feature-tour/trust-boundaries.md).

## 5. Header marker default

`H: HeaderState = HeaderPresent` on encoder stages so the common full-frame
path needs no turbofish. Body-only encoding uses `wrap` / `HeaderAbsent`
explicitly.

## 6. No renames bundled with this note

This audit records decisions; it does not rename public generated types.

## 7. `#[non_exhaustive]` policy for generated structs

| Struct | `#[non_exhaustive]` | Rationale |
|--------|---------------------|-----------|
| `{Msg}FixedFields` | **No** (exhaustive) | Schema field additions must surface as compile errors (§2 above) |
| `{Msg}Encoder` | No (all fields `pub(crate)`) | Constructed by the generated `wrap` / `wrap_and_apply_header` |
| `{Msg}Decoder` | No (all fields `pub(crate)`) | Constructed by the generated `try_decode` / `decode` / `wrap` |
| `{Msg}After{Element}` | No (all fields `pub(crate)`) | Only reachable through the consuming tail-stage chain |
| `{Msg}Complete` | No (all fields `pub(crate)`) | Reachable after writing all tails |
| `{Group}Encoder` | No (all fields `pub(crate)`) | Constructed by the generated group closure |
| `{Group}Decoder` | No (fields are `pub(crate)`) | Constructed by generated iterator / `wrap` |
| `{Group}EntryComplete` | No (fields `pub(crate)`) | Only produced by `add_checked` / `complete()` |
| `{Msg}EncodedLength` | No (fields `pub(crate)`) | Constructed by `compute_length()` |
| `{Msg}EncodedLengthAfter*` / `Complete` | No (fields `pub(crate)`) | Consuming stages, same as encoder |
| `{Msg}Schema` | No (unit struct) | Carries only consts |
| `ConnectStep` | **Yes** | New async-connect steps must not break exhaustiveness downstream |
| `GenerateError` | **Yes** | Future validation variants are additive |
| `ParseError` | No (existing public API) | Variants are well-established; `#[non_exhaustive]` would break existing handlers |

**Decision: keep generated consumer-facing structs non-exhaustive via `pub(crate)` fields rather than `#[non_exhaustive]`.** A downstream crate cannot construct one directly, so adding a field is not a breaking change. `#[non_exhaustive]` is reserved for public enums that will gain variants over time (`GenerateError`, `ConnectStep`).
Any future rename lands in one release with CHANGELOG entries.
