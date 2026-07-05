//! SBE schema inputs and normalized schema metadata.

use std::borrow::Cow;

/// Source input for an SBE schema.
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

/// Minimal normalized schema metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    /// SBE package name from the XML schema.
    pub package: String,
    /// SBE schema id.
    pub id: u16,
    /// SBE schema version.
    pub version: u16,
    /// SBE schema token IR.
    pub ir: Ir,
}

impl Schema {
    /// Create schema metadata.
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

    /// Create schema metadata from IR.
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
    fn schema_metadata_preserves_identity_fields() {
        let schema = Schema::new("fix.sbe", 42, 7);

        assert_eq!(schema.package, "fix.sbe");
        assert_eq!(schema.id, 42);
        assert_eq!(schema.version, 7);
    }
}
