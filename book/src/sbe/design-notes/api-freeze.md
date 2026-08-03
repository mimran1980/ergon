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

- Default generation can enable companions via
  `GenerationConfig::with_unchecked_companions(true)`.
- Safety contract lives on that method and in generated docs: validate with
  `decode` / `try_from` / `wrap` / `verify` first; do not carry unchecked
  access across stage transitions; calling field `_unchecked` without a
  proven extent is UB (not “safe garbage”).
- HFT hot loops after validation are an **intended** use case. Checked
  accessors remain the default API.

See [Trust boundaries](../feature-tour/trust-boundaries.md).

## 5. Header marker default

`H: HeaderState = HeaderPresent` on encoder stages so the common full-frame
path needs no turbofish. Body-only encoding uses `wrap` / `HeaderAbsent`
explicitly.

## 6. No renames bundled with this note

This audit records decisions; it does not rename public generated types.
Any future rename lands in one release with CHANGELOG entries.
