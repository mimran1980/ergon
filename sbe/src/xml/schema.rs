//! Top-level `<messageSchema>` walk, `xi:include` loading, and header validation.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use roxmltree::{Document, Node};

use crate::ir::{ByteOrder, Encoding, Ir, Presence, PrimitiveType, Signal, Token};

use super::attr::{
    collect_description, element_children, opt_u16_attr, parse_byte_order, string_attr, u16_attr,
    validate_sbe_name,
};
use super::error::{Fault, FaultKind};
use super::message::parse_message;
use super::registry::{TypeRegistry, compute_type_size};
use super::types::parse_types_node;
use super::warn::{WarnState, source_name, warn_once};

pub(crate) fn parse_schema(
    root: Node<'_, '_>,
    base_dir: Option<&Path>,
    seen: &mut HashSet<PathBuf>,
    initial_registry: TypeRegistry,
    warn_state: &WarnState,
) -> Result<Ir, Fault> {
    let package = string_attr(root, "package", "messageSchema @package")?;
    let id = u16_attr(root, "id", "messageSchema @id")?;
    let version = opt_u16_attr(root, "version", "messageSchema @version")?.unwrap_or(0);
    let byte_order = root
        .attribute("byteOrder")
        .map(parse_byte_order)
        .transpose()?
        .unwrap_or(ByteOrder::LittleEndian);

    let description = collect_description(root);
    let semantic_version = root.attribute("semanticVersion").map(str::to_string);
    let header_type = root
        .attribute("headerType")
        .unwrap_or("messageHeader")
        .to_string();

    let mut registry = initial_registry;
    let mut tokens = Vec::new();

    // First pass: Parse all types, including included files
    for child in element_children(root) {
        if child.tag_name().name() == "include" {
            if let Some(href) = child.attribute("href") {
                match read_include_file(href, base_dir, seen) {
                    Ok(included_content) => {
                        let included_doc = Document::parse(&included_content).map_err(|e| {
                            Fault::include_error(format!(
                                "failed to parse included file {href}: {e}"
                            ))
                        })?;
                        let included_root = included_doc.root().children().find(Node::is_element);
                        if let Some(inc_node) = included_root {
                            if inc_node.tag_name().name() == "types" {
                                parse_types_node(inc_node, &mut registry, &mut tokens, warn_state)?;
                            } else {
                                for sub_child in element_children(inc_node) {
                                    if sub_child.tag_name().name() == "types" {
                                        parse_types_node(
                                            sub_child,
                                            &mut registry,
                                            &mut tokens,
                                            warn_state,
                                        )?;
                                    }
                                }
                            }
                        }
                    }
                    Err(fault) => return Err(fault),
                }
            }
        } else if child.tag_name().name() == "types" {
            parse_types_node(child, &mut registry, &mut tokens, warn_state)?;
        } else if child.tag_name().name() == "message" {
            // messages are parsed in second pass
        } else {
            return Err(Fault::invalid(
                child,
                "messageSchema child",
                format!(
                    "unexpected element <{}> (expected <include>, <types>, or <message>)",
                    child.tag_name().name()
                ),
            ));
        }
    }

    // Gap 3: validate header type structure (must have the required fields).
    validate_header_type(&header_type, &registry)?;

    // Second pass: Parse all messages — also check for duplicate message names.
    let mut seen_message_names: HashSet<String> = HashSet::new();
    for child in element_children(root) {
        if child.tag_name().name() == "message" {
            let msg_name = string_attr(child, "name", "message @name")?;
            if !seen_message_names.insert(msg_name) {
                return Err(Fault::invalid(
                    child,
                    "duplicate message name",
                    string_attr(child, "name", "message @name")?,
                ));
            }
            parse_message(child, &header_type, &registry, &mut tokens, warn_state)?;
        }
    }

    Ok(Ir {
        package,
        id,
        version,
        byte_order,
        description,
        semantic_version,
        header_type,
        tokens,
    })
}

