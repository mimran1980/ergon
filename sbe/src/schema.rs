//! Schema handle for codegen: the resolved [`Ir`], which carries the package identity.
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
//! assert_eq!(schema.ir.package, "example");
//! assert_eq!(schema.ir.id, 1);
//! // Generator::new(config).generate(&schema)
//! ```

use crate::ir::{ByteOrder, Ir};

/// Parsed + resolved SBE schema ready for [`crate::Generator`].
///
/// Built with [`Schema::from_ir`] after [`crate::parse`] / [`crate::parse_file`].
///
/// Package, id and version live only on [`Ir`]: read `schema.ir.package`,
/// `schema.ir.id` and `schema.ir.version`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
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
        Self {
            ir: Ir {
                package: package.into(),
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
    /// assert_eq!(schema.ir.version, 0);
    /// ```
    #[must_use]
    pub fn from_ir(ir: Ir) -> Self {
        Self { ir }
    }
}

#[cfg(test)]
mod tests {
    use super::Schema;

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
        assert_eq!(schema.ir.package, "test_pkg");
        assert_eq!(schema.ir.id, 99);
        assert_eq!(schema.ir.version, 3);
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
