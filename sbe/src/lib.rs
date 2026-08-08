#![warn(missing_docs)]
// Crate-root allows are deliberately few. Codegen-specific noise lives on
// `codegen` (and other modules) via scoped attributes — not a 40-line blanket
// silence of workspace lint policy.
//
// Justified at crate root (schema/codegen reality, not laziness):
#![allow(clippy::too_many_arguments)] // Generator pipelines thread many schema/config params
#![allow(clippy::too_many_lines)] // Emit functions are inherently large token builders
#![allow(clippy::doc_markdown)] // SBE identifiers (blockLength, templateId) trip false positives
#![allow(clippy::cast_possible_truncation)] // Numeric widths constrained by schema validation
#![allow(clippy::cast_sign_loss)] // Same: ranges validated against primitive types

//! Opinionated, idiomatic Rust code generation for Simple Binary Encoding (SBE).
//!
//! [Simple Binary Encoding][sbe-spec] describes messages in XML; ergo-sbe
//! parses those schemas and emits safe, version-aware Rust codecs for
//! low-latency trading.
//!
//! # Documentation
//!
//! - **[ergo-sbe book](https://mimran1980.github.io/ergon/)** — getting started,
//!   feature tour, core concepts, configuration, recipes, design notes
//! - [Getting started](https://mimran1980.github.io/ergon/sbe/getting-started.html) ·
//!   [Feature tour](https://mimran1980.github.io/ergon/sbe/feature-tour.html) ·
//!   [Coming from sbe-tool](https://mimran1980.github.io/ergon/sbe/getting-started/from-sbe-tool.html) ·
//!   [Type-state design](https://mimran1980.github.io/ergon/sbe/design-notes/type-state.html)
//! - [Crate README](https://github.com/mimran1980/ergon/blob/main/sbe/README.md)
//!
//! ## In a few words
//!
//! - **Compile-time wire order** — calling `asks` before `bids` is a type error
//! - **Closure-based groups** — nested shape mirrors the schema, no `.parent()` hopscotch
//! - **Exact buffer sizing** — no oversize scratch buffers; works directly with
//!   Aeron `try_claim`
//! - **Three-tier trust boundary** — `try_*` constructors validate the buffer
//!   extent and return `Result`; bare `wrap` / `wrap_and_apply_header` /
//!   `decode` prove the same extent and **panic** if short; `unsafe fn
//!   *_unchecked` skips checks (caller proves the extent in `# Safety`)
//! - **Zero heap allocation** on generated hot paths; zero runtime dependencies
//! - **Domain types** — map wire `Decimal` to `rust_decimal::Decimal` with one
//!   line of config
//!
//! # Architecture
//!
//! | Layer | Module | Responsibility |
//! |-------|--------|----------------|
//! | Schema input | [`xml`], [`xsd`], [`schema`] | Parse SBE XML, optional XSD shape check, [`Schema`] |
//! | Intermediate | [`ir`], [`resolve`] | Token stream + offsets / block lengths |
//! | Options | [`config`] | Module name, conversions, domain objects, … |
//! | Codegen | [`codegen`] | Rust source modules |
//!
//! # Quick-start (`build.rs`) — doctest pins generated idioms
//!
//! Generated *application* types do not exist in this crate at doctest time.
//! The example below **runs the generator** on an inline schema and asserts on
//! the emitted source so chained encode / length-builder names cannot drift
//! unnoticed. For end-to-end encode/decode of real types, see
//! [`samples/sbe-feature-tour`](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour)
//! and the golden file
//! [`sbe/tests/golden/car_example.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/golden/car_example.rs).
//!
//! ```rust
//! use ergo_sbe::{parse, Generator, GenerationConfig, Schema};
//!
//! let schema_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
//! <messageSchema package="example.sbe" id="1" version="0"
//!                byteOrder="littleEndian">
//!   <types>
//!     <composite name="messageHeader">
//!       <type name="blockLength" primitiveType="uint16"/>
//!       <type name="templateId"   primitiveType="uint16"/>
//!       <type name="schemaId"     primitiveType="uint16"/>
//!       <type name="version"      primitiveType="uint16"/>
//!     </composite>
//!   </types>
//!   <message name="Car" id="1">
//!     <field name="serialNumber" id="1" type="uint64" offset="0"/>
//!     <field name="modelYear"    id="2" type="uint16" offset="8"/>
//!   </message>
//! </messageSchema>"#;
//!
//! let ir = parse(schema_xml).expect("parse schema");
//! let schema = Schema::from_ir(ir);
//! let output = Generator::new(GenerationConfig::new("my_messages"))
//!     .generate(&schema)
//!     .expect("generate");
//! let src = &output.modules().next().expect("one module").source;
//! assert!(src.contains("CarDecoder"));
//! assert!(src.contains("CarEncoder"));
//! assert!(src.contains("CarFixedFields"));
//! assert!(src.contains("wrap_and_apply_header"));
//! assert!(src.contains("compute_length_with_header") || src.contains("ENCODED_LENGTH"));
//! // write module.source to OUT_DIR and `include!` it from your crate
//! ```
//!
//! # What gets generated (how to use it)
//!
//! Names depend on your schema (`Car` below is illustrative). Prefer the
//! feature-tour sample and golden file over prose-only snippets.
//!
//! ## Composites = wire images (not `repr(C)` overlays)
//!
//! Generated composites are `#[repr(transparent)] struct Engine(pub [u8; N])`:
//! the value **is** the on-wire byte block. Accessors use explicit
//! `from_le_bytes` / `to_le_bytes` at schema offsets (portable; free on LE).
//! Default decode is a flyweight (`EngineDecoder { buf, pos }`) — zero-copy
//! into the message. Do **not** transmute the buffer to a padded `#[repr(C)]`
//! field struct: SBE is packed and may be unaligned. See the crate README
//! section *Composite layout & little-endian*.
//!
//! ## Decode flyweight
//!
//! → [`samples/sbe-feature-tour`](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs)
//!
//! ## Encode + type-state tails (buffer sizing)
//!
//! **Size the buffer first** with the staged `*EncodedLength` builder —
//! never guess with a large `vec![0u8; 4096]`. For fixed-only messages
//! use the const `ENCODED_LENGTH`.
//!
//! → [`samples/sbe-feature-tour`](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs)
//!
//! ## Buffer sizing
//!
//! Schema-aware helpers size the buffer **before** you write — including
//! nested groups and ragged var-data — so you do not hand-calculate wire length
//! for complicated messages. Prefer a **stack array** when the length is
//! `const` (`ENCODED_LENGTH` / `compute_encoded_length_*`); for runtime
//! lengths, size first then claim or encode into an exact-length slot.
//!
//! | Shape | API | Prefer |
//! |-------|-----|--------|
//! | Fixed-only | `HeartbeatEncoder::ENCODED_LENGTH` (const) | `[0u8; N]` stack |
//! | Flat / known tails | `compute_encoded_length_with_message_header(...)` | stack when const |
//! | Groups / nested / ragged | `CarEncodedLength::new()…encoded_length_with_header()` | exact claim / slot |
//!
//! ## Why wire order is compile-time
//!
//! SBE is positional. Two adjacent identical groups (e.g. bids then asks) are
//! common in market data; swapping them still produces “valid” bytes and only
//! fails in production. Named stage structs make the wrong order a type error.
//!
//! ## Field metadata (Java parity)
//!
//! → [`sbe/tests/java_parity_features_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/java_parity_features_test.rs)
//!
//! ## Conversion: `with_conversion` vs `with_domain_type`
//!
//! **Pick one style per selector** — `with_domain_type` already enables conversion.
//!
//! | Config | Generated decode | Generated encode |
//! |--------|------------------|------------------|
//! | [`GenerationConfig::with_conversion`] | `dec.price_as::<T>()?` | `enc.price_from(&t)?` |
//! | [`GenerationConfig::with_domain_type`] | `dec.try_price()? -> path::Type` | `enc.try_price(value)?` |
//!
//! ```rust
//! use ergo_sbe::{ConversionSelector, GenerationConfig};
//!
//! // A — pluggable: you implement TryFromSbe / TryToSbe
//! let _a = GenerationConfig::new("msgs")
//!     .with_conversion(ConversionSelector::named_type("Decimal"));
//!
//! // B — concrete Rust type (implies conversion for the same selector)
//! let _b = GenerationConfig::new("msgs")
//!     .with_domain_type(
//!         ConversionSelector::named_type("Decimal"),
//!         "rust_decimal::Decimal",
//!     );
//! ```
//!
//! See [`sbe/tests/comprehensive_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/comprehensive_test.rs)
//! (conversion and domain-object coverage).
//!
//! ## Domain objects
//!
//! [`GenerationConfig::with_domain_objects`] with a [`DomainVarData`] mode emits
//! owned structs. Use [`DomainVarData::Strings`] for text (`String`;
//! invalid UTF-8 → `InvalidUtf8` error) or [`DomainVarData::Bytes`] for `Vec<u8>`.
//!
//! See [`sbe/tests/domain_objects_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/domain_objects_test.rs)
//!
//! ## Multi-message dispatch
//!
//! See [`sbe/fuzz/fuzz_targets/any_message_frame_cursor.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/fuzz/fuzz_targets/any_message_frame_cursor.rs)
//!
//! ## Fixed arrays / char fields
//!
//! See [`sbe/tests/java_parity_features_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/java_parity_features_test.rs)
//!
//! ## Keywords in schema names
//!
//! Field `type` becomes `type_` (default append `"_"`). Override with
//! [`GenerationConfig::with_keyword_append_token`].
//!
//! ## XSD structural check
//!
//! Optional CI gate: [`validate_against_sbe_xsd`] or [`parse_with_xsd_validation`].
//! Official XSD text is embedded as [`SBE_XSD`].
//!
//! # Design
//!
//! - Wire-compatible with official SBE / sbe-tool baselines where tested
//! - Idiomatic Rust (type-state tails, borrow flyweights) — not a Java port
//! - Zero allocation on decode hot paths by default
//! - Version-aware accessors (`sinceVersion` / acting version)
//! - Unsafe only on explicit `_unchecked` / documented paths
//!
//! Benchmarks: [benchmarks chapter](https://mimran1980.github.io/ergon/sbe/benchmarks.html).
//!
//! [sbe-spec]: https://www.fixtrading.org/standards/sbe/

