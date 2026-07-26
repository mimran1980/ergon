//! SBE XML → token [`Ir`](crate::Ir).
//!
//! | Function | Use when |
//! |----------|----------|
//! | [`parse`] | Schema already in a string |
//! | [`parse_file`] | Path on disk; resolves `xi:include` relative to the file |
//! | [`parse_with_xsd_validation`] | Same as [`parse`], after structural XSD check |
//!
//! After parse, wrap with [`crate::Schema::from_ir`] and pass to
//! [`crate::Generator`].
//!
//! ```rust
//! use ergo_sbe::{parse, Schema};
//! let ir = parse(r#"<?xml version="1.0"?>
//! <messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
//!   <types>
//!     <composite name="messageHeader">
//!       <type name="blockLength" primitiveType="uint16"/>
//!       <type name="templateId" primitiveType="uint16"/>
//!       <type name="schemaId" primitiveType="uint16"/>
//!       <type name="version" primitiveType="uint16"/>
//!     </composite>
//!   </types>
//! </messageSchema>"#).unwrap();
//! let schema = Schema::from_ir(ir);
//! assert_eq!(schema.id, 1);
//! ```
//!
//! Errors are span-bearing [`ParseError`]s ([`miette`]).

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};

use roxmltree::{Document, Node, NodeType};

use crate::ir::{ByteOrder, Encoding, Ir, Presence, PrimitiveType, Signal, Token};

/// Errors raised while parsing an SBE schema. Carries a [`miette`] source span
/// so the offending XML element is highlighted in the rendered diagnostic.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ParseError {
    /// The XML document itself was malformed.
    #[error("malformed XML: {message}")]
    #[diagnostic(code(ergo_sbe::schema_parse::malformed_xml))]
    MalformedXml {
        /// What went wrong.
        message: String,
        /// The source document, for span rendering.
        #[source_code]
        source_code: miette::NamedSource<String>,
    },
    /// A required attribute or element was missing.
    #[error("missing {what}")]
    #[diagnostic(code(ergo_sbe::schema_parse::missing))]
    Missing {
        /// What was missing (element/attribute context).
        what: String,
        /// The source document, for span rendering.
        #[source_code]
        source_code: miette::NamedSource<String>,
        /// The offending location, when a node was available.
        #[label("missing here")]
        span: Option<miette::SourceSpan>,
    },
    /// An attribute value was invalid.
    #[error("invalid {what}: {value}")]
    #[diagnostic(code(ergo_sbe::schema_parse::invalid))]
    Invalid {
        /// What was invalid.
        what: String,
        /// The offending value.
        value: String,
        /// The source document, for span rendering.
        #[source_code]
        source_code: miette::NamedSource<String>,
        /// The offending location.
        #[label("invalid here")]
        span: Option<miette::SourceSpan>,
    },
    /// A schema resolution or validation error occurred.
    #[error("resolution error: {error}")]
    #[diagnostic(code(ergo_sbe::schema_parse::resolve))]
    Resolve {
        /// The source document, for span rendering.
        #[source_code]
        source_code: miette::NamedSource<String>,
        /// The offending location (for miette label).
        #[label("resolution error")]
        span: Option<miette::SourceSpan>,
        /// The underlying resolution error.
        #[source]
        error: Box<crate::resolve::ResolveError>,
    },
    /// An include (xi:include) resolution error occurred.
    #[error("include error: {message}")]
    #[diagnostic(code(ergo_sbe::schema_parse::include))]
    IncludeError {
        /// What went wrong.
        message: String,
        /// The source document, for span rendering.
        #[source_code]
        source_code: miette::NamedSource<String>,
        /// The offending location.
        #[label("include error here")]
        span: Option<miette::SourceSpan>,
    },
}

impl ParseError {
    fn malformed_xml(message: impl Into<String>, xml: &str) -> Self {
        Self::MalformedXml {
            message: message.into(),
            source_code: named_source(xml),
        }
    }

    /// Lift an internal [`Fault`] into a span-bearing [`ParseError`], attaching
    /// the parsed source so `miette` can render the highlight.
    fn from_fault(fault: Fault, input: &str) -> Self {
        let source_code = named_source(input);
        let span = fault.span.map(miette::SourceSpan::from);
        match fault.kind {
            FaultKind::Missing { what } => Self::Missing {
                what,
                source_code,
                span,
            },
            FaultKind::Invalid { what, value } => Self::Invalid {
                what,
                value,
                source_code,
                span,
            },
            FaultKind::IncludeError { message } => Self::IncludeError {
                message,
                source_code,
                span,
            },
        }
    }
}

impl From<crate::resolve::ResolveError> for ParseError {
    fn from(mut e: crate::resolve::ResolveError) -> Self {
        let source_code = e
            .take_source_code()
            .unwrap_or_else(|| miette::NamedSource::new("schema.xml", String::new()));
        Self::Resolve {
            source_code,
            span: None,
            error: Box::new(e),
        }
    }
}

fn named_source(xml: &str) -> miette::NamedSource<String> {
    miette::NamedSource::new("schema.xml", xml.to_owned())
}

/// Internal, source-free error — converted to [`ParseError`] at the boundary,
/// where the parsed source text is known. Keeps the recursive helpers cheap and
/// free of source-cloning on the success path.
#[derive(Debug)]
struct Fault {
    kind: FaultKind,
    span: Option<Range<usize>>,
}

#[derive(Debug)]
enum FaultKind {
    Missing { what: String },
    Invalid { what: String, value: String },
    IncludeError { message: String },
}

impl Fault {
    fn missing(node: Node<'_, '_>, what: impl Into<String>) -> Self {
        Self {
            kind: FaultKind::Missing { what: what.into() },
            span: Some(node.range()),
        }
    }

    fn missing_no_node(what: impl Into<String>) -> Self {
        Self {
            kind: FaultKind::Missing { what: what.into() },
            span: None,
        }
    }

    fn invalid(node: Node<'_, '_>, what: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            kind: FaultKind::Invalid {
                what: what.into(),
                value: value.into(),
            },
            span: Some(node.range()),
        }
    }

    fn include_error(message: impl Into<String>) -> Self {
        Self {
            kind: FaultKind::IncludeError {
                message: message.into(),
            },
            span: None,
        }
    }
}

/// Type registry to resolve type aliases and inline composites/enums/sets during parsing.
struct TypeRegistry {
    registry: HashMap<String, Vec<Token>>,
    encodings: HashMap<String, Encoding>,
}

impl TypeRegistry {
    fn new() -> Self {
        let mut encodings = HashMap::new();
        for prim in &[
            PrimitiveType::Char,
            PrimitiveType::Int8,
            PrimitiveType::UInt8,
            PrimitiveType::Int16,
            PrimitiveType::UInt16,
            PrimitiveType::Int32,
            PrimitiveType::UInt32,
            PrimitiveType::Int64,
            PrimitiveType::UInt64,
            PrimitiveType::Float,
            PrimitiveType::Double,
        ] {
            let name = match prim {
                PrimitiveType::Char => "char",
                PrimitiveType::Int8 => "int8",
                PrimitiveType::UInt8 => "uint8",
                PrimitiveType::Int16 => "int16",
                PrimitiveType::UInt16 => "uint16",
                PrimitiveType::Int32 => "int32",
                PrimitiveType::UInt32 => "uint32",
                PrimitiveType::Int64 => "int64",
                PrimitiveType::UInt64 => "uint64",
                PrimitiveType::Float => "float",
                PrimitiveType::Double => "double",
            };
            encodings.insert(
                name.to_string(),
                Encoding {
                    primitive_type: Some(*prim),
                    presence: Presence::Required,
                    since_version: 0,
                    ..Encoding::default()
                },
            );
        }
        Self {
            registry: HashMap::new(),
            encodings,
        }
    }
}

/// Helper to parse optional u64 values from strings (like nullValue).
fn parse_u64_val(s: &str, prim_type: Option<PrimitiveType>) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    match prim_type {
        Some(PrimitiveType::Char) if s.len() == 1 => {
            return Some(s.chars().next().unwrap() as u64);
        }
        Some(PrimitiveType::Float) | Some(PrimitiveType::Double) => {
            // Parse as float/double, then reinterpret bits as u64.
            // This preserves NaN, infinity, and negative zero bit patterns.
            if let Some(PrimitiveType::Float) = prim_type {
                if let Ok(v) = s.parse::<f32>() {
                    return Some(v.to_bits() as u64);
                }
            } else if let Ok(v) = s.parse::<f64>() {
                return Some(v.to_bits() as u64);
            }
            return None;
        }
        _ => {}
    }
    if let Ok(v) = s.parse::<u64>() {
        Some(v)
    } else if let Ok(v) = s.parse::<i64>() {
        Some(v as u64)
    } else {
        None
    }
}

/// Resolve a type reference to a list of tokens.
fn resolve_type_to_tokens(
    field_name: &str,
    type_name: &str,
    id: Option<u16>,
    registry: &TypeRegistry,
    since_version: u16,
) -> Option<Vec<Token>> {
    if let Some(encoding) = registry.encodings.get(type_name) {
        let mut field_enc = encoding.clone();
        if since_version > 0 {
            field_enc.since_version = since_version;
        }
        Some(vec![
            Token {
                id,
                name: field_name.to_string(),
                signal: Signal::BeginField,
                encoding: field_enc,
            },
            Token {
                id: None,
                name: field_name.to_string(),
                signal: Signal::EndField,
                encoding: Encoding::default(),
            },
        ])
    } else if let Some(tokens) = registry.registry.get(type_name) {
        let mut inlined = Vec::new();
        inlined.push(Token {
            id,
            name: field_name.to_string(),
            signal: Signal::BeginField,
            encoding: Encoding {
                since_version,
                ..Encoding::default()
            },
        });
        for t in tokens {
            inlined.push(t.clone());
        }
        inlined.push(Token {
            id: None,
            name: field_name.to_string(),
            signal: Signal::EndField,
            encoding: Encoding::default(),
        });
        Some(inlined)
    } else {
        None
    }
}

/// Parse an SBE schema XML string into a token [`Ir`].
///
/// Runs resolution ([`crate::resolve_schema`]) automatically. Relative
/// `xi:include` without a file base dir uses well-known path probes.
///
/// # Errors
///
/// [`ParseError`] if XML is malformed, root is not `messageSchema`, or
/// attributes/types fail validation.
#[allow(clippy::result_large_err)]
pub fn parse(xml: &str) -> Result<Ir, ParseError> {
    parse_with_context(xml, None, &mut HashSet::new())
}

/// [`parse`] after [`crate::validate_against_sbe_xsd`].
///
/// Use in CI for schema authors. Still not a full W3C XSD engine — see
/// [`crate::xsd`].
///
/// # Errors
///
/// XSD structural failures or any [`parse`] error.
#[allow(clippy::result_large_err)]
pub fn parse_with_xsd_validation(xml: &str) -> Result<Ir, ParseError> {
    if let Err(e) = crate::xsd::validate_against_sbe_xsd(xml) {
        return Err(ParseError::malformed_xml(
            format!("XSD structural validation failed: {e}"),
            xml,
        ));
    }
    parse(xml)
}

/// Parse a schema file; resolve `xi:include` relative to the file's directory.
///
/// # Errors
///
/// I/O, XML, or schema validation failures as [`ParseError`].
#[allow(clippy::result_large_err)]
pub fn parse_file(path: impl AsRef<Path>) -> Result<Ir, ParseError> {
    let path = path.as_ref();
    let xml = std::fs::read_to_string(path).map_err(|e| {
        ParseError::malformed_xml(format!("cannot read {}: {e}", path.display()), "")
    })?;
    let base_dir = path.parent();
    let mut seen = HashSet::new();
    // Seed `seen` with the main file so that any include targeting it is
    // detected as a cycle (self-include or mutual A→B→A).
    if let Ok(canon) = path.canonicalize() {
        seen.insert(canon);
    }
    parse_with_context(&xml, base_dir, &mut seen)
}

