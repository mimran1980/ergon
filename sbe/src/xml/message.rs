//! Parse `<message>` and its field / group / data children.

use std::collections::HashSet;

use roxmltree::Node;

use crate::ir::{Encoding, Presence, PrimitiveType, Signal, Token};

use super::attr::{
    collect_description, earliest_deprecated, element_children, is_primitive_name, opt_u16_attr,
    opt_usize_attr, parse_deprecated_attr, parse_presence, parse_primitive_type,
    preceding_xml_comments, reject_unknown_attrs, string_attr, structural, u16_attr,
    validate_sbe_name,
};
use super::error::Fault;
use super::registry::{TypeRegistry, parse_u64_val, resolve_type_to_tokens};
use super::warn::{WarnState, warn_once};

use crate::schema_attrs;

pub(crate) fn parse_message(
    node: Node<'_, '_>,
    header_type: &str,
    registry: &TypeRegistry,
    tokens: &mut Vec<Token>,
    warn_state: &WarnState,
) -> Result<(), Fault> {
    reject_unknown_attrs(node, "message", schema_attrs::MESSAGE)?;
    let name = string_attr(node, "name", "message @name")?;
    validate_sbe_name(node, &name, "message @name")?;
    let id = u16_attr(node, "id", "message @id")?;
    let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);
    let block_length = opt_u16_attr(node, "blockLength", "blockLength")?;
    let message_deprecated = parse_deprecated_attr(node)?;

    validate_message_member_order(node)?;

    tokens.push(Token {
        id: Some(id),
        name: name.clone(),
        signal: Signal::BeginMessage,
        encoding: Encoding {
            since_version,
            deprecated: message_deprecated,
            description: collect_description(node),
            semantic_type: node.attribute("semanticType").map(str::to_string),
            // Stash declared `blockLength` on the BeginMessage token so resolve can
            // honor schema padding (max(computed, declared)). Cleared/overwritten by
            // `resolve_message_offsets` with the final wire block length.
            offset: block_length.map(|b| b as usize),
            ..Encoding::default()
        },
        span: None,
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

    for child in element_children(node) {
        parse_message_child(child, registry, tokens, warn_state)?;
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
                let child_id: u16 = id_str.parse().map_err(|_| {
                    Fault::invalid(
                        child,
                        "field/group/data @id",
                        format!("'{id_str}' is not a valid u16"),
                    )
                })?;
                if !seen_ids.insert(child_id) {
                    return Err(Fault::invalid(
                        child,
                        "duplicate field/group/data id in message",
                        id_str.to_string(),
                    ));
                }
            }
            if let Some(offset_str) = child.attribute("offset") {
                let offset: usize = offset_str.parse().map_err(|_| {
                    Fault::invalid(
                        child,
                        "field @offset",
                        format!("'{offset_str}' is not a valid non-negative integer"),
                    )
                })?;
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

    tokens.push(structural(&name, Signal::EndMessage, Some(node.range())));
    Ok(())
}

pub(crate) fn validate_message_member_order(node: Node<'_, '_>) -> Result<(), Fault> {
    let mut phase = 0u8;
    for child in element_children(node) {
        let next_phase = match child.tag_name().name() {
            "field" => 0,
            "group" => 1,
            "data" => 2,
            _ => continue,
        };
        if next_phase < phase {
            return Err(Fault::invalid(
                child,
                "message member order",
                "fixed fields must precede groups, and groups must precede data fields",
            ));
        }
        phase = next_phase;
        if child.tag_name().name() == "group" {
            validate_message_member_order(child)?;
        }
    }
    Ok(())
}

pub(crate) fn parse_message_child(
    node: Node<'_, '_>,
    registry: &TypeRegistry,
    tokens: &mut Vec<Token>,
    warn_state: &WarnState,
) -> Result<(), Fault> {
    match node.tag_name().name() {
        "field" => {
            reject_unknown_attrs(node, "field", schema_attrs::FIELD_LIKE)?;
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
            let deprecated = earliest_deprecated(
                parse_deprecated_attr(node)?,
                type_encoding.and_then(|e| e.deprecated),
            );
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
                warn_once(
                    &format!(
                        "warning: nullValue specified on non-optional field '{field_name}' \
                         \u{2014} nullValue is only meaningful for optional fields"
                    ),
                    Some(node),
                    warn_state,
                );
            }
            let constant_value = if presence == Presence::Constant {
                let from_value_ref = node.attribute("valueRef");
                let from_constant_value = node.attribute("constantValue");
                if from_value_ref.is_none() && from_constant_value.is_none() {
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
                from_value_ref
                    .or(from_constant_value)
                    .map(|s| {
                        if from_value_ref.is_some() {
                            // valueRef format: "EnumName.ValidValue" — validate
                            // the enum and variant exist at parse time (sbe-tool
                            // rejects invalid valueRef).
                            if let Some((enum_name, _variant_name)) = s.split_once('.') {
                                if !registry.registry.contains_key(enum_name) {
                                    warn_once(
                                        &format!(
                                            "warning: valueRef '{s}' references unknown enum '{enum_name}'"
                                        ),
                                        Some(node),
                                        warn_state,
                                    );
                                }
                            }
                        }
                        s.to_string()
                    })
            } else {
                None
            };

            let field_description = collect_description(node);
            if let Some(resolved) = resolve_type_to_tokens(
                &field_name,
                &type_name,
                Some(id),
                registry,
                since_version,
                Some(node.range()),
                field_description,
            ) {
                let mut inlined = resolved;
                if let Some(first) = inlined.first_mut() {
                    if let Some(offset_str) = node.attribute("offset") {
                        match offset_str.parse::<usize>() {
                            Ok(offset) => first.encoding.offset = Some(offset),
                            Err(_) => {
                                return Err(Fault::invalid(
                                    node,
                                    "field @offset",
                                    format!("'{offset_str}' is not a valid non-negative integer"),
                                ));
                            }
                        }
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
                return Err(Fault::invalid(
                    node,
                    format!("type for field '{field_name}'"),
                    &type_name,
                ));
            }
        }
        "group" => {
            reject_unknown_attrs(node, "group", schema_attrs::GROUP)?;
            let group_name = string_attr(node, "name", "group @name")?;
            let id = u16_attr(node, "id", "group @id")?;
            let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            let group_deprecated = parse_deprecated_attr(node)?;
            let dimension_type = node
                .attribute("dimensionType")
                .unwrap_or("groupSizeEncoding");
            let group_block_length = match node.attribute("blockLength") {
                Some(s) => match s.parse::<usize>() {
                    Ok(v) => Some(v),
                    Err(_) => {
                        return Err(Fault::invalid(
                            node,
                            "group @blockLength",
                            format!("'{s}' is not a valid non-negative integer"),
                        ));
                    }
                },
                None => None,
            };

            tokens.push(Token {
                id: Some(id),
                name: group_name.clone(),
                signal: Signal::BeginGroup,
                encoding: Encoding {
                    since_version,
                    deprecated: group_deprecated,
                    description: collect_description(node),
                    offset: group_block_length,
                    ..Encoding::default()
                },
                span: None,
            });

            if let Some(dim_tokens) = registry.registry.get(dimension_type) {
                validate_dimension_composite(node, dimension_type, dim_tokens)?;
                tokens.extend(dim_tokens.clone());
            } else {
                return Err(Fault::invalid(node, "group dimensionType", dimension_type));
            }

            for child in element_children(node) {
                parse_message_child(child, registry, tokens, warn_state)?;
            }

            tokens.push(structural(
                &group_name,
                Signal::EndGroup,
                Some(node.range()),
            ));
        }
        "data" => {
            reject_unknown_attrs(node, "data", schema_attrs::FIELD_LIKE)?;
            let data_name = string_attr(node, "name", "data @name")?;
            let id = u16_attr(node, "id", "data @id")?;
            let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            let data_deprecated = parse_deprecated_attr(node)?;
            let type_name = node.attribute("type").unwrap_or("varDataEncoding");
            let data_presence = node
                .attribute("presence")
                .map(|value| parse_presence(node, value))
                .transpose()?
                .unwrap_or(Presence::Required);
            if data_presence != Presence::Required {
                return Err(Fault::invalid(
                    node,
                    "data presence",
                    "variable-length data cannot be optional or constant",
                ));
            }

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
                span: None,
            });

            if let Some(type_tokens) = registry.registry.get(type_name) {
                // FIX SBE variable-data encodings consist of exactly two
                // scalar members named `length` and `varData`, in that order.
                // The generated codecs write the prefix at the start of the
                // composite and the octets immediately after it, so accepting
                // a different layout would silently generate the wrong wire.
                let members: Vec<&Token> = type_tokens
                    .iter()
                    .filter(|token| token.signal == Signal::BeginField)
                    .collect();
                if members.len() != 2 || members[0].name != "length" || members[1].name != "varData"
                {
                    return Err(Fault::invalid(
                        node,
                        "data type",
                        format!("{type_name}: expected exactly 'length' then 'varData' members"),
                    ));
                }

                let length = members[0];
                let length_primitive = length.encoding.primitive_type.ok_or_else(|| {
                    Fault::invalid(
                        node,
                        "data length type",
                        format!("{type_name}.length must be a primitive unsigned integer"),
                    )
                })?;
                if !matches!(
                    length_primitive,
                    PrimitiveType::UInt8
                        | PrimitiveType::UInt16
                        | PrimitiveType::UInt32
                        | PrimitiveType::UInt64
                ) || length.encoding.presence != Presence::Required
                    || length.encoding.length.unwrap_or(1) != 1
                    || length.encoding.offset.is_some_and(|offset| offset != 0)
                {
                    return Err(Fault::invalid(
                        node,
                        "data length type",
                        format!(
                            "{type_name}.length must be a required scalar unsigned integer at offset 0"
                        ),
                    ));
                }

                let var_data = members[1];
                let expected_data_offset = length_primitive.size();
                if !matches!(
                    var_data.encoding.primitive_type,
                    Some(PrimitiveType::Char | PrimitiveType::UInt8)
                ) || var_data.encoding.presence != Presence::Required
                    || var_data.encoding.length.unwrap_or(0) != 0
                    || var_data
                        .encoding
                        .offset
                        .is_some_and(|offset| offset != expected_data_offset)
                {
                    return Err(Fault::invalid(
                        node,
                        "data payload type",
                        format!(
                            "{type_name}.varData must be required variable-length octets immediately after length"
                        ),
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

            tokens.push(structural(
                &data_name,
                Signal::EndVarData,
                Some(node.range()),
            ));
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

fn dimension_member_size(token: &Token) -> usize {
    let width = token
        .encoding
        .primitive_type
        .map(PrimitiveType::size)
        .unwrap_or(0);
    width.saturating_mul(token.encoding.length.unwrap_or(1))
}

fn validate_dimension_member(
    node: Node<'_, '_>,
    dimension_type: &str,
    field: &str,
    token: &Token,
) -> Result<(), Fault> {
    let qualified = format!("{dimension_type}.{field}");
    let length = token.encoding.length.unwrap_or(1);
    let unsigned = matches!(
        token.encoding.primitive_type,
        Some(
            PrimitiveType::UInt8
                | PrimitiveType::UInt16
                | PrimitiveType::UInt32
                | PrimitiveType::UInt64
        )
    );
    if !unsigned || token.encoding.presence != Presence::Required || length != 1 {
        return Err(Fault::invalid(
            node,
            format!("group dimensionType {qualified}"),
            "must be a required scalar unsigned integer",
        ));
    }
    Ok(())
}

fn validate_dimension_composite(
    node: Node<'_, '_>,
    dimension_type: &str,
    dim_tokens: &[Token],
) -> Result<(), Fault> {
    let fields: Vec<&Token> = dim_tokens
        .iter()
        .filter(|token| token.signal == Signal::BeginField)
        .collect();
    let block = fields
        .iter()
        .copied()
        .find(|token| token.name == "blockLength");
    let count = fields
        .iter()
        .copied()
        .find(|token| token.name == "numInGroup");
    let (Some(block), Some(count)) = (block, count) else {
        return Err(Fault::invalid(
            node,
            "group dimensionType",
            format!("{dimension_type}: expected 'blockLength' and 'numInGroup' fields"),
        ));
    };
    validate_dimension_member(node, dimension_type, "blockLength", block)?;
    validate_dimension_member(node, dimension_type, "numInGroup", count)?;

    let mut current = 0usize;
    let mut occupied: Vec<(&str, usize, usize)> = Vec::new();
    for field in fields {
        let size = dimension_member_size(field);
        let offset = field.encoding.offset.unwrap_or(current);
        let end = offset.checked_add(size).ok_or_else(|| {
            Fault::invalid(
                node,
                format!("group dimensionType {dimension_type}.{}", field.name),
                format!("offset {offset} is out of bounds"),
            )
        })?;
        for (other, start, other_end) in &occupied {
            if offset < *other_end && end > *start {
                return Err(Fault::invalid(
                    node,
                    format!("group dimensionType {dimension_type}.{}", field.name),
                    format!("overlaps {other} at [{start}, {other_end})"),
                ));
            }
        }
        occupied.push((field.name.as_str(), offset, end));
        current = current.max(end);
    }
    Ok(())
}
