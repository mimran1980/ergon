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

use std::collections::HashMap;
use std::ops::Range;

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
    #[error("resolution error: {0}")]
    #[diagnostic(code(ergosbe::schema_parse::resolve))]
    Resolve(#[from] crate::resolve::ResolveError),
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
    if let Some(PrimitiveType::Char) = prim_type {
        if s.len() == 1 {
            return Some(s.chars().next().unwrap() as u64);
        }
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

/// Parse an SBE schema document into the token IR.
///
/// # Errors
///
/// Returns a span-bearing [`ParseError`] if the XML is malformed, the root is
/// not a `<messageSchema>`, or a required SBE attribute is missing or invalid.
#[allow(clippy::result_large_err)]
pub fn parse(xml: &str) -> Result<Ir, ParseError> {
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
    let mut ir = parse_schema(root).map_err(|fault| ParseError::from_fault(fault, input))?;
    crate::resolve::resolve_schema(&mut ir)?;
    Ok(ir)
}

fn read_include_file(href: &str) -> Option<String> {
    if let Ok(content) = std::fs::read_to_string(href) {
        return Some(content);
    }
    let paths = [
        format!("simple-binary-encoding/sbe-samples/src/main/resources/{}", href),
        format!("simple-binary-encoding/sbe-benchmarks/src/main/resources/{}", href),
        format!("simple-binary-encoding/sbe-tool/src/test/resources/{}", href),
        format!("../simple-binary-encoding/sbe-samples/src/main/resources/{}", href),
        format!("../simple-binary-encoding/sbe-benchmarks/src/main/resources/{}", href),
        format!("../simple-binary-encoding/sbe-tool/src/test/resources/{}", href),
    ];
    for p in &paths {
        if let Ok(content) = std::fs::read_to_string(p) {
            return Some(content);
        }
    }
    None
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
            _ => {}
        }
    }
    Ok(())
}