pub(crate) fn validate_header_type(
    header_type: &str,
    registry: &TypeRegistry,
) -> Result<(), Fault> {
    let tokens = match registry.registry.get(header_type) {
        Some(t) if !t.is_empty() && t[0].signal == Signal::BeginComposite => t,
        _ => {
            return Err(Fault {
                kind: FaultKind::Invalid {
                    what: "headerType".to_string(),
                    value: format!("{header_type}: expected a defined composite"),
                },
                span: None,
            });
        }
    };

    let fields: HashMap<&str, &Token> = tokens
        .iter()
        .filter(|t| t.signal == Signal::BeginField)
        .map(|t| (t.name.as_str(), t))
        .collect();

    for required_name in &["blockLength", "templateId", "schemaId", "version"] {
        let Some(field) = fields.get(required_name) else {
            return Err(Fault {
                kind: FaultKind::Invalid {
                    what: "headerType".to_string(),
                    value: format!("{header_type}: missing required field '{required_name}'"),
                },
                span: None,
            });
        };
        if !matches!(
            field.encoding.primitive_type,
            Some(
                PrimitiveType::UInt8
                    | PrimitiveType::UInt16
                    | PrimitiveType::UInt32
                    | PrimitiveType::UInt64
            )
        ) || field.encoding.length.unwrap_or(1) != 1
            || field.encoding.presence == Presence::Optional
        {
            return Err(Fault {
                kind: FaultKind::Invalid {
                    what: "headerType".to_string(),
                    value: format!(
                        "{header_type}.{required_name}: expected a required or constant scalar unsigned integer"
                    ),
                },
                span: None,
            });
        }
    }

    for count_name in ["numGroups", "numVarDataFields"] {
        if let Some(field) = fields.get(count_name)
            && (!matches!(
                field.encoding.primitive_type,
                Some(
                    PrimitiveType::UInt8
                        | PrimitiveType::UInt16
                        | PrimitiveType::UInt32
                        | PrimitiveType::UInt64
                )
            ) || field.encoding.length.unwrap_or(1) != 1
                || field.encoding.presence == Presence::Optional)
        {
            return Err(Fault {
                kind: FaultKind::Invalid {
                    what: "headerType".to_string(),
                    value: format!(
                        "{header_type}.{count_name}: expected a required or constant scalar unsigned integer"
                    ),
                },
                span: None,
            });
        }
    }
    Ok(())
}

/// Resolve an included schema file path.
///
/// Resolution order:
/// 1. Relative to `base_dir` (when provided)
/// 2. Direct path (CWD-relative)
/// 3. Well-known submodule paths for the ergon repo layout
///
/// Returns `Ok(Some(content))` on success, `Ok(None)` if the file cannot be
/// found (fails silently), or `Err(Fault)` if a cycle is detected.
pub(crate) fn read_include_file(
    href: &str,
    base_dir: Option<&Path>,
    seen: &mut HashSet<PathBuf>,
) -> Result<String, Fault> {
    // Helper: try reading a path, record canonical form in `seen`.
    // Returns Ok(content) on success, Err on failure (file not found, read error, or cycle).
    fn try_read(href: &str, seen: &mut HashSet<PathBuf>) -> Result<String, Fault> {
        let p = Path::new(href);
        if let Ok(canon) = p.canonicalize() {
            if !seen.insert(canon.clone()) {
                return Err(Fault::include_error(format!(
                    "cyclic include detected: {}",
                    canon.display()
                )));
            }
            std::fs::read_to_string(&canon)
                .map_err(|e| Fault::include_error(format!("cannot read {}: {e}", canon.display())))
        } else {
            std::fs::read_to_string(p)
                .map_err(|e| Fault::include_error(format!("cannot read {href}: {e}")))
        }
    }

    // Helper: propagate cycle errors immediately, retry on other errors.
    // Ponytail: checks the message field; a dedicated error variant would be cleaner
    // but this is a single-use helper in a 200-line function.
    fn is_cycle(f: &Fault) -> bool {
        match &f.kind {
            FaultKind::IncludeError { message } => message.contains("cyclic"),
            _ => false,
        }
    }

    macro_rules! try_include {
        ($expr:expr) => {
            match $expr {
                Ok(content) => return Ok(content),
                Err(f) if is_cycle(&f) => return Err(f),
                Err(_) => {} // file not found or read error → try next path
            }
        };
    }

    if let Some(dir) = base_dir {
        let candidate = dir.join(href).to_string_lossy().to_string();
        try_include!(try_read(&candidate, seen));
    }

    try_include!(try_read(href, seen));

    let paths = [
        format!("sbe/tests/fixtures/schemas/{}", href),
        format!("../sbe/tests/fixtures/schemas/{}", href),
    ];
    for p in &paths {
        try_include!(try_read(p, seen));
    }

    Err(Fault::include_error(format!(
        "include file not found: {href}"
    )))
}