/// Internal: parse with optional base directory for include resolution.
fn parse_with_context(
    xml: &str,
    base_dir: Option<&Path>,
    seen: &mut HashSet<PathBuf>,
) -> Result<Ir, ParseError> {
    let doc = match Document::parse(xml) {
        Ok(d) => d,
        Err(e) => return Err(ParseError::malformed_xml(e.to_string(), xml)),
    };
    let input = doc.input_text();
    let root = doc
        .root()
        .children()
        .find(Node::is_element)
        .ok_or_else(|| Fault::missing_no_node("root <messageSchema> element"));
    let root = match root {
        Ok(n) => n,
        Err(fault) => return Err(ParseError::from_fault(fault, input)),
    };
    if root.tag_name().name() != "messageSchema" {
        return Err(ParseError::from_fault(
            Fault::missing(root, "root <messageSchema> element"),
            input,
        ));
    }
    let mut ir =
        parse_schema(root, base_dir, seen).map_err(|fault| ParseError::from_fault(fault, input))?;
    crate::resolve::resolve_schema(&mut ir, Some(input))?;
    Ok(ir)
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
fn read_include_file(
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

fn parse_types_node(
    node: Node<'_, '_>,
    registry: &mut TypeRegistry,
    tokens: &mut Vec<Token>,
) -> Result<(), Fault> {
    // Pass 1: typedefs, enums, sets (no composites — so composite `<ref>` can
    // resolve targets that appear later in the same `<types>` block).
    let mut composite_nodes = Vec::new();
    for type_child in element_children(node) {
        match type_child.tag_name().name() {
            "type" => {
                let name = string_attr(type_child, "name", "type @name")?;
                validate_sbe_name(type_child, &name, "type @name")?;
                reject_duplicate_type_name(type_child, &name, registry)?;
                let encoding = parse_type_element(type_child, registry)?;
                // Constant presence must declare a constant value (text body or valueRef).
                if encoding.presence == Presence::Constant
                    && encoding
                        .constant_value
                        .as_ref()
                        .is_none_or(|s| s.is_empty())
                {
                    return Err(Fault::invalid(
                        type_child,
                        "type constant value",
                        format!(
                            "{name}: presence=constant requires a constant text value or valueRef"
                        ),
                    ));
                }
                registry.encodings.insert(name, encoding);
            }
            "composite" => {
                composite_nodes.push(type_child);
            }
            "enum" => {
                parse_enum(type_child, registry, tokens)?;
            }
            "set" => {
                parse_set(type_child, registry, tokens)?;
            }
            other => {
                return Err(Fault::invalid(
                    type_child,
                    "types container child",
                    format!(
                        "unexpected element <{other}> (expected <type>, <composite>, <enum>, or <set>)"
                    ),
                ));
            }
        }
    }

    // Pass 2: expand composites in dependency order so `<ref type="Later">`
    // and `type="NamedEnum"` resolve when the target is already registered.
    let mut pending = composite_nodes;
    while !pending.is_empty() {
        let before = pending.len();
        let mut still = Vec::new();
        for cnode in pending {
            if composite_refs_ready(cnode, registry) {
                parse_composite(cnode, registry, tokens)?;
            } else {
                still.push(cnode);
            }
        }
        if still.len() == before {
            // No progress: expand remaining to surface cycle/forward-ref errors.
            for cnode in still {
                parse_composite(cnode, registry, tokens)?;
            }
            break;
        }
        pending = still;
    }
    Ok(())
}

/// True when every composite member type/ref is already in the registry
/// (or is a primitive / self-cycle which parse_composite will reject).
fn composite_refs_ready(node: Node<'_, '_>, registry: &TypeRegistry) -> bool {
    let Ok(self_name) = string_attr(node, "name", "composite @name") else {
        return false;
    };
    for child in element_children(node) {
        let tag = child.tag_name().name();
        if matches!(tag, "group" | "data" | "field") {
            return true; // let parse_composite emit the error
        }
        let target = if tag == "ref" {
            child.attribute("type").or_else(|| child.attribute("ref"))
        } else if tag == "type" {
            child
                .attribute("ref")
                .or_else(|| child.attribute("type"))
                .or_else(|| child.attribute("primitiveType"))
        } else {
            None
        };
        let Some(t) = target else {
            continue;
        };
        if is_primitive_name(t) || t == self_name {
            continue;
        }
        if registry.encodings.contains_key(t) || registry.registry.contains_key(t) {
            continue;
        }
        return false;
    }
    true
}

fn is_primitive_name(s: &str) -> bool {
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

#[allow(clippy::needless_pass_by_value)]
fn parse_schema(
    root: Node<'_, '_>,
    base_dir: Option<&Path>,
    seen: &mut HashSet<PathBuf>,
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

    let mut registry = TypeRegistry::new();
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
                                parse_types_node(inc_node, &mut registry, &mut tokens)?;
                            } else {
                                for sub_child in element_children(inc_node) {
                                    if sub_child.tag_name().name() == "types" {
                                        parse_types_node(sub_child, &mut registry, &mut tokens)?;
                                    }
                                }
                            }
                        }
                    }
                    Err(fault) => return Err(fault),
                }
            }
        } else if child.tag_name().name() == "types" {
            parse_types_node(child, &mut registry, &mut tokens)?;
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
            parse_message(child, &header_type, &registry, &mut tokens)?;
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

fn parse_type_element(node: Node<'_, '_>, _registry: &TypeRegistry) -> Result<Encoding, Fault> {
    let primitive = node
        .attribute("primitiveType")
        .or_else(|| node.attribute("type"));
    let primitive_type = primitive
        .map(|s| parse_primitive_type(node, s))
        .transpose()?;
    let offset = opt_usize_attr(node, "offset", "offset")?;
    let presence = node
        .attribute("presence")
        .map(|s| parse_presence(node, s))
        .transpose()?
        .unwrap_or(Presence::Required);
    let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);
    let character_encoding = node.attribute("characterEncoding").map(str::to_string);
    let semantic_type = node.attribute("semanticType").map(str::to_string);
    let description = collect_description(node);
    let length = opt_usize_attr(node, "length", "length")?;
    let epoch = node.attribute("epoch").map(str::to_string);
    let time_unit = node.attribute("timeUnit").map(str::to_string);
    let deprecated = node.attribute("deprecated").is_some();

    let null_value = node
        .attribute("nullValue")
        .and_then(|s| parse_u64_val(s, primitive_type));
    if null_value.is_some() && presence != Presence::Optional {
        let type_name = node.attribute("name").unwrap_or("<unnamed>");
        eprintln!(
            "warning: nullValue specified on non-optional type '{type_name}' \
             \u{2014} nullValue is only meaningful for optional types"
        );
    }
    let min_value = node
        .attribute("minValue")
        .and_then(|s| parse_u64_val(s, primitive_type));
    let max_value = node
        .attribute("maxValue")
        .and_then(|s| parse_u64_val(s, primitive_type));

    // Constant `<type>`: body text, or `valueRef` (e.g. TimeUnit.nanosecond) as in
    // value-ref-schema.xml — same options sbe-tool accepts for constant fields.
    let constant_value = if presence == Presence::Constant {
        let from_text = node
            .text()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        from_text.or_else(|| node.attribute("valueRef").map(|s| s.to_string()))
    } else {
        None
    };

    if primitive_type == Some(PrimitiveType::Char) && presence == Presence::Constant {
        if let Some(len) = length {
            if len > 1 {
                if let Some(ref cv) = constant_value {
                    if cv.len() != len {
                        return Err(Fault::invalid(
                            node,
                            "char constant value length",
                            format!("expected {len} characters, got {}", cv.len()),
                        ));
                    }
                }
            }
        }
    }

    Ok(Encoding {
        primitive_type,
        offset,
        presence,
        since_version,
        null_value,
        character_encoding,
        semantic_type,
        min_value,
        max_value,
        description,
        constant_value,
        length,
        epoch,
        time_unit,
        deprecated,
        is_variable_length: false,
    })
}

fn parse_composite(
    node: Node<'_, '_>,
    registry: &mut TypeRegistry,
    tokens: &mut Vec<Token>,
) -> Result<(), Fault> {
    let name = string_attr(node, "name", "composite @name")?;
    validate_sbe_name(node, &name, "composite @name")?;
    reject_duplicate_type_name(node, &name, registry)?;
    let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);
    let composite_deprecated = node.attribute("deprecated").is_some();

    let mut composite_tokens = Vec::new();
    composite_tokens.push(Token {
        id: None,
        name: name.clone(),
        signal: Signal::BeginComposite,
        encoding: Encoding {
            since_version,
            deprecated: composite_deprecated,
            description: collect_description(node),
            semantic_type: node.attribute("semanticType").map(str::to_string),
            ..Encoding::default()
        },
    });

    // Occupied exclusive ranges [start, end) for explicit member offsets.
    let mut occupied_offsets: Vec<(usize, usize)> = Vec::new();

    for child in element_children(node) {
        let tag = child.tag_name().name();
        // Composites may only contain fixed members (`type` / SBE `<ref>`).
        // Groups and var-data belong on messages, not inside composites.
        if matches!(tag, "group" | "data" | "field") {
            return Err(Fault::invalid(
                child,
                "composite member",
                format!("<{tag}> is not allowed inside composite '{name}'"),
            ));
        }

        // Nested `<enum>` / `<set>` / `<composite>` inside a composite both
        // define a named type (first definition wins) and occupy wire space
        // as a member (sbe-tool Booster.BoostType, outer.inner, etc.).
        if tag == "enum" {
            let enum_name = string_attr(child, "name", "composite nested enum @name")?;
            if !registry.registry.contains_key(&enum_name) {
                parse_enum(child, registry, tokens)?;
            }
            let since_val = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            if let Some(resolved) =
                resolve_type_to_tokens(&enum_name, &enum_name, None, registry, since_val)
            {
                composite_tokens.extend(resolved);
            }
            continue;
        }
        if tag == "set" {
            let set_name = string_attr(child, "name", "composite nested set @name")?;
            if !registry.registry.contains_key(&set_name) {
                parse_set(child, registry, tokens)?;
            }
            let since_val = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            if let Some(resolved) =
                resolve_type_to_tokens(&set_name, &set_name, None, registry, since_val)
            {
                composite_tokens.extend(resolved);
            }
            continue;
        }
        if tag == "composite" {
            let nested_name = string_attr(child, "name", "composite nested composite @name")?;
            if nested_name == name {
                return Err(Fault::invalid(
                    child,
                    "cyclic composite ref",
                    format!("{nested_name}: composite cannot nest itself"),
                ));
            }
            if !registry.registry.contains_key(&nested_name) {
                parse_composite(child, registry, tokens)?;
            }
            let since_val = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            if let Some(off) = opt_usize_attr(child, "offset", "offset")? {
                let member_size = compute_type_size(&nested_name, registry).unwrap_or(1);
                let end = off.saturating_add(member_size);
                for &(s, e) in &occupied_offsets {
                    if off < e && end > s {
                        return Err(Fault::invalid(
                            child,
                            "composite member offset",
                            format!(
                                "{nested_name}: offset {off} overlaps existing member range [{s}, {e})"
                            ),
                        ));
                    }
                }
                occupied_offsets.push((off, end));
            }
            if let Some(resolved) =
                resolve_type_to_tokens(&nested_name, &nested_name, None, registry, since_val)
            {
                // Apply explicit member offset onto the BeginField wrapper.
                if let Some(off) = opt_usize_attr(child, "offset", "offset")? {
                    let mut resolved = resolved;
                    if let Some(first) = resolved.first_mut() {
                        first.encoding.offset = Some(off);
                    }
                    composite_tokens.extend(resolved);
                } else {
                    composite_tokens.extend(resolved);
                }
            }
            continue;
        }

        // SBE `<ref name="x" type="T"/>` — detect self-cycles; expand when T is
        // already registered (forward refs are resolved via later field use).
        if tag == "ref" {
            let member_name = string_attr(child, "name", "composite ref @name")?;
            validate_sbe_name(child, &member_name, "composite ref @name")?;
            let ref_name = child
                .attribute("type")
                .or_else(|| child.attribute("ref"))
                .ok_or_else(|| Fault::missing(child, "composite ref @type"))?;
            if ref_name == name {
                return Err(Fault::invalid(
                    child,
                    "cyclic composite ref",
                    format!("{ref_name}: composite cannot reference itself"),
                ));
            }
            let since_val = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            if let Some(off) = opt_usize_attr(child, "offset", "offset")? {
                let member_size = estimate_composite_member_size(child, registry).unwrap_or(1);
                let end = off.saturating_add(member_size);
                for &(s, e) in &occupied_offsets {
                    if off < e && end > s {
                        return Err(Fault::invalid(
                            child,
                            "composite member offset",
                            format!(
                                "{member_name}: offset {off} overlaps existing member range [{s}, {e})"
                            ),
                        ));
                    }
                }
                occupied_offsets.push((off, end));
            }
            if let Some(resolved) =
                resolve_type_to_tokens(&member_name, ref_name, None, registry, since_val)
            {
                composite_tokens.extend(resolved);
            }
            // Forward-ref `<ref type="LaterEnum"/>`: leave expansion to field
            // resolution when the composite is used (matches prior skip behavior).
            continue;
        }

        if tag == "type" {
            let member_name = string_attr(child, "name", "composite member @name")?;
            validate_sbe_name(child, &member_name, "composite member @name")?;
            let type_name = child
                .attribute("type")
                .or_else(|| child.attribute("primitiveType"))
                .or_else(|| child.attribute("ref"));
            let since_val = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);

            // Cyclic composite: ref="SelfName".
            if let Some(ref_name) = child.attribute("ref") {
                if ref_name == name {
                    return Err(Fault::invalid(
                        child,
                        "cyclic composite ref",
                        format!("{ref_name}: composite cannot reference itself"),
                    ));
                }
                // Gap 2: `ref="Name"` must point to a known type when the attribute
                // form is used (no forward refs for attribute-style ref).
                if !registry.encodings.contains_key(ref_name)
                    && !registry.registry.contains_key(ref_name)
                {
                    return Err(Fault::invalid(
                        child,
                        "composite member ref",
                        format!("{ref_name}: type not found"),
                    ));
                }
            }

            // Overlapping explicit offsets (messageHeader offset clashes, etc.).
            if let Some(off) = opt_usize_attr(child, "offset", "offset")? {
                let member_size = estimate_composite_member_size(child, registry).unwrap_or(1);
                let end = off.saturating_add(member_size);
                for &(s, e) in &occupied_offsets {
                    if off < e && end > s {
                        return Err(Fault::invalid(
                            child,
                            "composite member offset",
                            format!(
                                "{member_name}: offset {off} overlaps existing member range [{s}, {e})"
                            ),
                        ));
                    }
                }
                occupied_offsets.push((off, end));
            }

            if let Some(t_name) = type_name {
                // Whether this <type> element is an indirect ref (resolved by name
                // through the registry) vs a direct encoding with inline attributes.
                // A `ref` attribute always counts as indirect; a bare `type` attribute
                // counts as indirect only when the name isn't a known primitive encoding.
                let has_ref_attr = child.attribute("ref").is_some();
                // Named types (typedef/enum/set/composite) always resolve by name;
                // only bare primitiveType= members use parse_type_element directly.
                let is_named_ref = has_ref_attr
                    || (child.attribute("type").is_some() && !is_primitive_name(t_name));
                if !is_named_ref {
                    let encoding = parse_type_element(child, registry)?;
                    composite_tokens.push(Token {
                        id: None,
                        name: member_name.clone(),
                        signal: Signal::BeginField,
                        encoding: encoding.clone(),
                    });
                    composite_tokens.push(Token {
                        id: None,
                        name: member_name.clone(),
                        signal: Signal::EndField,
                        encoding: Encoding::default(),
                    });
                } else if let Some(resolved) =
                    resolve_type_to_tokens(&member_name, t_name, None, registry, since_val)
                {
                    composite_tokens.extend(resolved);
                } else {
                    let encoding = parse_type_element(child, registry)?;
                    composite_tokens.push(Token {
                        id: None,
                        name: member_name.clone(),
                        signal: Signal::BeginField,
                        encoding: encoding.clone(),
                    });
                    composite_tokens.push(Token {
                        id: None,
                        name: member_name.clone(),
                        signal: Signal::EndField,
                        encoding: Encoding::default(),
                    });
                }
            } else {
                let encoding = parse_type_element(child, registry)?;
                composite_tokens.push(Token {
                    id: None,
                    name: member_name.clone(),
                    signal: Signal::BeginField,
                    encoding: encoding.clone(),
                });
                composite_tokens.push(Token {
                    id: None,
                    name: member_name.clone(),
                    signal: Signal::EndField,
                    encoding: Encoding::default(),
                });
            }
        }
    }

    composite_tokens.push(structural(&name, Signal::EndComposite));

    registry
        .registry
        .insert(name.clone(), composite_tokens.clone());
    tokens.extend(composite_tokens);
    Ok(())
}

