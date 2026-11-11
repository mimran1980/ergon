#![warn(missing_docs)]
// Lints that every module trips go here; the rest are scoped per module below.
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
//! - **[ergo-sbe book](https://mimran1980.github.io/ergon/):** getting
//!   started, a feature tour, core concepts, configuration, recipes, design
//!   notes and the [samples](https://mimran1980.github.io/ergon/samples/overview.html).
//! - [Getting started](https://mimran1980.github.io/ergon/sbe/getting-started.html) ·
//!   [Feature tour](https://mimran1980.github.io/ergon/sbe/feature-tour.html) ·
//!   [Coming from sbe-tool](https://mimran1980.github.io/ergon/sbe/getting-started/from-sbe-tool.html) ·
//!   [Type-state design](https://mimran1980.github.io/ergon/sbe/design-notes/type-state.html) ·
//!   [Samples](https://mimran1980.github.io/ergon/samples/overview.html)
//! - [Crate README](https://github.com/mimran1980/ergon/blob/main/sbe/README.md)
//!
//! ## Summary
//!
//! - **Compile-time wire order:** calling `asks` before `bids` is a type error.
//! - **Closure-based groups:** the nesting mirrors the schema and there is no
//!   `.parent()` call.
//! - **Exact buffer sizing:** no oversized scratch buffers, so an encoder can
//!   write straight into an Aeron `try_claim`.
//! - **Three-tier trust boundary:** `try_*` constructors check the buffer
//!   extent and return `Result`; bare `wrap`, `wrap_and_apply_header` and
//!   `decode` check the same extent and panic if it is short; `unsafe fn
//!   *_unchecked` skips the check, and its `# Safety` section says what the
//!   caller must prove.
//! - **Zero heap allocation** on generated hot paths, and no runtime
//!   dependencies.
//! - **Domain types:** one line of config maps a wire `Decimal` to
//!   `rust_decimal::Decimal`.
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
//! # Entry points
//!
//! | Job | Call |
//! |---|---|
//! | Parse and generate in a `build.rs` | [`generate`], then [`GeneratedModuleSet::write_to_out_dir`] |
//! | Include the module | [`sbe_mod!`] |
//! | Parse only (inspect the token IR) | [`parse`] with a [`SchemaSource`] |
//! | Several schemas sharing types | [`parse`] each (consumers [`SchemaSource::with_shared`]), [`Generator::generate_multi`], [`GeneratedModuleSet::write_to_out_dir`] |
//!
//! # Quick start
//!
//! A doctest in this crate cannot use generated types, so the example below
//! runs the generator on an inline schema and checks the emitted source for
//! the encoder and length-builder names. For end-to-end encode and decode, see
//! [`samples/sbe-feature-tour`](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour)
//! and the golden file
//! [`sbe/tests/golden/car_example.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/golden/car_example.rs).
//!
//! ```rust
//! use ergo_sbe::{GenerationConfig, SchemaSource, generate};
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
//! let output = generate(SchemaSource::xml(schema_xml), GenerationConfig::new("my_messages"))
//!     .expect("generate");
//! let src = &output.modules().next().expect("one module").source;
//! assert!(src.contains("CarDecoder"));
//! assert!(src.contains("CarEncoder"));
//! assert!(src.contains("CarFixedFields"));
//! assert!(src.contains("wrap_and_apply_header"));
//! assert!(src.contains("compute_length_with_header") || src.contains("ENCODED_LENGTH"));
//! // in build.rs: output.write_to_out_dir()?, then ergo_sbe::sbe_mod!(my_messages);
//! ```
//!
//! # What gets generated
//!
//! Names depend on your schema; `Car` and `Engine` below are examples. The
//! feature-tour sample and the golden file show the full generated surface.
//!
//! ## Composites are wire images
//!
//! A generated composite is `#[repr(transparent)] struct Engine(pub [u8; N])`:
//! the value is the wire bytes. Accessors read and write them with
//! `from_le_bytes` / `to_le_bytes` at schema offsets, which costs nothing on a
//! little-endian host. Decoding uses a zero-copy flyweight
//! (`EngineDecoder { buf, pos }`). Do not transmute the buffer to a padded
//! `#[repr(C)]` struct: SBE fields are packed and may be unaligned. The crate
//! README section *Composite layout & little-endian* has the detail.
//!
//! ## Encoding and buffer sizing
//!
//! Size the buffer before you write. The generated helpers account for nested
//! groups and var-data of any length, so you never work out a wire length by
//! hand or guess with `vec![0u8; 4096]`. Use a stack array when the length is
//! `const`; otherwise size first, then claim or encode into a slot of exactly
//! that length.
//!
//! | Message shape | API | Buffer |
//! |-------|-----|--------|
//! | Fixed fields only | `HeartbeatEncoder::ENCODED_LENGTH` (const) | `[0u8; N]` on the stack |
//! | Flat groups and var-data | `compute_encoded_length_with_message_header(...)` | stack when const |
//! | Nested groups or entry var-data | `CarEncodedLength::new()…encoded_length_with_header()` | exact claim or slot |
//!
//! [`samples/sbe-feature-tour`](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs)
//! shows decoding, encoding and sizing on real types.
//!
//! ## Why wire order is checked at compile time
//!
//! SBE is positional. Two adjacent groups of the same shape, such as bids then
//! asks, are common in market data, and swapping them still produces valid
//! bytes that fail only in production. Each encoder stage is its own type, so
//! the wrong order does not compile.
//!
//! ## Field metadata
//!
//! The constants that mirror sbe-tool's Java metadata are exercised in
//! [`sbe/tests/java_parity_features_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/java_parity_features_test.rs).
//!
//! ## Conversion: `with_conversion` or `with_domain_type`
//!
//! Pick one style per selector; `with_domain_type` already enables conversion.
//!
//! | Config | Generated decode | Generated encode |
//! |--------|------------------|------------------|
//! | [`GenerationConfig::with_conversion`] | `dec.price_as::<T>()?` | `enc.price_from(&t)?` |
//! | [`GenerationConfig::with_domain_type`] | `dec.price()? -> path::Type` | `enc.price(value)?` |
//!
//! ```rust
//! use ergo_sbe::{ConversionSelector, GenerationConfig};
//!
//! // Pluggable: you implement TryFromSbe / TryToSbe.
//! let _a = GenerationConfig::new("msgs")
//!     .with_conversion(ConversionSelector::named_type("Decimal"));
//!
//! // One concrete Rust type, which implies conversion for the same selector.
//! let _b = GenerationConfig::new("msgs")
//!     .with_domain_type(
//!         ConversionSelector::named_type("Decimal"),
//!         "rust_decimal::Decimal",
//!     );
//! ```
//!
//! [`sbe/tests/comprehensive_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/comprehensive_test.rs)
//! covers conversions and domain objects.
//!
//! ## Domain objects
//!
//! [`GenerationConfig::with_domain_objects`] emits owned structs. Pass
//! [`DomainVarData::Strings`] for `String` var-data, where invalid UTF-8 is an
//! `InvalidUtf8` error, or [`DomainVarData::Bytes`] for `Vec<u8>`. See
//! [`sbe/tests/domain_objects_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/domain_objects_test.rs).
//!
//! ## Multi-message dispatch
//!
//! [`sbe/fuzz/fuzz_targets/any_message_frame_cursor.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/fuzz/fuzz_targets/any_message_frame_cursor.rs)
//! drives `AnyMessage` and `FrameCursor`.
//!
//! ## Fixed arrays and char fields
//!
//! See [`sbe/tests/java_parity_features_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/java_parity_features_test.rs).
//!
//! ## Keywords in schema names
//!
//! A field named `type` becomes `type_`. Change the appended `"_"` with
//! [`GenerationConfig::with_keyword_append_token`].
//!
//! ## XSD structural check
//!
//! [`validate_against_sbe_xsd`] checks a schema against the official XSD,
//! which is embedded as [`SBE_XSD`]; [`SchemaSource::with_xsd_validation`]
//! runs the same check inside [`parse`]. It is opt-in and checks shape only;
//! it is not a full W3C validator. [`parse`] itself always rejects malformed
//! XML, a bad root, unexpected elements and unknown attributes.
//!
//! # Design
//!
//! - Wire-compatible with official SBE and sbe-tool where tested.
//! - Idiomatic Rust (type-state tails, borrowing flyweights), not a Java port.
//! - No allocation on decode hot paths by default.
//! - Accessors respect `sinceVersion` and the acting version.
//! - `unsafe` only on the explicit `_unchecked` paths, each documented.
//!
//! Benchmark results are in the [benchmarks chapter](https://mimran1980.github.io/ergon/sbe/benchmarks.html).
//!
//! [sbe-spec]: https://www.fixtrading.org/standards/sbe/

