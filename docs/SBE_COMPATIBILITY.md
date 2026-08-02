# ergo-sbe SBE compatibility profile

Date: 2026-08-02  
Target release: **0.2.0**  
Status: normative for crates that claim wire compatibility with this profile

This document is the **precise** compatibility claim for `ergo-sbe`. Do **not**
advertise unqualified “SBE binary compatibility.” Use this profile name and the
evidence links below.

## Profile identity

| Item | Value |
|------|--------|
| Profile name | `ergo-sbe-fix-sbe-0.2` |
| FIX SBE family | FIX Simple Binary Encoding (SBE) XML schemas |
| XML namespace(s) | `http://fixprotocol.io/2016/sbe` (and plain `messageSchema` roots accepted by the parser) |
| Reference generator | Real Logic [simple-binary-encoding](https://github.com/real-logic/simple-binary-encoding) **submodule pin** in this monorepo (`simple-binary-encoding/`) |
| Reference Rust baseline | Official sbe-tool Rust output under `sbe/tests/sbe_tool_reference/` |
| Byte order | Little-endian and big-endian schemas |
| Framing | Message header composite + body; no Aeron-specific framing in the codec |

Pin the submodule revision in release notes for each publish. Regenerate
reference crates with `scripts/regenerate-sbe-tool-reference.sh` after schema
or baseline changes.

## Supported message shapes (with bidirectional evidence)

“Bidirectional” means: ergon encode bytes match pinned sbe-tool encode bytes
**and** each side can decode the other’s bytes for the claimed fields (or a
documented cross-decode fixture).

| Feature | Support | Evidence |
|---------|---------|----------|
| Fixed primitives (u8–u64, i8–i64, f32/f64, char) | Full | `sbe_tool_wire_parity_test`, `java_parity_features_test`, all-types LE/BE fixtures |
| Enums / sets | Full | parity + comprehensive tests |
| Composites / nested composites | Full | composite layout tests, Car engine |
| Optional presence + null sentinels | Full (0.2 null-width fix) | nullification unit tests + group optional matrix |
| Constant fields | Full | baseline / example schema |
| Repeating groups (flat + nested) | Full | L3, Car fuel/performance, proptest |
| Variable-length data | Full | Car manufacturer/model, maxLength enforcement |
| Multi-template `AnyMessage` dispatch | Full when `with_dispatch(true)` | baseline AnyMessage tests |
| Schema evolution (`sinceVersion`, acting block length) | Full | multi_schema_versioning_test |
| Domain DTOs / converters | Supported (fallible in 0.2) | domain_objects_test |

## Partial / qualified support

| Feature | Qualification |
|---------|----------------|
| Official FIX SBE Conformance suite | **Wired** — suite tests 1–3 (flat + group, schema evolution inject, var-data). Ergon respond bytes are **byte-identical** to Real Logic UnderTest goldens; official `RLValidator` accepts them when `FIX_SBE_CONFORMANCE_HOME` points at a built [fix-sbe-conformance](https://github.com/FIXTradingCommunity/fix-sbe-conformance) tree. Lane: `cargo test -p ergo-sbe --test fix_sbe_conformance_test` and `scripts/run-fix-sbe-conformance.sh`. |
| Unknown enum discriminants | Must not transmute invalid tags into enum variants; use validated decode paths. |
| Custom dimension composites | Supported when they match the documented dimension layout helpers; exotic layouts need fixtures. |

## Explicitly unsupported (not claimed)

- SBE over arbitrary non-FIX framing without an application-defined outer frame
- Automatic JSON/XML codec generation
- Server-side Aeron Cluster consensus (see `ergo-aeron-cluster` client-only scope)
- Nightly-only APIs, speculative SIMD bulk paths as default

## Trust lanes (API contract)

| Lane | Names | Memory safety |
|------|--------|----------------|
| Checked (safe) | `wrap`, `wrap_and_apply_header`, `decode`, `AnyMessage::decode`, `decode_frame` | One validation boundary; returns `Result`; no caller-owned extent precondition |
| Trusted (unsafe) | `*_unchecked` twins (`#[doc(hidden)]` until HFT-008 keep) | Caller proves non-overflowing extents; `# Safety` required |

`try_wrap*` aliases are **removed** in 0.2.

Public `_unchecked` keep decisions: `docs/evidence/unchecked-keep-manifest.json`
(currently all `keep: false`; zero-check cores are **module-private** `unsafe fn`
until a future keep=true decision).

## Generation profiles (HFT-009)

```rust
use ergo_sbe::{GenerationConfig, GenerationProfile};

// Hot path only: no Display/Debug, field meta, or AnyMessage dispatch
let lean = GenerationConfig::new("feed").profile(GenerationProfile::HftLean);

// Default conveniences on
let full = GenerationConfig::new("feed").profile(GenerationProfile::Full);
```

Individual knobs (`with_display_debug`, `with_meta_attributes`, `with_dispatch`)
may still override after `profile(...)`.

## Evidence commands (release)

```sh
cargo test -p ergo-sbe --test sbe_tool_wire_parity_test
cargo test -p ergo-sbe --test sbe_tool_multi_schema_wire_parity_test
cargo test -p ergo-sbe --test hft_001_soundness_test
cargo test -p ergo-sbe --test java_parity_features_test
cargo test -p ergo-sbe --test fix_sbe_conformance_test
cargo test -p ergo-sbe --lib
# Full FIX suite lane (builds Java injector/validator when needed):
scripts/run-fix-sbe-conformance.sh
# When environment allows:
# just test-all
```

Archive commit hash, toolchain (`rustc -Vv`), and command outputs under the
release evidence manifest (HFT-011).

## Evolution policy

- Older wire versions: decode with acting version; fields with
  `sinceVersion > acting` are absent (Option / skip).
- Newer wire versions: unknown trailing content after declared fixed + traversed
  groups/var-data is application-defined; do not invent values for unknown
  templates (`UnknownTemplateLength`).
