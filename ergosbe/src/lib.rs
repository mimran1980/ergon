//! Opinionated, idiomatic Rust code generation for Simple Binary Encoding.
//!
//! `ErgoSBE` is intentionally split into a small set of stable concepts:
//! schema input, generation options, and generated Rust modules. The first
//! releases will keep this API narrow while the SBE XML and IR layers settle.

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
