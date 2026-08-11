//! Attribute, name, and small XML node helpers.

use roxmltree::{Node, NodeType};

use crate::ir::{ByteOrder, Encoding, Presence, PrimitiveType, Signal, Token};

use super::error::{Fault, FaultKind};
use super::registry::TypeRegistry;

pub(crate) fn is_primitive_name(s: &str) -> bool {
    matches!(
        s,
        "char"
            | "int8"
            | "uint8"
            | "int16"
            | "uint16"
            | "int32"
            | "uint32"
            | "int64"
            | "uint64"
            | "float"
            | "double"
    )
}

pub(crate) fn structural(
    name: &str,
    signal: Signal,
    span: Option<std::ops::Range<usize>>,
) -> Token {
    Token {
        id: None,
        name: name.to_string(),
        signal,
        encoding: Encoding::default(),
        span,
    }
}

pub(crate) fn element_children<'a, 'input>(
    node: Node<'a, 'input>,
) -> impl Iterator<Item = Node<'a, 'input>> {
    // Skip <description> and <comment> children — their text is already
    // collected by collect_description() which scans node.children() directly.
    node.children().filter(|c| {
        c.is_element() && c.tag_name().name() != "description" && c.tag_name().name() != "comment"
    })
}

/// Collect all documentation sources for an element and merge them into a
/// single description string. Handles:
///
/// - `description="..."` attribute
/// - `<description>text</description>` child element
/// - `<comment>text</comment>` child element/tag
/// - `<!-- ... -->` XML comments (nearest preceding siblings, not children)
///
/// Sources are combined in this deterministic order, space-separated.
/// Multi-line text and whitespace are preserved. Preceding-sibling XML
/// comments are associated with the nearest following element — they are
/// never duplicated to both container and child.
pub(crate) fn collect_description(node: Node<'_, '_>) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();

    if let Some(d) = node.attribute("description") {
        parts.push(d.trim().to_string());
    }

    // 2-3. <description> and <comment> child elements
    for child in node.children() {
        if child.is_element() {
            let name = child.tag_name().name();
            if name == "description" || name == "comment" {
                let text: String = child
                    .children()
                    .filter(|c| c.node_type() == NodeType::Text)
                    .filter_map(|c| c.text())
                    .collect::<String>();
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
        }
    }

    // 4. Preceding-sibling XML comments (nearest-element association).
    // Replaces the old child-comment scan so a comment before a child
    // element is associated with that child, not with both container
    // and child.
    parts.extend(preceding_xml_comments(node));

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

/// Collect XML comments that immediately precede `node` as siblings.
/// Walks previous siblings, collecting comments and skipping whitespace-only
/// text nodes, stopping at the first non-comment, non-whitespace element.
/// Comments are returned in document order.
pub(crate) fn preceding_xml_comments(node: Node<'_, '_>) -> Vec<String> {
    let mut comments = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(current) = sibling {
        match current.node_type() {
            NodeType::Comment => {
                if let Some(text) = current
                    .text()
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                {
                    comments.push(text.to_owned());
                }
            }
            NodeType::Text if current.text().is_some_and(|text| text.trim().is_empty()) => {}
            _ => break,
        }
        sibling = current.prev_sibling();
    }
    comments.reverse();
    comments
}

pub(crate) fn string_attr(node: Node<'_, '_>, name: &str, what: &str) -> Result<String, Fault> {
    node.attribute(name)
        .map(str::to_string)
        .ok_or_else(|| Fault::missing(node, what))
}

/// SBE / Rust-friendly identifier: `[A-Za-z_][A-Za-z0-9_]*`.
pub(crate) fn is_valid_sbe_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(crate) fn validate_sbe_name(node: Node<'_, '_>, name: &str, what: &str) -> Result<(), Fault> {
    if is_valid_sbe_name(name) {
        Ok(())
    } else {
        Err(Fault::invalid(
            node,
            what,
            format!("{name}: must match [A-Za-z_][A-Za-z0-9_]*"),
        ))
    }
}

pub(crate) fn reject_duplicate_type_name(
    node: Node<'_, '_>,
    name: &str,
    registry: &TypeRegistry,
) -> Result<(), Fault> {
    if registry.encodings.contains_key(name) || registry.registry.contains_key(name) {
        Err(Fault::invalid(
            node,
            "duplicate type name",
            format!("{name}: type/enum/set/composite already defined"),
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn u16_attr(node: Node<'_, '_>, name: &str, what: &str) -> Result<u16, Fault> {
    node.attribute(name)
        .ok_or_else(|| Fault::missing(node, what))
        .and_then(|s| s.parse::<u16>().map_err(|_| Fault::invalid(node, what, s)))
}

pub(crate) fn opt_u16_attr(
    node: Node<'_, '_>,
    name: &str,
    what: &str,
) -> Result<Option<u16>, Fault> {
    node.attribute(name)
        .map(|s| s.parse::<u16>().map_err(|_| Fault::invalid(node, what, s)))
        .transpose()
}

pub(crate) fn opt_usize_attr(
    node: Node<'_, '_>,
    name: &str,
    what: &str,
) -> Result<Option<usize>, Fault> {
    node.attribute(name)
        .map(|s| {
            s.parse::<usize>()
                .map_err(|_| Fault::invalid(node, what, s))
        })
        .transpose()
}

/// Parse the `deprecated` attribute as a non-negative schema version.
/// Returns `Ok(true)` when the attribute holds a valid non-negative u16;
/// `Ok(false)` when absent; and `Err` for non-numeric, negative, or
/// overflowing values.
pub(crate) fn parse_deprecated_attr(node: Node<'_, '_>) -> Result<bool, Fault> {
    match opt_u16_attr(node, "deprecated", "deprecated")? {
        Some(_version) => Ok(true),
        None => Ok(false),
    }
}

pub(crate) fn parse_byte_order(s: &str) -> Result<ByteOrder, Fault> {
    match s {
        "littleEndian" => Ok(ByteOrder::LittleEndian),
        "bigEndian" => Ok(ByteOrder::BigEndian),
        _ => Err(Fault {
            kind: FaultKind::Invalid {
                what: "byteOrder".to_string(),
                value: s.to_string(),
            },
            span: None,
        }),
    }
}

pub(crate) fn parse_presence(node: Node<'_, '_>, s: &str) -> Result<Presence, Fault> {
    match s {
        "required" => Ok(Presence::Required),
        "optional" => Ok(Presence::Optional),
        "constant" => Ok(Presence::Constant),
        _ => Err(Fault::invalid(node, "presence", s)),
    }
}

pub(crate) fn parse_primitive_type(node: Node<'_, '_>, s: &str) -> Result<PrimitiveType, Fault> {
    Ok(match s {
        "char" => PrimitiveType::Char,
        "int8" => PrimitiveType::Int8,
        "uint8" => PrimitiveType::UInt8,
        "int16" => PrimitiveType::Int16,
        "uint16" => PrimitiveType::UInt16,
        "int32" => PrimitiveType::Int32,
        "uint32" => PrimitiveType::UInt32,
        "int64" => PrimitiveType::Int64,
        "uint64" => PrimitiveType::UInt64,
        "float" => PrimitiveType::Float,
        "double" => PrimitiveType::Double,
        _ => return Err(Fault::invalid(node, "primitive type", s)),
    })
}
