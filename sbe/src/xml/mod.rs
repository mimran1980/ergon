//! SBE XML → token [`Ir`].
//!
//! One entry point, [`parse`], takes a [`SchemaSource`]: a file
//! ([`SchemaSource::file`], resolving `xi:include` relative to it) or inline
//! XML ([`SchemaSource::xml`]), optionally seeded from a shared schema or
//! XSD-checked first.
//!
//! After parse, wrap with [`crate::Schema::from_ir`] and pass to
//! [`crate::Generator`], or skip both with [`crate::generate`].
//!
//! ```rust
//! use ergo_sbe::{parse, Schema, SchemaSource};
//! let ir = parse(SchemaSource::xml(r#"<?xml version="1.0"?>
//! <messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
//!   <types>
//!     <composite name="messageHeader">
//!       <type name="blockLength" primitiveType="uint16"/>
//!       <type name="templateId" primitiveType="uint16"/>
//!       <type name="schemaId" primitiveType="uint16"/>
//!       <type name="version" primitiveType="uint16"/>
//!     </composite>
//!   </types>
//! </messageSchema>"#)).unwrap();
//! let schema = Schema::from_ir(ir);
//! assert_eq!(schema.ir.id, 1);
//! ```
//!
//! Errors are span-bearing [`ParseError`]s ([`miette`]).

mod attr;
mod entry;
mod error;
mod message;
mod registry;
mod schema;
mod types;
mod warn;

#[cfg(test)]
mod tests;

pub use entry::{SchemaSource, parse};
pub use error::{IncludeCause, ParseError};
pub(crate) use registry::parse_sbe_float;

// Test-visible re-exports of parse internals (xml is already #[doc(hidden)]).
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use error::{Fault, FaultKind};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use registry::{TypeRegistry, compute_type_size, parse_u64_val, resolve_type_to_tokens};
