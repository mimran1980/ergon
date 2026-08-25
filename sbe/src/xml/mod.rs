//! SBE XML → token [`Ir`](crate::Ir).
//!
//! | Function | Use when |
//! |----------|----------|
//! | [`parse`] | Schema already in a string |
//! | [`parse_file`] | Path on disk; resolves `xi:include` relative to the file |
//! | [`parse_with_xsd_validation`] | Same as [`parse`], after structural XSD check |
//!
//! After parse, wrap with [`crate::Schema::from_ir`] and pass to
//! [`crate::Generator`].
//!
//! ```rust
//! use ergo_sbe::{parse, Schema};
//! let ir = parse(r#"<?xml version="1.0"?>
//! <messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
//!   <types>
//!     <composite name="messageHeader">
//!       <type name="blockLength" primitiveType="uint16"/>
//!       <type name="templateId" primitiveType="uint16"/>
//!       <type name="schemaId" primitiveType="uint16"/>
//!       <type name="version" primitiveType="uint16"/>
//!     </composite>
//!   </types>
//! </messageSchema>"#).unwrap();
//! let schema = Schema::from_ir(ir);
//! assert_eq!(schema.id(), 1);
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

pub(crate) use entry::{ParsedFile, parse_file_with_deps, parse_file_with_shared_deps};
pub use entry::{
    parse, parse_file, parse_file_with_shared, parse_with_shared, parse_with_xsd_validation,
};
pub use error::{IncludeCause, ParseError};

// Test-visible re-exports of parse internals (xml is already #[doc(hidden)]).
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use error::{Fault, FaultKind};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use registry::{TypeRegistry, compute_type_size, parse_u64_val, resolve_type_to_tokens};
