#![warn(missing_docs)]
#![allow(unused)] // legacy codegen module still contains unused helpers
#![allow(clippy::pedantic)] // legacy codegen is being tightened incrementally
#![allow(clippy::nursery)] // experimental lints on stable code
#![allow(clippy::panic)] // codegen uses panic/expect for irrecoverable states
#![allow(clippy::too_many_arguments)] // codegen functions need many params
#![allow(clippy::manual_memcpy)] // explicit loop in codegen is intentional
#![allow(clippy::needless_range_loop)] // codegen uses index-based loops
#![allow(clippy::unnecessary_cast)] // schema value casting
#![allow(clippy::useless_format)] // codegen generates format strings
#![allow(clippy::items_after_statements)] // codegen structure
#![allow(clippy::explicit_counter_loop)] // codegen uses counter loops
#![allow(clippy::uninlined_format_args)] // legacy string-template code
#![allow(clippy::unwrap_used)] // legacy test helpers and config
#![allow(clippy::collapsible_if)] // intentional readability in codegen
#![allow(clippy::unreadable_literal)] // schema constants with specific bit patterns
#![allow(clippy::match_same_arms)] // SBE signal dispatch with matching bodies
#![allow(clippy::needless_borrow)] // explicit in generated code patterns
#![allow(clippy::use_self)] // codegen uses concrete type names
#![allow(clippy::missing_const_for_fn)] // runtime buffer ops cannot be const
#![allow(clippy::result_large_err)] // error types carry context
#![allow(clippy::similar_names)] // codegen variable naming
#![allow(clippy::redundant_clone)] // intentional clarity in codegen
#![allow(clippy::doc_markdown)] // SBE terms like blockLength are schema identifiers
#![allow(clippy::ref_option)] // legacy IR model API
#![allow(clippy::map_unwrap_or)] // legacy resolver pattern
#![allow(clippy::expect_used)] // expect() is intentional in codegen
#![allow(clippy::redundant_closure_for_method_calls)] // generated code
#![allow(clippy::unnecessary_unwrap)] // codegen uses unwrap_or pattern
#![allow(clippy::cast_lossless)] // u8/u32 -> u64 in IR is intentional
#![allow(clippy::cast_possible_truncation)] // checked by schema validation
#![allow(clippy::cast_sign_loss)] // schema validation ensures valid ranges
#![allow(clippy::cast_precision_loss)] // float conversions are explicit
#![allow(clippy::if_same_then_else)] // SBE signal dispatch patterns
#![allow(clippy::should_panic_without_expect)] // test patterns
#![allow(clippy::too_many_lines)] // codegen.rs is inherently large
#![allow(clippy::module_name_repetitions)] // codegen uses descriptive names
#![allow(clippy::option_if_let_else)] // legacy control-flow patterns
#![allow(clippy::match_wildcard_for_single_variants)] // exhaustive match
#![allow(clippy::single_match_else)] // semantic intent
#![allow(clippy::fn_params_excessive_bools)] // codegen parameter style
#![allow(clippy::cast_enum_constructor)] // SBE value construction
#![allow(clippy::ptr_as_ptr)] // pointer casts are explicit
#![allow(clippy::only_used_in_recursion)] // domain_types threaded through recursive codegen

