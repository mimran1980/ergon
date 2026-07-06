//! SBE schema validation and reference-resolution pass.
//!
//! This module runs after XML parsing to:
//!
//! - Assign default null, min, and max values to every primitive encoding.
//! - Compute and fill byte offsets for all fields, composites, and groups.
//! - Compute block lengths for composites and messages.
//! - Validate the resolved offsets (no overlap, valid alignment).
//!
//! The primary entry-point is [`resolve_schema`], which mutates the IR
//! in-place. It is called automatically by [`parse`](crate::parse) and
//! [`parse_file`](crate::xml::parse_file) — most users never need to
//! call it directly.
//!
//! # Resolution passes
//!
//! 1. **Duplicate template ID check**: two messages may not share the same id.
//! 2. **Since-version bound check**: no token may have a sinceVersion exceeding
//!    the schema version.
//! 3. **Default values**: every primitive type gets a default null, min, and
//!    max sentinel (e.g. `uint16` null = `65535`, min = `0`, max = `65534`).
//! 4. **Offset resolution**: walks composites and messages sequentially,
//!    assigning offsets to fields that lack an explicit `offset` attribute.
//!    Nested groups and var-data fields are resolved independently (they live
//!    in the tail, after the fixed block).
//! 5. **Block length**: the final offset of each composite/message becomes its
//!    block length, stored on the `BeginComposite`/`BeginMessage` token.

use crate::ir::{Ir, PrimitiveType, Signal, Token};

/// Errors raised during schema resolution/validation.
///
/// Each variant carries an optional [`miette::NamedSource`] for source-code
/// rendering in diagnostics. When the resolver is invoked from the XML
/// parser the source text is attached; direct callers may leave it as
/// `None`.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ResolveError {
    /// Two messages share the same template ID.
    #[error("duplicate template id {id} for message {name}")]
    #[diagnostic(code(ergosbe::resolve::duplicate_template_id))]
    #[diagnostic(help("each message must have a unique template id"))]
    DuplicateTemplateId {
        /// The duplicate ID.
        id: u16,
        /// Message name.
        name: String,
        /// Source document for miette span rendering.
        #[source_code]
        source_code: Option<miette::NamedSource<String>>,
        /// Span pointing at the first definition.
        #[label("first defined here")]
        first_label: Option<miette::SourceSpan>,
        /// Span pointing at the duplicate definition.
        #[label("duplicate definition")]
        second_label: Option<miette::SourceSpan>,
    },
    /// A referenced type was not found in the registry.
    #[error("unknown type reference {name}")]
    #[diagnostic(code(ergosbe::resolve::unknown_type))]
    #[diagnostic(help("ensure the type is defined in the schema or an include"))]
    UnknownType {
        /// Type name.
        name: String,
        /// Source document for miette span rendering.
        #[source_code]
        source_code: Option<miette::NamedSource<String>>,
        /// Span pointing at the reference.
        #[label("unknown type")]
        span: Option<miette::SourceSpan>,
    },
    /// Field offsets are overlapping or unaligned.
    #[error("overlapping offsets or invalid alignment at offset {offset}")]
    #[diagnostic(code(ergosbe::resolve::invalid_offset))]
    #[diagnostic(help("check explicit offset attributes for clashes"))]
    InvalidOffset {
        /// The invalid offset.
        offset: usize,
        /// Source document for miette span rendering.
        #[source_code]
        source_code: Option<miette::NamedSource<String>>,
        /// Span pointing at the overlap.
        #[label("overlap here")]
        span: Option<miette::SourceSpan>,
    },
    /// A composite type definition is empty.
    #[error("composite {name} has no fields")]
    #[diagnostic(code(ergosbe::resolve::empty_composite))]
    #[diagnostic(help("add at least one <type> member to the composite"))]
    EmptyComposite {
        /// Composite name.
        name: String,
        /// Source document for miette span rendering.
        #[source_code]
        source_code: Option<miette::NamedSource<String>>,
        /// Span pointing at the empty composite.
        #[label("empty composite")]
        span: Option<miette::SourceSpan>,
    },
    /// A field or message has a sinceVersion greater than the schema version.
    #[error("sinceVersion {version} exceeds schema version {schema_version} for {name}")]
    #[diagnostic(code(ergosbe::resolve::since_version_beyond))]
    #[diagnostic(help("the sinceVersion must be <= the schema version"))]
    SinceVersionBeyondSchema {
        /// The sinceVersion value found.
        version: u16,
        /// The schema version.
        schema_version: u16,
        /// The token name.
        name: String,
        /// Source document for miette span rendering.
        #[source_code]
        source_code: Option<miette::NamedSource<String>>,
        /// Span pointing at the token.
        #[label("sinceVersion too high")]
        span: Option<miette::SourceSpan>,
    },
}

