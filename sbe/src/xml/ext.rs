//! Extension-content capture: foreign-namespace attributes and elements.
//!
//! Classification is local-name-first (sbe-tool parity): a grammar local name
//! always wins, so foreign-prefixed grammar elements (`ns2:message`) keep
//! parsing as grammar. Only an unknown local name whose namespace is present
//! and differs from the owning document's root namespace (and is not the SBE
//! namespace itself) is extension content.
//! Unnamespaced unknowns stay rejected by the existing allow-lists.

use roxmltree::{Node, NodeType};

use crate::ir::{ExtensionAttr, ExtensionElement, Extensions};

/// Local names the SBE grammar owns. Matched first, in any namespace.
const GRAMMAR_LOCAL_NAMES: &[&str] = &[
    "messageSchema",
    "types",
    "message",
    "field",
    "group",
    "data",
    "enum",
    "set",
    "composite",
    "type",
    "ref",
    "validValue",
    "choice",
    "include",
    "description",
    "comment",
];

/// The SBE namespace URI: never extension content, so a misspelt grammar
/// name (`sbe:enumm`) is rejected whatever namespace the root carries.
const SBE_NAMESPACE: &str = "http://fixprotocol.io/2016/sbe";

/// True when `child` is extension content rather than SBE grammar.
///
/// Grammar local names match first. Otherwise the child must be namespaced
/// and sit in a namespace other than the document root's own and other than
/// the SBE namespace.
pub(crate) fn is_extension_element(child: Node<'_, '_>) -> bool {
    let local = child.tag_name().name();
    if GRAMMAR_LOCAL_NAMES.contains(&local) {
        return false;
    }
    let Some(ns) = child.tag_name().namespace() else {
        return false;
    };
    let root_ns = child.document().root_element().tag_name().namespace();
    ns != SBE_NAMESPACE && root_ns != Some(ns)
}

/// Extension attributes on a grammar node, in document order: every
/// namespaced attribute, and every unprefixed one the SBE grammar does not
/// define. On the elements `attr::reject_unknown_attrs` checks, typos of
/// grammar names were rejected earlier; on type-level elements they are
/// yielded as marks. `xmlns*` declarations are never yielded.
pub(crate) fn extension_attrs(node: Node<'_, '_>) -> Vec<ExtensionAttr> {
    node.attributes()
        .filter(|a| {
            if a.name().starts_with("xmlns") {
                return false;
            }
            a.namespace().is_some() || !crate::schema_attrs::is_grammar_attr(a.name())
        })
        .map(|a| ExtensionAttr {
            local_name: a.name().to_string(),
            namespace: a.namespace().map(str::to_string),
            value: a.value().to_string(),
        })
        .collect()
}

/// Capture one foreign element and its whole subtree verbatim.
fn capture_element(element: Node<'_, '_>) -> ExtensionElement {
    let attributes = element
        .attributes()
        .filter(|a| !a.name().starts_with("xmlns"))
        .map(|a| ExtensionAttr {
            local_name: a.name().to_string(),
            namespace: a.namespace().map(str::to_string),
            value: a.value().to_string(),
        })
        .collect();
    let mut text = String::new();
    let mut has_text = false;
    for child in element.children() {
        if child.node_type() == NodeType::Text {
            has_text = true;
            text.push_str(child.text().unwrap_or(""));
        }
    }
    let elements = element
        .children()
        .filter(|c| c.is_element())
        .map(capture_element)
        .collect();
    ExtensionElement {
        local_name: element.tag_name().name().to_string(),
        namespace: element.tag_name().namespace().map(str::to_string),
        attributes,
        text: has_text.then_some(text),
        elements,
    }
}

/// Extension content of one schema element: its extension attributes plus
/// foreign-namespace children in document order.
pub(crate) fn extension_content(node: Node<'_, '_>) -> Extensions {
    Extensions {
        attributes: extension_attrs(node),
        elements: node
            .children()
            .filter(|c| c.is_element() && is_extension_element(*c))
            .map(capture_element)
            .collect(),
    }
}

/// Append `from` onto `into` in walk order.
pub(crate) fn merge_extensions(into: &mut Extensions, from: Extensions) {
    into.attributes.extend(from.attributes);
    into.elements.extend(from.elements);
}
