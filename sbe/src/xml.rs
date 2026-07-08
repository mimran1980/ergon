//! SBE XML schema parsing into the token IR.
//!
//! Uses [`roxmltree`] (DOM) so SBE's mixed-order `<type>`/`<enum>`/`<set>`/
//! `<composite>`/`<message>` and forward references resolve naturally, and XML
//! comments are retained as nodes (a later slice maps them to rustdoc alongside
//! `description` attributes). Schema files are KB-scale, so DOM is effectively
//! free here.
//!
//! Errors are [`ParseError`]s with [`miette`] source spans pointing at the
//! offending element, so consumers get a rendered, navigable diagnostic.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};

use roxmltree::{Document, Node};

use crate::ir::{ByteOrder, Encoding, Ir, Presence, PrimitiveType, Signal, Token};

/// Errors raised while parsing an SBE schema. Carries a [`miette`] source span
/// so the offending XML element is highlighted in the rendered diagnostic.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ParseError {
    /// The XML document itself was malformed.
    #[error("malformed XML: {message}")]
    #[diagnostic(code(ergosbe::schema_parse::malformed_xml))]
    MalformedXml {
        /// What went wrong.
        message: String,
        /// The source document, for span rendering.
        #[source_code]
        source_code: miette::NamedSource<String>,
    },
    /// A required attribute or element was missing.
    #[error("missing {what}")]
    #[diagnostic(code(ergosbe::schema_parse::missing))]
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
    #[diagnostic(code(ergosbe::schema_parse::invalid))]
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
    #[diagnostic(code(ergosbe::schema_parse::resolve))]
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
    #[diagnostic(code(ergosbe::schema_parse::include))]
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
    /// Build a `MalformedXml` error from the raw input (no document was parsed).
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

/// Parse an SBE schema document (raw XML string) into the token IR.
///
/// Includes are resolved via the [`parse_file`] function with base-dir
/// awareness. When called without a file path, relative includes fall
/// back to a set of well-known submodule directory probes.
///
/// # Errors
///
/// Returns a span-bearing [`ParseError`] if the XML is malformed, the root is
/// not a `<messageSchema>`, or a required SBE attribute is missing or invalid.
#[allow(clippy::result_large_err)]
pub fn parse(xml: &str) -> Result<Ir, ParseError> {
    parse_with_context(xml, None, &mut HashSet::new())
}

