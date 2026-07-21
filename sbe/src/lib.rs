#![warn(missing_docs)]
#![allow(unused)] // ponytail: pre-existing in 5600-line codegen.rs
#![allow(clippy::pedantic)] // ponytail: pre-existing in codegen (3000+ line file)
#![allow(clippy::nursery)] // ponytail: experimental lints on stable code
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

//! Opinionated, idiomatic Rust code generation for Simple Binary Encoding (SBE).
//!
//! [Simple Binary Encoding][sbe-spec] (SBE) is a wire-format designed for
//! low-latency financial messaging. It describes messages via XML schemas;
//! ErgoSBE reads those schemas and produces safe, fast, version-aware Rust
//! codecs.
//!
//! # Architecture
//!
//! The library is split into a small set of stable concepts:
//!
//! | Layer | Module | Responsibility |
//! |-------|--------|----------------|
//! | Schema Input | [`xml`], [`schema`] | Parse SBE XML, resolve includes, validate |
//! | Intermediate Repr | [`ir`], [`resolve`] | Normalised token stream, offset/block-length pass |
//! | Generation Options | [`config`] | Module name, wire-compatibility policy |
//! | Code Generation | [`codegen`] | Rust source production |
//!
//! # Quick-start
//!
//! Parse a schema, configure generation, and produce Rust source modules:
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
//!
//! let config = GenerationConfig::new("my_messages");
//! let generator = Generator::new(config);
//!
//! let output = generator.generate(&schema).unwrap();
//! assert!(output.modules().any(|m| m.path == "my_messages.rs"));
//! ```
//!
//! # Using generated code
//!
//! 1. Write an SBE XML schema (or use one from the official examples).
//! 2. In your `build.rs`, parse the schema and generate Rust source.
//! 3. In your Rust code, import the generated module.
//! 4. Decode: create a decoder from a buffer, read fields.
//! 5. Encode: create an encoder, set fields, write to a buffer.
//!
//! See the [getting-started docs](../docs/guide/getting-started.md) for full walkthroughs.
//!
//! # Design philosophy
//!
//! ErgoSBE is designed for **low-latency trading** systems. Key principles:
//!
//! - **Wire compatible**: generated bytes match official SBE byte-for-byte.
//! - **Idiomatic Rust**: not Java translated to Rust. Decoders are `Copy`
//!   flyweights; encoders use type-state for tail fields.
//! - **Zero allocation by default**: decoders borrow the input buffer.
//! - **Version-aware**: all accessors respect the wire message version.
//! - **No `unsafe` by default**: `unsafe` is opt-in via `_unchecked` methods.
//!
//! See [`design/DECISIONS.md`](https://github.com/mimran1980/ErgoSBE/blob/first_cut/sbe/design/DECISIONS.md)
//! for the full design rationale. Pillar overview: crate README under `sbe/`.
//!
//! [sbe-spec]: https://www.fixtrading.org/standards/sbe/

/// Rust source generation from a resolved [`Schema`].
pub mod codegen;
/// Generation options (module name, compatibility mode).
pub mod config;
/// Intermediate representation of SBE tokens and encodings.
pub mod ir;
/// Offset / block-length resolution pass over IR.
pub mod resolve;
/// High-level schema model built from IR.
pub mod schema;
/// SBE XML parse + XInclude resolution.
pub mod xml;

pub use codegen::{GenerateError, GeneratedModule, GeneratedModuleSet, Generator};
pub use config::{ConversionSelector, GenerationConfig};
pub use ir::{ByteOrder, Encoding, Ir, Presence, PrimitiveType, Signal, Token};
pub use resolve::{ResolveError, resolve_schema};
pub use schema::Schema;
pub use xml::{ParseError, parse, parse_file};
