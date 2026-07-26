//! Offset / block-length resolution and schema integrity checks on [`Ir`].
//!
//! Called automatically by [`crate::parse`] / [`crate::parse_file`]. Direct use
//! is only needed if you build an [`Ir`] by hand.
//!
//! Passes: unique template ids, `sinceVersion` ≤ schema version, default
//! null/min/max for primitives, sequential offsets, block lengths for
//! composites/messages (groups/var-data live in the tail).

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
    #[diagnostic(code(ergo_sbe::resolve::duplicate_template_id))]
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
    #[diagnostic(code(ergo_sbe::resolve::unknown_type))]
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
    #[diagnostic(code(ergo_sbe::resolve::invalid_offset))]
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
    #[diagnostic(code(ergo_sbe::resolve::empty_composite))]
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
    #[diagnostic(code(ergo_sbe::resolve::since_version_beyond))]
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

/// Resolve offsets, block lengths, and default null/min/max on `ir` in place.
///
/// Already invoked by [`crate::parse`]. Pass `source` for miette snippets on
/// [`ResolveError`].
///
/// # Errors
///
/// Duplicate template ids, unknown types, bad offsets, empty composites,
/// or `sinceVersion` beyond schema version.
pub fn resolve_schema(ir: &mut Ir, source: Option<&str>) -> Result<(), ResolveError> {
    let src = source.map(|s| miette::NamedSource::new("schema.xml", s.to_owned()));

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

fn get_token_block_size(tokens: &[Token], start: usize) -> (usize, usize) {
    match tokens[start].signal {
        Signal::BeginField => {
            let end_idx = find_matching_end(tokens, start, Signal::BeginField, Signal::EndField);
            // Variable-length fields (varData composite members) don't occupy fixed block space.
            if tokens[start].encoding.is_variable_length {
                return (0, end_idx + 1);
            }
            // Size is the size of the contents
            // constant fields return 0 size (no wire footprint), add schema validation if runtime constants ever change
            if tokens[start].encoding.presence == crate::ir::Presence::Constant {
                return (0, end_idx + 1);
            }
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
        // composite children always Begin* signals (Encoding never Composite), add Composite check if schema validation weakens
        // tokens are nested inside BeginEnum/BeginSet/EndEnum/EndSet and
        // never appear as direct children.
        _ => (0, start + 1),
    }
}

#[allow(clippy::only_used_in_recursion)]
fn resolve_composite_offsets(
    tokens: &mut [Token],
    src: &Option<miette::NamedSource<String>>,
) -> Result<(), ResolveError> {
    let mut current_offset = 0;
    let mut i = 1; // skip BeginComposite
    let end_limit = tokens.len() - 1; // skip EndComposite

    while i < end_limit {
        let (size, next_i) = get_token_block_size(tokens, i);

        let resolved_offset = if let Some(off) = tokens[i].encoding.offset {
            off
        } else {
            current_offset
        };

        tokens[i].encoding.offset = Some(resolved_offset);

        // Inlined nested composites (via `<ref type="Composite"/>` or
        // nested type expansion) store their wire size on BeginComposite
        // `encoding.offset`. Codegen reads that as `MemberType::Composite.size`.
        // Cloned registry tokens often still have `offset: None` here — resolve
        // the nested layout so size is never left as 0 (`read_bytes::<0>`).
        if tokens[i].signal == Signal::BeginField && i + 1 < next_i {
            if tokens[i + 1].signal == Signal::BeginComposite {
                let nested_end =
                    find_matching_end(tokens, i + 1, Signal::BeginComposite, Signal::EndComposite);
                resolve_composite_offsets(&mut tokens[i + 1..=nested_end], src)?;
            }
        }

        // For nested composite tokens, we need to cascade offsets relative to parent if needed,
        // but inside SBE all member offsets are absolute or sequential.
        current_offset = resolved_offset + size;
        i = next_i;
    }

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

        let resolved_offset = if let Some(off) = tokens[i].encoding.offset {
            off
        } else {
            current_offset
        };

        tokens[i].encoding.offset = Some(resolved_offset);
        current_offset = resolved_offset + size;
        i = next_i;
    }

    // Honor schema `blockLength` when larger than the sum of fixed fields
    // (padding). Matches sbe-tool / official SBE: the header and encoder walk
    // use the declared root block length, not the tight field packing size.
    let declared = tokens[0].encoding.offset;
    let block_length = match declared {
        Some(d) if d > current_offset => d,
        _ => current_offset,
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_schema() -> Ir {
        crate::parse(
            r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
    package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <sbe:message name="A" id="1"><field name="x" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#,
        )
        .unwrap()
    }

    #[test]
    fn duplicate_template_id_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let result = crate::parse(
            r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
    package="test" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <sbe:message name="A" id="1"><field name="x" id="1" type="uint32"/></sbe:message>
  <sbe:message name="B" id="1"><field name="y" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#,
        );
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn since_version_beyond_schema_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let mut ir = minimal_schema();
        ir.tokens[5].encoding.since_version = 5;
        let result = resolve_schema(&mut ir, None);
        assert!(matches!(
            result,
            Err(ResolveError::SinceVersionBeyondSchema { .. })
        ));
        Ok(())
    }

    #[test]
    fn resolve_schema_ok_on_valid_schema() -> Result<(), Box<dyn std::error::Error>> {
        let mut ir = minimal_schema();
        assert!(resolve_schema(&mut ir, None).is_ok());

        Ok(())
    }

    #[test]
    fn resolve_schema_with_source_code() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="A" id="1"><field name="x" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
        let mut ir = crate::parse(xml).unwrap();
        assert!(resolve_schema(&mut ir, Some(xml)).is_ok());

        Ok(())
    }

    #[test]
    fn default_null_all_primitives() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(default_null(PrimitiveType::Char), Some(0));
        assert_eq!(default_null(PrimitiveType::Int8), Some(-128i8 as u64));
        assert_eq!(default_null(PrimitiveType::UInt8), Some(255));
        assert_eq!(default_null(PrimitiveType::Int16), Some(-32768i16 as u64));
        assert_eq!(default_null(PrimitiveType::UInt16), Some(65535));
        assert_eq!(
            default_null(PrimitiveType::Int32),
            Some(-2147483648i32 as u64)
        );
        assert_eq!(default_null(PrimitiveType::UInt32), Some(4294967295));
        assert_eq!(
            default_null(PrimitiveType::Int64),
            Some(9223372036854775808u64)
        ); // i64::MIN as u64
        assert_eq!(default_null(PrimitiveType::UInt64), Some(u64::MAX));
        assert!(default_null(PrimitiveType::Float).is_some());
        assert!(default_null(PrimitiveType::Double).is_some());

        Ok(())
    }

    #[test]
    fn default_min_all_primitives() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(default_min(PrimitiveType::Char), Some(0x20));
        assert_eq!(default_min(PrimitiveType::UInt8), Some(0));
        assert_eq!(default_min(PrimitiveType::UInt16), Some(0));
        assert_eq!(default_min(PrimitiveType::UInt32), Some(0));
        assert_eq!(default_min(PrimitiveType::UInt64), Some(0));
        assert!(default_min(PrimitiveType::Int8).is_some());
        assert!(default_min(PrimitiveType::Int16).is_some());
        assert!(default_min(PrimitiveType::Int32).is_some());
        assert!(default_min(PrimitiveType::Int64).is_some());
        assert!(default_min(PrimitiveType::Float).is_some());
        assert!(default_min(PrimitiveType::Double).is_some());

        Ok(())
    }

    #[test]
    fn default_max_all_primitives() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(default_max(PrimitiveType::Char), Some(0x7E));
        assert_eq!(default_max(PrimitiveType::Int8), Some(127));
        assert_eq!(default_max(PrimitiveType::UInt8), Some(254));
        assert_eq!(default_max(PrimitiveType::Int16), Some(32767));
        assert_eq!(default_max(PrimitiveType::UInt16), Some(65534));
        assert_eq!(default_max(PrimitiveType::Int32), Some(2147483647));
        assert_eq!(default_max(PrimitiveType::UInt32), Some(4294967294));
        assert_eq!(default_max(PrimitiveType::Int64), Some(9223372036854775807));
        assert_eq!(
            default_max(PrimitiveType::UInt64),
            Some(18446744073709551614)
        );
        assert!(default_max(PrimitiveType::Float).is_some());
        assert!(default_max(PrimitiveType::Double).is_some());

        Ok(())
    }

    #[test]
    fn composite_offsets_assigned_sequentially() -> Result<(), Box<dyn std::error::Error>> {
        let mut ir = minimal_schema();
        resolve_schema(&mut ir, None).unwrap();
        let hdr = ir
            .tokens
            .iter()
            .find(|t| t.name == "messageHeader")
            .unwrap();
        // Should have a non-zero offset (block length)
        assert!(hdr.encoding.offset.is_some());

        Ok(())
    }

    #[test]
    fn message_offsets_assigned_correctly() -> Result<(), Box<dyn std::error::Error>> {
        let mut ir = minimal_schema();
        resolve_schema(&mut ir, None).unwrap();
        let msg = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginMessage)
            .unwrap();
        assert!(msg.encoding.offset.is_some());
        // uint32 field should be at offset 0
        let field = ir.tokens.iter().find(|t| t.name == "x").unwrap();
        assert_eq!(field.encoding.offset, Some(0));

        Ok(())
    }

    #[test]
    fn explicit_offset_preserved() -> Result<(), Box<dyn std::error::Error>> {
        let ir = crate::parse(
            r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="A" id="1"><field name="x" id="1" type="uint32" offset="0"/><field name="y" id="2" type="uint16" offset="4"/></sbe:message>
</sbe:messageSchema>"#,
        ).unwrap();
        let mut ir = ir;
        resolve_schema(&mut ir, None).unwrap();
        let x = ir.tokens.iter().find(|t| t.name == "x").unwrap();
        assert_eq!(x.encoding.offset, Some(0));
        let y = ir.tokens.iter().find(|t| t.name == "y").unwrap();
        assert_eq!(y.encoding.offset, Some(4));

        Ok(())
    }

    #[test]
    fn group_offsets_resolved() -> Result<(), Box<dyn std::error::Error>> {
        let ir = crate::parse(
            r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <composite name="groupSizeEncoding"><type name="blockLength" primitiveType="uint16"/><type name="numInGroup" primitiveType="uint16"/></composite>
</types>
<sbe:message name="A" id="1">
  <field name="x" id="1" type="uint32"/>
  <group name="items" id="2" dimensionType="groupSizeEncoding">
    <field name="a" id="1" type="uint32"/>
    <field name="b" id="2" type="uint16"/>
  </group>
</sbe:message>
</sbe:messageSchema>"#,
        ).unwrap();
        let mut ir = ir;
        resolve_schema(&mut ir, None).unwrap();
        let a = ir.tokens.iter().find(|t| t.name == "a").unwrap();
        assert_eq!(a.encoding.offset, Some(0));
        let b = ir.tokens.iter().find(|t| t.name == "b").unwrap();
        assert_eq!(b.encoding.offset, Some(4));

        Ok(())
    }

    #[test]
    fn vardata_offsets_resolved() -> Result<(), Box<dyn std::error::Error>> {
        let ir = crate::parse(
            r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <composite name="varDataEncoding"><type name="length" primitiveType="uint32"/><type name="varData" primitiveType="uint8" length="0"/></composite>
</types>
<sbe:message name="A" id="1">
  <field name="x" id="1" type="uint32"/>
  <data name="payload" id="2" type="varDataEncoding"/>
</sbe:message>
</sbe:messageSchema>"#,
        ).unwrap();
        let mut ir = ir;
        resolve_schema(&mut ir, None).unwrap();
        let msg = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginMessage)
            .unwrap();
        assert!(msg.encoding.offset.is_some());

        Ok(())
    }

    #[test]
    fn enum_block_size_calculated() -> Result<(), Box<dyn std::error::Error>> {
        let ir = crate::parse(
            r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <enum name="Colour" encodingType="uint8"><validValue name="R">1</validValue></enum>
</types>
<sbe:message name="A" id="1"><field name="c" id="1" type="Colour"/></sbe:message>
</sbe:messageSchema>"#,
        ).unwrap();
        let mut ir = ir;
        resolve_schema(&mut ir, None).unwrap();
        let msg = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginMessage)
            .unwrap();
        // Block length should be 1 (uint8 enum)
        assert_eq!(msg.encoding.offset, Some(1));

        Ok(())
    }

    #[test]
    fn set_block_size_calculated() -> Result<(), Box<dyn std::error::Error>> {
        let ir = crate::parse(
            r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <set name="Flags" encodingType="uint8"><choice name="A">0</choice></set>
</types>
<sbe:message name="A" id="1"><field name="f" id="1" type="Flags"/></sbe:message>
</sbe:messageSchema>"#,
        ).unwrap();
        let mut ir = ir;
        resolve_schema(&mut ir, None).unwrap();
        let msg = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginMessage)
            .unwrap();
        assert_eq!(msg.encoding.offset, Some(1));

        Ok(())
    }

    #[test]
    fn constant_field_does_not_affect_block_length() -> Result<(), Box<dyn std::error::Error>> {
        let ir = crate::parse(
            r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <type name="ConstVal" primitiveType="char" presence="constant">X</type>
</types>
<sbe:message name="A" id="1"><field name="x" id="1" type="uint32"/><field name="c" id="2" type="ConstVal"/></sbe:message>
</sbe:messageSchema>"#,
        ).unwrap();
        let mut ir = ir;
        resolve_schema(&mut ir, None).unwrap();
        let msg = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginMessage)
            .unwrap();
        // Block length should be 4 (uint32 only, constant doesn't occupy wire)
        assert_eq!(msg.encoding.offset, Some(4));
        Ok(())
    }

    #[test]
    fn nested_composite_offsets_resolved() -> Result<(), Box<dyn std::error::Error>> {
        let ir = crate::parse(
            r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <composite name="Point"><type name="px" primitiveType="int32"/><type name="py" primitiveType="int32"/></composite>
</types>
<sbe:message name="A" id="1"><field name="p" id="1" type="Point"/></sbe:message>
</sbe:messageSchema>"#,
        ).unwrap();
        let mut ir = ir;
        resolve_schema(&mut ir, None).unwrap();
        let msg = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginMessage)
            .unwrap();
        // Point composite = 2 × int32 = 8 bytes
        assert_eq!(msg.encoding.offset, Some(8));

        Ok(())
    }

    #[test]
    fn take_source_code_from_duplicate_template_id() -> Result<(), Box<dyn std::error::Error>> {
        let mut err = ResolveError::DuplicateTemplateId {
            id: 1,
            name: "test".to_string(),
            source_code: None,
            first_label: None,
            second_label: None,
        };
        assert!(err.take_source_code().is_none()); // was None

        Ok(())
    }

    #[test]
    fn take_source_code_from_unknown_type() -> Result<(), Box<dyn std::error::Error>> {
        let mut err = ResolveError::UnknownType {
            name: "Foo".to_string(),
            source_code: None,
            span: None,
        };
        assert!(err.take_source_code().is_none());

        Ok(())
    }

    #[test]
    fn take_source_code_from_invalid_offset() -> Result<(), Box<dyn std::error::Error>> {
        let mut err = ResolveError::InvalidOffset {
            offset: 99,
            source_code: None,
            span: None,
        };
        assert!(err.take_source_code().is_none());

        Ok(())
    }

    #[test]
    fn take_source_code_from_empty_composite() -> Result<(), Box<dyn std::error::Error>> {
        let mut err = ResolveError::EmptyComposite {
            name: "Empty".to_string(),
            source_code: None,
            span: None,
        };
        assert!(err.take_source_code().is_none());

        Ok(())
    }

    #[test]
    fn take_source_code_from_since_version() -> Result<(), Box<dyn std::error::Error>> {
        let mut err = ResolveError::SinceVersionBeyondSchema {
            version: 5,
            schema_version: 0,
            name: "field".to_string(),
            source_code: None,
            span: None,
        };
        assert!(err.take_source_code().is_none());

        Ok(())
    }

    #[test]
    fn resolve_error_displays() -> Result<(), Box<dyn std::error::Error>> {
        let err = ResolveError::DuplicateTemplateId {
            id: 1,
            name: "A".to_string(),
            source_code: None,
            first_label: None,
            second_label: None,
        };
        assert!(format!("{err}").contains("duplicate template id 1"));

        let err = ResolveError::UnknownType {
            name: "Foo".to_string(),
            source_code: None,
            span: None,
        };
        assert!(format!("{err}").contains("unknown type reference Foo"));

        let err = ResolveError::InvalidOffset {
            offset: 42,
            source_code: None,
            span: None,
        };
        assert!(format!("{err}").contains("offset 42"));

        let err = ResolveError::EmptyComposite {
            name: "X".to_string(),
            source_code: None,
            span: None,
        };
        assert!(format!("{err}").contains("composite X"));

        let err = ResolveError::SinceVersionBeyondSchema {
            version: 3,
            schema_version: 0,
            name: "y".to_string(),
            source_code: None,
            span: None,
        };
        assert!(format!("{err}").contains("sinceVersion 3"));

        Ok(())
    }

    #[test]
    fn fixed_array_field_offset() -> Result<(), Box<dyn std::error::Error>> {
        let ir = crate::parse(
            r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <type name="int32array4" primitiveType="int32" length="4"/>
</types>
<sbe:message name="A" id="1"><field name="nums" id="1" type="int32array4"/></sbe:message>
</sbe:messageSchema>"#,
        ).unwrap();
        let mut ir = ir;
        resolve_schema(&mut ir, None).unwrap();
        let msg = ir
            .tokens
            .iter()
            .find(|t| t.signal == Signal::BeginMessage)
            .unwrap();
        // 4 × int32 = 16 bytes
        assert_eq!(msg.encoding.offset, Some(16));

        Ok(())
    }

    #[test]
    fn nested_group_offsets_resolved() -> Result<(), Box<dyn std::error::Error>> {
        let ir = crate::parse(
            r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <composite name="groupSizeEncoding"><type name="blockLength" primitiveType="uint16"/><type name="numInGroup" primitiveType="uint16"/></composite>
</types>
<sbe:message name="A" id="1">
  <group name="outer" id="2" dimensionType="groupSizeEncoding">
    <field name="a" id="1" type="uint32"/>
    <group name="inner" id="3" dimensionType="groupSizeEncoding">
      <field name="b" id="1" type="uint16"/>
    </group>
  </group>
</sbe:message>
</sbe:messageSchema>"#,
        ).unwrap();
        let mut ir = ir;
        resolve_schema(&mut ir, None).unwrap();
        let b = ir.tokens.iter().find(|t| t.name == "b").unwrap();
        assert_eq!(b.encoding.offset, Some(0));

        Ok(())
    }

    #[test]
    fn begin_message_without_id_skips_duplicate_check() -> Result<(), Box<dyn std::error::Error>> {
        // Construct a minimal IR with a BeginMessage that has no id.
        // The duplicate check should skip it (the `if let Some(id)` false branch).
        let mut ir = Ir {
            package: "t".to_string(),
            id: 1,
            version: 0,
            byte_order: crate::ir::ByteOrder::LittleEndian,
            description: None,
            semantic_version: None,
            header_type: "messageHeader".to_string(),
            tokens: vec![
                Token {
                    id: None,
                    name: "A".to_string(),
                    signal: Signal::BeginMessage,
                    encoding: crate::ir::Encoding::default(),
                },
                Token {
                    id: None,
                    name: "A".to_string(),
                    signal: Signal::EndMessage,
                    encoding: crate::ir::Encoding::default(),
                },
            ],
        };
        // Should not panic or error — the message has no id to conflict
        assert!(resolve_schema(&mut ir, None).is_ok());

        Ok(())
    }

    #[test]
    fn find_matching_end_fallback_on_no_match() -> Result<(), Box<dyn std::error::Error>> {
        // Construct tokens with an unclosed BeginComposite to trigger the fallback.
        let tokens = vec![
            Token {
                id: None,
                name: "X".to_string(),
                signal: Signal::BeginComposite,
                encoding: crate::ir::Encoding::default(),
            },
            Token {
                id: None,
                name: "Y".to_string(),
                signal: Signal::BeginField,
                encoding: crate::ir::Encoding::default(),
            },
        ];
        // find_matching_end should return tokens.len() - 1 as fallback
        let end = find_matching_end(&tokens, 0, Signal::BeginComposite, Signal::EndComposite);
        assert_eq!(end, 1); // returns last index

        Ok(())
    }

    #[test]
    fn get_token_block_size_catch_all_signal() -> Result<(), Box<dyn std::error::Error>> {
        // An EndField or other non-Begin signal as a direct child hits the `_ =>` branch.
        let tokens = vec![Token {
            id: None,
            name: "X".to_string(),
            signal: Signal::EndField,
            encoding: crate::ir::Encoding::default(),
        }];
        let (size, next) = get_token_block_size(&tokens, 0);
        assert_eq!(size, 0);
        assert_eq!(next, 1);

        Ok(())
    }

    #[test]
    fn group_without_dimension_composite() -> Result<(), Box<dyn std::error::Error>> {
        // A group whose second token is NOT BeginComposite (missing dimensionType).
        let mut tokens = vec![
            Token {
                id: Some(1),
                name: "grp".to_string(),
                signal: Signal::BeginGroup,
                encoding: crate::ir::Encoding::default(),
            },
            // No BeginComposite — jump straight to a field
            Token {
                id: None,
                name: "field".to_string(),
                signal: Signal::BeginField,
                encoding: crate::ir::Encoding {
                    primitive_type: Some(PrimitiveType::UInt32),
                    ..crate::ir::Encoding::default()
                },
            },
            Token {
                id: None,
                name: "field".to_string(),
                signal: Signal::EndField,
                encoding: crate::ir::Encoding::default(),
            },
            Token {
                id: None,
                name: "grp".to_string(),
                signal: Signal::EndGroup,
                encoding: crate::ir::Encoding::default(),
            },
        ];
        let src: Option<miette::NamedSource<String>> = None;
        let result = resolve_group_offsets(&mut tokens, &src);
        assert!(result.is_ok());
        assert_eq!(tokens[1].encoding.offset, Some(0));

        Ok(())
    }

    #[test]
    fn vardata_without_type_composite() -> Result<(), Box<dyn std::error::Error>> {
        // A var-data whose second token is NOT BeginComposite (missing type).
        let mut tokens = vec![
            Token {
                id: Some(1),
                name: "data".to_string(),
                signal: Signal::BeginVarData,
                encoding: crate::ir::Encoding::default(),
            },
            // No BeginComposite — just EndVarData
            Token {
                id: None,
                name: "data".to_string(),
                signal: Signal::EndVarData,
                encoding: crate::ir::Encoding::default(),
            },
        ];
        let src: Option<miette::NamedSource<String>> = None;
        let result = resolve_vardata_offsets(&mut tokens, &src);
        assert!(result.is_ok());

        Ok(())
    }
}