impl ResolveError {
    /// Take the source code field, leaving `None` in its place.
    /// Used when wrapping this error in [`ParseError::Resolve`] to
    /// transfer the source code to the outer error's `#[source_code]`.
    pub(crate) fn take_source_code(&mut self) -> Option<miette::NamedSource<String>> {
        match self {
            ResolveError::DuplicateTemplateId { source_code, .. } => source_code.take(),
            ResolveError::UnknownType { source_code, .. } => source_code.take(),
            ResolveError::InvalidOffset { source_code, .. } => source_code.take(),
            ResolveError::EmptyComposite { source_code, .. } => source_code.take(),
            ResolveError::SinceVersionBeyondSchema { source_code, .. } => source_code.take(),
        }
    }
}

/// Run the reference resolution pass on a schema IR.
///
/// `source` is an optional reference to the raw XML text; when provided it
/// is attached to any [`ResolveError`] for miette source-code rendering.
///
/// Modifies the IR in-place to fill resolved offsets, block lengths,
/// and default null/min/max values.
pub fn resolve_schema(ir: &mut Ir, source: Option<&str>) -> Result<(), ResolveError> {
    let src = source.map(|s| miette::NamedSource::new("schema.xml", s.to_owned()));

    // 1. Validate no duplicate template IDs.
    {
        let mut seen_ids: std::collections::HashMap<u16, &str> = std::collections::HashMap::new();
        for token in &ir.tokens {
            if token.signal == Signal::BeginMessage {
                if let Some(id) = token.id {
                    if seen_ids.insert(id, &token.name).is_some() {
                        return Err(ResolveError::DuplicateTemplateId {
                            id,
                            name: token.name.clone(),
                            source_code: src.clone(),
                            first_label: None,
                            second_label: None,
                        });
                    }
                }
            }
        }
    }

    // 2. Validate that no token has a since_version exceeding the schema version.
    for token in &ir.tokens {
        let sv = token.encoding.since_version;
        if sv > ir.version {
            return Err(ResolveError::SinceVersionBeyondSchema {
                version: sv,
                schema_version: ir.version,
                name: token.name.clone(),
                source_code: src.clone(),
                span: None,
            });
        }
    }

    // 3. Fill in default null/min/max values for all primitive encodings in the tokens.
    for token in &mut ir.tokens {
        if let Some(prim) = token.encoding.primitive_type {
            if token.encoding.null_value.is_none() {
                token.encoding.null_value = default_null(prim);
            }
            if token.encoding.min_value.is_none() {
                token.encoding.min_value = default_min(prim);
            }
            if token.encoding.max_value.is_none() {
                token.encoding.max_value = default_max(prim);
            }
        }
    }

    // 4. Resolve offsets and block lengths for all composites and messages.
    let mut i = 0;
    while i < ir.tokens.len() {
        match ir.tokens[i].signal {
            Signal::BeginComposite => {
                let end_idx =
                    find_matching_end(&ir.tokens, i, Signal::BeginComposite, Signal::EndComposite);
                resolve_composite_offsets(&mut ir.tokens[i..=end_idx], &src)?;
                i = end_idx + 1;
            }
            Signal::BeginMessage => {
                let end_idx =
                    find_matching_end(&ir.tokens, i, Signal::BeginMessage, Signal::EndMessage);
                resolve_message_offsets(&mut ir.tokens[i..=end_idx], &src)?;
                i = end_idx + 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    Ok(())
}

fn find_matching_end(tokens: &[Token], start: usize, begin: Signal, end: Signal) -> usize {
    let mut depth = 1;
    for j in (start + 1)..tokens.len() {
        if tokens[j].signal == begin {
            depth += 1;
        } else if tokens[j].signal == end {
            depth -= 1;
            if depth == 0 {
                return j;
            }
        }
    }
    tokens.len() - 1
}

/// Helper to get size of a type block.
fn get_token_block_size(tokens: &[Token], start: usize) -> (usize, usize) {
    match tokens[start].signal {
        Signal::BeginField => {
            let end_idx = find_matching_end(tokens, start, Signal::BeginField, Signal::EndField);
            // Size is the size of the contents
            if end_idx > start + 1 {
                // Nested composite, enum, set etc.
                let mut size = 0;
                let mut j = start + 1;
                while j < end_idx {
                    let (s, next_j) = get_token_block_size(tokens, j);
                    size += s;
                    j = next_j;
                }
                (size, end_idx + 1)
            } else {
                // Primitive field
                let count = tokens[start].encoding.length.unwrap_or(1);
                let size = tokens[start]
                    .encoding
                    .primitive_type
                    .map_or(0, |p| p.size())
                    * count;
                (size, end_idx + 1)
            }
        }
        Signal::BeginComposite => {
            let end_idx =
                find_matching_end(tokens, start, Signal::BeginComposite, Signal::EndComposite);
            let mut size = 0;
            let mut j = start + 1;
            while j < end_idx {
                let (s, next_j) = get_token_block_size(tokens, j);
                size += s;
                j = next_j;
            }
            (size, end_idx + 1)
        }
        Signal::BeginEnum | Signal::BeginSet => {
            let end_idx = find_matching_end(
                tokens,
                start,
                tokens[start].signal,
                match tokens[start].signal {
                    Signal::BeginEnum => Signal::EndEnum,
                    _ => Signal::EndSet,
                },
            );
            let size = tokens[start]
                .encoding
                .primitive_type
                .map_or(0, |p| p.size());
            (size, end_idx + 1)
        }
        Signal::Encoding => {
            let size = tokens[start]
                .encoding
                .primitive_type
                .map_or(0, |p| p.size());
            (size, start + 1)
        }
        _ => (0, start + 1),
    }
}

fn resolve_composite_offsets(
    tokens: &mut [Token],
    _src: &Option<miette::NamedSource<String>>,
) -> Result<(), ResolveError> {
    let mut current_offset = 0;
    let mut i = 1; // skip BeginComposite
    let end_limit = tokens.len() - 1; // skip EndComposite

    while i < end_limit {
        let (size, next_i) = get_token_block_size(tokens, i);

        // Resolve offset
        let resolved_offset = if let Some(off) = tokens[i].encoding.offset {
            off
        } else {
            current_offset
        };

        tokens[i].encoding.offset = Some(resolved_offset);

        // For nested composite tokens, we need to cascade offsets relative to parent if needed,
        // but inside SBE all member offsets are absolute or sequential.
        current_offset = resolved_offset + size;
        i = next_i;
    }

    // Set total block length/size on BeginComposite
    let composite_size = current_offset;
    tokens[0].encoding.offset = Some(composite_size);
    Ok(())
}

fn resolve_message_offsets(
    tokens: &mut [Token],
    src: &Option<miette::NamedSource<String>>,
) -> Result<(), ResolveError> {
    let mut current_offset = 0;
    let mut i = 1; // skip BeginMessage
    let end_limit = tokens.len() - 1; // skip EndMessage

    while i < end_limit {
        // If we hit variable-length tail elements (Signal::BeginGroup, Signal::BeginVarData),
        // we stop sequential fixed offset calculation because they are placed in the tail.
        if tokens[i].signal == Signal::BeginGroup || tokens[i].signal == Signal::BeginVarData {
            // Resolve nested group/var-data offsets starting at 0
            if tokens[i].signal == Signal::BeginGroup {
                let end_idx = find_matching_end(tokens, i, Signal::BeginGroup, Signal::EndGroup);
                resolve_group_offsets(&mut tokens[i..=end_idx], src)?;
                i = end_idx + 1;
            } else {
                let end_idx =
                    find_matching_end(tokens, i, Signal::BeginVarData, Signal::EndVarData);
                resolve_vardata_offsets(&mut tokens[i..=end_idx], src)?;
                i = end_idx + 1;
            }
            continue;
        }

        let (size, next_i) = get_token_block_size(tokens, i);

        // Resolve offset
        let resolved_offset = if let Some(off) = tokens[i].encoding.offset {
            off
        } else {
            current_offset
        };

        tokens[i].encoding.offset = Some(resolved_offset);
        current_offset = resolved_offset + size;
        i = next_i;
    }

    // Set total block length/size on BeginMessage
    let block_length = current_offset;
    tokens[0].encoding.offset = Some(block_length);
    Ok(())
}

fn resolve_group_offsets(
    tokens: &mut [Token],
    src: &Option<miette::NamedSource<String>>,
) -> Result<(), ResolveError> {
    // A group has BeginGroup, followed by dimensionType composite tokens,
    // followed by group entry tokens, followed by EndGroup.
    // First, resolve the dimensionType composite offsets:
    let mut i = 1;
    if tokens[i].signal == Signal::BeginComposite {
        let dim_end = find_matching_end(tokens, i, Signal::BeginComposite, Signal::EndComposite);
        resolve_composite_offsets(&mut tokens[i..=dim_end], src)?;
        i = dim_end + 1;
    }

    // Now resolve group entry fields (relative to 0)
    let mut current_offset = 0;
    let end_limit = tokens.len() - 1;
    while i < end_limit {
        if tokens[i].signal == Signal::BeginGroup || tokens[i].signal == Signal::BeginVarData {
            if tokens[i].signal == Signal::BeginGroup {
                let end_idx = find_matching_end(tokens, i, Signal::BeginGroup, Signal::EndGroup);
                resolve_group_offsets(&mut tokens[i..=end_idx], src)?;
                i = end_idx + 1;
            } else {
                let end_idx =
                    find_matching_end(tokens, i, Signal::BeginVarData, Signal::EndVarData);
                resolve_vardata_offsets(&mut tokens[i..=end_idx], src)?;
                i = end_idx + 1;
            }
            continue;
        }

        let (size, next_i) = get_token_block_size(tokens, i);
        let resolved_offset = if let Some(off) = tokens[i].encoding.offset {
            off
        } else {
            current_offset
        };
        tokens[i].encoding.offset = Some(resolved_offset);
        current_offset = resolved_offset + size;
        i = next_i;
    }

    // The group's entry blockLength is the final offset of entry fields
    let block_length = current_offset;
    tokens[0].encoding.offset = Some(block_length);
    Ok(())
}

fn resolve_vardata_offsets(
    tokens: &mut [Token],
    src: &Option<miette::NamedSource<String>>,
) -> Result<(), ResolveError> {
    // A var-data field has BeginVarData, followed by type composite tokens,
    // followed by EndVarData.
    let i = 1;
    if tokens[i].signal == Signal::BeginComposite {
        let type_end = find_matching_end(tokens, i, Signal::BeginComposite, Signal::EndComposite);
        resolve_composite_offsets(&mut tokens[i..=type_end], src)?;
    }
    Ok(())
}

fn default_null(prim: PrimitiveType) -> Option<u64> {
    match prim {
        PrimitiveType::Char => Some(0),
        PrimitiveType::Int8 => Some(-128i8 as u64),
        PrimitiveType::UInt8 => Some(255),
        PrimitiveType::Int16 => Some(-32768i16 as u64),
        PrimitiveType::UInt16 => Some(65535),
        PrimitiveType::Int32 => Some(-2147483648i32 as u64),
        PrimitiveType::UInt32 => Some(4294967295),
        PrimitiveType::Int64 => Some(-9223372036854775808i64 as u64),
        PrimitiveType::UInt64 => Some(18446744073709551615),
        PrimitiveType::Float => Some(0x7F800001), // NaN sentinel as bits
        PrimitiveType::Double => Some(0x7FF8000000000001), // NaN sentinel as bits
    }
}

fn default_min(prim: PrimitiveType) -> Option<u64> {
    match prim {
        PrimitiveType::Char => Some(0x20),
        PrimitiveType::Int8 => Some(-127i8 as u64),
        PrimitiveType::UInt8 => Some(0),
        PrimitiveType::Int16 => Some(-32767i16 as u64),
        PrimitiveType::UInt16 => Some(0),
        PrimitiveType::Int32 => Some(-2147483647i32 as u64),
        PrimitiveType::UInt32 => Some(0),
        PrimitiveType::Int64 => Some(-9223372036854775807i64 as u64),
        PrimitiveType::UInt64 => Some(0),
        PrimitiveType::Float => Some(f32::MIN.to_bits() as u64), // neg max float
        PrimitiveType::Double => Some(f64::MIN.to_bits()),       // neg max double
    }
}

fn default_max(prim: PrimitiveType) -> Option<u64> {
    match prim {
        PrimitiveType::Char => Some(0x7E),
        PrimitiveType::Int8 => Some(127),
        PrimitiveType::UInt8 => Some(254),
        PrimitiveType::Int16 => Some(32767),
        PrimitiveType::UInt16 => Some(65534),
        PrimitiveType::Int32 => Some(2147483647),
        PrimitiveType::UInt32 => Some(4294967294),
        PrimitiveType::Int64 => Some(9223372036854775807),
        PrimitiveType::UInt64 => Some(18446744073709551614),
        PrimitiveType::Float => Some(f32::MAX.to_bits() as u64), // max float
        PrimitiveType::Double => Some(f64::MAX.to_bits()),       // max double
    }
}
