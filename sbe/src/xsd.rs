//! Optional SBE **XSD-shaped** structural validation (not a full W3C engine).
//!
//! | Item | Purpose |
//! |------|---------|
//! | [`SBE_XSD`] | Official FPL `sbe.xsd` text (for external tools / docs) |
//! | [`validate_against_sbe_xsd`] | Pure-Rust checks: root, children, known attributes |
//! | [`crate::parse_with_xsd_validation`] | Validate then [`crate::parse`] |
//!
//! Semantic IR rules (offsets, types, duplicates) still come from the main
//! parser / resolver.
//!
//! # Relationship to [`crate::parse`]
//!
//! [`parse`](crate::parse) is **always on** and rejects malformed XML, a bad
//! root, unexpected elements, and unknown attributes on its own. This module
//! is an **opt-in, deliberately stricter** gate for schema authors; it is
//! not a prerequisite for parsing.
//!
//! Attribute allow-lists are shared with the parser (one private
//! `schema_attrs` module owns them) so the two cannot drift apart.
//!
//! Where the published XSD is stricter than sbe-tool itself, sbe-tool wins:
//! the XSD marks `messageSchema/@version` `use="required"`, but upstream's own
//! test resources omit it, so requiring it here would reject schemas the
//! reference implementation accepts. Only `@id` is required.
//!
//! Vendor extensions the published XSD does not declare but sbe-tool accepts
//! (`characterEncoding` on `<data>`, `unit` on `<type>`, `jsonValue` on
//! `<validValue>` / `<choice>`, `package` on `<types>`) are permitted, as are
//! all namespaced attributes (`xsi:*`, `xi:*`, and vendor namespaces such as
//! Binance's `mbx:*`). Rejecting those would fail real-world schemas.

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
///
/// # Example
///
/// ```rust
/// use ergo_sbe::{validate_against_sbe_xsd, SBE_XSD};
/// # let xml = r#"<?xml version="1.0"?><messageSchema package="t" id="1" version="0"
/// # byteOrder="littleEndian"><types><composite name="messageHeader">
/// # <type name="blockLength" primitiveType="uint16"/>
/// # <type name="templateId" primitiveType="uint16"/>
/// # <type name="schemaId" primitiveType="uint16"/>
/// # <type name="version" primitiveType="uint16"/>
/// # </composite></types></messageSchema>"#;
/// // Also validates against the bundled SBE XSD:
/// validate_against_sbe_xsd(xml)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
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

    // `id` is required: without it there is no schema identity to encode.
    //
    // `version` is NOT required here even though the published XSD marks it
    // `use="required"`. sbe-tool's own test resources ship schemas that omit
    // it (e.g. `basic-schema.xml`, `new-order-single-schema.xml`), so the
    // reference implementation does not enforce it either, and `parse`
    // defaults it to 0. Enforcing it would reject upstream-mirrored fixtures
    // — a validator stricter than both the parser and the reference tool only
    // produces false positives.
    if root.attribute("id").is_none() {
        return Err(XsdValidationError::MissingAttribute { attr: "id" });
    }
    check_attrs("messageSchema", root, crate::schema_attrs::MESSAGE_SCHEMA)?;

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
        // Any namespaced attribute is outside the SBE grammar — `xsi:*`,
        // `xi:*`, and vendor extensions alike (Binance ships `mbx:exponent`).
        // Note `attr.name()` is the LOCAL name, so a `contains(':')` test
        // never fires; the namespace is what identifies these.
        if attr.namespace().is_some() {
            continue;
        }
        let name = local_name(attr.name());
        if name.starts_with("xmlns") {
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
    // `package` on <types> is not in the published XSD but sbe-tool emits and
    // accepts it (it scopes generated types for that block).
    check_attrs("types", node, &["package"])?;
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
                    // `unit` is an sbe-tool extension carried by real schemas.
                    "unit",
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
    check_attrs("enum", node, crate::schema_attrs::ENUM)?;
    for child in node.children().filter(|n| n.is_element()) {
        let name = local_name(child.tag_name().name());
        match name {
            "validValue" => check_attrs(
                "validValue",
                child,
                // `jsonValue` is an sbe-tool extension used by real schemas.
                &[
                    "name",
                    "description",
                    "sinceVersion",
                    "deprecated",
                    "jsonValue",
                ],
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
                // `jsonValue` is an sbe-tool extension used by real schemas.
                &[
                    "name",
                    "description",
                    "sinceVersion",
                    "deprecated",
                    "jsonValue",
                ],
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
    check_attrs("message", node, crate::schema_attrs::MESSAGE)?;
    for child in node.children().filter(|n| n.is_element()) {
        let name = local_name(child.tag_name().name());
        match name {
            "field" => check_attrs("field", child, crate::schema_attrs::FIELD_LIKE)?,
            "group" => validate_group(child)?,
            "data" => check_attrs("data", child, crate::schema_attrs::FIELD_LIKE)?,
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
    check_attrs("group", node, crate::schema_attrs::GROUP)?;
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
    fn accepts_enum_set_group_and_var_data_shapes() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
        <messageSchema package="t" id="1" version="0">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <composite name="groupSizeEncoding">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="numInGroup" primitiveType="uint16"/>
            </composite>
            <composite name="varStringEncoding">
              <type name="length" primitiveType="uint32"/>
              <type name="varData" primitiveType="uint8" length="0"/>
            </composite>
            <enum name="Side" encodingType="uint8">
              <validValue name="Buy">1</validValue>
              <validValue name="Sell">2</validValue>
            </enum>
            <set name="Flags" encodingType="uint8">
              <choice name="Firm">0</choice>
            </set>
          </types>
          <message name="Order" id="1">
            <field name="side" id="1" type="Side"/>
            <group name="fills" id="2" dimensionType="groupSizeEncoding">
              <field name="quantity" id="3" type="uint32"/>
              <data name="venue" id="4" type="varStringEncoding"/>
            </group>
            <data name="account" id="5" type="varStringEncoding"/>
          </message>
        </messageSchema>"#;

        validate_against_sbe_xsd(xml)?;
        Ok(())
    }

    #[test]
    fn rejects_malformed_missing_attributes_and_unknown_type_shapes() {
        type ValidationCase<'a> = (&'a str, &'a str, fn(&XsdValidationError) -> bool);
        let cases: [ValidationCase<'_>; 4] = [
            (
                "<messageSchema",
                "malformed XML",
                |error: &XsdValidationError| matches!(error, XsdValidationError::MalformedXml(_)),
            ),
            (
                r#"<messageSchema package="t" version="0"/>"#,
                "missing schema id",
                |error: &XsdValidationError| {
                    matches!(error, XsdValidationError::MissingAttribute { attr: "id" })
                },
            ),
            (
                r#"<messageSchema package="t" id="1" version="0" surprise="yes"/>"#,
                "unknown root attribute",
                |error: &XsdValidationError| {
                    matches!(error, XsdValidationError::UnexpectedAttribute { .. })
                },
            ),
            (
                r#"<messageSchema package="t" id="1" version="0"><types><unknown/></types></messageSchema>"#,
                "unknown type element",
                |error: &XsdValidationError| {
                    matches!(error, XsdValidationError::UnexpectedElement { .. })
                },
            ),
        ];

        for (xml, context, predicate) in cases {
            let result = validate_against_sbe_xsd(xml);
            assert!(
                result.as_ref().is_err_and(predicate),
                "{context}: {result:?}"
            );
        }
    }

    #[test]
    fn embedded_xsd_is_present() {
        assert!(SBE_XSD.contains("messageSchema"));
        assert!(SBE_XSD.contains("xs:schema"));
    }

    fn enum_null_schema(encoding: &str, null_value: &str) -> String {
        format!(
            r#"<?xml version="1.0"?>
            <messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
              <types>
                <composite name="messageHeader">
                  <type name="blockLength" primitiveType="uint16"/>
                  <type name="templateId" primitiveType="uint16"/>
                  <type name="schemaId" primitiveType="uint16"/>
                  <type name="version" primitiveType="uint16"/>
                </composite>
                <enum name="Code" encodingType="{encoding}" nullValue="{null_value}">
                  <validValue name="Ok">0</validValue>
                </enum>
              </types>
              <message name="M" id="1">
                <field name="code" id="1" type="Code"/>
              </message>
            </messageSchema>"#
        )
    }

    #[test]
    fn accepts_unsigned_enum_null_value() -> Result<(), Box<dyn std::error::Error>> {
        let xml = enum_null_schema("uint8", "99");
        validate_against_sbe_xsd(&xml)?;
        crate::parse_with_xsd_validation(&xml)?;
        Ok(())
    }

    #[test]
    fn accepts_signed_enum_null_value() -> Result<(), Box<dyn std::error::Error>> {
        let xml = enum_null_schema("int8", "-1");
        validate_against_sbe_xsd(&xml)?;
        crate::parse_with_xsd_validation(&xml)?;
        Ok(())
    }

    #[test]
    fn rejects_unknown_non_namespaced_enum_attribute() {
        let xml = r#"<?xml version="1.0"?>
        <messageSchema package="t" id="1" version="0">
          <types>
            <enum name="Code" encodingType="uint8" surprise="yes">
              <validValue name="Ok">0</validValue>
            </enum>
          </types>
        </messageSchema>"#;
        assert!(matches!(
            validate_against_sbe_xsd(xml),
            Err(XsdValidationError::UnexpectedAttribute {
                element,
                attr
            }) if element == "enum" && attr == "surprise"
        ));
    }
}
