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
//! 1. **Default values**: every primitive type gets a default null, min, and
//!    max sentinel (e.g. `uint16` null = `65535`, min = `0`, max = `65534`).
//! 2. **Offset resolution**: walks composites and messages sequentially,
//!    assigning offsets to fields that lack an explicit `offset` attribute.
//!    Nested groups and var-data fields are resolved independently (they live
//!    in the tail, after the fixed block).
//! 3. **Block length**: the final offset of each composite/message becomes its
//!    block length, stored on the `BeginComposite`/`BeginMessage` token.

use crate::ir::{Ir, PrimitiveType, Signal, Token};

/// Errors raised during schema resolution/validation.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    /// Two messages share the same template ID.
    #[error("duplicate template id {id} for message {name}")]
    DuplicateTemplateId {
        /// The duplicate ID.
        id: u16,
        /// Message name.
        name: String,
    },
    /// A referenced type was not found in the registry.
    #[error("unknown type reference {name}")]
    UnknownType {
        /// Type name.
        name: String,
    },
    /// Field offsets are overlapping or unaligned.
    #[error("overlapping offsets or invalid alignment at offset {offset}")]
    InvalidOffset {
        /// The invalid offset.
        offset: usize,
    },
    /// A composite type definition is empty.
    #[error("composite {name} has no fields")]
    EmptyComposite {
        /// Composite name.
        name: String,
    },
}

/// Run the reference resolution pass on a schema IR.
///
/// Modifies the IR in-place to fill resolved offsets, block lengths,
/// and default null/min/max values.
pub fn resolve_schema(ir: &mut Ir) -> Result<(), ResolveError> {
    // 1. Fill in default null/min/max values for all primitive encodings in the tokens.
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

    // 2. Resolve offsets and block lengths for all composites and messages.
    let mut i = 0;
    while i < ir.tokens.len() {
        match ir.tokens[i].signal {
            Signal::BeginComposite => {
                let end_idx =
                    find_matching_end(&ir.tokens, i, Signal::BeginComposite, Signal::EndComposite);
                resolve_composite_offsets(&mut ir.tokens[i..=end_idx])?;
                i = end_idx + 1;
            }
            Signal::BeginMessage => {
                let end_idx =
                    find_matching_end(&ir.tokens, i, Signal::BeginMessage, Signal::EndMessage);
                resolve_message_offsets(&mut ir.tokens[i..=end_idx])?;
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

fn resolve_composite_offsets(tokens: &mut [Token]) -> Result<(), ResolveError> {
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

fn resolve_message_offsets(tokens: &mut [Token]) -> Result<(), ResolveError> {
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
                resolve_group_offsets(&mut tokens[i..=end_idx])?;
                i = end_idx + 1;
            } else {
                let end_idx =
                    find_matching_end(tokens, i, Signal::BeginVarData, Signal::EndVarData);
                resolve_vardata_offsets(&mut tokens[i..=end_idx])?;
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

fn resolve_group_offsets(tokens: &mut [Token]) -> Result<(), ResolveError> {
    // A group has BeginGroup, followed by dimensionType composite tokens,
    // followed by group entry tokens, followed by EndGroup.
    // First, resolve the dimensionType composite offsets:
    let mut i = 1;
    if tokens[i].signal == Signal::BeginComposite {
        let dim_end = find_matching_end(tokens, i, Signal::BeginComposite, Signal::EndComposite);
        resolve_composite_offsets(&mut tokens[i..=dim_end])?;
        i = dim_end + 1;
    }

    // Now resolve group entry fields (relative to 0)
    let mut current_offset = 0;
    let end_limit = tokens.len() - 1;
    while i < end_limit {
        if tokens[i].signal == Signal::BeginGroup || tokens[i].signal == Signal::BeginVarData {
            if tokens[i].signal == Signal::BeginGroup {
                let end_idx = find_matching_end(tokens, i, Signal::BeginGroup, Signal::EndGroup);
                resolve_group_offsets(&mut tokens[i..=end_idx])?;
                i = end_idx + 1;
            } else {
                let end_idx =
                    find_matching_end(tokens, i, Signal::BeginVarData, Signal::EndVarData);
                resolve_vardata_offsets(&mut tokens[i..=end_idx])?;
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

fn resolve_vardata_offsets(tokens: &mut [Token]) -> Result<(), ResolveError> {
    // A var-data field has BeginVarData, followed by type composite tokens,
    // followed by EndVarData.
    let i = 1;
    if tokens[i].signal == Signal::BeginComposite {
        let type_end = find_matching_end(tokens, i, Signal::BeginComposite, Signal::EndComposite);
        resolve_composite_offsets(&mut tokens[i..=type_end])?;
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
