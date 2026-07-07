//! Generated SBE types for dynamic schema/row messages.
//!
//! This module includes the ErgoSBE-generated Rust codecs for DynamicSchema
//! and DynamicRow. The generated file lives at `src/gen/persist_sbe.rs` and
//! is checked into the repo (regenerate via `cargo build -p ergosbe --example gen-persist`).
//!
//! # Types
//!
//! - [`DynamicSchemaDecoder`] / [`DynamicSchemaEncoder`] — register a table schema
//! - [`DynamicRowDecoder`] / [`DynamicRowEncoder`] — encode/decode a single row
//!
//! # Wire format
//!
//! Groups with variable-length string fields store their lengths in the
//! group entries and the actual string data in the trailing `symbolTable`
//! varData blob.  String fields in the symbolTable are packed sequentially:
//! string N's bytes are at offset `sum(lengths[0..N])` into the blob.
//!
//! See `sbe_schema.xml` for the full field layout.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(clippy::identity_op)]
#![allow(clippy::eq_op)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::manual_range_contains)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]

include!("gen/persist_sbe.rs");