/// Re-exported so `build.rs` can return [`miette::Result`] without an extra
/// dependency. Enable the crate's `fancy` feature for graphical rendering
/// (source snippet + span) instead of the plain fallback.
pub use miette;

/// Build-script entry points ([`generate`], [`sbe_mod!`]).
#[allow(
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::result_large_err
)]
pub mod build;
/// Codec generation ([`Generator`]).
// quote!-heavy emit code trips these style lints.
#[allow(
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
/// Generator options ([`GenerationConfig`]).
#[allow(
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
#[allow(clippy::pedantic, clippy::nursery, clippy::unnecessary_wraps)]
pub mod schema;
/// Structured IR for codegen (internal).
mod schema_attrs;
#[allow(clippy::pedantic, clippy::nursery, clippy::cast_lossless)]
pub(crate) mod structured_ir;
/// Schema parsing ([`parse`], [`SchemaSource`]).
#[allow(
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

pub use build::{BuildError, generate};
pub use codegen::{GenerateError, GeneratedModule, GeneratedModuleSet, Generator};
pub use config::{
    CompositeMapping, ConversionSelector, DomainImpl, DomainVarData, EnumMapping, EnumVariantInfo,
    FieldInfo, FieldKind, GenerationConfig, GenerationProfile, ItemContext, ItemKind,
    SetChoiceInfo,
};
pub use ir::{ByteOrder, Encoding, Ir, Presence, PrimitiveType, Signal, Token};
pub use resolve::{ResolveError, resolve_schema};
pub use schema::Schema;
pub use xml::{IncludeCause, ParseError, SchemaSource, parse};
pub use xsd::{SBE_XSD, XsdValidationError, validate_against_sbe_xsd};

/// Chrono timestamp converters (feature `chrono`).
///
/// Use [`GenerationConfig::with_domain_type`] with
/// `"chrono::DateTime<chrono::Utc>"` or `"chrono::NaiveDateTime"` to
/// generate fallible domain getters and setters that convert between SBE `i64`
/// wire values and chrono datetime types.
#[cfg(feature = "chrono")]
pub mod chrono_converters;

// Re-export optional dependencies so generated codecs can name the types
// without the consumer adding them directly. The matching codec methods
// (into_<field>_as_compact_str, etc.) are emitted only when the generator
// itself was built with the feature, so enable it on ergo-sbe in both
// `[build-dependencies]` and `[dependencies]`.
#[cfg(feature = "bytes")]
pub use bytes;
#[cfg(feature = "compact_str")]
pub use compact_str;
#[cfg(feature = "smol_str")]
pub use smol_str;

// Header-state markers (`HeaderPresent` / `HeaderAbsent`) live in each
// generated module's `sbe_rt` (see `generate_sbe_rt_src`). They are not
// re-exported here: generated codecs seal against their own `sbe_rt::HeaderState`,
// so a shared `ergo_sbe::header_state` type would not unify with `H`.