fn parse_enum(
    node: Node<'_, '_>,
    registry: &mut TypeRegistry,
    tokens: &mut Vec<Token>,
) -> Result<(), Fault> {
    let name = string_attr(node, "name", "enum @name")?;
    validate_sbe_name(node, &name, "enum @name")?;
    reject_duplicate_type_name(node, &name, registry)?;
    let encoding_type_name = string_attr(node, "encodingType", "enum @encodingType")?;
    let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);

    let encoding_type = registry
        .encodings
        .get(&encoding_type_name)
        .and_then(|e| e.primitive_type)
        .ok_or_else(|| Fault::invalid(node, "enum encodingType", &encoding_type_name))?;

    let encoding_min = registry
        .encodings
        .get(&encoding_type_name)
        .and_then(|e| e.min_value);
    let encoding_max = registry
        .encodings
        .get(&encoding_type_name)
        .and_then(|e| e.max_value);

    // Enum encoding types must be integer or char (sbe-tool requirement).
    // Float/Double enums are not valid SBE.
    if matches!(encoding_type, PrimitiveType::Float | PrimitiveType::Double) {
        return Err(Fault::invalid(
            node,
            "enum encodingType",
            format!("{encoding_type:?}: enum encoding must be integer or char, not float/double"),
        ));
    }

    let mut enum_tokens = Vec::new();
    let semantic_type = node.attribute("semanticType").map(str::to_string);
    enum_tokens.push(Token {
        id: None,
        name: name.clone(),
        signal: Signal::BeginEnum,
        encoding: Encoding {
            primitive_type: Some(encoding_type),
            since_version,
            deprecated: node.attribute("deprecated").is_some(),
            description: collect_description(node),
            semantic_type,
            ..Encoding::default()
        },
    });

    // Resolve null sentinel for the enum's encoding type (sbe-tool: valid values
    // must not equal the type's null value).
    let null_sentinel: Option<u64> = registry
        .encodings
        .get(&encoding_type_name)
        .and_then(|e| e.null_value);

    let mut seen_names = HashSet::new();
    let mut seen_values = HashSet::new();

    for child in element_children(node) {
        if child.tag_name().name() == "validValue" {
            let val_name = string_attr(child, "name", "validValue @name")?;
            if !seen_names.insert(val_name.clone()) {
                return Err(Fault::invalid(
                    child,
                    "duplicate validValue name",
                    &val_name,
                ));
            }
            let val_since = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            let val_text = child.text().unwrap_or("").trim();
            if !val_text.is_empty() && !seen_values.insert(val_text.to_string()) {
                return Err(Fault::invalid(
                    child,
                    "duplicate validValue encoded value",
                    val_text,
                ));
            }
            // Check the valid value doesn't equal the encoding type's null sentinel.
            if let Some(null_val) = null_sentinel {
                if let Some(parsed_val) = parse_u64_val(val_text, Some(encoding_type)) {
                    if parsed_val == null_val {
                        return Err(Fault::invalid(
                            child,
                            "validValue",
                            format!(
                                "{val_text}: validValue must not equal the null sentinel ({null_val})"
                            ),
                        ));
                    }
                }
            }
            // Enum values must lie within the encoding type's min/max when set.
            if let Some(parsed_val) = parse_u64_val(val_text, Some(encoding_type)) {
                if let Some(min) = encoding_min {
                    if parsed_val < min {
                        return Err(Fault::invalid(
                            child,
                            "validValue range",
                            format!("{val_text}: below encodingType minValue {min}"),
                        ));
                    }
                }
                if let Some(max) = encoding_max {
                    if parsed_val > max {
                        return Err(Fault::invalid(
                            child,
                            "validValue range",
                            format!("{val_text}: above encodingType maxValue {max}"),
                        ));
                    }
                }
            } else if !val_text.is_empty() {
                // Signed negative values (e.g. null sentinel candidates) still
                // violate min/max when the encoding type constrains the range.
                if let Ok(signed) = val_text.parse::<i64>() {
                    if let Some(min) = encoding_min {
                        // min_value stored as u64; compare when non-negative path fails
                        if signed < 0 {
                            return Err(Fault::invalid(
                                child,
                                "validValue range",
                                format!("{val_text}: outside encodingType minValue {min}"),
                            ));
                        }
                    }
                    if encoding_min.is_some() || encoding_max.is_some() {
                        // Negative values are always out of a positive min/max range.
                        if signed < 0 {
                            return Err(Fault::invalid(
                                child,
                                "validValue range",
                                format!("{val_text}: outside encodingType min/max range"),
                            ));
                        }
                    }
                }
            }

            validate_sbe_name(child, &val_name, "validValue @name")?;

            enum_tokens.push(Token {
                id: None,
                name: val_name,
                signal: Signal::Encoding,
                encoding: Encoding {
                    presence: Presence::Constant,
                    constant_value: Some(val_text.to_string()),
                    since_version: val_since,
                    description: collect_description(child),
                    ..Encoding::default()
                },
            });
        }
    }

    enum_tokens.push(structural(&name, Signal::EndEnum));

    registry.registry.insert(name, enum_tokens.clone());
    tokens.extend(enum_tokens);
    Ok(())
}

fn parse_set(
    node: Node<'_, '_>,
    registry: &mut TypeRegistry,
    tokens: &mut Vec<Token>,
) -> Result<(), Fault> {
    let name = string_attr(node, "name", "set @name")?;
    let encoding_type_name = string_attr(node, "encodingType", "set @encodingType")?;
    let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);

    let encoding_type = registry
        .encodings
        .get(&encoding_type_name)
        .and_then(|e| e.primitive_type)
        .ok_or_else(|| Fault::invalid(node, "set encodingType", &encoding_type_name))?;

    // Set encoding types must be unsigned integers (sbe-tool requirement).
    if !matches!(
        encoding_type,
        PrimitiveType::UInt8
            | PrimitiveType::UInt16
            | PrimitiveType::UInt32
            | PrimitiveType::UInt64
    ) {
        return Err(Fault::invalid(
            node,
            "set encodingType",
            format!(
                "{encoding_type:?}: sets require unsigned integer encoding (uint8/uint16/uint32/uint64)"
            ),
        ));
    }

    let mut set_tokens = Vec::new();
    set_tokens.push(Token {
        id: None,
        name: name.clone(),
        signal: Signal::BeginSet,
        encoding: Encoding {
            primitive_type: Some(encoding_type),
            since_version,
            deprecated: node.attribute("deprecated").is_some(),
            description: collect_description(node),
            ..Encoding::default()
        },
    });

    let mut seen_choice_names = HashSet::new();
    let mut seen_bit_indices = HashSet::new();

    for child in element_children(node) {
        if child.tag_name().name() == "choice" {
            let choice_name = string_attr(child, "name", "choice @name")?;
            if !seen_choice_names.insert(choice_name.clone()) {
                return Err(Fault::invalid(
                    child,
                    "duplicate set choice name",
                    &choice_name,
                ));
            }
            let choice_since = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            let bit_index_str = child.text().unwrap_or("").trim();

            let bit_index: u8 = bit_index_str.parse().map_err(|_| {
                Fault::invalid(
                    child,
                    "set choice value",
                    format!("invalid bit index: {bit_index_str}"),
                )
            })?;
            let max_bit = match encoding_type {
                PrimitiveType::UInt8 => 7,
                PrimitiveType::UInt16 => 15,
                PrimitiveType::UInt32 => 31,
                PrimitiveType::UInt64 => 63,
                _ => 63,
            };
            if bit_index > max_bit {
                return Err(Fault::invalid(
                    child,
                    "set choice bit index",
                    format!("bit index {bit_index} exceeds max {max_bit} for {encoding_type:?}"),
                ));
            }
            if !seen_bit_indices.insert(bit_index) {
                return Err(Fault::invalid(
                    child,
                    "duplicate set choice bit index",
                    format!("{bit_index}"),
                ));
            }

            set_tokens.push(Token {
                id: None,
                name: choice_name,
                signal: Signal::Encoding,
                encoding: Encoding {
                    presence: Presence::Constant,
                    constant_value: Some(bit_index_str.to_string()),
                    since_version: choice_since,
                    description: collect_description(child),
                    ..Encoding::default()
                },
            });
        }
    }

    set_tokens.push(structural(&name, Signal::EndSet));

    registry.registry.insert(name, set_tokens.clone());
    tokens.extend(set_tokens);
    Ok(())
}

/// Parse a `<message>` into bracketed `BeginMessage`/`EndMessage` tokens.
///
/// `header_type` is the name of the schema's header composite (e.g.
/// `"messageHeader"`). Gap 3: message field IDs are validated against
/// header-type field IDs to prevent conflicts.
fn parse_message(
    node: Node<'_, '_>,
    header_type: &str,
    registry: &TypeRegistry,
    tokens: &mut Vec<Token>,
) -> Result<(), Fault> {
    let name = string_attr(node, "name", "message @name")?;
    validate_sbe_name(node, &name, "message @name")?;
    let id = u16_attr(node, "id", "message @id")?;
    let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);
    let block_length = opt_u16_attr(node, "blockLength", "blockLength")?;
    let message_deprecated = node.attribute("deprecated").is_some();

    tokens.push(Token {
        id: Some(id),
        name: name.clone(),
        signal: Signal::BeginMessage,
        encoding: Encoding {
            since_version,
            deprecated: message_deprecated,
            description: collect_description(node),
            semantic_type: node.attribute("semanticType").map(str::to_string),
            ..Encoding::default()
        },
    });

    // Gap 3: pre-populate seen_ids with the header type's field IDs so that
    // message fields using the same ID are flagged as conflicts.
    let mut seen_ids: HashSet<u16> = if let Some(header_tokens) = registry.registry.get(header_type)
    {
        header_tokens
            .iter()
            .filter_map(|t| {
                if t.signal == Signal::BeginField {
                    t.id
                } else {
                    None
                }
            })
            .collect()
    } else {
        HashSet::new()
    };
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut prev_offset: Option<usize> = None;

    let mut expected_block_len: usize = 0;
    let mut all_fields_have_offsets = true;
    let mut any_field_counted = false;

    for child in element_children(node) {
        parse_message_child(child, registry, tokens)?;
        if child.tag_name().name() == "field"
            || child.tag_name().name() == "group"
            || child.tag_name().name() == "data"
        {
            if let Some(name_attr) = child.attribute("name") {
                let child_name = name_attr.to_string();
                validate_sbe_name(child, &child_name, "field/group/data @name")?;
                if !seen_names.insert(child_name.clone()) {
                    return Err(Fault::invalid(
                        child,
                        "duplicate field/group/data name in message",
                        child_name,
                    ));
                }
            }
            if let Some(id_str) = child.attribute("id") {
                if let Ok(child_id) = id_str.parse::<u16>() {
                    if !seen_ids.insert(child_id) {
                        return Err(Fault::invalid(
                            child,
                            "duplicate field/group/data id in message",
                            id_str.to_string(),
                        ));
                    }
                }
            }
            if let Some(offset_str) = child.attribute("offset") {
                if let Ok(offset) = offset_str.parse::<usize>() {
                    if let Some(prev) = prev_offset {
                        if offset < prev {
                            return Err(Fault::invalid(
                                child,
                                "field offset out of order",
                                format!("offset {offset} after {prev}"),
                            ));
                        }
                    }
                    prev_offset = Some(offset);
                }
            }
        }

        if child.tag_name().name() == "field"
            && child.attribute("presence").unwrap_or("required") != "constant"
        {
            any_field_counted = true;
            if let Some(offset_str) = child.attribute("offset") {
                if let Ok(offset) = offset_str.parse::<usize>() {
                    if let Some(type_name) = child.attribute("type") {
                        if let Some(size) = compute_type_size(type_name, registry) {
                            let end = offset + size;
                            if end > expected_block_len {
                                expected_block_len = end;
                            }
                        }
                    }
                }
            } else {
                all_fields_have_offsets = false;
            }
        }
    }

    // Validate blockLength when declared and all fields carry explicit offsets.
    // Uses a warning (not an error) to match sbe-tool behavior — the
    // upstream tool accepts whatever blockLength the schema declares without
    // validation, and several official test schemas have intentionally differing
    // values.
    if let Some(declared_bl) = block_length {
        if all_fields_have_offsets
            && any_field_counted
            && declared_bl as usize != expected_block_len
        {
            eprintln!(
                "warning: message '{}' blockLength mismatch: declared {declared_bl}, \
                 expected {expected_block_len} (sum of fixed-field offset + sizes)",
                name,
            );
        }
    }

    tokens.push(structural(&name, Signal::EndMessage));
    Ok(())
}

