//! Normalised schema handle for codegen: identity is one [`Ir`].
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
//! assert_eq!(schema.package(), "example");
//! assert_eq!(schema.id(), 1);
//! // Generator::new(config).generate(&schema)
//! ```

use crate::ir::{ByteOrder, Ir};

/// Parsed + resolved SBE schema ready for [`crate::Generator`].
///
/// Built with [`Schema::from_ir`] after [`crate::parse`] / [`crate::parse_file`].
/// Package, id, and version are read from the single stored [`Ir`] so a
/// divergent identity is unrepresentable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    ir: Ir,
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
    /// assert_eq!(schema.version(), 0);
    /// ```
    #[must_use]
    pub fn from_ir(ir: Ir) -> Self {
        Self { ir }
    }

    /// SBE package name from the XML schema (e.g. `"fix.sbe"`, `"baseline"`).
    #[must_use]
    pub fn package(&self) -> &str {
        &self.ir.package
    }

    /// SBE schema id — identifies this schema on the wire.
    #[must_use]
    pub const fn id(&self) -> u16 {
        self.ir.id
    }

    /// SBE schema version — incremented when the schema evolves.
    #[must_use]
    pub const fn version(&self) -> u16 {
        self.ir.version
    }

    /// Resolved token IR ready for code generation.
    #[must_use]
    pub const fn ir(&self) -> &Ir {
        &self.ir
    }

    /// Mutate the single stored IR. Package/id/version accessors follow this
    /// value, so header validation, provenance comments, and `SCHEMA_HASH`
    /// cannot disagree with the IR used for layout.
    pub fn ir_mut(&mut self) -> &mut Ir {
        &mut self.ir
    }

    /// Consume the schema and return the stored IR.
    #[must_use]
    pub fn into_ir(self) -> Ir {
        self.ir
    }
}

#[cfg(test)]
mod tests {
    use super::Schema;

    #[test]
    fn schema_metadata_preserves_identity_fields() -> Result<(), Box<dyn std::error::Error>> {
        let schema = Schema::new("fix.sbe", 42, 7);

        assert_eq!(schema.package(), "fix.sbe");
        assert_eq!(schema.id(), 42);
        assert_eq!(schema.version(), 7);

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
        assert_eq!(schema.package(), "test_pkg");
        assert_eq!(schema.id(), 99);
        assert_eq!(schema.version(), 3);
        assert_eq!(schema.ir().header_type, "customHeader");

        Ok(())
    }

    #[test]
    fn schema_new_has_correct_ir_defaults() -> Result<(), Box<dyn std::error::Error>> {
        use crate::ir::ByteOrder;
        let schema = Schema::new("pkg", 1, 0);
        assert_eq!(schema.ir().package, "pkg");
        assert_eq!(schema.ir().id, 1);
        assert_eq!(schema.ir().version, 0);
        assert_eq!(schema.ir().byte_order, ByteOrder::LittleEndian);
        assert!(schema.ir().description.is_none());
        assert!(schema.ir().semantic_version.is_none());
        assert_eq!(schema.ir().header_type, "messageHeader");
        assert!(schema.ir().tokens.is_empty());

        Ok(())
    }

    #[test]
    fn ir_mut_is_the_only_identity_source() -> Result<(), Box<dyn std::error::Error>> {
        use crate::{GenerationConfig, Generator, parse};

        let xml = r#"<?xml version="1.0"?>
            <messageSchema package="orig" id="1" version="0" byteOrder="littleEndian">
              <types>
                <composite name="messageHeader">
                  <type name="blockLength" primitiveType="uint16"/>
                  <type name="templateId" primitiveType="uint16"/>
                  <type name="schemaId" primitiveType="uint16"/>
                  <type name="version" primitiveType="uint16"/>
                </composite>
              </types>
              <message name="M" id="1">
                <field name="v" id="1" type="uint8"/>
              </message>
            </messageSchema>"#;
        let mut schema = Schema::from_ir(parse(xml)?);
        schema.ir_mut().package = "mutated".into();
        schema.ir_mut().id = 42;
        schema.ir_mut().version = 9;
        assert_eq!(schema.package(), "mutated");
        assert_eq!(schema.id(), 42);
        assert_eq!(schema.version(), 9);
        assert_eq!(schema.ir().package, "mutated");
        assert_eq!(schema.into_ir().id, 42);

        let mut schema = Schema::from_ir(parse(xml)?);
        schema.ir_mut().package = "mutated".into();
        schema.ir_mut().id = 42;
        schema.ir_mut().version = 9;
        let src = Generator::new(GenerationConfig::new("mut_id"))
            .generate(&schema)?
            .into_parts()
            .0
            .into_iter()
            .next()
            .ok_or("generated module")?
            .source;
        assert!(
            src.contains("package `mutated` id 42 version 9"),
            "provenance must follow ir_mut: {src}"
        );
        assert!(
            src.contains("SCHEMA_ID: u16 = 42") || src.contains("SCHEMA_ID: u16 = 42u16"),
            "generated SCHEMA_ID must follow ir_mut: {src}"
        );
        Ok(())
    }
}
