//! SBE schema inputs and normalised schema metadata.
//!
//! # See also
//!
//! [`crate::parse`], [`crate::Generator`], [`crate::GenerationConfig`].
//!
//! Defines the [`Schema`] and [`SchemaSource`] types that represent
//! a parsed SBE schema at the ergo-sbe boundary. A `Schema` holds the
//! package identity (`package`, `id`, `version`) plus the resolved
//! token [`Ir`].
//!
//! # Usage
//!
//! ```rust
//! use ergo_sbe::{parse, Schema};
//!
//! let ir = parse(r#"<?xml version="1.0"?>
//! <messageSchema package="example" id="1" version="0"
//!                byteOrder="littleEndian">
//!   <types>
//!     <composite name="messageHeader">
//!       <type name="blockLength" primitiveType="uint16"/>
//!       <type name="templateId"   primitiveType="uint16"/>
//!       <type name="schemaId"     primitiveType="uint16"/>
//!       <type name="version"      primitiveType="uint16"/>
//!     </composite>
//!   </types>
//! </messageSchema>"#).unwrap();
//!
//! let schema = Schema::from_ir(ir);
//! assert_eq!(schema.package, "example");
//! assert_eq!(schema.id, 1);
//! ```
//!
//! # Schema creation
//!
//! - [`Schema::from_ir`] — from a parsed token IR.
//! - [`Schema::new`] — directly from metadata (when you already have the
//!   schema identity and will populate the IR separately).

use std::borrow::Cow;

/// Source input for an SBE schema.
///
/// Currently only supports XML. The `Cow` variant lets you pass either
/// borrowed or owned string content without unnecessary cloning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaSource<'a> {
    /// XML schema content held in memory.
    Xml(Cow<'a, str>),
}

impl<'a> SchemaSource<'a> {
    /// Build a schema source from borrowed XML.
    #[must_use]
    pub const fn borrowed_xml(xml: &'a str) -> Self {
        Self::Xml(Cow::Borrowed(xml))
    }

    /// Build a schema source from owned XML.
    #[must_use]
    pub const fn owned_xml(xml: String) -> Self {
        Self::Xml(Cow::Owned(xml))
    }
}

use crate::ir::{ByteOrder, Ir};

/// Normalised schema metadata after parsing and resolution.
///
/// Holds the schema's package identity and the full token IR that
/// the [`Generator`](crate::Generator) consumes to produce Rust code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    /// SBE package name from the XML schema (e.g. `"fix.sbe"`, `"baseline"`).
    pub package: String,
    /// SBE schema id — identifies this schema on the wire.
    pub id: u16,
    /// SBE schema version — incremented when the schema evolves.
    pub version: u16,
    /// Resolved token IR ready for code generation.
    pub ir: Ir,
}

impl Schema {
    /// Create schema metadata from identity fields.
    ///
    /// The resulting schema has an empty token IR. Use this when you
    /// plan to set the tokens manually, or for testing.
    #[must_use]
    pub fn new(package: impl Into<String>, id: u16, version: u16) -> Self {
        let package_str = package.into();
        Self {
            package: package_str.clone(),
            id,
            version,
            ir: Ir {
                package: package_str,
                id,
                version,
                byte_order: ByteOrder::LittleEndian,
                description: None,
                semantic_version: None,
                header_type: "messageHeader".to_string(),
                tokens: Vec::new(),
            },
        }
    }

    /// Create schema metadata from a parsed token IR.
    ///
    /// Typically the output of [`parse`](crate::parse):
    ///
    /// ```ignore
    /// use ergo_sbe::{parse, Schema};
    /// let ir = parse(schema_xml).unwrap();
    /// let schema = Schema::from_ir(ir);
    /// ```
    #[must_use]
    pub fn from_ir(ir: Ir) -> Self {
        Self {
            package: ir.package.clone(),
            id: ir.id,
            version: ir.version,
            ir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Schema, SchemaSource};

    #[test]
    fn schema_metadata_preserves_identity_fields() {
        let schema = Schema::new("fix.sbe", 42, 7);

        assert_eq!(schema.package, "fix.sbe");
        assert_eq!(schema.id, 42);
        assert_eq!(schema.version, 7);
    }

    #[test]
    fn schema_source_constructors_hold_xml() {
        let borrowed = SchemaSource::borrowed_xml("<messageSchema/>");
        let owned = SchemaSource::owned_xml("<messageSchema/>".to_string());
        match &borrowed {
            SchemaSource::Xml(cow) => assert!(matches!(cow, std::borrow::Cow::Borrowed(_))),
        }
        match &owned {
            SchemaSource::Xml(cow) => assert!(matches!(cow, std::borrow::Cow::Owned(_))),
        }
    }

    #[test]
    fn schema_from_ir_preserves_metadata() {
        use crate::ir::{ByteOrder, Ir};
        let ir = Ir {
            package: "test_pkg".to_string(),
            id: 99,
            version: 3,
            byte_order: ByteOrder::BigEndian,
            description: Some("test".to_string()),
            semantic_version: Some("1.0".to_string()),
            header_type: "customHeader".to_string(),
            tokens: Vec::new(),
        };
        let schema = Schema::from_ir(ir);
        assert_eq!(schema.package, "test_pkg");
        assert_eq!(schema.id, 99);
        assert_eq!(schema.version, 3);
        assert_eq!(schema.ir.header_type, "customHeader");
    }

    #[test]
    fn schema_new_has_correct_ir_defaults() {
        use crate::ir::ByteOrder;
        let schema = Schema::new("pkg", 1, 0);
        assert_eq!(schema.ir.package, "pkg");
        assert_eq!(schema.ir.id, 1);
        assert_eq!(schema.ir.version, 0);
        assert_eq!(schema.ir.byte_order, ByteOrder::LittleEndian);
        assert!(schema.ir.description.is_none());
        assert!(schema.ir.semantic_version.is_none());
        assert_eq!(schema.ir.header_type, "messageHeader");
        assert!(schema.ir.tokens.is_empty());
    }
}