/// Parse the `<messageSchema>` root into the [`Ir`].
fn parse_schema(root: Node<'_, '_>) -> Result<Ir, Fault> {
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
                if let Some(included_content) = read_include_file(href) {
                    if let Ok(included_doc) = Document::parse(&included_content) {
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
                }
            }
        } else if child.tag_name().name() == "types" {
            parse_types_node(child, &mut registry, &mut tokens)?;
        }
    }

    // Second pass: Parse all messages
    for child in element_children(root) {
        if child.tag_name().name() == "message" {
            parse_message(child, &registry, &mut tokens)?;
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
    let primitive = node.attribute("primitiveType").or_else(|| node.attribute("type"));
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

    let null_value = node.attribute("nullValue")
        .and_then(|s| parse_u64_val(s, primitive_type));
    let min_value = node.attribute("minValue")
        .and_then(|s| parse_u64_val(s, primitive_type));
    let max_value = node.attribute("maxValue")
        .and_then(|s| parse_u64_val(s, primitive_type));

    let constant_value = if presence == Presence::Constant {
        node.text().map(|s| s.trim().to_string())
    } else {
        None
    };

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

    let mut composite_tokens = Vec::new();
    composite_tokens.push(Token {
        id: None,
        name: name.clone(),
        signal: Signal::BeginComposite,
        encoding: Encoding {
            since_version,
            description: node.attribute("description").map(str::to_string),
            semantic_type: node.attribute("semanticType").map(str::to_string),
            ..Encoding::default()
        },
    });

    for child in element_children(node) {
        if child.tag_name().name() == "type" {
            let member_name = string_attr(child, "name", "composite member @name")?;
            let type_name = child.attribute("type").or_else(|| child.attribute("primitiveType"));
            let since_val = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);

            if let Some(t_name) = type_name {
                if let Some(resolved) = resolve_type_to_tokens(&member_name, t_name, None, registry, since_val) {
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

    registry.registry.insert(name.clone(), composite_tokens.clone());
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

    let encoding_type = registry.encodings.get(&encoding_type_name)
        .and_then(|e| e.primitive_type)
        .ok_or_else(|| Fault::invalid(node, "enum encodingType", &encoding_type_name))?;

    let mut enum_tokens = Vec::new();
    enum_tokens.push(Token {
        id: None,
        name: name.clone(),
        signal: Signal::BeginEnum,
        encoding: Encoding {
            primitive_type: Some(encoding_type),
            since_version,
            description: node.attribute("description").map(str::to_string),
            ..Encoding::default()
        },
    });

    for child in element_children(node) {
        if child.tag_name().name() == "validValue" {
            let val_name = string_attr(child, "name", "validValue @name")?;
            let val_since = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            let val_text = child.text().unwrap_or("").trim();

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

    registry.registry.insert(name.clone(), enum_tokens.clone());
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

    let encoding_type = registry.encodings.get(&encoding_type_name)
        .and_then(|e| e.primitive_type)
        .ok_or_else(|| Fault::invalid(node, "set encodingType", &encoding_type_name))?;

    let mut set_tokens = Vec::new();
    set_tokens.push(Token {
        id: None,
        name: name.clone(),
        signal: Signal::BeginSet,
        encoding: Encoding {
            primitive_type: Some(encoding_type),
            since_version,
            description: node.attribute("description").map(str::to_string),
            ..Encoding::default()
        },
    });

    for child in element_children(node) {
        if child.tag_name().name() == "choice" {
            let choice_name = string_attr(child, "name", "choice @name")?;
            let choice_since = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            let bit_index = child.text().unwrap_or("").trim();

            set_tokens.push(Token {
                id: None,
                name: choice_name,
                signal: Signal::Encoding,
                encoding: Encoding {
                    presence: Presence::Constant,
                    constant_value: Some(bit_index.to_string()),
                    since_version: choice_since,
                    description: child.attribute("description").map(str::to_string),
                    ..Encoding::default()
                },
            });
        }
    }

    set_tokens.push(structural(&name, Signal::EndSet));

    registry.registry.insert(name.clone(), set_tokens.clone());
    tokens.extend(set_tokens);
    Ok(())
}

/// Parse a `<message>` into bracketed `BeginMessage`/`EndMessage` tokens.
fn parse_message(
    node: Node<'_, '_>,
    registry: &TypeRegistry,
    tokens: &mut Vec<Token>,
) -> Result<(), Fault> {
    let name = string_attr(node, "name", "message @name")?;
    let id = u16_attr(node, "id", "message @id")?;
    let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);

    tokens.push(Token {
        id: Some(id),
        name: name.clone(),
        signal: Signal::BeginMessage,
        encoding: Encoding {
            since_version,
            description: node.attribute("description").map(str::to_string),
            semantic_type: node.attribute("semanticType").map(str::to_string),
            ..Encoding::default()
        },
    });

    for child in element_children(node) {
        parse_message_child(child, registry, tokens)?;
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

            if let Some(resolved) = resolve_type_to_tokens(&field_name, &type_name, Some(id), registry, since_version) {
                let mut inlined = resolved;
                if let Some(offset_str) = node.attribute("offset") {
                    if let Ok(offset) = offset_str.parse::<usize>() {
                        if let Some(first) = inlined.first_mut() {
                            first.encoding.offset = Some(offset);
                        }
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
            let dimension_type = node.attribute("dimensionType").unwrap_or("groupSizeEncoding");

            tokens.push(Token {
                id: Some(id),
                name: group_name.clone(),
                signal: Signal::BeginGroup,
                encoding: Encoding {
                    since_version,
                    description: node.attribute("description").map(str::to_string),
                    ..Encoding::default()
                },
            });

            if let Some(dim_tokens) = registry.registry.get(dimension_type) {
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
            let type_name = node.attribute("type").unwrap_or("varDataEncoding");

            tokens.push(Token {
                id: Some(id),
                name: data_name.clone(),
                signal: Signal::BeginVarData,
                encoding: Encoding {
                    since_version,
                    description: node.attribute("description").map(str::to_string),
                    ..Encoding::default()
                },
            });

            if let Some(type_tokens) = registry.registry.get(type_name) {
                tokens.extend(type_tokens.clone());
            } else {
                return Err(Fault::invalid(node, "data type", type_name));
            }

            tokens.push(structural(&data_name, Signal::EndVarData));
        }
        _ => {}
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
  <message name="Car" id="1" blockLength="17" semanticType="">
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

    fn field(name: &str, id: Option<u16>, primitive: PrimitiveType, offset: Option<usize>) -> [Token; 2] {
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
                semantic_type: Some("".to_string()),
                ..Encoding::default()
            },
        });
        expected.extend(field("serialNumber", Some(1), PrimitiveType::UInt64, Some(0)));
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
        crate::resolve::resolve_schema(&mut expected_ir).unwrap();

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
}
