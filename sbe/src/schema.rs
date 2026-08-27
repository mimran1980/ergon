//! Normalised schema handle for codegen: package identity + resolved [`Ir`].
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
//! // Generator::new(config).generate(&schema)
//! ```

use crate::ir::{ByteOrder, Ir};

/// Parsed + resolved SBE schema ready for [`crate::Generator`].
///
/// Built with [`Schema::from_ir`] after [`crate::parse`] / [`crate::parse_file`].
///
/// Package, id, and version are copied from the IR at construction. A single
/// identity source (`Ir` only) is a 1.0 migration and is not shipped on 0.x.
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
    pub(crate) fn new(package: impl Into<String>, id: u16, version: u16) -> Self {
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

    /// Wrap a resolved [`Ir`] as a [`Schema`] for codegen.
    ///
    /// ```rust
    /// use ergo_sbe::{parse, Schema};
    /// # let xml = r#"<?xml version="1.0"?><messageSchema package="t" id="1" version="0"
    /// # byteOrder="littleEndian"><types><composite name="messageHeader">
    /// # <type name="blockLength" primitiveType="uint16"/>
    /// # <type name="templateId" primitiveType="uint16"/>
    /// # <type name="schemaId" primitiveType="uint16"/>
    /// # <type name="version" primitiveType="uint16"/>
    /// # </composite></types></messageSchema>"#;
    /// let schema = Schema::from_ir(parse(xml).unwrap());
    /// assert_eq!(schema.version, 0);
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
    use super::Schema;

    #[test]
    fn schema_metadata_preserves_identity_fields() -> Result<(), Box<dyn std::error::Error>> {
        let schema = Schema::new("fix.sbe", 42, 7);

        assert_eq!(schema.package, "fix.sbe");
        assert_eq!(schema.id, 42);
        assert_eq!(schema.version, 7);

        Ok(())
    }

    #[test]
    fn schema_from_ir_preserves_metadata() -> Result<(), Box<dyn std::error::Error>> {
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

        Ok(())
    }

    #[test]
    fn schema_new_has_correct_ir_defaults() -> Result<(), Box<dyn std::error::Error>> {
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

        Ok(())
    }
}