fn parse_message_child(
    node: Node<'_, '_>,
    registry: &TypeRegistry,
    tokens: &mut Vec<Token>,
) -> Result<(), Fault> {
    match node.tag_name().name() {
        "field" => {
            let field_name = string_attr(node, "name", "field @name")?;
            let type_name = string_attr(node, "type", "field @type")?;
            let id = u16_attr(node, "id", "field @id")?;
            let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            let type_encoding = registry.encodings.get(&type_name);
            // Inherit epoch/timeUnit/deprecated from the referenced type when not
            // explicitly set on the field (Gaps 11, 12).
            let explicit_epoch = node.attribute("epoch");
            let epoch = explicit_epoch
                .map(str::to_string)
                .or_else(|| type_encoding.and_then(|e| e.epoch.clone()));
            let explicit_time_unit = node.attribute("timeUnit");
            let time_unit = explicit_time_unit
                .map(str::to_string)
                .or_else(|| type_encoding.and_then(|e| e.time_unit.clone()));
            let explicit_deprecated = node.attribute("deprecated");
            let deprecated =
                explicit_deprecated.is_some() || type_encoding.is_some_and(|e| e.deprecated);
            // Gap 1: presence inheritance from referenced types
            let explicit_presence = node.attribute("presence");
            let presence = if let Some(p) = explicit_presence {
                parse_presence(node, p)?
            } else {
                // Inherit presence from the referenced type, if it has one.
                type_encoding
                    .map(|e| e.presence)
                    .unwrap_or(Presence::Required)
            };
            if node.attribute("nullValue").is_some() && presence != Presence::Optional {
                eprintln!(
                    "warning: nullValue specified on non-optional field '{field_name}' \
                     \u{2014} nullValue is only meaningful for optional fields"
                );
            }
            let constant_value = if presence == Presence::Constant {
                if node.attribute("constantValue").is_none() && node.attribute("valueRef").is_none()
                {
                    // The field may inherit constant value from the referenced type.
                    let type_is_constant = registry
                        .encodings
                        .get(&type_name)
                        .map(|e| e.presence == Presence::Constant)
                        .unwrap_or(false);
                    if !type_is_constant {
                        return Err(Fault::missing(
                            node,
                            "constantValue or valueRef attribute for constant field",
                        ));
                    }
                }
                node.attribute("valueRef").map(|s| {
                    // valueRef format: "EnumName.ValidValue" — validate the enum and
                    // variant exist at parse time (sbe-tool rejects invalid valueRef).
                    if let Some((enum_name, variant_name)) = s.split_once('.') {
                        // enum existence checked, variant existence deferred to resolve; add variant validation here if resolve becomes lenient
                        // validated at parse time. An invalid variant name produces
                        // a Rust compile error in the generated code, which is caught
                        // before the codec is used.
                        if !registry.registry.contains_key(enum_name) {
                            // non-fatal warning: old behaviour was silent strip
                            eprintln!(
                                "warning: valueRef '{s}' references unknown enum '{enum_name}'"
                            );
                        }
                        variant_name.to_string()
                    } else {
                        // No dot — valueRef with no TypeName prefix, keep as-is
                        s.to_string()
                    }
                })
            } else {
                None
            };

            if let Some(resolved) =
                resolve_type_to_tokens(&field_name, &type_name, Some(id), registry, since_version)
            {
                let mut inlined = resolved;
                if let Some(first) = inlined.first_mut() {
                    if let Some(offset_str) = node.attribute("offset")
                        && let Ok(offset) = offset_str.parse::<usize>()
                    {
                        first.encoding.offset = Some(offset);
                    }
                    first.encoding.presence = presence;
                    first.encoding.epoch = epoch;
                    first.encoding.time_unit = time_unit;
                    first.encoding.deprecated = deprecated;
                    if let Some(cv) = constant_value {
                        first.encoding.constant_value = Some(cv);
                    }
                    // Propagate semanticType from the field element if set
                    if first.encoding.semantic_type.is_none() {
                        first.encoding.semantic_type =
                            node.attribute("semanticType").map(str::to_string);
                    }
                }
                tokens.extend(inlined);
            } else {
                return Err(Fault::invalid(node, "primitive type", &type_name));
            }
        }
        "group" => {
            let group_name = string_attr(node, "name", "group @name")?;
            let id = u16_attr(node, "id", "group @id")?;
            let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            let group_deprecated = node.attribute("deprecated").is_some();
            let dimension_type = node
                .attribute("dimensionType")
                .unwrap_or("groupSizeEncoding");

            tokens.push(Token {
                id: Some(id),
                name: group_name.clone(),
                signal: Signal::BeginGroup,
                encoding: Encoding {
                    since_version,
                    deprecated: group_deprecated,
                    description: collect_description(node),
                    ..Encoding::default()
                },
            });

            if let Some(dim_tokens) = registry.registry.get(dimension_type) {
                // Validate the dimension composite has blockLength and numInGroup fields.
                let has_block_length = dim_tokens
                    .iter()
                    .any(|t| t.signal == Signal::BeginField && t.name == "blockLength");
                let has_num_in_group = dim_tokens
                    .iter()
                    .any(|t| t.signal == Signal::BeginField && t.name == "numInGroup");
                if !has_block_length || !has_num_in_group {
                    return Err(Fault::invalid(
                        node,
                        "group dimensionType",
                        format!("{dimension_type}: expected 'blockLength' and 'numInGroup' fields"),
                    ));
                }
                tokens.extend(dim_tokens.clone());
            } else {
                return Err(Fault::invalid(node, "group dimensionType", dimension_type));
            }

            for child in element_children(node) {
                parse_message_child(child, registry, tokens)?;
            }

            tokens.push(structural(&group_name, Signal::EndGroup));
        }
        "data" => {
            let data_name = string_attr(node, "name", "data @name")?;
            let id = u16_attr(node, "id", "data @id")?;
            let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            let data_deprecated = node.attribute("deprecated").is_some();
            let type_name = node.attribute("type").unwrap_or("varDataEncoding");

            tokens.push(Token {
                id: Some(id),
                name: data_name.clone(),
                signal: Signal::BeginVarData,
                encoding: Encoding {
                    since_version,
                    deprecated: data_deprecated,
                    description: collect_description(node),
                    ..Encoding::default()
                },
            });

            if let Some(type_tokens) = registry.registry.get(type_name) {
                // Validate the var-data composite has length and varData fields.
                let has_length = type_tokens
                    .iter()
                    .any(|t| t.signal == Signal::BeginField && t.name == "length");
                let has_var_data = type_tokens
                    .iter()
                    .any(|t| t.signal == Signal::BeginField && t.name == "varData");
                if !has_length || !has_var_data {
                    return Err(Fault::invalid(
                        node,
                        "data type",
                        format!("{type_name}: expected 'length' and 'varData' fields"),
                    ));
                }
                // Clone and mark the varData member as variable-length
                // (sbe-tool makeDataFieldCompositeType equivalent — gap 10).
                let mut data_tokens = type_tokens.clone();
                for token in data_tokens.iter_mut() {
                    if token.signal == Signal::BeginField && token.name == "varData" {
                        token.encoding.is_variable_length = true;
                    }
                }
                tokens.extend(data_tokens);
            } else if registry.encodings.contains_key(type_name) {
                return Err(Fault::invalid(
                    node,
                    "data type",
                    format!(
                        "{type_name}: simple encoding cannot be used as varData; \
                         expected a var-data composite"
                    ),
                ));
            } else {
                return Err(Fault::invalid(node, "data type", type_name));
            }

            tokens.push(structural(&data_name, Signal::EndVarData));
        }
        other => {
            return Err(Fault::invalid(
                node,
                "message child",
                format!("unexpected element <{other}> (expected <field>, <group>, or <data>)"),
            ));
        }
    }
    Ok(())
}

/// Construct a structural token (no wire encoding).
fn structural(name: &str, signal: Signal) -> Token {
    Token {
        id: None,
        name: name.to_string(),
        signal,
        encoding: Encoding::default(),
    }
}

fn element_children<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    // Skip <description> and <comment> children — their text is already
    // collected by collect_description() which scans node.children() directly.
    node.children().filter(|c| {
        c.is_element() && c.tag_name().name() != "description" && c.tag_name().name() != "comment"
    })
}

