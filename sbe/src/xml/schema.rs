//! Top-level `<messageSchema>` walk, `xi:include` loading, and header validation.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use roxmltree::{Document, Node};

use crate::ir::{ByteOrder, Encoding, Ir, Presence, PrimitiveType, Signal, Token};

use super::attr::{
    collect_description, element_children, opt_u16_attr, parse_byte_order, reject_unknown_attrs,
    string_attr, u16_attr, validate_sbe_name,
};
use super::error::{Fault, FaultKind, IncludeCause};
use super::message::parse_message;
use super::registry::{TypeRegistry, compute_type_size};
use super::types::parse_types_node;
use super::warn::{WarnState, warn_once};

/// DFS include graph: the stack is the in-flight visit path; `finished` files
/// are skipped (diamond/shared includes), not treated as cycles.
pub(crate) struct IncludeWalk {
    stack: Vec<PathBuf>,
    finished: HashSet<PathBuf>,
}

impl IncludeWalk {
    pub(crate) fn new() -> Self {
        Self {
            stack: Vec::new(),
            finished: HashSet::new(),
        }
    }

    pub(crate) fn with_root(root: PathBuf) -> Self {
        Self {
            stack: vec![root],
            finished: HashSet::new(),
        }
    }

    /// `Ok(true)` load, `Ok(false)` already ingested, `Err` cycle.
    fn classify(&self, canon: &Path) -> Result<bool, IncludeCause> {
        if self.stack.iter().any(|p| p == canon) {
            let mut chain = self.stack.clone();
            chain.push(canon.to_path_buf());
            return Err(IncludeCause::Cycle { chain });
        }
        if self.finished.contains(canon) {
            return Ok(false);
        }
        Ok(true)
    }
}

enum IncludeHit {
    Loaded(PathBuf, String),
    Skip,
}

pub(crate) fn parse_schema(
    root: Node<'_, '_>,
    base_dir: Option<&Path>,
    walk: &mut IncludeWalk,
    initial_registry: TypeRegistry,
    warn_state: &WarnState,
    dependencies: &mut Vec<PathBuf>,
) -> Result<Ir, Fault> {
    reject_unknown_attrs(root, "messageSchema", crate::schema_attrs::MESSAGE_SCHEMA)?;
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
                ingest_include(
                    href,
                    Some(child.range()),
                    base_dir,
                    walk,
                    &mut registry,
                    &mut tokens,
                    warn_state,
                    dependencies,
                )?;
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

/// Load one included document (and any includes it itself declares).
fn ingest_include(
    href: &str,
    span: Option<std::ops::Range<usize>>,
    base_dir: Option<&Path>,
    walk: &mut IncludeWalk,
    registry: &mut TypeRegistry,
    tokens: &mut Vec<Token>,
    warn_state: &WarnState,
    dependencies: &mut Vec<PathBuf>,
) -> Result<(), Fault> {
    let (resolved, included_content) = match read_include_file(href, base_dir, walk) {
        Ok(IncludeHit::Skip) => return Ok(()),
        Ok(IncludeHit::Loaded(path, content)) => (path, content),
        Err(mut fault) => {
            if fault.span.is_none() {
                fault.span = span.clone();
            }
            return Err(fault);
        }
    };
    if !dependencies.iter().any(|p| p == &resolved) {
        dependencies.push(resolved.clone());
    }
    walk.stack.push(resolved.clone());
    let result = ingest_included_document(
        href,
        span,
        resolved.parent(),
        &included_content,
        walk,
        registry,
        tokens,
        warn_state,
        dependencies,
    );
    walk.stack.pop();
    walk.finished.insert(resolved);
    result
}

fn ingest_included_document(
    href: &str,
    span: Option<std::ops::Range<usize>>,
    inc_base: Option<&Path>,
    included_content: &str,
    walk: &mut IncludeWalk,
    registry: &mut TypeRegistry,
    tokens: &mut Vec<Token>,
    warn_state: &WarnState,
    dependencies: &mut Vec<PathBuf>,
) -> Result<(), Fault> {
    let included_doc = Document::parse(included_content).map_err(|e| Fault {
        kind: FaultKind::Invalid {
            what: format!("included file {href}"),
            value: e.to_string(),
        },
        span: span.clone(),
    })?;
    let Some(inc_node) = included_doc.root().children().find(Node::is_element) else {
        return Ok(());
    };
    if inc_node.tag_name().name() == "types" {
        parse_types_node(inc_node, registry, tokens, warn_state)?;
        return Ok(());
    }
    for sub_child in element_children(inc_node) {
        match sub_child.tag_name().name() {
            "include" => {
                if let Some(nested) = sub_child.attribute("href") {
                    ingest_include(
                        nested,
                        Some(sub_child.range()),
                        inc_base,
                        walk,
                        registry,
                        tokens,
                        warn_state,
                        dependencies,
                    )?;
                }
            }
            "types" => parse_types_node(sub_child, registry, tokens, warn_state)?,
            _ => {}
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
/// Returns the resolved path and file contents, or `Err(Fault)` if a cycle
/// is detected or the file cannot be found. A file already fully ingested
/// in this walk is [`IncludeHit::Skip`] (shared/diamond include).
fn read_include_file(
    href: &str,
    base_dir: Option<&Path>,
    walk: &IncludeWalk,
) -> Result<IncludeHit, Fault> {
    fn is_not_found(f: &Fault) -> bool {
        matches!(
            &f.kind,
            FaultKind::Include {
                cause: IncludeCause::NotFound,
                ..
            }
        )
    }
    fn is_cycle(f: &Fault) -> bool {
        matches!(
            &f.kind,
            FaultKind::Include {
                cause: IncludeCause::Cycle { .. },
                ..
            }
        )
    }

    let mut attempted = Vec::new();
    let mut try_one = |raw: PathBuf| -> Result<IncludeHit, Fault> {
        attempted.push(raw.clone());
        match raw.canonicalize() {
            Ok(canon) => match walk.classify(&canon) {
                Err(cause) => Err(Fault::include(href, attempted.clone(), cause)),
                Ok(false) => Ok(IncludeHit::Skip),
                Ok(true) => match std::fs::read_to_string(&canon) {
                    Ok(content) => Ok(IncludeHit::Loaded(canon, content)),
                    Err(source) => Err(Fault::include(
                        href,
                        attempted.clone(),
                        IncludeCause::Io {
                            path: canon,
                            source,
                        },
                    )),
                },
            },
            Err(_) => match std::fs::read_to_string(&raw) {
                Ok(content) => Ok(IncludeHit::Loaded(raw, content)),
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => Err(
                    Fault::include(href, attempted.clone(), IncludeCause::NotFound),
                ),
                Err(source) => Err(Fault::include(
                    href,
                    attempted.clone(),
                    IncludeCause::Io { path: raw, source },
                )),
            },
        }
    };

    macro_rules! try_include {
        ($path:expr) => {
            match try_one($path) {
                Ok(ok) => return Ok(ok),
                Err(f) if is_cycle(&f) => return Err(f),
                Err(f) if is_not_found(&f) => {}
                Err(f) => return Err(f),
            }
        };
    }

    if let Some(dir) = base_dir {
        try_include!(dir.join(href));
    }
    try_include!(PathBuf::from(href));
    try_include!(PathBuf::from(format!("sbe/tests/fixtures/schemas/{href}")));
    try_include!(PathBuf::from(format!(
        "../sbe/tests/fixtures/schemas/{href}"
    )));

    Err(Fault::include(href, attempted, IncludeCause::NotFound))
}
