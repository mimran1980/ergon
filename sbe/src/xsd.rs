//! Optional SBE XSD-aligned structural validation.
//!
//! Full W3C XSD evaluation needs a native libxml (or similar) binding. This
//! module instead:
//! 1. embeds the official FIX Protocol Limited `sbe.xsd`, and
//! 2. runs a pure-Rust structural check that mirrors the XSD's element model
//!    (root, allowed children, required attributes).
//!
//! Use this as a CI gate before codegen; the main parser remains the source of
//! truth for semantic IR construction.

use miette::Diagnostic;
use thiserror::Error;

/// Official FPL SBE 1.0 Draft Standard XSD, embedded for tooling / external
/// validators. Content matches Real Logic's `sbe-tool` resource
/// `fpl/sbe.xsd`.
pub const SBE_XSD: &str = include_str!("xsd/sbe.xsd");

/// Errors from [`validate_against_sbe_xsd`].
#[derive(Debug, Error, Diagnostic)]
pub enum XsdValidationError {
    /// Input is not well-formed XML.
    #[error("XML parse error: {0}")]
    #[diagnostic(code(ergo_sbe::xsd::malformed))]
    MalformedXml(String),

    /// Root element is not `messageSchema`.
    #[error("root element must be messageSchema, found `{found}`")]
    #[diagnostic(code(ergo_sbe::xsd::bad_root))]
    BadRoot {
        /// Tag that was found.
        found: String,
    },

    /// Required schema attribute is missing.
    #[error("messageSchema is missing required attribute `{attr}`")]
    #[diagnostic(code(ergo_sbe::xsd::missing_attr))]
    MissingAttribute {
        /// Attribute name.
        attr: &'static str,
    },

    /// Element is not allowed by the SBE XSD element model.
    #[error("element `{element}` is not allowed under `{parent}` by sbe.xsd")]
    #[diagnostic(code(ergo_sbe::xsd::unexpected_element))]
    UnexpectedElement {
        /// Parent element local name.
        parent: String,
        /// Child element local name.
        element: String,
    },

    /// Attribute is not recognised on this element.
    #[error("attribute `{attr}` is not allowed on `{element}` by sbe.xsd")]
    #[diagnostic(code(ergo_sbe::xsd::unexpected_attr))]
    UnexpectedAttribute {
        /// Element local name.
        element: String,
        /// Attribute name.
        attr: String,
    },
}

fn local_name(tag: &str) -> &str {
    tag.rsplit(':').next().unwrap_or(tag)
}

/// Validate `xml` against the SBE XSD element model (structural, pure Rust).
///
/// This is **not** a full XSD processor. It catches schema-shape mistakes
/// that the XSD would reject (wrong root, illegal children, unknown attrs on
/// core elements). Semantic checks (duplicate ids, type resolution, …) remain
/// in [`crate::parse`] / resolve.
pub fn validate_against_sbe_xsd(xml: &str) -> Result<(), XsdValidationError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| XsdValidationError::MalformedXml(e.to_string()))?;
    let root = doc.root_element();
    let root_name = local_name(root.tag_name().name());
    if root_name != "messageSchema" {
        return Err(XsdValidationError::BadRoot {
            found: root_name.to_string(),
        });
    }

    // XSD marks package/id/version as optional strings/ints, but practical SBE
    // schemas always carry them; require id + version like the Real Logic
    // parser effectively does for IR generation.
    for attr in ["id", "version"] {
        if root.attribute(attr).is_none() {
            return Err(XsdValidationError::MissingAttribute { attr });
        }
    }
    check_attrs(
        "messageSchema",
        root,
        &[
            "package",
            "id",
            "version",
            "semanticVersion",
            "description",
            "byteOrder",
            "headerType",
            "xmlns",
            "xsi",
        ],
    )?;

    for child in root.children().filter(|n| n.is_element()) {
        let name = local_name(child.tag_name().name());
        match name {
            "types" => validate_types(child)?,
            "message" => validate_message(child)?,
            // XInclude is outside the stock XSD but supported by both tools.
            "include" => {}
            other => {
                return Err(XsdValidationError::UnexpectedElement {
                    parent: "messageSchema".into(),
                    element: other.into(),
                });
            }
        }
    }
    Ok(())
}

fn check_attrs(
    element: &str,
    node: roxmltree::Node<'_, '_>,
    allowed: &[&str],
) -> Result<(), XsdValidationError> {
    for attr in node.attributes() {
        let name = local_name(attr.name());
        // Allow xmlns:* and xsi:* freely.
        if name.starts_with("xmlns") || attr.namespace().is_some_and(|ns| {
            ns.contains("XMLSchema-instance") || ns.contains("www.w3.org/2000/xmlns")
        }) {
            continue;
        }
        if attr.name().contains(':') {
            // Prefixed attrs (xsi:schemaLocation, etc.)
            continue;
        }
        if !allowed.contains(&name) {
            return Err(XsdValidationError::UnexpectedAttribute {
                element: element.into(),
                attr: name.into(),
            });
        }
    }
    Ok(())
}

fn validate_types(node: roxmltree::Node<'_, '_>) -> Result<(), XsdValidationError> {
    check_attrs("types", node, &[])?;
    for child in node.children().filter(|n| n.is_element()) {
        let name = local_name(child.tag_name().name());
        match name {
            "type" => check_attrs(
                "type",
                child,
                &[
                    "name",
                    "primitiveType",
                    "length",
                    "presence",
                    "nullValue",
                    "minValue",
                    "maxValue",
                    "characterEncoding",
                    "epoch",
                    "timeUnit",
                    "semanticType",
                    "description",
                    "sinceVersion",
                    "deprecated",
                    "offset",
                    "valueRef",
                ],
            )?,
            "composite" => validate_composite(child)?,
            "enum" => validate_enum(child)?,
            "set" => validate_set(child)?,
            other => {
                return Err(XsdValidationError::UnexpectedElement {
                    parent: "types".into(),
                    element: other.into(),
                });
            }
        }
    }
    Ok(())
}