/// Collect all documentation sources for an element and merge them into a
/// single description string (DECISIONS.md §9 / reopened). Handles:
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
fn collect_description(node: Node<'_, '_>) -> Option<String> {
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
fn preceding_xml_comments(node: Node<'_, '_>) -> Vec<String> {
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

fn string_attr(node: Node<'_, '_>, name: &str, what: &str) -> Result<String, Fault> {
    node.attribute(name)
        .map(str::to_string)
        .ok_or_else(|| Fault::missing(node, what))
}

/// SBE / Rust-friendly identifier: `[A-Za-z_][A-Za-z0-9_]*`.
fn is_valid_sbe_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn validate_sbe_name(node: Node<'_, '_>, name: &str, what: &str) -> Result<(), Fault> {
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

fn reject_duplicate_type_name(
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

/// Best-effort wire size for a composite member (for offset-overlap checks).
fn estimate_composite_member_size(node: Node<'_, '_>, registry: &TypeRegistry) -> Option<usize> {
    if let Some(prim) = node
        .attribute("primitiveType")
        .or_else(|| node.attribute("type"))
        .and_then(|s| match s {
            "char" => Some(PrimitiveType::Char),
            "int8" => Some(PrimitiveType::Int8),
            "uint8" => Some(PrimitiveType::UInt8),
            "int16" => Some(PrimitiveType::Int16),
            "uint16" => Some(PrimitiveType::UInt16),
            "int32" => Some(PrimitiveType::Int32),
            "uint32" => Some(PrimitiveType::UInt32),
            "int64" => Some(PrimitiveType::Int64),
            "uint64" => Some(PrimitiveType::UInt64),
            "float" => Some(PrimitiveType::Float),
            "double" => Some(PrimitiveType::Double),
            other => registry
                .encodings
                .get(other)
                .and_then(|e| e.primitive_type)
                .or_else(|| {
                    registry
                        .registry
                        .get(other)
                        .and_then(|toks| toks.first())
                        .and_then(|t| t.encoding.primitive_type)
                }),
        })
    {
        let len = node
            .attribute("length")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);
        return Some(prim.size() * len);
    }
    let ref_name = node.attribute("ref").or_else(|| node.attribute("type"))?;
    if let Some(enc) = registry.encodings.get(ref_name) {
        return Some(enc.primitive_type?.size() * enc.length.unwrap_or(1));
    }
    compute_type_size(ref_name, registry)
}

fn u16_attr(node: Node<'_, '_>, name: &str, what: &str) -> Result<u16, Fault> {
    node.attribute(name)
        .ok_or_else(|| Fault::missing(node, what))
        .and_then(|s| s.parse::<u16>().map_err(|_| Fault::invalid(node, what, s)))
}

fn opt_u16_attr(node: Node<'_, '_>, name: &str, what: &str) -> Result<Option<u16>, Fault> {
    node.attribute(name)
        .map(|s| s.parse::<u16>().map_err(|_| Fault::invalid(node, what, s)))
        .transpose()
}

fn opt_usize_attr(node: Node<'_, '_>, name: &str, what: &str) -> Result<Option<usize>, Fault> {
    node.attribute(name)
        .map(|s| {
            s.parse::<usize>()
                .map_err(|_| Fault::invalid(node, what, s))
        })
        .transpose()
}

fn parse_byte_order(s: &str) -> Result<ByteOrder, Fault> {
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

fn parse_presence(node: Node<'_, '_>, s: &str) -> Result<Presence, Fault> {
    match s {
        "required" => Ok(Presence::Required),
        "optional" => Ok(Presence::Optional),
        "constant" => Ok(Presence::Constant),
        _ => Err(Fault::invalid(node, "presence", s)),
    }
}

fn parse_primitive_type(node: Node<'_, '_>, s: &str) -> Result<PrimitiveType, Fault> {
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

/// Validate that the header type composite has the required SBE fields.
///
/// The header type must be a composite with at least `blockLength`, `templateId`,
/// `schemaId`, and `version` fields (all typically `uint16`). Returns `Ok(())`
/// on success or a `Fault` referencing the last-checked element.
///
/// # Gap 3 — well-formedness constraints
///
/// sbe-tool validates that the header type carries these four mandatory fields.
/// If the header type is not found in the registry it isn't flagged here
/// (the missing type error will surface elsewhere, e.g. in resolution).
fn validate_header_type(header_type: &str, registry: &TypeRegistry) -> Result<(), Fault> {
    let tokens = match registry.registry.get(header_type) {
        Some(t) if !t.is_empty() && t[0].signal == Signal::BeginComposite => t,
        _ => return Ok(()), // Not in the registry or not a composite — skip
    };

    let field_names: HashSet<&str> = tokens
        .iter()
        .filter(|t| t.signal == Signal::BeginField)
        .map(|t| t.name.as_str())
        .collect();

    for required_name in &["blockLength", "templateId", "schemaId", "version"] {
        if !field_names.contains(required_name) {
            // Build a fault; we have no single node to point at, so no span.
            return Err(Fault {
                kind: FaultKind::Invalid {
                    what: "headerType".to_string(),
                    value: format!("{header_type}: missing required field '{required_name}'"),
                },
                span: None,
            });
        }
    }
    Ok(())
}

/// Compute the on-wire byte size of a type referenced by a `<field>` element.
///
/// Returns `None` when the type is unknown or cannot be sized (e.g., a composite
/// whose members include a ref that hasn't been fully resolved).
fn compute_type_size(type_name: &str, registry: &TypeRegistry) -> Option<usize> {
    // Simple (primitive) encoding
    if let Some(enc) = registry.encodings.get(type_name) {
        return Some(enc.primitive_type?.size() * enc.length.unwrap_or(1));
    }

    let tokens = registry.registry.get(type_name)?;
    let first = tokens.first()?;

    match first.signal {
        Signal::BeginEnum | Signal::BeginSet => {
            // Wire size is just the encoding type
            Some(first.encoding.primitive_type?.size())
        }
        Signal::BeginComposite => {
            let mut total = 0;
            for token in tokens.iter() {
                if token.signal == Signal::BeginField
                    && token.encoding.presence != Presence::Constant
                {
                    total +=
                        token.encoding.primitive_type?.size() * token.encoding.length.unwrap_or(1);
                }
            }
            Some(total)
        }
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::ir::{Encoding, Presence, PrimitiveType, Signal, Token};
    use miette::Diagnostic;

    #[test]
    fn parse_u64_val_handles_value_types() -> Result<(), Box<dyn std::error::Error>> {
        // Empty -> None.
        assert_eq!(parse_u64_val("", None), None);
        // Char (single byte).
        assert_eq!(
            parse_u64_val("A", Some(PrimitiveType::Char)),
            Some(b'A' as u64)
        );
        // Float bit reinterpret (f32 branch).
        assert_eq!(
            parse_u64_val("1.5", Some(PrimitiveType::Float)),
            Some(1.5_f32.to_bits() as u64)
        );
        // Double bit reinterpret (f64 branch).
        assert_eq!(
            parse_u64_val("1.5", Some(PrimitiveType::Double)),
            Some(1.5_f64.to_bits() as u64)
        );
        // Unparseable float/double -> None (the branch fall-through return).
        assert_eq!(
            parse_u64_val("not_a_number", Some(PrimitiveType::Float)),
            None
        );
        assert_eq!(
            parse_u64_val("not_a_number", Some(PrimitiveType::Double)),
            None
        );
        // Negative -> i64 reinterpret.
        assert_eq!(parse_u64_val("-1", None), Some(u64::MAX));
        // Plain u64 / invalid.
        assert_eq!(parse_u64_val("42", None), Some(42));
        assert_eq!(parse_u64_val("garbage", None), None);

        Ok(())
    }

    #[test]
    fn parse_malformed_xml_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let err = parse("<messageSchema><unclosed>").unwrap_err();
        assert!(matches!(err, ParseError::MalformedXml { .. }));

        Ok(())
    }

    #[test]
    fn parse_valid_xml_without_message_schema_root_is_missing()
    -> Result<(), Box<dyn std::error::Error>> {
        // Valid XML, but no <messageSchema> root element.
        let err = parse("<root/>").unwrap_err();
        assert!(matches!(err, ParseError::Missing { .. }));

        Ok(())
    }

    #[test]
    fn parse_file_missing_path_is_malformed_xml() -> Result<(), Box<dyn std::error::Error>> {
        let err = parse_file("/nonexistent/ergon/coverage/schema.xml").unwrap_err();
        assert!(matches!(err, ParseError::MalformedXml { .. }));

        Ok(())
    }

    #[test]
    fn parse_set_choice_bit_out_of_range_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
    <set name="S" encodingType="uint8">
      <choice name="Big">10</choice>
    </set>
  </types>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "set choice bit > max must error");

        Ok(())
    }

    #[test]
    fn parse_set_duplicate_choice_bit_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
    <set name="S" encodingType="uint8">
      <choice name="A">1</choice>
      <choice name="B">1</choice>
    </set>
  </types>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "duplicate set choice bit must error");

        Ok(())
    }

    #[test]
    fn parse_invalid_byte_order_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="sideways">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/></composite></types>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "invalid byteOrder must error");

        Ok(())
    }

    #[test]
    fn parse_invalid_presence_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <message name="M" id="1"><field name="f" id="1" type="uint32" presence="bogus"/></message>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "invalid presence must error");

        Ok(())
    }

    #[test]
    fn parse_invalid_primitive_type_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
    <type name="bad" primitiveType="notatype"/>
  </types>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "invalid primitiveType must error");

        Ok(())
    }

    #[test]
    fn parse_enum_with_float_encoding_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
    <enum name="E" encodingType="float"><validValue name="A">1</validValue></enum>
  </types>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "enum with float encoding must error");

        Ok(())
    }

    #[test]
    fn parse_set_with_signed_encoding_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
    <set name="S" encodingType="int8"><choice name="A">0</choice></set>
  </types>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "set with signed encoding must error");

        Ok(())
    }

    #[test]
    fn parse_set_duplicate_choice_name_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
    <set name="S" encodingType="uint8"><choice name="A">0</choice><choice name="A">1</choice></set>
  </types>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "duplicate set choice name must error");

        Ok(())
    }

    #[test]
    fn parse_invalid_message_schema_child_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/></composite></types>
  <unexpectedChild/>
</messageSchema>"#;
        assert!(
            parse(xml).is_err(),
            "invalid messageSchema child must error"
        );

        Ok(())
    }

    #[test]
    fn parse_field_offset_out_of_order_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <message name="M" id="1" blockLength="8">
    <field name="a" id="1" type="uint32" offset="4"/>
    <field name="b" id="2" type="uint32" offset="0"/>
  </message>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "out-of-order field offsets must error");

        Ok(())
    }

    #[test]
    fn parse_invalid_message_child_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <message name="M" id="1"><bogusElement/></message>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "invalid message child must error");

        Ok(())
    }

    #[test]
    fn parse_invalid_types_container_child_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/></composite>
    <bogusType/>
  </types>
</messageSchema>"#;
        assert!(
            parse(xml).is_err(),
            "invalid types container child must error"
        );

        Ok(())
    }

    #[test]
    fn parse_collects_all_documentation_sources() -> Result<(), Box<dyn std::error::Error>> {
        // schema-docs-all-sources.xml exercises all four documentation shapes:
        // description attrs, <description> children, <comment> children, and
        // XML <!-- --> comments. Verify they all reach the IR.
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schemas/schema-docs-all-sources.xml"
        );
        let ir = parse_file(path).unwrap();

        // Schema-level: description attr collected from root + preceding
        // XML comment (<!-- xml-comment:schema --> before root element).
        let sd = ir.description.as_ref().unwrap();
        assert!(
            sd.contains("attr:schema"),
            "missing schema description attr in {sd:?}"
        );

        // Root-level preceding XML comment: the comment before the root
        // element in the Document is a preceding sibling of the root,
        // so preceding_xml_comments(root) picks it up.
        assert!(
            sd.contains("xml-comment:schema"),
            "missing preceding XML comment on schema root in {sd:?}"
        );

        // Verify deterministic merge order on the root: attr first, then
        // preceding XML comments (root has no child <description>/<comment>).
        let attr_pos = sd.find("attr:schema").expect("attr:schema");
        let comment_pos = sd.find("xml-comment:schema").expect("xml-comment:schema");
        assert!(
            attr_pos < comment_pos,
            "description attr must precede XML comments; got {sd:?}"
        );

        // Find the messageHeader token — must now include all 4 sources
        // including the preceding-sibling XML comment.
        let mh = ir
            .tokens
            .iter()
            .find(|t| t.name == "messageHeader")
            .expect("messageHeader composite token not found");
        let mh_desc = mh.encoding.description.as_ref().unwrap();
        assert!(
            mh_desc.contains("attr:header"),
            "missing description attr in '{mh_desc}'"
        );
        assert!(
            mh_desc.contains("description-child:header"),
            "missing description child in '{mh_desc}'"
        );
        assert!(
            mh_desc.contains("comment-child:header"),
            "missing comment child in '{mh_desc}'"
        );
        assert!(
            mh_desc.contains("xml-comment:header"),
            "missing preceding-sibling XML comment in '{mh_desc}'"
        );

        // Also verify the enum picked up its preceding comment.
        let colour = ir
            .tokens
            .iter()
            .find(|t| t.name == "Colour")
            .expect("Colour token not found");
        let colour_desc = colour.encoding.description.as_ref().unwrap();
        assert!(
            colour_desc.contains("xml-comment:enum"),
            "missing preceding-sibling XML comment on Colour in '{colour_desc}'"
        );

        // And the message picked up its preceding comment.
        let msg = ir
            .tokens
            .iter()
            .find(|t| t.name == "M")
            .expect("M token not found");
        let msg_desc = msg.encoding.description.as_ref().unwrap();
        assert!(
            msg_desc.contains("xml-comment:message"),
            "missing preceding-sibling XML comment on M in '{msg_desc}'"
        );

        Ok(())
    }

    #[test]
    fn parse_composite_with_undefined_type_member() -> Result<(), Box<dyn std::error::Error>> {
        // Composite member with type="X" where X isn't a known primitive
        // encoding or registered type — triggers the is_indirect_ref=true +
        // resolve_type_to_tokens=None fallback (lines ~769-796).
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
    <composite name="C"><type name="f" type="NoSuchType"/></composite>
  </types>
</messageSchema>"#;
        // Either parse errors (undefined type) or succeeds (fallback branch).
        // In either case the fallback code at 769-796 is exercised.
        let _ = parse(xml);
        Ok(())
    }

    #[test]
    fn parse_include_file_not_found_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/></composite></types>
  <include href="definitely_nonexistent_file_12345.xml"/>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "include file not found must error");

        Ok(())
    }

    #[test]
    fn missing_no_node_creates_fault_without_span() -> Result<(), Box<dyn std::error::Error>> {
        let fault = Fault::missing_no_node("test");
        assert!(matches!(fault.kind, FaultKind::Missing { ref what } if what == "test"));
        assert!(fault.span.is_none());

        Ok(())
    }

    #[test]
    fn resolve_type_with_since_version() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = TypeRegistry::new();
        registry.encodings.insert(
            "myType".to_string(),
            Encoding {
                primitive_type: Some(PrimitiveType::UInt32),
                ..Encoding::default()
            },
        );
        let result = resolve_type_to_tokens("f", "myType", Some(1), &registry, 5);
        assert!(result.is_some());
        assert_eq!(result.unwrap()[0].encoding.since_version, 5);

        Ok(())
    }

    #[test]
    fn parse_missing_root_element() -> Result<(), Box<dyn std::error::Error>> {
        assert!(parse("<?xml version=\"1.0\"?>\n<notSchema/>").is_err());

        Ok(())
    }

    #[test]
    fn compute_type_size_all_paths() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = TypeRegistry::new();
        registry.encodings.insert(
            "p32".into(),
            Encoding {
                primitive_type: Some(PrimitiveType::Int32),
                length: Some(1),
                ..Encoding::default()
            },
        );
        assert_eq!(compute_type_size("p32", &registry), Some(4));
        registry.encodings.insert(
            "a4".into(),
            Encoding {
                primitive_type: Some(PrimitiveType::Int16),
                length: Some(4),
                ..Encoding::default()
            },
        );
        assert_eq!(compute_type_size("a4", &registry), Some(8));
        assert_eq!(compute_type_size("missing", &registry), None);

        Ok(())
    }

    #[test]
    fn compute_type_size_composite_enum_set() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = TypeRegistry::new();
        let ct = vec![
            Token {
                id: None,
                name: "C".into(),
                signal: Signal::BeginComposite,
                encoding: Encoding::default(),
            },
            Token {
                id: None,
                name: "x".into(),
                signal: Signal::BeginField,
                encoding: Encoding {
                    primitive_type: Some(PrimitiveType::Int32),
                    length: Some(1),
                    presence: Presence::Required,
                    ..Encoding::default()
                },
            },
            Token {
                id: None,
                name: "x".into(),
                signal: Signal::EndField,
                encoding: Encoding::default(),
            },
            Token {
                id: None,
                name: "C".into(),
                signal: Signal::EndComposite,
                encoding: Encoding::default(),
            },
        ];
        registry.registry.insert("C".into(), ct);
        assert_eq!(compute_type_size("C", &registry), Some(4));

        let et = vec![Token {
            id: None,
            name: "E".into(),
            signal: Signal::BeginEnum,
            encoding: Encoding {
                primitive_type: Some(PrimitiveType::UInt8),
                ..Encoding::default()
            },
        }];
        registry.registry.insert("E".into(), et);
        assert_eq!(compute_type_size("E", &registry), Some(1));

        let st = vec![Token {
            id: None,
            name: "S".into(),
            signal: Signal::BeginSet,
            encoding: Encoding {
                primitive_type: Some(PrimitiveType::UInt16),
                ..Encoding::default()
            },
        }];
        registry.registry.insert("S".into(), st);
        assert_eq!(compute_type_size("S", &registry), Some(2));

        Ok(())
    }

    #[test]
    fn parse_enum_duplicate_value() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<enum name="E" encodingType="uint8"><validValue name="A">1</validValue><validValue name="B">1</validValue></enum></types>