/// Re-exported so `build.rs` can return [`miette::Result`] without an extra
/// dependency. Enable the crate's `fancy` feature for graphical rendering
/// (source snippet + span) instead of the plain fallback.
pub use miette;

/// Cargo `build.rs` helpers ([`generate_to_out_dir`], [`sbe_mod!`]).
#[allow(
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::result_large_err
)]
pub mod build;
/// Codec generation ([`Generator`]).
// Scoped: quote!/token emit paths and submodule re-exports trip style lints and
// unused_import on `pub(crate) use` hubs. Prefer fixing real dead code over
// growing this list — do not re-blanket the crate root.
#[allow(
    unused,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::manual_memcpy,
    clippy::needless_range_loop,
    clippy::unnecessary_cast,
    clippy::useless_format,
    clippy::items_after_statements,
    clippy::explicit_counter_loop,
    clippy::uninlined_format_args,
    clippy::collapsible_if,
    clippy::unreadable_literal,
    clippy::match_same_arms,
    clippy::needless_borrow,
    clippy::use_self,
    clippy::missing_const_for_fn,
    clippy::result_large_err,
    clippy::similar_names,
    clippy::redundant_clone,
    clippy::ref_option,
    clippy::map_unwrap_or,
    clippy::redundant_closure_for_method_calls,
    clippy::unnecessary_unwrap,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::if_same_then_else,
    clippy::should_panic_without_expect,
    clippy::module_name_repetitions,
    clippy::option_if_let_else,
    clippy::match_wildcard_for_single_variants,
    clippy::single_match_else,
    clippy::fn_params_excessive_bools,
    clippy::cast_enum_constructor,
    clippy::ptr_as_ptr,
    // Recursive-descent group encoder helper; legitimate recursion.
    clippy::only_used_in_recursion
)]
pub mod codegen;
/// [`GenerationConfig`] — conversions, domain objects, keywords, etc.
#[allow(
    dead_code,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used
)]
pub mod config;
/// Token [`Ir`] (usually via [`Schema`]).
#[doc(hidden)]
#[allow(clippy::pedantic, clippy::nursery)]
pub mod ir;
/// Offset resolution ([`resolve_schema`]; called by parse).
#[doc(hidden)]
#[allow(
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::result_large_err,
    clippy::collapsible_if,
    clippy::needless_range_loop
)]
pub mod resolve;
/// [`Schema`] handle for codegen.
#[allow(
    dead_code,
    clippy::pedantic,
    clippy::nursery,
    clippy::unnecessary_wraps
)]
pub mod schema;
/// Structured IR for codegen (internal).
#[allow(
    dead_code,
    unused_imports,
    unused_variables,
    clippy::pedantic,
    clippy::nursery,
    clippy::cast_lossless
)]
pub(crate) mod structured_ir;
/// XML parse ([`parse`], [`parse_file`]).
#[allow(
    unused,
    dead_code,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::manual_memcpy,
    clippy::needless_range_loop,
    clippy::collapsible_if,
    clippy::match_same_arms,
    clippy::similar_names,
    clippy::redundant_clone,
    clippy::option_if_let_else,
    clippy::module_name_repetitions,
    clippy::items_after_statements,
    clippy::uninlined_format_args,
    clippy::result_large_err,
    clippy::unnecessary_cast
)]
pub mod xml;
/// Optional XSD-shaped validation ([`validate_against_sbe_xsd`], [`SBE_XSD`]).
#[allow(
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used
)]
pub mod xsd;

