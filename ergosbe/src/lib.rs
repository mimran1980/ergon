#![warn(missing_docs)]

//! Opinionated, idiomatic Rust code generation for Simple Binary Encoding (SBE).
//!
//! [Simple Binary Encoding][sbe-spec] (SBE) is a wire-format designed for
//! low-latency financial messaging. It describes messages via XML schemas;
//! `ErgoSBE` reads those schemas and produces safe, fast, version-aware Rust
//! codecs.
//!
//! The library is split into a small set of stable concepts:
//!
//! | Layer | Crate module | Responsibility |
//! |-------|-------------|----------------|
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
//! use ergosbe::{parse, Generator, GenerationConfig, Schema};
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
//! let config = GenerationConfig::low_latency("my_messages");
//! let generator = Generator::new(config);
//!
//! let output = generator.generate(&schema);
//! assert!(output.modules().any(|m| m.path == "my_messages.rs"));
//! ```
//!
//! [sbe-spec]: https://www.fixtrading.org/standards/sbe/

pub mod codegen;
pub mod config;
pub mod ir;
pub mod resolve;
pub mod schema;
pub mod xml;

pub use codegen::{GeneratedModule, GeneratedModuleSet, Generator};
pub use config::{CompatibilityMode, GenerationConfig};
pub use ir::{ByteOrder, Encoding, Ir, Presence, PrimitiveType, Signal, Token};
pub use resolve::{ResolveError, resolve_schema};
pub use schema::{Schema, SchemaSource};
pub use xml::{ParseError, parse};