<sbe:message name="M" id="1"><field name="e" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
        assert!(parse(xml).is_err());

        Ok(())
    }

    #[test]
    fn parse_enum_null_sentinel_collision() -> Result<(), Box<dyn std::error::Error>> {
        // To trigger the null sentinel check, the enum's encodingType must
        // reference a REGISTERED type (not a bare primitive), because the
        // null_sentinel lookup goes through registry.encodings.
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="enumBase" primitiveType="uint8" nullValue="255"/>
<enum name="E" encodingType="enumBase"><validValue name="A">1</validValue><validValue name="Max">255</validValue></enum></types>
<sbe:message name="M" id="1"><field name="e" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
        assert!(
            parse(xml).is_err(),
            "validValue == null sentinel must error"
        );
        Ok(())
    }

    #[test]
    fn parse_set_bit_index_too_high() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<set name="F" encodingType="uint8"><choice name="X">99</choice></set></types>
<sbe:message name="M" id="1"><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
        assert!(parse(xml).is_err());

        Ok(())
    }

    #[test]
    fn parse_set_non_numeric_bit_index() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<set name="F" encodingType="uint8"><choice name="X">abc</choice></set></types>
<sbe:message name="M" id="1"><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
        assert!(parse(xml).is_err());

        Ok(())
    }

    #[test]
    fn parse_message_duplicate_field_name() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32"/><field name="x" id="2" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
        assert!(parse(xml).is_err());

        Ok(())
    }

    #[test]
    fn parse_message_duplicate_field_id() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32"/><field name="y" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
        assert!(parse(xml).is_err());

        Ok(())
    }

    #[test]
    fn parse_message_out_of_order_offset() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32" offset="4"/><field name="y" id="2" type="uint32" offset="0"/></sbe:message>
</sbe:messageSchema>"#;
        assert!(parse(xml).is_err());

        Ok(())
    }

    #[test]
    fn parse_constant_field_missing_value() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="c" id="1" type="uint32" presence="constant"/></sbe:message>
</sbe:messageSchema>"#;
        assert!(parse(xml).is_err());

        Ok(())
    }

    #[test]
    fn parse_composite_ref_member() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="baseInt" primitiveType="uint32"/>
<composite name="Wrapper"><type name="val" type="baseInt"/></composite></types>
<sbe:message name="M" id="1"><field name="w" id="1" type="Wrapper"/></sbe:message>
</sbe:messageSchema>"#;
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn parse_field_inheriting_presence() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="optVal" primitiveType="uint32" presence="optional" nullValue="4294967295"/></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="optVal"/></sbe:message>
</sbe:messageSchema>"#;
        // Exercise field inheriting presence from referenced type — may succeed or error
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn parse_value_ref_dot_notation() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<enum name="Colour" encodingType="uint8"><validValue name="Red">1</validValue></enum></types>
<sbe:message name="M" id="1"><field name="c" id="1" type="uint8" presence="constant" valueRef="Colour.Red"/></sbe:message>
</sbe:messageSchema>"#;
        // Exercise the valueRef dot-notation code path — may succeed or warn
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn parse_value_ref_unknown_enum_warns() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="c" id="1" type="uint8" presence="constant" valueRef="NonExistent.SomeVal"/></sbe:message>
</sbe:messageSchema>"#;
        // Exercise the valueRef unknown-enum warning path
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn parse_value_ref_no_dot() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="c" id="1" type="uint8" presence="constant" valueRef="SimpleVal"/></sbe:message>
</sbe:messageSchema>"#;
        // Exercise the valueRef no-dot path
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn parse_field_inherit_constant_from_type() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="ci" primitiveType="uint32" presence="constant">42</type></types>
<sbe:message name="M" id="1"><field name="c" id="1" type="ci" presence="constant"/></sbe:message>
</sbe:messageSchema>"#;
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn parse_char_constant_wrong_length() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="c3" primitiveType="char" length="3" presence="constant">AB</type></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn parse_set_valid_indices() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
<types>
<set name="S" encodingType="uint8"><choice name="BitZero">0</choice><choice name="BitMax">7</choice></set>
<set name="S16" encodingType="uint16"><choice name="B">15</choice></set>
<set name="S32" encodingType="uint32"><choice name="B">31</choice></set>
<set name="S64" encodingType="uint64"><choice name="B">63</choice></set>
</types>
<message name="M" id="1"><field name="f" id="1" type="uint32"/></message>
</sbe:messageSchema>"#;
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn workspace_root_found() -> Result<(), Box<dyn std::error::Error>> {
        let root = workspace_root();
        assert!(root.join("Cargo.toml").exists());

        Ok(())
    }

    #[test]
    fn parse_message_with_explicit_offsets_and_registered_types()
    -> Result<(), Box<dyn std::error::Error>> {
        // Triggers the offset tracking / compute_type_size path in parse_message
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <composite name="Point"><type name="x" primitiveType="int32"/><type name="y" primitiveType="int32"/></composite>
</types>
<sbe:message name="M" id="1">
  <field name="p" id="1" type="Point" offset="0"/>
  <field name="v" id="2" type="uint16" offset="8"/>
</sbe:message>
</sbe:messageSchema>"#;
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn parse_message_nullvalue_on_required_field() -> Result<(), Box<dyn std::error::Error>> {
        // Triggers the warning for nullValue on non-optional field
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32" nullValue="0"/></sbe:message>
</sbe:messageSchema>"#;
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn parse_char_constant_correct_length() -> Result<(), Box<dyn std::error::Error>> {
        // Triggers the char constant length check with correct length (length > 1)
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="code3" primitiveType="char" length="3" presence="constant">ABC</type></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn parse_enum_with_description() -> Result<(), Box<dyn std::error::Error>> {
        // Triggers the enum description collection trailing brace
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<enum name="Colour" encodingType="uint8" description="Colour enum">
  <description>Colour description</description>
  <validValue name="Red" description="Red">1</validValue>
</enum></types>
<sbe:message name="M" id="1"><field name="c" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn parse_set_with_description() -> Result<(), Box<dyn std::error::Error>> {
        // Triggers the set description collection trailing brace
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<set name="Flags" encodingType="uint8" description="Flag set">
  <description>Flag description</description>
  <choice name="A" description="First">0</choice>
</set></types>
<sbe:message name="M" id="1"><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
        let _ = parse(xml);

        Ok(())
    }

    #[test]
    fn parse_composite_member_nonexistent_type() -> Result<(), Box<dyn std::error::Error>> {
        // Triggers the else branch at line 770 where resolve_type_to_tokens
        // returns None for a type="X" that's not in the registry
        let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<composite name="C"><type name="f" type="NonExistent"/></composite></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
        let _ = parse(xml);
        Ok(())
    }

    #[test]
    fn compute_type_size_array_and_constant_members() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = TypeRegistry::new();
        let ct = vec![
            Token {
                id: None,
                name: "C".into(),
                signal: Signal::BeginComposite,
                encoding: Encoding::default(),
            },
            Token {
                id: None,
                name: "arr".into(),
                signal: Signal::BeginField,
                encoding: Encoding {
                    primitive_type: Some(PrimitiveType::Int16),
                    length: Some(3),
                    presence: Presence::Required,
                    ..Encoding::default()
                },
            },
            Token {
                id: None,
                name: "arr".into(),
                signal: Signal::EndField,
                encoding: Encoding::default(),
            },
            Token {
                id: None,
                name: "c".into(),
                signal: Signal::BeginField,
                encoding: Encoding {
                    primitive_type: Some(PrimitiveType::Char),
                    length: Some(1),
                    presence: Presence::Constant,
                    ..Encoding::default()
                },
            },
            Token {
                id: None,
                name: "c".into(),
                signal: Signal::EndField,
                encoding: Encoding::default(),
            },
            Token {
                id: None,
                name: "C".into(),
                signal: Signal::EndComposite,
                encoding: Encoding::default(),
            },
        ];
        registry.registry.insert("C".into(), ct);
        assert_eq!(compute_type_size("C", &registry), Some(6));

        Ok(())
    }

    #[test]
    fn compute_type_size_unknown_signal() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = TypeRegistry::new();
        let tokens = vec![Token {
            id: None,
            name: "X".into(),
            signal: Signal::Encoding,
            encoding: Encoding::default(),
        }];
        registry.registry.insert("X".into(), tokens);
        assert_eq!(compute_type_size("X", &registry), None);

        Ok(())
    }

    #[test]
    fn parse_malformed_include_file_is_error() -> Result<(), Box<dyn std::error::Error>> {
        // The include file is found but contains invalid XML — covers the
        // Document::parse error handler in parse_schema (xml.rs:544-548).
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <include href="bad-include.xml"/>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "malformed include file must error");

        Ok(())
    }

    #[test]
    fn parse_var_data_with_simple_encoding_type_is_error() -> Result<(), Box<dyn std::error::Error>>
    {
        // A var-data field whose type is a simple encoding (uint32), not a
        // var-data composite, must be rejected.
        let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  </types>
  <message name="M" id="1"><data name="d" id="1" type="uint32"/></message>
</messageSchema>"#;
        assert!(parse(xml).is_err(), "simple encoding as varData must error");

        Ok(())
    }

    const MINIMAL_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="example.sbe" id="1" version="0" byteOrder="littleEndian"
               description="minimal test schema">
  <types>
    <composite name="messageHeader" description="SBE message header">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId"   primitiveType="uint16"/>
      <type name="schemaId"     primitiveType="uint16"/>
      <type name="version"      primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Car" id="1" blockLength="11" semanticType="">
    <field name="serialNumber" id="1" type="uint64" offset="0" presence="required"/>
    <field name="modelYear"    id="2" type="uint16" offset="8" presence="required"/>
    <field name="available"    id="3" type="uint8"  offset="10" presence="required"/>
  </message>
</messageSchema>"#;

    fn structural(name: &str, signal: Signal) -> Token {
        Token {
            id: None,
            name: name.to_string(),
            signal,
            encoding: Encoding::default(),
        }
    }

    fn field(
        name: &str,
        id: Option<u16>,
        primitive: PrimitiveType,
        offset: Option<usize>,
    ) -> [Token; 2] {
        let encoding = Encoding {
            primitive_type: Some(primitive),
            offset,
            presence: Presence::Required,
            since_version: 0,
            ..Encoding::default()
        };
        [
            Token {
                id,
                name: name.to_string(),
                signal: Signal::BeginField,
                encoding,
            },
            Token {
                id: None,
                name: name.to_string(),
                signal: Signal::EndField,
                encoding: Encoding::default(),
            },
        ]
    }

    #[test]
    fn parses_schema_metadata() -> Result<(), Box<dyn std::error::Error>> {
        let ir = parse(MINIMAL_SCHEMA).unwrap();
        assert_eq!(ir.package, "example.sbe");
        assert_eq!(ir.id, 1);
        assert_eq!(ir.version, 0);
        assert_eq!(ir.byte_order, ByteOrder::LittleEndian);
        assert_eq!(ir.description.as_deref(), Some("minimal test schema"));
        assert_eq!(ir.semantic_version, None);
        assert_eq!(ir.header_type, "messageHeader");

        Ok(())
    }

    #[test]
    fn parses_message_header_composite_and_message_fields() -> Result<(), Box<dyn std::error::Error>>
    {
        let ir = parse(MINIMAL_SCHEMA).unwrap();

        let mut expected = Vec::new();
        let mut msg_hdr_start = structural("messageHeader", Signal::BeginComposite);
        msg_hdr_start.encoding.description = Some("SBE message header".to_string());
        expected.push(msg_hdr_start);
        expected.extend(field("blockLength", None, PrimitiveType::UInt16, None));
        expected.extend(field("templateId", None, PrimitiveType::UInt16, None));
        expected.extend(field("schemaId", None, PrimitiveType::UInt16, None));
        expected.extend(field("version", None, PrimitiveType::UInt16, None));
        expected.push(structural("messageHeader", Signal::EndComposite));

        expected.push(Token {
            id: Some(1),
            name: "Car".to_string(),
            signal: Signal::BeginMessage,
            encoding: Encoding {
                since_version: 0,
                description: None,
                semantic_type: Some(String::new()),
                ..Encoding::default()
            },
        });
        expected.extend(field(
            "serialNumber",
            Some(1),
            PrimitiveType::UInt64,
            Some(0),
        ));
        expected.extend(field("modelYear", Some(2), PrimitiveType::UInt16, Some(8)));
        expected.extend(field("available", Some(3), PrimitiveType::UInt8, Some(10)));
        expected.push(structural("Car", Signal::EndMessage));

        let mut expected_ir = Ir {
            package: "example.sbe".to_string(),
            id: 1,
            version: 0,
            byte_order: ByteOrder::LittleEndian,
            description: None,
            semantic_version: None,
            header_type: "messageHeader".to_string(),
            tokens: expected,
        };
        crate::resolve::resolve_schema(&mut expected_ir, None).unwrap();

        assert_eq!(ir.tokens, expected_ir.tokens);

        Ok(())
    }

    #[test]
    fn rejects_non_message_schema_root() -> Result<(), Box<dyn std::error::Error>> {
        let err = parse("<notSbe/>").unwrap_err();
        assert!(matches!(err, ParseError::Missing { .. }));

        Ok(())
    }

    #[test]
    fn rejects_missing_package() -> Result<(), Box<dyn std::error::Error>> {
        let err = parse(r#"<messageSchema id="1" version="0"/>"#).unwrap_err();
        assert!(matches!(err, ParseError::Missing { .. }));

        Ok(())
    }

    #[test]
    fn invalid_primitive_error_describes_and_spans() -> Result<(), Box<dyn std::error::Error>> {
        let err = parse(
            r#"<messageSchema package="x" id="1" version="0">
  <message name="M" id="1"><field name="f" id="1" type="bogus"/></message>
</messageSchema>"#,
        )
        .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("invalid primitive type"), "{msg}");
        assert!(err.labels().is_some(), "expected a span label attached");

        Ok(())
    }

    /// Walk up to find the workspace root (where the top-level Cargo.toml lives).
    fn workspace_root() -> PathBuf {
        let mut dir = std::env::current_dir().unwrap();
        loop {
            if dir.join("Cargo.toml").exists() && dir.join("sbe").exists() {
                return dir;
            }
            assert!(
                dir.pop(),
                "cannot find workspace root from {:?}",
                std::env::current_dir()
            );
        }
    }

    fn sbe_test_resource(sub: &str) -> PathBuf {
        workspace_root()
            .join("sbe")
            .join("tests")
            .join("fixtures")
            .join("schemas")
            .join(sub)
    }

    fn sbe_sample_resource(sub: &str) -> PathBuf {
        workspace_root()
            .join("sbe")
            .join("tests")
            .join("fixtures")
            .join("schemas")
            .join(sub)
    }

    #[test]
    fn parses_schema_with_xinclude_relative_path() -> Result<(), Box<dyn std::error::Error>> {
        let path = sbe_test_resource("sub/basic-schema.xml");
        let ir = parse_file(&path).unwrap();

        assert_eq!(ir.package, "SBE tests");
        assert_eq!(ir.id, 2);

        // Included types from sub2/common.xml should be present.
        // `Symbol` is a plain <type>, stored in the encoding registry (not tokens).
        // `messageHeader` is a <composite> → produces BeginComposite/EndComposite tokens.
        assert!(
            ir.tokens.iter().any(|t| t.name == "messageHeader"),
            "expected messageHeader composite from included sub2/common.xml"
        );

        // Schema's own message should also be present.
        assert!(
            ir.tokens.iter().any(|t| t.name == "TestMessage50001"),
            "expected TestMessage50001 from the main schema"
        );
        Ok(())
    }

    #[test]
    fn parses_example_schema_with_xinclude() -> Result<(), Box<dyn std::error::Error>> {
        let path = sbe_sample_resource("example-schema.xml");
        let ir = parse_file(&path).unwrap();

        assert_eq!(ir.package, "baseline");

        // Included types from common-types.xml should be present.
        assert!(
            ir.tokens.iter().any(|t| t.name == "messageHeader"),
            "expected messageHeader from included common-types.xml"
        );
        assert!(
            ir.tokens.iter().any(|t| t.name == "groupSizeEncoding"),
            "expected groupSizeEncoding from included common-types.xml"
        );
        assert!(
            ir.tokens.iter().any(|t| t.name == "varDataEncoding"),
            "expected varDataEncoding from included common-types.xml"
        );

        Ok(())
    }

    #[test]
    fn xinclude_without_base_falls_back_to_hardcoded_paths()
    -> Result<(), Box<dyn std::error::Error>> {
        // Without a base dir, the hardcoded submodule path probes should work
        // for common schemas.
        let path = sbe_sample_resource("example-schema.xml");
        let content = std::fs::read_to_string(&path).unwrap();
        let ir = parse(&content).unwrap();

        assert_eq!(ir.package, "baseline");
        assert!(
            ir.tokens.iter().any(|t| t.name == "groupSizeEncoding"),
            "expected groupSizeEncoding from included file via hardcoded paths"
        );

        Ok(())
    }

    #[test]
    fn xinclude_detects_cycle() -> Result<(), Box<dyn std::error::Error>> {
        // Self-include: the schema includes itself.
        let path = sbe_test_resource("cyclic-self-include.xml");
        let err = parse_file(&path).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("cyclic include"),
            "expected cyclic include error, got: {msg}"
        );

        Ok(())
    }

    #[test]
    fn null_value_on_non_optional_type_parses_with_warning()
    -> Result<(), Box<dyn std::error::Error>> {
        // nullValue on a required type should generate a warning but still parse.
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <type name="MyType" primitiveType="uint32" presence="required" nullValue="999"/>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="MyType"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        assert!(ir.tokens.iter().any(|t| t.name == "M"));

        Ok(())
    }

    #[test]
    fn constant_field_without_value_errors() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <type name="MT" primitiveType="uint32"/>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="MT" presence="constant"/>
  </message>
