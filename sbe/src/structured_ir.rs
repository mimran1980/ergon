//! Structured intermediate representation — walks the flat token IR into
//! typed message, group, field, and var-data structures consumed by the
//! code generator. Query helpers resolve dimension/var-data composites.
//!
//! The code generator reads these structures; it never walks raw tokens
//! directly.

use crate::codegen::{find_matching_end, to_pascal_case, to_snake_case};
use crate::ir::{ByteOrder, Presence, PrimitiveType, Signal, Token};

pub(crate) struct SchemaElements {
    pub(crate) composites: Vec<Vec<Token>>,
    pub(crate) enums: Vec<Vec<Token>>,
    pub(crate) sets: Vec<Vec<Token>>,
    pub(crate) messages: Vec<Vec<Token>>,
}

pub(crate) fn partition_tokens(tokens: &[Token]) -> SchemaElements {
    let mut composites = Vec::new();
    let mut enums = Vec::new();
    let mut sets = Vec::new();
    let mut messages = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i].signal {
            Signal::BeginComposite => {
                let end =
                    find_matching_end(tokens, i, Signal::BeginComposite, Signal::EndComposite);
                composites.push(tokens[i..=end].to_vec());
                i = end + 1;
            }
            Signal::BeginEnum => {
                let end = find_matching_end(tokens, i, Signal::BeginEnum, Signal::EndEnum);
                enums.push(tokens[i..=end].to_vec());
                i = end + 1;
            }
            Signal::BeginSet => {
                let end = find_matching_end(tokens, i, Signal::BeginSet, Signal::EndSet);
                sets.push(tokens[i..=end].to_vec());
                i = end + 1;
            }
            Signal::BeginMessage => {
                let end = find_matching_end(tokens, i, Signal::BeginMessage, Signal::EndMessage);
                messages.push(tokens[i..=end].to_vec());
                i = end + 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    SchemaElements {
        composites,
        enums,
        sets,
        messages,
    }
}

/// Shared bool detection: name convention OR semanticType="Boolean".
/// Used by codegen to emit `_bool()` accessors. Must match the predicate
/// in generate_enum. Does NOT use value-based heuristics — those are
/// reserved for [`is_bool_value_enum`] which powers `enable_bool_domain_type`.
pub(crate) fn is_bool_enum(elements: &SchemaElements, enum_name: &str) -> bool {
    enum_name == "BooleanType"
        || elements.enums.iter().any(|e| {
            e[0].name == enum_name && e[0].encoding.semantic_type.as_deref() == Some("Boolean")
        })
}

/// Extended detection: name, semanticType, OR exactly two valid values
/// forming a recognisable true/false pair with discriminants 0 and 1.
/// Used by `enable_bool_domain_type()` to auto-register `bool` converters
/// for schemas that don't use the canonical `BooleanType` naming.
///
/// Only the canonical `{0, 1}` representation is supported by auto-detection.
/// Other boolean encodings (e.g. `Yes=5, No=3`) should use explicit
/// `with_conversion` instead.
pub(crate) fn is_bool_value_enum(elements: &SchemaElements, enum_name: &str) -> bool {
    if is_bool_enum(elements, enum_name) {
        return true;
    }
    elements.enums.iter().any(|e| {
        if e[0].name != enum_name {
            return false;
        }
        let value_tokens: Vec<&crate::ir::Token> = e
            .iter()
            .filter(|t| t.signal == crate::ir::Signal::Encoding)
            .collect();
        if value_tokens.len() != 2 {
            return false;
        }
        let names: Vec<&str> = value_tokens.iter().map(|t| t.name.as_str()).collect();
        if !is_boolean_value_pair(names[0], names[1]) {
            return false;
        }
        // Only auto-detect when discriminants are exactly 0 and 1.
        // Arbitrary values (e.g. Yes=5, No=3) require explicit with_conversion.
        // Enum discriminant values are stored in encoding.constant_value (as strings).
        let has_disc_0 = value_tokens
            .iter()
            .any(|t| t.encoding.constant_value.as_deref() == Some("0"));
        let has_disc_1 = value_tokens
            .iter()
            .any(|t| t.encoding.constant_value.as_deref() == Some("1"));
        has_disc_0 && has_disc_1
    })
}

/// Heuristic: do the two enum value names form a true/false pair?
pub(crate) fn is_boolean_value_pair(a: &str, b: &str) -> bool {
    let (lower, upper) = if a.eq_ignore_ascii_case("true")
        || a.eq_ignore_ascii_case("yes")
        || a.eq_ignore_ascii_case("y")
        || a.eq_ignore_ascii_case("t")
    {
        (a, b)
    } else if b.eq_ignore_ascii_case("true")
        || b.eq_ignore_ascii_case("yes")
        || b.eq_ignore_ascii_case("y")
        || b.eq_ignore_ascii_case("t")
    {
        (b, a)
    } else {
        return false;
    };
    // Now `lower` is the truthy name; `upper` must be the falsy counterpart.
    upper.eq_ignore_ascii_case("false")
        || upper.eq_ignore_ascii_case("no")
        || upper.eq_ignore_ascii_case("n")
        || upper.eq_ignore_ascii_case("f")
}

pub(crate) struct MessageStructure {
    pub(crate) name: String,
    pub(crate) id: u16,
    pub(crate) since_version: u16,
    pub(crate) description: Option<String>,
    pub(crate) deprecated: bool,
    pub(crate) semantic_type: Option<String>,
    /// Root fixed-block length in bytes (schema `blockLength` when declared and
    /// larger than the tight field packing, else the computed field span).
    pub(crate) block_length: usize,
    pub(crate) fields: Vec<MessageField>,
    pub(crate) groups: Vec<MessageGroup>,
    pub(crate) var_data: Vec<MessageVarData>,
}

#[derive(Clone)]
pub(crate) struct MessageField {
    pub(crate) name: String,
    pub(crate) id: Option<u16>,
    pub(crate) offset: usize,
    pub(crate) presence: Presence,
    pub(crate) since_version: u16,
    pub(crate) null_value: Option<u64>,
    pub(crate) min_value: Option<u64>,
    pub(crate) max_value: Option<u64>,
    pub(crate) description: Option<String>,
    pub(crate) deprecated: bool,
    pub(crate) semantic_type: Option<String>,
    pub(crate) constant_value: Option<String>,
    /// Epoch for timestamp fields (e.g. `"unix"`); feeds `MetaAttribute::Epoch`.
    pub(crate) epoch: Option<String>,
    /// Time unit (e.g. `"nanosecond"`); feeds `MetaAttribute::TimeUnit`.
    pub(crate) time_unit: Option<String>,
    /// Character encoding for fixed char arrays (e.g. `"ASCII"`).
    pub(crate) character_encoding: Option<String>,
    pub(crate) field_type: FieldType,
}

#[derive(Clone)]
pub(crate) enum FieldType {
    Primitive(PrimitiveType, Option<usize>),
    Composite {
        name: String,
        size: usize,
    },
    Enum {
        name: String,
        encoding_type: PrimitiveType,
    },
    Set {
        name: String,
        encoding_type: PrimitiveType,
    },
}

impl FieldType {
    pub(crate) fn size(&self) -> usize {
        match self {
            Self::Primitive(p, length) => p.size() * length.unwrap_or(1),
            Self::Composite { size, .. } => *size,
            Self::Enum { encoding_type, .. } | Self::Set { encoding_type, .. } => {
                encoding_type.size()
            }
        }
    }

    pub(crate) fn rust_type_name(&self) -> String {
        match self {
            Self::Primitive(p, length) => {
                let base = rust_type(*p);
                if let Some(len) = length {
                    format!("[{}; {}]", base, len)
                } else {
                    base.to_string()
                }
            }
            Self::Composite { name, .. } | Self::Enum { name, .. } | Self::Set { name, .. } => {
                to_pascal_case(name)
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct MessageGroup {
    pub(crate) name: String,
    pub(crate) id: u16,
    pub(crate) since_version: u16,
    pub(crate) description: Option<String>,
    pub(crate) dimension_type: String,
    pub(crate) fields: Vec<MessageField>,
    pub(crate) groups: Vec<MessageGroup>,
    pub(crate) var_data: Vec<MessageVarData>,
    pub(crate) block_length: usize,
}

impl MessageGroup {
    /// Block length effective at runtime: the max of the schema-declared
    /// `blockLength` and the tight span of the group's fixed fields.
    pub(crate) fn effective_block_length(&self) -> usize {
        let computed = self.fields.iter().fold(0, |acc, f| {
            let size = f.field_type.size();
            acc.max(f.offset + size)
        });
        self.block_length.max(computed)
    }
}

#[derive(Clone)]
pub(crate) struct MessageVarData {
    pub(crate) name: String,
    pub(crate) id: u16,
    pub(crate) since_version: u16,
    pub(crate) description: Option<String>,
    pub(crate) type_name: String,
    pub(crate) max_length: Option<usize>,
    pub(crate) character_encoding: Option<String>,
}

pub(crate) fn parse_message_structure(
    tokens: &[Token],
    elements: &SchemaElements,
) -> MessageStructure {
    let begin_token = &tokens[0];
    let name = begin_token.name.clone();
    let id = begin_token.id.unwrap_or(0);
    let since_version = begin_token.encoding.since_version;
    let description = begin_token.encoding.description.clone();
    let deprecated = begin_token.encoding.deprecated;
    let semantic_type = begin_token.encoding.semantic_type.clone();
    // Populated by `resolve_message_offsets` (max of computed field span and
    // any schema-declared blockLength padding).
    let block_length = begin_token.encoding.offset.unwrap_or(0);

    let mut fields = Vec::new();
    let mut groups = Vec::new();
    let mut var_data = Vec::new();

    let mut i = 1;
    let end_limit = tokens.len() - 1;
    while i < end_limit {
        match tokens[i].signal {
            Signal::BeginField => {
                let end = find_matching_end(tokens, i, Signal::BeginField, Signal::EndField);
                let f = parse_field_structure(&tokens[i..=end], elements);
                fields.push(f);
                i = end + 1;
            }
            Signal::BeginGroup => {
                let end = find_matching_end(tokens, i, Signal::BeginGroup, Signal::EndGroup);
                let g = parse_group_structure(&tokens[i..=end], elements);
                groups.push(g);
                i = end + 1;
            }
            Signal::BeginVarData => {
                let end = find_matching_end(tokens, i, Signal::BeginVarData, Signal::EndVarData);
                let vd = parse_vardata_structure(&tokens[i..=end]);
                var_data.push(vd);
                i = end + 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    MessageStructure {
        name,
        id,
        since_version,
        description,
        deprecated,
        semantic_type,
        block_length,
        fields,
        groups,
        var_data,
    }
}

pub(crate) fn parse_field_structure(tokens: &[Token], elements: &SchemaElements) -> MessageField {
    let begin = &tokens[0];
    let name = begin.name.clone();
    let id = begin.id;
    let offset = begin.encoding.offset.unwrap_or(0);
    let presence = begin.encoding.presence;
    let since_version = begin.encoding.since_version;
    let null_value = begin.encoding.null_value;
    let min_value = begin.encoding.min_value;
    let max_value = begin.encoding.max_value;
    let description = begin.encoding.description.clone();
    let deprecated = begin.encoding.deprecated;
    let semantic_type = begin.encoding.semantic_type.clone();
    let constant_value = begin.encoding.constant_value.clone();
    let epoch = begin.encoding.epoch.clone();
    let time_unit = begin.encoding.time_unit.clone();
    let character_encoding = begin.encoding.character_encoding.clone();

    let field_type = if tokens.len() > 2 {
        let inner_signal = tokens[1].signal;
        let inner_name = tokens[1].name.clone();
        match inner_signal {
            Signal::BeginComposite => {
                let size = elements
                    .composites
                    .iter()
                    .find(|c| c[0].name == inner_name)
                    .and_then(|c| c[0].encoding.offset)
                    .unwrap_or(0);
                FieldType::Composite {
                    name: inner_name,
                    size,
                }
            }
            Signal::BeginEnum => {
                let encoding_type = tokens[1]
                    .encoding
                    .primitive_type
                    .unwrap_or(PrimitiveType::UInt8);
                FieldType::Enum {
                    name: inner_name,
                    encoding_type,
                }
            }
            Signal::BeginSet => {
                let encoding_type = tokens[1]
                    .encoding
                    .primitive_type
                    .unwrap_or(PrimitiveType::UInt8);
                FieldType::Set {
                    name: inner_name,
                    encoding_type,
                }
            }
            _ => FieldType::Primitive(
                begin
                    .encoding
                    .primitive_type
                    .unwrap_or(PrimitiveType::UInt8),
                begin.encoding.length,
            ),
        }
    } else {
        FieldType::Primitive(
            begin
                .encoding
                .primitive_type
                .unwrap_or(PrimitiveType::UInt8),
            begin.encoding.length,
        )
    };

    MessageField {
        name,
        id,
        offset,
        presence,
        since_version,
        null_value,
        min_value,
        max_value,
        description,
        deprecated,
        semantic_type,
        constant_value,
        epoch,
        time_unit,
        character_encoding,
        field_type,
    }
}

pub(crate) fn parse_group_structure(tokens: &[Token], elements: &SchemaElements) -> MessageGroup {
    let begin = &tokens[0];
    let name = begin.name.clone();
    let id = begin.id.unwrap_or(0);
    let since_version = begin.encoding.since_version;
    let description = begin.encoding.description.clone();
    let block_length = begin.encoding.offset.unwrap_or(0);

    let mut dimension_type = "groupSizeEncoding".to_string();
    let mut fields = Vec::new();
    let mut groups = Vec::new();
    let mut var_data = Vec::new();

    let mut i = 1;
    if tokens[i].signal == Signal::BeginComposite {
        dimension_type = tokens[i].name.clone();
        let dim_end = find_matching_end(tokens, i, Signal::BeginComposite, Signal::EndComposite);
        i = dim_end + 1;
    }

    let end_limit = tokens.len() - 1;
    while i < end_limit {
        match tokens[i].signal {
            Signal::BeginField => {
                let end = find_matching_end(tokens, i, Signal::BeginField, Signal::EndField);
                fields.push(parse_field_structure(&tokens[i..=end], elements));
                i = end + 1;
            }
            Signal::BeginGroup => {
                let end = find_matching_end(tokens, i, Signal::BeginGroup, Signal::EndGroup);
                groups.push(parse_group_structure(&tokens[i..=end], elements));
                i = end + 1;
            }
            Signal::BeginVarData => {
                let end = find_matching_end(tokens, i, Signal::BeginVarData, Signal::EndVarData);
                var_data.push(parse_vardata_structure(&tokens[i..=end]));
                i = end + 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    MessageGroup {
        name,
        id,
        since_version,
        description,
        dimension_type,
        fields,
        groups,
        var_data,
        block_length,
    }
}

pub(crate) fn parse_vardata_structure(tokens: &[Token]) -> MessageVarData {
    let begin = &tokens[0];
    let name = begin.name.clone();
    let id = begin.id.unwrap_or(0);
    let since_version = begin.encoding.since_version;
    let description = begin.encoding.description.clone();

    let mut type_name = "varDataEncoding".to_string();
    let mut max_length = None;
    // characterEncoding is declared on the <data> element, not the composite.
    let mut character_encoding = begin.encoding.character_encoding.clone();
    if tokens.len() > 2 && tokens[1].signal == Signal::BeginComposite {
        type_name = tokens[1].name.clone();
        // Scan composite members for the length field's max_value and any
        // characterEncoding declaration.
        let comp_end = find_matching_end(tokens, 1, Signal::BeginComposite, Signal::EndComposite);
        let mut i = 2;
        while i < comp_end {
            if tokens[i].signal == Signal::BeginField {
                if tokens[i].name == "length" {
                    max_length = tokens[i].encoding.max_value.map(|v| v as usize);
                }
                if character_encoding.is_none() {
                    character_encoding = tokens[i].encoding.character_encoding.clone();
                }
            }
            i += 1;
        }
    }

    MessageVarData {
        name,
        id,
        since_version,
        description,
        type_name,
        max_length,
        character_encoding,
    }
}

pub(crate) fn rust_type(prim: PrimitiveType) -> &'static str {
    match prim {
        PrimitiveType::Char => "u8",
        PrimitiveType::Int8 => "i8",
        PrimitiveType::UInt8 => "u8",
        PrimitiveType::Int16 => "i16",
        PrimitiveType::UInt16 => "u16",
        PrimitiveType::Int32 => "i32",
        PrimitiveType::UInt32 => "u32",
        PrimitiveType::Int64 => "i64",
        PrimitiveType::UInt64 => "u64",
        PrimitiveType::Float => "f32",
        PrimitiveType::Double => "f64",
    }
}

pub(crate) struct CompositeMember {
    pub(crate) name: String,
    pub(crate) offset: usize,
    pub(crate) since_version: u16,
    pub(crate) member_type: MemberType,
}

#[derive(Clone)]
pub(crate) enum MemberType {
    Primitive {
        prim: PrimitiveType,
        length: Option<usize>,
        presence: Presence,
        constant_value: Option<String>,
    },
    Composite {
        name: String,
        size: usize,
    },
    Enum {
        name: String,
        encoding_type: PrimitiveType,
    },
    Set {
        name: String,
        encoding_type: PrimitiveType,
    },
}

pub(crate) fn parse_composite_members(tokens: &[Token]) -> Vec<CompositeMember> {
    let mut members = Vec::new();
    let mut i = 1;
    let end_limit = tokens.len() - 1;
    while i < end_limit {
        if tokens[i].signal == Signal::BeginField {
            let name = tokens[i].name.clone();
            let offset = tokens[i].encoding.offset.unwrap_or(0);
            let since_version = tokens[i].encoding.since_version;
            let presence = tokens[i].encoding.presence;
            let constant_value = tokens[i].encoding.constant_value.clone();
            let length = tokens[i].encoding.length;

            let member_type = if i + 2 < tokens.len()
                && tokens[i + 1].signal == Signal::BeginComposite
            {
                let comp_name = tokens[i + 1].name.clone();
                // Prefer resolved composite size on BeginComposite; fall back to
                // scanning nested field tokens (nested `<ref>` clones can lag).
                let size = tokens[i + 1]
                    .encoding
                    .offset
                    .filter(|&s| s > 0)
                    .unwrap_or_else(|| {
                        let end = find_matching_end(
                            tokens,
                            i + 1,
                            Signal::BeginComposite,
                            Signal::EndComposite,
                        );
                        let mut sz = 0usize;
                        let mut j = i + 2;
                        while j < end {
                            if tokens[j].signal == Signal::BeginField {
                                if tokens[j].encoding.presence != Presence::Constant
                                    && !tokens[j].encoding.is_variable_length
                                {
                                    let prim_sz =
                                        tokens[j].encoding.primitive_type.map_or(0, |p| p.size());
                                    let len = tokens[j].encoding.length.unwrap_or(1);
                                    // Nested type (enum/set/composite) size from encoding type.
                                    let nested = if j + 1 < end {
                                        match tokens[j + 1].signal {
                                            Signal::BeginEnum | Signal::BeginSet => tokens[j + 1]
                                                .encoding
                                                .primitive_type
                                                .map_or(0, |p| p.size()),
                                            Signal::BeginComposite => {
                                                tokens[j + 1].encoding.offset.unwrap_or(0)
                                            }
                                            _ => prim_sz * len,
                                        }
                                    } else {
                                        prim_sz * len
                                    };
                                    sz += if nested > 0 { nested } else { prim_sz * len };
                                }
                                j = find_matching_end(
                                    tokens,
                                    j,
                                    Signal::BeginField,
                                    Signal::EndField,
                                ) + 1;
                            } else {
                                j += 1;
                            }
                        }
                        sz
                    });
                MemberType::Composite {
                    name: comp_name,
                    size,
                }
            } else if i + 2 < tokens.len() && tokens[i + 1].signal == Signal::BeginEnum {
                let enum_name = tokens[i + 1].name.clone();
                let encoding_type = tokens[i + 1]
                    .encoding
                    .primitive_type
                    .unwrap_or(PrimitiveType::UInt8);
                MemberType::Enum {
                    name: enum_name,
                    encoding_type,
                }
            } else if i + 2 < tokens.len() && tokens[i + 1].signal == Signal::BeginSet {
                let set_name = tokens[i + 1].name.clone();
                let encoding_type = tokens[i + 1]
                    .encoding
                    .primitive_type
                    .unwrap_or(PrimitiveType::UInt8);
                MemberType::Set {
                    name: set_name,
                    encoding_type,
                }
            } else {
                let prim = tokens[i]
                    .encoding
                    .primitive_type
                    .unwrap_or(PrimitiveType::UInt8);
                MemberType::Primitive {
                    prim,
                    length,
                    presence,
                    constant_value,
                }
            };

            members.push(CompositeMember {
                name,
                offset,
                since_version,
                member_type,
            });

            let end = find_matching_end(tokens, i, Signal::BeginField, Signal::EndField);
            i = end + 1;
        } else {
            i += 1;
        }
    }
    members
}

pub(crate) fn get_dimension_info(
    elements: &SchemaElements,
    dim_type: &str,
) -> (String, usize, String, String) {
    let raw_name = dim_type;
    let name = to_pascal_case(raw_name);
    let mut size = 4;
    let mut bl = "block_length".to_string();
    let mut num = "num_in_group".to_string();
    if let Some(comp) = elements.composites.iter().find(|c| c[0].name == raw_name) {
        size = comp[0].encoding.offset.unwrap_or(4);
        let members = parse_composite_members(comp);
        for m in members {
            let lower = m.name.to_lowercase();
            if lower.contains("blocklength") {
                bl = to_snake_case(&m.name);
            } else if lower.contains("numingroup") || lower.contains("count") {
                num = to_snake_case(&m.name);
            }
        }
    }
    (name, size, bl, num)
}

/// Returns (offset, size, primitive) of the numInGroup field within a dimension
/// composite. The primitive drives the encoder's `count` parameter width so a
/// schema whose dimensionType declares numInGroup as uint32 (e.g. Binance's
/// default `groupSizeEncoding`) writes all 4 bytes, not just 2.
pub(crate) fn get_dim_num_layout(
    elements: &SchemaElements,
    dim_type: &str,
) -> (usize, usize, PrimitiveType) {
    let raw_name = dim_type;
    let mut offset = 2;
    let mut size = 2;
    let mut prim = PrimitiveType::UInt16;
    if let Some(comp) = elements.composites.iter().find(|c| c[0].name == raw_name) {
        let members = parse_composite_members(comp);
        for m in members {
            let lower = m.name.to_lowercase();
            if lower.contains("numingroup") || lower.contains("count") {
                offset = m.offset;
                if let MemberType::Primitive {
                    prim: p, length, ..
                } = &m.member_type
                {
                    prim = *p;
                    size = p.size() * length.unwrap_or(1);
                }
            }
        }
    }
    (offset, size, prim)
}

/// Returns (offset, size, primitive) of the blockLength field within a
/// dimension composite. Dimension members may be reordered, padded, and use
/// any unsigned integer width supported by SBE.
pub(crate) fn get_dim_block_layout(
    elements: &SchemaElements,
    dim_type: &str,
) -> (usize, usize, PrimitiveType) {
    let mut offset = 0;
    let mut size = 2;
    let mut prim = PrimitiveType::UInt16;
    if let Some(comp) = elements
        .composites
        .iter()
        .find(|composite| composite[0].name == dim_type)
    {
        for member in parse_composite_members(comp) {
            if member.name.to_lowercase().contains("blocklength") {
                offset = member.offset;
                if let MemberType::Primitive {
                    prim: primitive,
                    length,
                    ..
                } = member.member_type
                {
                    prim = primitive;
                    size = primitive.size() * length.unwrap_or(1);
                }
            }
        }
    }
    (offset, size, prim)
}

pub(crate) fn get_vardata_info(
    elements: &SchemaElements,
    type_name: &str,
) -> (String, usize, String, PrimitiveType) {
    let raw_name = type_name;
    let name = to_pascal_case(raw_name);
    let mut size = 4;
    let mut len_field = "length".to_string();
    let mut prim = PrimitiveType::UInt32;
    if let Some(comp) = elements.composites.iter().find(|c| c[0].name == raw_name) {
        let members = parse_composite_members(comp);
        for m in members {
            if m.name == "length" {
                len_field = to_snake_case(&m.name);
                if let MemberType::Primitive { prim: p, .. } = m.member_type {
                    prim = p;
                }
            }
            if m.name == "varData" {
                size = m.offset;
            }
        }
    }
    (name, size, len_field, prim)
}

/// Name of the concrete decoder stage entered after consuming tail component `i`.
/// `stage_prefix` is the owner decoder's name (e.g. `CarDecoder` or
/// `BidsEntryDecoder`); the final component yields `{prefix}Complete`, earlier
/// ones yield `{prefix}After{FieldPascal}`.
pub(crate) fn decoder_stage_after_ident(
    stage_prefix: &str,
    field_pascal: &str,
    i: usize,
    total_tail: usize,
    span: proc_macro2::Span,
) -> syn::Ident {
    if i == total_tail - 1 {
        syn::Ident::new(&format!("{stage_prefix}Complete"), span)
    } else {
        syn::Ident::new(&format!("{stage_prefix}After{field_pascal}"), span)
    }
}

/// One tail group component of an owner (message or entry), resolved for codegen.
pub(crate) struct OwnerTailGroup {
    pub(crate) accessor_snake: String,
    pub(crate) field_pascal: String,
    pub(crate) group_decoder_ident: String,
    pub(crate) entry_decoder_ident: String,
}

/// One tail var-data component of an owner, resolved for codegen.
pub(crate) struct OwnerTailVarData {
    pub(crate) accessor_snake: String,
    pub(crate) field_pascal: String,
    pub(crate) type_pascal: String,
    pub(crate) prefix_size: usize,
    pub(crate) len_field: String,
    pub(crate) len_type: PrimitiveType,
    pub(crate) max_length: Option<usize>,
    pub(crate) name: String,
    pub(crate) character_encoding: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Encoding;

    fn token(name: &str, signal: Signal, encoding: Encoding) -> Token {
        Token {
            id: None,
            name: name.to_string(),
            signal,
            encoding,
            span: None,
        }
    }

    #[test]
    fn unresolved_nested_composite_size_falls_back_to_member_scan() {
        let primitive = Encoding {
            primitive_type: Some(PrimitiveType::UInt16),
            length: Some(2),
            ..Encoding::default()
        };
        let tokens = vec![
            token("Outer", Signal::BeginComposite, Encoding::default()),
            token("nested", Signal::BeginField, Encoding::default()),
            token("Inner", Signal::BeginComposite, Encoding::default()),
            token("values", Signal::BeginField, primitive),
            token("values", Signal::Encoding, Encoding::default()),
            token("values", Signal::EndField, Encoding::default()),
            token("Inner", Signal::EndComposite, Encoding::default()),
            token("nested", Signal::EndField, Encoding::default()),
            token("Outer", Signal::EndComposite, Encoding::default()),
        ];

        let members = parse_composite_members(&tokens);

        assert_eq!(members.len(), 1);
        assert!(matches!(
            members[0].member_type,
            MemberType::Composite { size: 4, .. }
        ));
    }

    fn enum_elements(
        name: &str,
        semantic_type: Option<&str>,
        values: &[(&str, u16)],
    ) -> SchemaElements {
        let mut enum_tokens = vec![token(
            name,
            Signal::BeginEnum,
            Encoding {
                semantic_type: semantic_type.map(str::to_string),
                ..Encoding::default()
            },
        )];
        enum_tokens.extend(values.iter().map(|&(value, disc)| {
            token(
                value,
                Signal::Encoding,
                Encoding {
                    presence: crate::ir::Presence::Constant,
                    constant_value: Some(disc.to_string()),
                    ..Encoding::default()
                },
            )
        }));
        enum_tokens.push(token(name, Signal::EndEnum, Encoding::default()));
        SchemaElements {
            composites: Vec::new(),
            enums: vec![enum_tokens],
            sets: Vec::new(),
            messages: Vec::new(),
        }
    }

    #[test]
    fn bool_enum_detection_covers_names_semantics_and_value_pairs() {
        let ordinary = enum_elements("Enabled", None, &[("Yes", 0), ("No", 1)]);
        assert!(is_bool_value_enum(&ordinary, "Enabled"));
        assert!(!is_bool_value_enum(&ordinary, "Other"));

        let reversed = enum_elements("Active", None, &[("false", 1), ("TRUE", 0)]);
        assert!(is_bool_value_enum(&reversed, "Active"));

        let non_boolean = enum_elements("Side", None, &[("Buy", 0), ("Sell", 1)]);
        assert!(!is_bool_value_enum(&non_boolean, "Side"));

        // Auto-detection requires discriminants 0/1; arbitrary values are rejected.
        let non_canonical_disc = enum_elements("Choice", None, &[("Yes", 5), ("No", 3)]);
        assert!(!is_bool_value_enum(&non_canonical_disc, "Choice"));

        let semantic = enum_elements("Flag", Some("Boolean"), &[("Off", 0), ("On", 1)]);
        assert!(is_bool_enum(&semantic, "Flag"));
        assert!(is_bool_value_enum(&semantic, "Flag"));

        let canonical = enum_elements("BooleanType", None, &[("Zero", 0), ("One", 1)]);
        assert!(is_bool_enum(&canonical, "BooleanType"));
    }
}