fn validate_composite(node: roxmltree::Node<'_, '_>) -> Result<(), XsdValidationError> {
    check_attrs(
        "composite",
        node,
        &[
            "name",
            "description",
            "semanticType",
            "sinceVersion",
            "deprecated",
            "offset",
        ],
    )?;
    for child in node.children().filter(|n| n.is_element()) {
        let name = local_name(child.tag_name().name());
        match name {
            "type" | "enum" | "set" | "ref" | "composite" => {}
            "description" | "comment" => {}
            other => {
                return Err(XsdValidationError::UnexpectedElement {
                    parent: "composite".into(),
                    element: other.into(),
                });
            }
        }
    }
    Ok(())
}

fn validate_enum(node: roxmltree::Node<'_, '_>) -> Result<(), XsdValidationError> {
    check_attrs(
        "enum",
        node,
        &[
            "name",
            "encodingType",
            "description",
            "sinceVersion",
            "deprecated",
            "semanticType",
        ],
    )?;
    for child in node.children().filter(|n| n.is_element()) {
        let name = local_name(child.tag_name().name());
        match name {
            "validValue" => check_attrs(
                "validValue",
                child,
                &["name", "description", "sinceVersion", "deprecated"],
            )?,
            "description" | "comment" => {}
            other => {
                return Err(XsdValidationError::UnexpectedElement {
                    parent: "enum".into(),
                    element: other.into(),
                });
            }
        }
    }
    Ok(())
}

fn validate_set(node: roxmltree::Node<'_, '_>) -> Result<(), XsdValidationError> {
    check_attrs(
        "set",
        node,
        &[
            "name",
            "encodingType",
            "description",
            "sinceVersion",
            "deprecated",
            "semanticType",
        ],
    )?;
    for child in node.children().filter(|n| n.is_element()) {
        let name = local_name(child.tag_name().name());
        match name {
            "choice" => check_attrs(
                "choice",
                child,
                &["name", "description", "sinceVersion", "deprecated"],
            )?,
            "description" | "comment" => {}
            other => {
                return Err(XsdValidationError::UnexpectedElement {
                    parent: "set".into(),
                    element: other.into(),
                });
            }
        }
    }
    Ok(())
}

fn validate_message(node: roxmltree::Node<'_, '_>) -> Result<(), XsdValidationError> {
    check_attrs(
        "message",
        node,
        &[
            "name",
            "id",
            "description",
            "blockLength",
            "semanticType",
            "sinceVersion",
            "deprecated",
        ],
    )?;
    for child in node.children().filter(|n| n.is_element()) {
        let name = local_name(child.tag_name().name());
        match name {
            "field" => check_attrs(
                "field",
                child,
                &[
                    "name",
                    "id",
                    "type",
                    "description",
                    "offset",
                    "presence",
                    "valueRef",
                    "semanticType",
                    "sinceVersion",
                    "deprecated",
                    "epoch",
                    "timeUnit",
                ],
            )?,
            "group" => validate_group(child)?,
            "data" => check_attrs(
                "data",
                child,
                &[
                    "name",
                    "id",
                    "type",
                    "description",
                    "semanticType",
                    "sinceVersion",
                    "deprecated",
                ],
            )?,
            "description" | "comment" => {}
            other => {
                return Err(XsdValidationError::UnexpectedElement {
                    parent: "message".into(),
                    element: other.into(),
                });
            }
        }
    }
    Ok(())
}

fn validate_group(node: roxmltree::Node<'_, '_>) -> Result<(), XsdValidationError> {
    check_attrs(
        "group",
        node,
        &[
            "name",
            "id",
            "description",
            "dimensionType",
            "blockLength",
            "semanticType",
            "sinceVersion",
            "deprecated",
        ],
    )?;
    for child in node.children().filter(|n| n.is_element()) {
        let name = local_name(child.tag_name().name());
        match name {
            "field" | "group" | "data" | "description" | "comment" => {}
            other => {
                return Err(XsdValidationError::UnexpectedElement {
                    parent: "group".into(),
                    element: other.into(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_minimal_valid_schema() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
        <messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <type name="u32" primitiveType="uint32"/>
          </types>
          <message name="M" id="1">
            <field name="x" id="1" type="u32"/>
          </message>
        </messageSchema>"#;
        validate_against_sbe_xsd(xml)?;
        Ok(())
    }

    #[test]
    fn rejects_bad_root() {
        let xml = r#"<?xml version="1.0"?><notSchema id="1" version="0"/>"#;
        assert!(matches!(
            validate_against_sbe_xsd(xml),
            Err(XsdValidationError::BadRoot { .. })
        ));
    }

    #[test]
    fn rejects_unknown_message_child() {
        let xml = r#"<?xml version="1.0"?>
        <messageSchema id="1" version="0">
          <types/>
          <message name="M" id="1">
            <notAField name="x"/>
          </message>
        </messageSchema>"#;
        assert!(matches!(
            validate_against_sbe_xsd(xml),
            Err(XsdValidationError::UnexpectedElement { .. })
        ));
    }

    #[test]
    fn embedded_xsd_is_present() {
        assert!(SBE_XSD.contains("messageSchema"));
        assert!(SBE_XSD.contains("xs:schema"));
    }
}