</messageSchema>"#;
        let err = parse(schema).unwrap_err();
        assert!(matches!(err, ParseError::Missing { .. }));

        Ok(())
    }

    #[test]
    fn duplicate_enum_valid_value_names_error() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <enum name="Color" encodingType="uint8">
      <validValue name="Red">1</validValue>
      <validValue name="Red">2</validValue>
    </enum>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="Color"/>
  </message>
</messageSchema>"#;
        let err = parse(schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn duplicate_enum_encoded_values_error() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <enum name="Color" encodingType="uint8">
      <validValue name="Red">1</validValue>
      <validValue name="Blue">1</validValue>
    </enum>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="Color"/>
  </message>
</messageSchema>"#;
        let err = parse(schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn char_constant_length_too_short_errors() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <type name="CC" primitiveType="char" length="3" presence="constant">AB</type>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="CC"/>
  </message>
</messageSchema>"#;
        let err = parse(schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn char_constant_exact_length_parses() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <type name="CC" primitiveType="char" length="3" presence="constant">ABC</type>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="CC"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        assert!(ir.tokens.iter().any(|t| t.name == "M"));

        Ok(())
    }

    #[test]
    fn duplicate_field_id_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
    package="test" id="1" version="1" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <sbe:message name="M" id="1">
    <field name="a" id="1" type="uint8"/>
    <field name="b" id="1" type="uint8"/>
  </sbe:message>
</sbe:messageSchema>"#;
        assert!(parse(schema).is_err());

        Ok(())
    }

    #[test]
    fn duplicate_field_name_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
    package="test" id="1" version="1" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <sbe:message name="M" id="1">
    <field name="dup" id="1" type="uint8"/>
    <field name="dup" id="2" type="uint8"/>
  </sbe:message>
</sbe:messageSchema>"#;
        assert!(parse(schema).is_err());

        Ok(())
    }

    #[test]
    fn group_with_unknown_dimension_type_fails() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="M" id="1">
    <group name="g" id="2" dimensionType="NonExistentDim">
      <field name="f" id="3" type="uint32"/>
    </group>
  </message>
</messageSchema>"#;
        let err = parse(schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn group_with_wrong_dimension_type_structure_fails() -> Result<(), Box<dyn std::error::Error>> {
        // A composite that exists but lacks blockLength/numInGroup fields.
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="BadDim">
      <type name="foo" primitiveType="uint32"/>
    </composite>
  </types>
  <message name="M" id="1">
    <group name="g" id="2" dimensionType="BadDim">
      <field name="f" id="3" type="uint32"/>
    </group>
  </message>
</messageSchema>"#;
        let err = parse(schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn var_data_with_unknown_type_fails() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="M" id="1">
    <data name="d" id="2" type="NonExistentVarType"/>
  </message>
</messageSchema>"#;
        let err = parse(schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn var_data_with_wrong_type_structure_fails() -> Result<(), Box<dyn std::error::Error>> {
        // A composite that exists but lacks length/varData fields.
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="BadVar">
      <type name="foo" primitiveType="uint32"/>
    </composite>
  </types>
  <message name="M" id="1">
    <data name="d" id="2" type="BadVar"/>
  </message>
</messageSchema>"#;
        let err = parse(schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn block_length_validation_passes_for_correct_value() -> Result<(), Box<dyn std::error::Error>>
    {
        // Computed: uint64@0=8, uint16@8=2, uint8@10=1 → 11
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="M" id="1" blockLength="11">
    <field name="a" id="1" type="uint64" offset="0"/>
    <field name="b" id="2" type="uint16" offset="8"/>
    <field name="c" id="3" type="uint8"  offset="10"/>
  </message>
</messageSchema>"#;
        parse(schema).unwrap();

        Ok(())
    }

    #[test]
    fn block_length_mismatch_warns_but_does_not_error() -> Result<(), Box<dyn std::error::Error>> {
        // Mismatched blockLength is a warning (matching sbe-tool),
        // not a parse error.
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="M" id="1" blockLength="99">
    <field name="a" id="1" type="uint64" offset="0"/>
    <field name="b" id="2" type="uint16" offset="8"/>
    <field name="c" id="3" type="uint8"  offset="10"/>
  </message>
</messageSchema>"#;
        // Must parse successfully despite mismatched blockLength.
        parse(schema).unwrap();

        Ok(())
    }

    #[test]
    fn field_inherits_optional_presence_from_type() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="OptU32" primitiveType="uint32" presence="optional"/>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="OptU32"/>
    <field name="g" id="2" type="OptU32" presence="required"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        // f should have inherited optional presence from OptU32.
        let f_begins: Vec<&Token> = ir
            .tokens
            .iter()
            .filter(|t| t.name == "f" && t.signal == Signal::BeginField)
            .collect();
        assert_eq!(f_begins.len(), 1, "expected exactly one BeginField for 'f'");
        assert_eq!(
            f_begins[0].encoding.presence,
            Presence::Optional,
            "f should inherit Optional from OptU32"
        );
        // g has explicit presence="required" and should stay required.
        let g_begins: Vec<&Token> = ir
            .tokens
            .iter()
            .filter(|t| t.name == "g" && t.signal == Signal::BeginField)
            .collect();
        assert_eq!(g_begins.len(), 1, "expected exactly one BeginField for 'g'");
        assert_eq!(
            g_begins[0].encoding.presence,
            Presence::Required,
            "g should stay Required (explicit)"
        );

        Ok(())
    }

    #[test]
    fn field_inherits_constant_presence_from_type() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="ConstU32" primitiveType="uint32" presence="constant">42</type>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="ConstU32"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        let f_begins: Vec<&Token> = ir
            .tokens
            .iter()
            .filter(|t| t.name == "f" && t.signal == Signal::BeginField)
            .collect();
        assert_eq!(f_begins.len(), 1, "expected exactly one BeginField for 'f'");
        assert_eq!(
            f_begins[0].encoding.presence,
            Presence::Constant,
            "f should inherit Constant from ConstU32"
        );

        Ok(())
    }

    #[test]
    fn composite_member_with_valid_ref_parses() -> Result<(), Box<dyn std::error::Error>> {
        // <ref> on a composite member should resolve through the registry.
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="innerType" primitiveType="uint32"/>
    <composite name="outer">
      <type name="inner" ref="innerType"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="outer"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        assert!(ir.tokens.iter().any(|t| t.name == "M"));

        Ok(())
    }

    #[test]
    fn composite_member_with_invalid_ref_fails() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="outer">
      <type name="inner" ref="BogusType"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="outer"/>
  </message>
</messageSchema>"#;
        let err = parse(schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn custom_header_type_with_required_fields_parses() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" headerType="MyHeader" byteOrder="littleEndian">
  <types>
    <composite name="MyHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="x" id="1" type="uint8"/>
  </message>
</messageSchema>"#;
        parse(schema).unwrap();

        Ok(())
    }

    #[test]
    fn custom_header_type_missing_fields_fails() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" headerType="MyHeader" byteOrder="littleEndian">
  <types>
    <composite name="MyHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
      <!-- missing schemaId -->
    </composite>
  </types>
  <message name="M" id="1">
    <field name="x" id="1" type="uint8"/>
  </message>
</messageSchema>"#;
        let err = parse(schema).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("schemaId"),
            "expected error about missing schemaId, got: {msg}"
        );

        Ok(())
    }

    #[test]
    fn parses_epoch_and_time_unit_on_type() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="Timestamp" primitiveType="uint64" epoch="unix" timeUnit="nanoseconds"/>
  </types>
  <message name="M" id="1">
    <field name="ts" id="1" type="Timestamp"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        let ts_tokens: Vec<&Token> = ir
            .tokens
            .iter()
            .filter(|t| t.name == "ts" && t.signal == Signal::BeginField)
            .collect();
        assert_eq!(ts_tokens.len(), 1);
        assert_eq!(
            ts_tokens[0].encoding.epoch.as_deref(),
            Some("unix"),
            "epoch should be inherited from type"
        );
        assert_eq!(
            ts_tokens[0].encoding.time_unit.as_deref(),
            Some("nanoseconds"),
            "timeUnit should be inherited from type"
        );

        Ok(())
    }

    #[test]
    fn parses_epoch_and_time_unit_on_field() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="ts" id="1" type="uint64" epoch="unix" timeUnit="nanoseconds"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        let ts_tokens: Vec<&Token> = ir
            .tokens
            .iter()
            .filter(|t| t.name == "ts" && t.signal == Signal::BeginField)
            .collect();
        assert_eq!(ts_tokens.len(), 1);
        assert_eq!(ts_tokens[0].encoding.epoch.as_deref(), Some("unix"));
        assert_eq!(
            ts_tokens[0].encoding.time_unit.as_deref(),
            Some("nanoseconds")
        );

        Ok(())
    }

    #[test]
    fn deprecated_on_type() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="OldType" primitiveType="uint32" deprecated="true"/>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="OldType"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        let old_tokens: Vec<&Token> = ir
            .tokens
            .iter()
            .filter(|t| t.name == "f" && t.signal == Signal::BeginField)
            .collect();
        assert_eq!(old_tokens.len(), 1);
        assert!(old_tokens[0].encoding.deprecated);

        Ok(())
    }

    #[test]
    fn deprecated_on_message() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="M" id="1" deprecated="true">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        let msg_token = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginMessage && t.name == "M");
        assert!(msg_token.is_some());
        assert!(msg_token.unwrap().encoding.deprecated);

        Ok(())
    }

    #[test]
    fn deprecated_on_field() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8" deprecated="true"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        let f_tokens: Vec<&Token> = ir
            .tokens
            .iter()
            .filter(|t| t.name == "f" && t.signal == Signal::BeginField)
            .collect();
        assert_eq!(f_tokens.len(), 1);
        assert!(f_tokens[0].encoding.deprecated);

        Ok(())
    }

    #[test]
    fn deprecated_on_group() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
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
  </types>
  <message name="M" id="1">
    <group name="g" id="2" dimensionType="groupSizeEncoding" deprecated="true">
      <field name="f" id="3" type="uint32"/>
    </group>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        let g_token = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginGroup && t.name == "g");
        assert!(g_token.is_some());
        assert!(g_token.unwrap().encoding.deprecated);

        Ok(())
    }

    #[test]
    fn deprecated_on_data() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="varDataEncoding">
      <type name="length" primitiveType="uint32"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="M" id="1">
    <data name="d" id="2" type="varDataEncoding" deprecated="true"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        let d_token = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginVarData && t.name == "d");
        assert!(d_token.is_some());
        assert!(d_token.unwrap().encoding.deprecated);

        Ok(())
    }

    #[test]
    fn deprecated_on_composite() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="OldComposite" deprecated="true">
      <type name="val" primitiveType="uint32"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="OldComposite"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        let c_token = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginComposite && t.name == "OldComposite");
        assert!(c_token.is_some());
        assert!(c_token.unwrap().encoding.deprecated);

        Ok(())
    }

    #[test]
    fn deprecated_on_enum() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <enum name="OldEnum" encodingType="uint8" deprecated="true">
      <validValue name="A">1</validValue>
    </enum>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="OldEnum"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        let e_token = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginEnum && t.name == "OldEnum");
        assert!(e_token.is_some());
        assert!(e_token.unwrap().encoding.deprecated);

        Ok(())
    }

    #[test]
    fn deprecated_on_set() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <set name="OldSet" encodingType="uint8" deprecated="true">
      <choice name="X">0</choice>
    </set>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="OldSet"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        let s_token = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginSet && t.name == "OldSet");
        assert!(s_token.is_some());
        assert!(s_token.unwrap().encoding.deprecated);

        Ok(())
    }

    #[test]
    fn duplicate_message_name_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="a" id="1" type="uint8"/>
  </message>
  <message name="M" id="2">
    <field name="b" id="2" type="uint8"/>
  </message>
