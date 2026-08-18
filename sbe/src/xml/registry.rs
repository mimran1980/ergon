//! Type registry and named-type → token expansion.

use std::collections::HashMap;

use roxmltree::Node;

use crate::ir::{Encoding, Ir, Presence, PrimitiveType, Signal, Token};

use super::error::Fault;
use super::warn::{WarnState, warn_once};

pub(crate) struct TypeRegistry {
    pub(crate) registry: HashMap<String, Vec<Token>>,
    pub(crate) encodings: HashMap<String, Encoding>,
}

impl TypeRegistry {
    pub(crate) fn new() -> Self {
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

    /// Rebuild a registry from an already-parsed schema's flat token stream,
    /// so a second schema can resolve its composite/enum/set types without
    /// re-declaring or re-`<include>`-ing them. Bare top-level `<type>`
    /// typedefs don't round-trip through `Ir` (they're inlined and dropped
    /// during parsing) — only composites, enums, and sets are recovered here.
    pub(crate) fn from_parsed_schema(ir: &Ir) -> Self {
        let mut registry = Self::new();
        let mut i = 0;
        while i < ir.tokens.len() {
            let (begin, end) = match ir.tokens[i].signal {
                Signal::BeginComposite => (Signal::BeginComposite, Signal::EndComposite),
                Signal::BeginEnum => (Signal::BeginEnum, Signal::EndEnum),
                Signal::BeginSet => (Signal::BeginSet, Signal::EndSet),
                _ => {
                    i += 1;
                    continue;
                }
            };
            let end_idx = crate::codegen::find_matching_end(&ir.tokens, i, begin, end);
            registry
                .registry
                .insert(ir.tokens[i].name.clone(), ir.tokens[i..=end_idx].to_vec());
            i = end_idx + 1;
        }
        registry
    }
}

/// Helper to parse optional u64 values from strings (like nullValue).
/// Returns `None` for empty strings; `Some` for valid values.
/// Prefer [`try_parse_u64_val`] when an attribute is present and must fail closed.
pub(crate) fn parse_u64_val(s: &str, prim_type: Option<PrimitiveType>) -> Option<u64> {
    try_parse_u64_val(s, prim_type).ok().flatten()
}

/// Fallible null/min/max parser. Empty → `Ok(None)`. Present-but-invalid → `Err`.
pub(crate) fn try_parse_u64_val(
    s: &str,
    prim_type: Option<PrimitiveType>,
) -> Result<Option<u64>, String> {
    if s.is_empty() {
        return Ok(None);
    }
    match prim_type {
        Some(PrimitiveType::Char) if s.len() == 1 => {
            return Ok(Some(s.chars().next().unwrap() as u64));
        }
        Some(PrimitiveType::Float) | Some(PrimitiveType::Double) => {
            // Parse as float/double, then reinterpret bits as u64.
            // This preserves NaN, infinity, and negative zero bit patterns.
            if let Some(PrimitiveType::Float) = prim_type {
                if let Ok(v) = s.parse::<f32>() {
                    return Ok(Some(v.to_bits() as u64));
                }
            } else if let Ok(v) = s.parse::<f64>() {
                return Ok(Some(v.to_bits() as u64));
            }
            return Err(format!("'{s}' is not a valid float null/min/max value"));
        }
        Some(prim) if prim.is_unsigned_int() => {
            let v = s
                .parse::<u64>()
                .map_err(|_| format!("'{s}' is not a valid integer null/min/max value"))?;
            if let Some(max) = prim.unsigned_max()
                && v > max
            {
                return Err(format!("'{s}' is out of range for {prim:?}"));
            }
            return Ok(Some(v));
        }
        Some(prim) if prim.is_signed_int() => {
            let v = s
                .parse::<i64>()
                .map_err(|_| format!("'{s}' is not a valid integer null/min/max value"))?;
            if let Some((min, max)) = prim.signed_range()
                && !(min..=max).contains(&v)
            {
                return Err(format!("'{s}' is out of range for {prim:?}"));
            }
            return Ok(Some(v as u64));
        }
        _ => {}
    }
    if let Ok(v) = s.parse::<u64>() {
        Ok(Some(v))
    } else if let Ok(v) = s.parse::<i64>() {
        Ok(Some(v as u64))
    } else {
        Err(format!("'{s}' is not a valid integer null/min/max value"))
    }
}

/// Compare a parsed wire-bit pattern against declared min/max.
///
/// Signed values are stored as `v as u64` two's-complement bits. Comparing
/// those bits as unsigned rejects legal negatives (`int8` `-1` vs `maxValue=5`).
pub(crate) fn value_in_declared_range(
    prim: PrimitiveType,
    value_bits: u64,
    min_bits: Option<u64>,
    max_bits: Option<u64>,
) -> Result<(), String> {
    if prim.is_signed_int() {
        let v = value_bits as i64;
        if let Some(min) = min_bits {
            let min = min as i64;
            if v < min {
                return Err(format!("{v}: below encodingType minValue {min}"));
            }
        }
        if let Some(max) = max_bits {
            let max = max as i64;
            if v > max {
                return Err(format!("{v}: above encodingType maxValue {max}"));
            }
        }
        return Ok(());
    }
    if let Some(min) = min_bits
        && value_bits < min
    {
        return Err(format!("{value_bits}: below encodingType minValue {min}"));
    }
    if let Some(max) = max_bits
        && value_bits > max
    {
        return Err(format!("{value_bits}: above encodingType maxValue {max}"));
    }
    Ok(())
}

/// Resolve a type reference to a list of tokens.
pub(crate) fn resolve_type_to_tokens(
    field_name: &str,
    type_name: &str,
    id: Option<u16>,
    registry: &TypeRegistry,
    since_version: u16,
    span: Option<std::ops::Range<usize>>,
    description: Option<String>,
) -> Option<Vec<Token>> {
    if let Some(encoding) = registry.encodings.get(type_name) {
        let mut field_enc = encoding.clone();
        if since_version > 0 {
            field_enc.since_version = since_version;
        }
        if description.is_some() {
            field_enc.description = description;
        }
        Some(vec![
            Token {
                id,
                name: field_name.to_string(),
                signal: Signal::BeginField,
                encoding: field_enc,
                span: span.clone(),
            },
            Token {
                id: None,
                name: field_name.to_string(),
                signal: Signal::EndField,
                encoding: Encoding::default(),
                span,
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
                description,
                ..Encoding::default()
            },
            span: span.clone(),
        });
        for t in tokens {
            inlined.push(t.clone());
        }
        inlined.push(Token {
            id: None,
            name: field_name.to_string(),
            signal: Signal::EndField,
            encoding: Encoding::default(),
            span,
        });
        Some(inlined)
    } else {
        None
    }
}

/// Best-effort wire size for a composite member (for offset-overlap checks).
pub(crate) fn estimate_composite_member_size(
    node: Node<'_, '_>,
    registry: &TypeRegistry,
) -> Option<usize> {
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

pub(crate) fn compute_type_size(type_name: &str, registry: &TypeRegistry) -> Option<usize> {
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
