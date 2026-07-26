#![warn(missing_docs)]
#![allow(unused)] // pre-existing in 5600-line codegen.rs
#![allow(clippy::pedantic)] // pre-existing in codegen (3000+ line file)
#![allow(clippy::nursery)] // experimental lints on stable code
#![allow(clippy::panic)] // codegen uses panic/expect for irrecoverable states
#![allow(clippy::too_many_arguments)] // codegen functions need many params
#![allow(clippy::manual_memcpy)] // explicit loop in codegen is intentional
#![allow(clippy::needless_range_loop)] // codegen uses index-based loops
#![allow(clippy::unnecessary_cast)] // schema value casting
#![allow(clippy::useless_format)] // codegen generates format strings
#![allow(clippy::items_after_statements)] // codegen structure
#![allow(clippy::explicit_counter_loop)] // codegen uses counter loops
#![allow(clippy::uninlined_format_args)] // pre-existing in old string template code
#![allow(clippy::unwrap_used)] // pre-existing in test helpers and config
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
#![allow(clippy::ref_option)] // pre-existing in IR model
#![allow(clippy::map_unwrap_or)] // pre-existing pattern
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
#![allow(clippy::option_if_let_else)] // pre-existing patterns
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
//! ## Decode flyweight
//!
//! ```ignore
//! let car = CarDecoder::try_from(buf)?;           // checks header + schema id
//! let n = car.serial_number();                    // u64
//! let year = car.model_year();                    // u16
//! let acting = car.acting_version();              // wire version
//! ```
//!
//! Trusted buffers (already validated): `CarDecoder::wrap(...)`.
//!
//! ## Encode + type-state tails
//!
//! Fixed fields first, then groups / var-data in **wire order** (compile-time
//! stages prevent out-of-order writes):
//!
//! ```ignore
//! // Const length → stack array (no heap).
//! let mut buf = [0u8; CarEncoder::ENCODED_LENGTH];
//! let done = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?
//!     .serial_number(1234)
//!     .model_year(2013)
//!     .fuel_figures(1, |g| {
//!         g.add(|e| { e.speed(30).mpg(35.5); Ok(()) })?;
//!         Ok(())
//!     })?
//!     .manufacturer(b"Honda")?;
//! let len = done.encoded_length_with_header();
//! ```
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
//! | Fixed-only | `CarEncoder::ENCODED_LENGTH` | `[0u8; N]` on the stack |
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
//! ```ignore
//! assert_eq!(CarDecoder::SERIAL_NUMBER_ID, 1);
//! assert_eq!(CarDecoder::SERIAL_NUMBER_ENCODING_OFFSET, 0);
//! CarDecoder::serial_number_meta_attribute(sbe_rt::MetaAttribute::Presence);
//! ```
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
//! Generated use after **A**:
//!
//! ```ignore
//! enc.price_from(&app_price)?;
//! let app: MyType = dec.price_as()?;
//! let wire = dec.price_value(); // wire composite still available
//! ```
//!
//! Generated use after **B**:
//!
//! ```ignore
//! enc.price(rust_decimal::Decimal::new(12345, 2));
//! let p: rust_decimal::Decimal = dec.price();
//! ```
//!
//! ## Domain objects
//!
//! [`GenerationConfig::enable_domain_objects`]`(`[`DomainVarData`]`)` emits
//! owned structs. Use [`DomainVarData::LossyStrings`] for text (`String`;
//! invalid UTF-8 → `""`) or [`DomainVarData::Bytes`] for `Vec<u8>`.
//!
//! ```ignore
//! // build.rs: .enable_domain_objects(DomainVarData::LossyStrings)
//! let dto = CarDomain::from(CarDecoder::try_from(buf)?);
//! assert_eq!(dto.manufacturer, "Honda");
//! let n = dto.encode(&mut out)?; // re-encode; range-checks min/max on integers
//! ```
//!
//! ## Multi-message dispatch
//!
//! ```ignore
//! match AnyMessage::decode_frame(buf, 0, frame_len)? {
//!     AnyMessage::Car(c) => { let _ = c.serial_number(); }
//!     AnyMessage::Heartbeat(h) => { let _ = h.sequence(); }
//! }
//! for frame in FrameCursor::new(stream, FramingPolicy::LengthPrefixedU32Le) {
//!     let _ = frame?;
//! }
//! ```
//!
//! ## Fixed arrays / char fields
//!
//! ```ignore
//! enc.put_some_numbers(1, 2, 3, 4);       // unrolled put for length 2..=8
//! enc.vehicle_code_str("abcdef")?;        // zero-padded; FixedArrayTooLong if too long
//! let n = dec.copy_vehicle_code(&mut dst);
//! ```
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
//! - Wire-compatible with official SBE / Aeron baselines where tested
//! - Idiomatic Rust (type-state tails, borrow flyweights) — not a Java port
//! - Zero allocation on decode hot paths by default
//! - Version-aware accessors (`sinceVersion` / acting version)
//! - Unsafe only on explicit `_unchecked` / documented paths
//!
//! Full narrative docs: [crate README](https://github.com/mimran1980/ergon/blob/first_cut/sbe/README.md).
//! Benchmarks: [BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/first_cut/sbe/BENCHMARKS.md).
//!
//! [sbe-spec]: https://www.fixtrading.org/standards/sbe/

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
    BuildError, generate_str_to_dir, generate_str_to_out_dir, generate_to_out_dir, out_dir,
};
pub use codegen::{GenerateError, GeneratedModule, GeneratedModuleSet, Generator};
pub use config::{ConversionSelector, DomainVarData, GenerationConfig};
pub use ir::{ByteOrder, Encoding, Ir, Presence, PrimitiveType, Signal, Token};
pub use resolve::{ResolveError, resolve_schema};
pub use schema::Schema;
pub use xml::{ParseError, parse, parse_file, parse_with_xsd_validation};
pub use xsd::{SBE_XSD, XsdValidationError, validate_against_sbe_xsd};