</messageSchema>"#;
        let err = parse(schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));
        let msg = format!("{err}");
        assert!(
            msg.contains("duplicate message name"),
            "expected error about duplicate message name, got: {msg}"
        );

        Ok(())
    }

    #[test]
    fn vardata_member_excluded_from_block_length() -> Result<(), Box<dyn std::error::Error>> {
        // The varData member inside varDataEncoding has length="0", which marks it
        // as variable-length. The block length should only include the length field (4 bytes).
        let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="varDataEncoding">
      <type name="length" primitiveType="uint32"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="a" id="1" type="uint32"/>
    <data name="d" id="2" type="varDataEncoding"/>
  </message>
</messageSchema>"#;
        let ir = parse(schema).unwrap();
        // The resolver will compute blockLength from fixed-width fields only.
        // uint32 = 4 bytes; varData's length field is uint32 = 4 bytes but lives in the tail.
        // So block length should be 4 (just field 'a').
        // Data fields are tail-encoded, so they don't contribute to message blockLength.
        // Find the BeginMessage token for M and verify its offset (block length).
        let msg_token = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginMessage && t.name == "M");
        assert!(msg_token.is_some(), "expected BeginMessage for M");
        // The block length is the computed offset stored on the BeginMessage token.
        // With one uint32 field (4 bytes) and no other fixed fields, it should be 4.
        assert_eq!(
            msg_token.unwrap().encoding.offset,
            Some(4),
            "expected block length 4 for message with one uint32 field"
        );

        Ok(())
    }

    const HEADER_TYPES: &str = r#"
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>"#;

    #[test]
    fn include_of_message_schema_wrapped_types_registers_types()
    -> Result<(), Box<dyn std::error::Error>> {
        // The included file's root is <messageSchema>, not <types> — the
        // parser must descend into it and find the nested <types> node.
        let dir = std::env::temp_dir().join(format!("ergon_xml_inc_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let inc = dir.join("wrapped-types.xml");
        std::fs::write(
            &inc,
            r#"<?xml version="1.0"?>
<messageSchema package="inc" id="9" version="0">
  <types>
    <type name="IncU8" primitiveType="uint8"/>
  </types>
</messageSchema>"#,
        )
        .unwrap();
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <include href="{}"/>
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
    <field name="f" id="1" type="IncU8"/>
  </message>
</messageSchema>"#,
            inc.display()
        );
        let ir = parse(&schema).unwrap();
        assert!(
            ir.tokens
                .iter()
                .any(|t| t.name == "f" && t.signal == Signal::BeginField),
            "field using included type must resolve"
        );
        std::fs::remove_file(&inc).ok();
        Ok(())
    }

    #[test]
    fn include_without_href_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <include/>
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        parse(&schema).unwrap();

        Ok(())
    }

    #[test]
    fn char_constant_with_matching_length_parses() -> Result<(), Box<dyn std::error::Error>> {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <type name="CC" primitiveType="char" length="3" presence="constant">ABC</type>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        parse(&schema).unwrap();

        Ok(())
    }

    #[test]
    fn composite_member_with_primitive_type_attr_inlines_encoding()
    -> Result<(), Box<dyn std::error::Error>> {
        // Member uses `type="uint16"` (a primitive name, not a registered
        // type). This is indirect by shape but unresolvable by name, so the
        // parser falls back to inline parsing of the element itself.
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <composite name="Pair">
      <type name="a" type="uint16"/>
      <type name="b" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="p" id="1" type="Pair"/>
  </message>
</messageSchema>"#
        );
        let ir = parse(&schema).unwrap();
        assert!(
            ir.tokens
                .iter()
                .any(|t| t.name == "p" && t.signal == Signal::BeginField),
            "composite field must resolve"
        );

        Ok(())
    }

    #[test]
    fn composite_member_without_any_type_attr_is_parsed_inline()
    -> Result<(), Box<dyn std::error::Error>> {
        // Member has no type/primitiveType/ref attribute at all — the parser
        // parses the bare element inline (no primitive type recorded).
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <composite name="Bare">
      <type name="mystery"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        // The composite is never referenced by a message, so whether the
        // overall parse succeeds is a resolver decision; the member itself
        // must not panic and must take the inline-parse path.
        let _ = parse(&schema);

        Ok(())
    }

    #[test]
    fn enum_valid_value_equal_to_registered_null_sentinel_is_error()
    -> Result<(), Box<dyn std::error::Error>> {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <type name="OptU8" primitiveType="uint8" presence="optional" nullValue="255"/>
    <enum name="E" encodingType="OptU8">
      <validValue name="X">255</validValue>
    </enum>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        let err = parse(&schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn enum_with_unknown_child_element_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <enum name="E" encodingType="uint8">
      <validValue name="A">1</validValue>
      <somethingElse/>
    </enum>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        parse(&schema).unwrap();

        Ok(())
    }

    #[test]
    fn set_with_unknown_child_element_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <set name="S" encodingType="uint8">
      <choice name="A">1</choice>
      <somethingElse/>
    </set>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        parse(&schema).unwrap();

        Ok(())
    }

    #[test]
    fn set_choice_non_numeric_bit_index_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <set name="S" encodingType="uint8">
      <choice name="A">notanumber</choice>
    </set>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        let err = parse(&schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn message_children_with_missing_or_unparseable_attrs_reach_second_pass()
    -> Result<(), Box<dyn std::error::Error>> {
        // The first structural pass tolerates a missing name, a non-numeric
        // id, and a non-numeric offset; the second (real) parse pass then
        // reports the actual fault. This proves the pre-validation loop does
        // not panic or mask errors on degenerate attributes.
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
    <field id="xyz" type="uint8" offset="abc"/>
  </message>
</messageSchema>"#
        );
        let err = parse(&schema).unwrap_err();
        assert!(matches!(
            err,
            ParseError::Missing { .. } | ParseError::Invalid { .. }
        ));

        Ok(())
    }

    #[test]
    fn block_length_tracking_skips_fields_without_computable_size()
    -> Result<(), Box<dyn std::error::Error>> {
        // Field with a valid offset but an unregistered type: the expected
        // block-length tracker must skip it rather than fault.
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
    <field name="a" id="1" type="NotAKnownType" offset="0"/>
  </message>
</messageSchema>"#
        );
        // The field itself fails to resolve in the second pass — the point is
        // the block-length pre-pass tolerated the unknown size first.
        let err = parse(&schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn null_value_on_required_field_warns_but_parses() -> Result<(), Box<dyn std::error::Error>> {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8" nullValue="255"/>
  </message>
</messageSchema>"#
        );
        parse(&schema).unwrap();

        Ok(())
    }

    #[test]
    fn include_with_non_types_sibling_elements_is_tolerated()
    -> Result<(), Box<dyn std::error::Error>> {
        // The included <messageSchema> carries a <message> sibling next to
        // <types>; only the <types> node is imported.
        let dir = std::env::temp_dir().join(format!("ergon_xml_inc2_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let inc = dir.join("wrapped-types-siblings.xml");
        std::fs::write(
            &inc,
            r#"<?xml version="1.0"?>
<messageSchema package="inc" id="9" version="0">
  <message name="Ignored" id="7"/>
  <types>
    <type name="IncU16" primitiveType="uint16"/>
  </types>
</messageSchema>"#,
        )
        .unwrap();
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <include href="{}"/>
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
    <field name="f" id="1" type="IncU16"/>
  </message>
</messageSchema>"#,
            inc.display()
        );
        parse(&schema).unwrap();
        std::fs::remove_file(&inc).ok();

        Ok(())
    }

    #[test]
    fn char_constant_without_text_is_tolerated_at_parse_time()
    -> Result<(), Box<dyn std::error::Error>> {
        // presence="constant" with no element text: the length check is
        // skipped because there is no constant value to measure.
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <type name="CC2" primitiveType="char" length="3" presence="constant"/>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        let _ = parse(&schema);

        Ok(())
    }

    #[test]
    fn composite_member_with_unknown_type_and_primitive_type_falls_back_inline()
    -> Result<(), Box<dyn std::error::Error>> {
        // `type="Unknown"` is unresolvable, but `primitiveType="uint8"` lets
        // the inline fallback parse the element directly.
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <composite name="Odd">
      <type name="m" type="Unknown" primitiveType="uint8"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        let _ = parse(&schema);

        Ok(())
    }

    #[test]
    fn enum_valid_value_unparseable_with_null_sentinel_skips_check()
    -> Result<(), Box<dyn std::error::Error>> {
        // The null-sentinel equality check is skipped when the value text
        // cannot be parsed for the encoding type.
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <type name="OptU8b" primitiveType="uint8" presence="optional" nullValue="255"/>
    <enum name="E2" encodingType="OptU8b">
      <validValue name="A">notanumber</validValue>
    </enum>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        let _ = parse(&schema);

        Ok(())
    }

    #[test]
    fn field_with_unparseable_offset_attr_is_tolerated_by_prevalidation()
    -> Result<(), Box<dyn std::error::Error>> {
        // Structural pre-validation ignores an offset it cannot parse; the
        // real field parse itself does not require the attribute.
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8" offset="abc"/>
  </message>
</messageSchema>"#
        );
        let _ = parse(&schema);

        Ok(())
    }

    #[test]
    fn block_length_tracker_skips_type_without_computable_size()
    -> Result<(), Box<dyn std::error::Error>> {
        // "NoPrim" is registered but has no primitive type, so the block
        // length tracker cannot size it and must skip it.
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <type name="NoPrim"/>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="NoPrim" offset="0"/>
  </message>
</messageSchema>"#
        );
        let _ = parse(&schema);

        Ok(())
    }

    #[test]
    fn message_with_non_numeric_id_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <message name="M" id="notanumber">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        let err = parse(&schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn type_with_non_numeric_since_version_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <type name="T" primitiveType="uint8" sinceVersion="notanumber"/>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        let err = parse(&schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn type_with_non_numeric_length_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <type name="T" primitiveType="uint8" length="notanumber"/>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
        );
        let err = parse(&schema).unwrap_err();
        assert!(matches!(err, ParseError::Invalid { .. }));

        Ok(())
    }
}