/// Parse an SBE schema file, resolving `<xi:include href="..."/>`
/// elements relative to the parent directory of `path`.
///
/// # Errors
///
/// Returns a span-bearing [`ParseError`] on XML parse failure, I/O error,
/// or schema validation error.
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
/// 3. Well-known submodule paths for the ErgoSBE repo layout
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

    // 1. Relative to the parent schema's directory.
    if let Some(dir) = base_dir {
        let candidate = dir.join(href).to_string_lossy().to_string();
        try_include!(try_read(&candidate, seen));
    }

    // 2. Direct (CWD-relative) probe.
    try_include!(try_read(href, seen));

    // 3. Local fixtures directory.
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
    for type_child in element_children(node) {
        match type_child.tag_name().name() {
            "type" => {
                let name = string_attr(type_child, "name", "type @name")?;
                let encoding = parse_type_element(type_child, registry)?;
                registry.encodings.insert(name, encoding);
            }
            "composite" => {
                parse_composite(type_child, registry, tokens)?;
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
    Ok(())
}

/// Parse the `<messageSchema>` root into the [`Ir`].
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

    let description = root.attribute("description").map(str::to_string);
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

/// Parse a `<type>` element.
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
    let description = node.attribute("description").map(str::to_string);
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

    let constant_value = if presence == Presence::Constant {
        node.text().map(|s| s.trim().to_string())
    } else {
        None
    };

    // Validate char constant value length matches the declared length.
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

/// Parse a `<composite>` into bracketed `BeginComposite`/`EndComposite` tokens.
fn parse_composite(
    node: Node<'_, '_>,
    registry: &mut TypeRegistry,
    tokens: &mut Vec<Token>,
) -> Result<(), Fault> {
    let name = string_attr(node, "name", "composite @name")?;
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
            description: node.attribute("description").map(str::to_string),
            semantic_type: node.attribute("semanticType").map(str::to_string),
            ..Encoding::default()
        },
    });

    for child in element_children(node) {
        if child.tag_name().name() == "type" {
            let member_name = string_attr(child, "name", "composite member @name")?;
            let type_name = child
                .attribute("type")
                .or_else(|| child.attribute("primitiveType"))
                .or_else(|| child.attribute("ref"));
            let since_val = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);

            // Gap 2: validate that a ref attribute points to an existing type.
            if let Some(ref_name) = child.attribute("ref") {
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

            if let Some(t_name) = type_name {
                // Whether this <type> element is an indirect ref (resolved by name
                // through the registry) vs a direct encoding with inline attributes.
                // A `ref` attribute always counts as indirect; a bare `type` attribute
                // counts as indirect only when the name isn't a known primitive encoding.
                let has_ref_attr = child.attribute("ref").is_some();
                let is_indirect_ref = has_ref_attr
                    || (child.attribute("type").is_some()
                        && !registry.encodings.contains_key(t_name));
                if !is_indirect_ref {
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

/// Parse an `<enum>` into bracketed `BeginEnum`/`EndEnum` tokens.
fn parse_enum(
    node: Node<'_, '_>,
    registry: &mut TypeRegistry,
    tokens: &mut Vec<Token>,
) -> Result<(), Fault> {
    let name = string_attr(node, "name", "enum @name")?;
    let encoding_type_name = string_attr(node, "encodingType", "enum @encodingType")?;
    let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);

    let encoding_type = registry
        .encodings
        .get(&encoding_type_name)
        .and_then(|e| e.primitive_type)
        .ok_or_else(|| Fault::invalid(node, "enum encodingType", &encoding_type_name))?;

    // Enum encoding types must be integer or char (Aeron requirement).
    // Float/Double enums are not valid SBE.
    if matches!(encoding_type, PrimitiveType::Float | PrimitiveType::Double) {
        return Err(Fault::invalid(
            node,
            "enum encodingType",
            format!("{encoding_type:?}: enum encoding must be integer or char, not float/double"),
        ));
    }

    let mut enum_tokens = Vec::new();
    enum_tokens.push(Token {
        id: None,
        name: name.clone(),
        signal: Signal::BeginEnum,
        encoding: Encoding {
            primitive_type: Some(encoding_type),
            since_version,
            deprecated: node.attribute("deprecated").is_some(),
            description: node.attribute("description").map(str::to_string),
            ..Encoding::default()
        },
    });

    // Resolve null sentinel for the enum's encoding type (Aeron: valid values
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

            enum_tokens.push(Token {
                id: None,
                name: val_name,
                signal: Signal::Encoding,
                encoding: Encoding {
                    presence: Presence::Constant,
                    constant_value: Some(val_text.to_string()),
                    since_version: val_since,
                    description: child.attribute("description").map(str::to_string),
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

/// Parse a `<set>` into bracketed `BeginSet`/`EndSet` tokens.
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

    // Set encoding types must be unsigned integers (Aeron requirement).
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
            description: node.attribute("description").map(str::to_string),
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

            // Validate bit index is a valid number within the encoding width.
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
                    description: child.attribute("description").map(str::to_string),
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
            description: node.attribute("description").map(str::to_string),
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

    // Block-length tracking
    let mut expected_block_len: usize = 0;
    let mut all_fields_have_offsets = true;
    let mut any_field_counted = false;

    for child in element_children(node) {
        parse_message_child(child, registry, tokens)?;
        // Collect field IDs, names, and offsets for validation
        if child.tag_name().name() == "field"
            || child.tag_name().name() == "group"
            || child.tag_name().name() == "data"
        {
            if let Some(name_attr) = child.attribute("name") {
                let child_name = name_attr.to_string();
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

        // Track expected block length from fixed-size fields
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
    // Uses a warning (not an error) to match Aeron sbe-tool behavior — the
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
            let deprecated = explicit_deprecated.is_some()
                || type_encoding.map_or(false, |e| e.deprecated);
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
                    // variant exist at parse time (Aeron rejects invalid valueRef).
                    if let Some((enum_name, variant_name)) = s.split_once('.') {
                        // ponytail: validate enum exists; deferred validation of variant
                        // existence is done in IR resolution.
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
                    description: node.attribute("description").map(str::to_string),
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
                    description: node.attribute("description").map(str::to_string),
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
                // (Aeron's makeDataFieldCompositeType equivalent — gap 10).
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
    node.children().filter(Node::is_element)
}

fn string_attr(node: Node<'_, '_>, name: &str, what: &str) -> Result<String, Fault> {
    node.attribute(name)
        .map(str::to_string)
        .ok_or_else(|| Fault::missing(node, what))
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
/// Aeron validates that the header type carries these four mandatory fields.
/// If the header type is not found in the registry it isn't flagged here
/// (the missing type error will surface elsewhere, e.g. in resolution).
fn validate_header_type(header_type: &str, registry: &TypeRegistry) -> Result<(), Fault> {
    let tokens = match registry.registry.get(header_type) {
        Some(t) if !t.is_empty() && t[0].signal == Signal::BeginComposite => t,
        _ => return Ok(()), // Not in the registry or not a composite — skip
    };

    // Collect the field names present in the composite.
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

    // Composite, enum, or set stored in the registry
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
    fn parses_schema_metadata() {
        let ir = parse(MINIMAL_SCHEMA).unwrap();
        assert_eq!(ir.package, "example.sbe");
        assert_eq!(ir.id, 1);
        assert_eq!(ir.version, 0);
        assert_eq!(ir.byte_order, ByteOrder::LittleEndian);
        assert_eq!(ir.description.as_deref(), Some("minimal test schema"));
        assert_eq!(ir.semantic_version, None);
        assert_eq!(ir.header_type, "messageHeader");
    }

    #[test]
    fn parses_message_header_composite_and_message_fields() {
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
    }

    #[test]
    fn rejects_non_message_schema_root() {
        let err = parse("<notSbe/>").unwrap_err();
        assert!(matches!(err, ParseError::Missing { .. }));
    }

    #[test]
    fn rejects_missing_package() {
        let err = parse(r#"<messageSchema id="1" version="0"/>"#).unwrap_err();
        assert!(matches!(err, ParseError::Missing { .. }));
    }

    #[test]
    fn invalid_primitive_error_describes_and_spans() {
        let err = parse(
            r#"<messageSchema package="x" id="1" version="0">
  <message name="M" id="1"><field name="f" id="1" type="bogus"/></message>
</messageSchema>"#,
        )
        .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("invalid primitive type"), "{msg}");
        assert!(err.labels().is_some(), "expected a span label attached");
    }

    // ── XInclude tests ─────────────────────────────────────────────────

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
    fn parses_schema_with_xinclude_relative_path() {
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
    }

    #[test]
    fn parses_example_schema_with_xinclude() {
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
    }

    #[test]
    fn xinclude_without_base_falls_back_to_hardcoded_paths() {
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
    }

    #[test]
    fn xinclude_detects_cycle() {
        // Self-include: the schema includes itself.
        let path = sbe_test_resource("cyclic-self-include.xml");
        let err = parse_file(&path).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("cyclic include"),
            "expected cyclic include error, got: {msg}"
        );
    }

    // ── Validation tests ─────────────────────────────────────────────

    #[test]
    fn null_value_on_non_optional_type_parses_with_warning() {
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
    }

    #[test]
    fn constant_field_without_value_errors() {
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
    }

    #[test]
    fn duplicate_enum_valid_value_names_error() {
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
    }

    #[test]
    fn duplicate_enum_encoded_values_error() {
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
    }

    #[test]
    fn char_constant_length_too_short_errors() {
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
    }

    #[test]
    fn char_constant_exact_length_parses() {
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
    }

    #[test]
    fn duplicate_field_id_is_rejected() {
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
    }

    #[test]
    fn duplicate_field_name_is_rejected() {
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
    }

    #[test]
    fn group_with_unknown_dimension_type_fails() {
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
    }

    #[test]
    fn group_with_wrong_dimension_type_structure_fails() {
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
    }

    #[test]
    fn var_data_with_unknown_type_fails() {
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
    }

    #[test]
    fn var_data_with_wrong_type_structure_fails() {
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
    }

    #[test]
    fn block_length_validation_passes_for_correct_value() {
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
    }

    #[test]
    fn block_length_mismatch_warns_but_does_not_error() {
        // Mismatched blockLength is a warning (matching Aeron sbe-tool),
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
    }

    // ── Gap 1: presence inheritance from referenced types ─────────────

    #[test]
    fn field_inherits_optional_presence_from_type() {
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
    }

    #[test]
    fn field_inherits_constant_presence_from_type() {
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
    }

    // ── Gap 2: composite child ref attributes ─────────────────────────

    #[test]
    fn composite_member_with_valid_ref_parses() {
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
    }

    #[test]
    fn composite_member_with_invalid_ref_fails() {
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
    }

    // ── Gap 3: header type well-formedness ─────────────────────────────

    #[test]
    fn custom_header_type_with_required_fields_parses() {
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
    }

    #[test]
    fn custom_header_type_missing_fields_fails() {
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
    }

    // ── Gap 10/11: epoch and timeUnit on types and fields ──────────────

    #[test]
    fn parses_epoch_and_time_unit_on_type() {
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
    }

    #[test]
    fn parses_epoch_and_time_unit_on_field() {
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
        assert_eq!(
            ts_tokens[0].encoding.epoch.as_deref(),
            Some("unix")
        );
        assert_eq!(
            ts_tokens[0].encoding.time_unit.as_deref(),
            Some("nanoseconds")
        );
    }

    // ── Gap 12: deprecated attribute on all elements ────────────────────

    #[test]
    fn deprecated_on_type() {
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
    }

    #[test]
    fn deprecated_on_message() {
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
    }

    #[test]
    fn deprecated_on_field() {
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
    }

    #[test]
    fn deprecated_on_group() {
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
    }

    #[test]
    fn deprecated_on_data() {
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
    }

    #[test]
    fn deprecated_on_composite() {
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
    }

    #[test]
    fn deprecated_on_enum() {
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
    }

    #[test]
    fn deprecated_on_set() {
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
    }

    // ── Gap 13: duplicate message name ─────────────────────────────────

    #[test]
    fn duplicate_message_name_is_rejected() {
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
    }

    // ── Gap 10: varData variable-length member does not contribute to block length ──

    #[test]
    fn vardata_member_excluded_from_block_length() {
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
    }
}