//! Opinionated, idiomatic Rust code generation for Simple Binary Encoding (SBE).
//!
//! [Simple Binary Encoding][sbe-spec] describes messages in XML; ergo-sbe
//! parses those schemas and emits safe, version-aware Rust codecs for
//! low-latency trading.
//!
//! ## In a few words
//!
//! - **Compile-time wire order** — calling `asks` before `bids` is a type error
//! - **Closure-based groups** — nested shape mirrors the schema, no `.parent()` hopscotch
//! - **Exact buffer sizing** — no oversize scratch buffers; works directly with
//!   Aeron `try_claim`
//! - **Checked entry points** — `try_from` / `try_wrap` for untrusted input;
//!   `wrap` for trusted — explicit in the type system
//! - **Zero heap allocation** on generated hot paths; zero runtime dependencies
//! - **Domain types** — map wire `Decimal` to `rust_decimal::Decimal` with one
//!   line of config
//!
//! Full feature walkthrough: [crate README](https://github.com/mimran1980/ergon/blob/main/sbe/README.md).
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
//! # Quick-start (`build.rs`)
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
//! let ir = parse(schema_xml).unwrap();
//! let schema = Schema::from_ir(ir);
//! let output = Generator::new(GenerationConfig::new("my_messages"))
//!     .generate(&schema)
//!     .unwrap();
//! assert!(output.modules().any(|m| m.path == "my_messages.rs"));
//! // write module.source to OUT_DIR and `include!` it from your crate
//! ```
//!
//! # What gets generated (how to use it)
//!
//! Names depend on your schema (`Car` below is illustrative). Examples use
//! `ignore` because the types only exist after codegen.
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
//! | [`GenerationConfig::with_domain_type`] | `dec.price() -> path::Type` | `enc.price(value)` |
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
//! See [`sbe/converter`](https://github.com/mimran1980/ergon/blob/main/sbe/src/converter.rs)
//! and [`sbe/tests/conversion_selector_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/conversion_selector_test.rs)
//!
//! ## Domain objects
//!
//! [`GenerationConfig::enable_domain_objects`]`(`[`DomainVarData`]`)` emits
//! owned structs. Use [`DomainVarData::LossyStrings`] for text (`String`;
//! invalid UTF-8 → `""`) or [`DomainVarData::Bytes`] for `Vec<u8>`.
//!
//! See [`sbe/tests/domain_objects_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/domain_objects_test.rs)
//!
//! ## Multi-message dispatch
//!
//! See [`sbe/tests/frame_cursor_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/frame_cursor_test.rs)
//!
//! ## Fixed arrays / char fields
//!
//! See [`sbe/tests/fixed_array_helpers_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/fixed_array_helpers_test.rs)
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
//! Full narrative docs: [crate README](https://github.com/mimran1980/ergon/blob/first_cut/sbe/README.md).
//! Benchmarks: [BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/first_cut/sbe/BENCHMARKS.md).
//!
//! [sbe-spec]: https://www.fixtrading.org/standards/sbe/

/// Re-exported so `build.rs` can return [`miette::Result`] without an extra
/// dependency. Enable the crate's `fancy` feature for graphical rendering
/// (source snippet + span) instead of the plain fallback.
pub use miette;

/// Cargo `build.rs` helpers ([`generate_to_out_dir`], [`sbe_mod!`]).
pub mod build;
/// Codec generation ([`Generator`]).
pub mod codegen;
/// [`GenerationConfig`] — conversions, domain objects, keywords, etc.
pub mod config;
/// Token [`Ir`] (usually via [`Schema`]).
pub mod ir;
/// Offset resolution ([`resolve_schema`]; called by parse).
pub mod resolve;
/// [`Schema`] handle for codegen.
pub mod schema;
/// Structured IR for codegen (internal).
pub(crate) mod structured_ir;
/// XML parse ([`parse`], [`parse_file`]).
pub mod xml;
/// Optional XSD-shaped validation ([`validate_against_sbe_xsd`], [`SBE_XSD`]).
pub mod xsd;

pub use build::{
    BuildError, generate_str_to_dir, generate_str_to_out_dir, generate_to_dir, generate_to_out_dir,
    out_dir,
};
pub use codegen::{GenerateError, GeneratedModule, GeneratedModuleSet, Generator};
pub use config::{ConversionSelector, DomainVarData, GenerationConfig};
pub use ir::{ByteOrder, Encoding, Ir, Presence, PrimitiveType, Signal, Token};
pub use resolve::{ResolveError, resolve_schema};
pub use schema::Schema;
pub use xml::{
    ParseError, parse, parse_file, parse_file_with_shared, parse_with_shared,
    parse_with_xsd_validation,
};
pub use xsd::{SBE_XSD, XsdValidationError, validate_against_sbe_xsd};
