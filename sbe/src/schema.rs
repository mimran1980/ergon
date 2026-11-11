//! Schema handle for codegen: the resolved [`Ir`], which carries the package identity.
//!
//! ```rust
//! use ergo_sbe::{parse, Schema, SchemaSource};
//!
//! let ir = parse(SchemaSource::xml(r#"<?xml version="1.0"?>
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
//! </messageSchema>"#)).unwrap();
//!
//! let schema = Schema::from_ir(ir);
//! assert_eq!(schema.ir.package, "example");
//! assert_eq!(schema.ir.id, 1);
//! // Generator::new(config).generate(&schema)
//! ```

use crate::ir::Ir;

/// Parsed + resolved SBE schema ready for [`crate::Generator`].
///
/// Built with [`Schema::from_ir`] after [`crate::parse`].
///
/// Package, id and version live only on [`Ir`]: read `schema.ir.package`,
/// `schema.ir.id` and `schema.ir.version`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    /// Resolved token IR ready for code generation.
    pub ir: Ir,
}

impl Schema {
    /// An empty little-endian schema for codegen unit tests.
    #[cfg(test)]
    pub(crate) fn new(package: impl Into<String>, id: u16, version: u16) -> Self {
        Self {
            ir: Ir {
                package: package.into(),
                id,
                version,
                byte_order: crate::ir::ByteOrder::LittleEndian,
                description: None,
                semantic_version: None,
                header_type: "messageHeader".to_string(),
                tokens: Vec::new(),
                sources: Vec::new(),
            },
        }
    }

    /// Wrap a resolved [`Ir`] as a [`Schema`] for codegen.
    ///
    /// ```rust
    /// use ergo_sbe::{parse, Schema, SchemaSource};
    /// # let xml = r#"<?xml version="1.0"?><messageSchema package="t" id="1" version="0"
    /// # byteOrder="littleEndian"><types><composite name="messageHeader">
    /// # <type name="blockLength" primitiveType="uint16"/>
    /// # <type name="templateId" primitiveType="uint16"/>
    /// # <type name="schemaId" primitiveType="uint16"/>
    /// # <type name="version" primitiveType="uint16"/>
    /// # </composite></types></messageSchema>"#;
    /// let schema = Schema::from_ir(parse(SchemaSource::xml(xml)).unwrap());
    /// assert_eq!(schema.ir.version, 0);
    /// ```
    #[must_use]
    pub const fn from_ir(ir: Ir) -> Self {
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
            sources: Vec::new(),
            tokens: Vec::new(),
        };
        let schema = Schema::from_ir(ir);
        assert_eq!(schema.ir.package, "test_pkg");
        assert_eq!(schema.ir.id, 99);
        assert_eq!(schema.ir.version, 3);
        assert_eq!(schema.ir.header_type, "customHeader");

        Ok(())
    }
}