pub use build::{
    BuildError, generate_str_to_dir, generate_str_to_out_dir, generate_to_dir, generate_to_out_dir,
    out_dir,
};
pub use codegen::{GenerateError, GeneratedModule, GeneratedModuleSet, Generator};
pub use config::{
    ConversionSelector, DomainVarData, EnumVariantInfo, FieldInfo, GenerationConfig,
    GenerationProfile, ItemContext, ItemKind, SetChoiceInfo,
};
pub use ir::{ByteOrder, Encoding, Ir, Presence, PrimitiveType, Signal, Token};
pub use resolve::{ResolveError, resolve_schema};
pub use schema::Schema;
pub use xml::{
    ParseError, parse, parse_file, parse_file_with_shared, parse_with_shared,
    parse_with_xsd_validation,
};
pub use xsd::{SBE_XSD, XsdValidationError, validate_against_sbe_xsd};

/// Chrono timestamp converters — feature-gated behind `chrono`.
///
/// Use [`GenerationConfig::with_domain_type`] with
/// `"chrono::DateTime<chrono::Utc>"` or `"chrono::NaiveDateTime"` to
/// generate `try_*` / `try_set_*` methods that convert between SBE `i64`
/// wire values and chrono datetime types.
#[cfg(feature = "chrono")]
pub mod chrono_converters;

// Re-export optional dependencies so generated codecs can name the types
// without the consumer adding them directly. Feature-gated codec methods
// (into_<field>_as_compact_str, etc.) use these paths.
#[cfg(feature = "compact_str")]
pub use compact_str;
#[cfg(feature = "smol_str")]
pub use smol_str;
#[cfg(feature = "bytes")]
pub use bytes;

// Header-state markers (`HeaderPresent` / `HeaderAbsent`) live in each
// generated module's `sbe_rt` (see `generate_sbe_rt_src`). They are not
// re-exported here: generated codecs seal against their own `sbe_rt::HeaderState`,
// so a shared `ergo_sbe::header_state` type would not unify with `H`.
