//! Parse `<types>`: type / composite / enum / set elements.

use std::collections::HashSet;

use roxmltree::Node;

use crate::ir::{Encoding, Presence, PrimitiveType, Signal, Token};

use super::attr::{
    collect_description, element_children, is_primitive_name, opt_u16_attr, opt_usize_attr,
    parse_deprecated_attr, parse_presence, parse_primitive_type, preceding_xml_comments,
    reject_duplicate_type_name, string_attr, structural, u16_attr, validate_sbe_name,
};
use super::error::Fault;
use super::registry::{
    TypeRegistry, compute_type_size, estimate_composite_member_size, parse_u64_val,
    resolve_type_to_tokens,
};
use super::warn::{WarnState, warn_once};

pub(crate) fn parse_types_node(
    node: Node<'_, '_>,
    registry: &mut TypeRegistry,
    tokens: &mut Vec<Token>,
    warn_state: &WarnState,
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
                let encoding = parse_type_element(type_child, registry, warn_state)?;
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
                parse_enum(warn_state, type_child, registry, tokens)?;
            }
            "set" => {
                parse_set(warn_state, type_child, registry, tokens)?;
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
                parse_composite(warn_state, cnode, registry, tokens)?;
            } else {
                still.push(cnode);
            }
        }
        if still.len() == before {
            // No progress: expand remaining to surface cycle/forward-ref errors.
            for cnode in still {
                parse_composite(warn_state, cnode, registry, tokens)?;
            }
            break;
        }
        pending = still;
    }
    Ok(())
}

/// True when every composite member type/ref is already in the registry
/// (or is a primitive / self-cycle which parse_composite will reject).
pub(crate) fn composite_refs_ready(node: Node<'_, '_>, registry: &TypeRegistry) -> bool {
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

pub(crate) fn parse_type_element(
    node: Node<'_, '_>,
    _registry: &TypeRegistry,
    warn_state: &WarnState,
) -> Result<Encoding, Fault> {
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
    let deprecated = parse_deprecated_attr(node)?;

    let null_value = node
        .attribute("nullValue")
        .and_then(|s| parse_u64_val(s, primitive_type));
    if null_value.is_some() && presence != Presence::Optional {
        let type_name = node.attribute("name").unwrap_or("<unnamed>");
        warn_once(
            &format!(
                "warning: nullValue specified on non-optional type '{type_name}' \
                 \u{2014} nullValue is only meaningful for optional types"
            ),
            Some(node),
            warn_state,
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

pub(crate) fn parse_composite(
    warn_state: &WarnState,
    node: Node<'_, '_>,
    registry: &mut TypeRegistry,
    tokens: &mut Vec<Token>,
) -> Result<(), Fault> {
    let name = string_attr(node, "name", "composite @name")?;
    validate_sbe_name(node, &name, "composite @name")?;
    reject_duplicate_type_name(node, &name, registry)?;
    let since_version = opt_u16_attr(node, "sinceVersion", "sinceVersion")?.unwrap_or(0);
    let composite_deprecated = parse_deprecated_attr(node)?;

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
        span: None,
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
                parse_enum(warn_state, child, registry, tokens)?;
            }
            let since_val = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            if let Some(resolved) = resolve_type_to_tokens(
                &enum_name,
                &enum_name,
                None,
                registry,
                since_val,
                Some(child.range()),
                None,
            ) {
                composite_tokens.extend(resolved);
            }
            continue;
        }
        if tag == "set" {
            let set_name = string_attr(child, "name", "composite nested set @name")?;
            if !registry.registry.contains_key(&set_name) {
                parse_set(warn_state, child, registry, tokens)?;
            }
            let since_val = opt_u16_attr(child, "sinceVersion", "sinceVersion")?.unwrap_or(0);
            if let Some(resolved) = resolve_type_to_tokens(
                &set_name,
                &set_name,
                None,
                registry,
                since_val,
                Some(child.range()),
                None,
            ) {
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
                parse_composite(warn_state, child, registry, tokens)?;
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
            if let Some(resolved) = resolve_type_to_tokens(
                &nested_name,
                &nested_name,
                None,
                registry,
                since_val,
                Some(child.range()),
                None,
            ) {
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
            if let Some(resolved) = resolve_type_to_tokens(
                &member_name,
                ref_name,
                None,
                registry,
                since_val,
                Some(child.range()),
                None,
            ) {
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
                    let mut encoding = parse_type_element(child, registry, warn_state)?;
                    // SBE var-data payload member: never contributes fixed composite
                    // size (length prefix alone is the wire header). Mark even when
                    // `length` is omitted (defaults to 1 in type attrs) so uint8
                    // length prefixes don't produce a false 2-byte VarDataEncoding.
                    if member_name == "varData" || encoding.length == Some(0) {
                        encoding.is_variable_length = true;
                    }
                    composite_tokens.push(Token {
                        id: None,
                        name: member_name.clone(),
                        signal: Signal::BeginField,
                        encoding: encoding.clone(),
                        span: None,
                    });
                    composite_tokens.push(Token {
                        id: None,
                        name: member_name.clone(),
                        signal: Signal::EndField,
                        encoding: Encoding::default(),
                        span: None,
                    });
                } else if let Some(resolved) = resolve_type_to_tokens(
                    &member_name,
                    t_name,
                    None,
                    registry,
                    since_val,
                    Some(child.range()),
                    None,
                ) {
                    composite_tokens.extend(resolved);
                } else {
                    let mut encoding = parse_type_element(child, registry, warn_state)?;
                    if member_name == "varData" || encoding.length == Some(0) {
                        encoding.is_variable_length = true;
                    }
                    composite_tokens.push(Token {
                        id: None,
                        name: member_name.clone(),
                        signal: Signal::BeginField,
                        encoding: encoding.clone(),
                        span: None,
                    });
                    composite_tokens.push(Token {
                        id: None,
                        name: member_name.clone(),
                        signal: Signal::EndField,
                        encoding: Encoding::default(),
                        span: None,
                    });
                }
            } else {
                let mut encoding = parse_type_element(child, registry, warn_state)?;
                if member_name == "varData" || encoding.length == Some(0) {
                    encoding.is_variable_length = true;
                }
                composite_tokens.push(Token {
                    id: None,
                    name: member_name.clone(),
                    signal: Signal::BeginField,
                    encoding: encoding.clone(),
                    span: None,
                });
                composite_tokens.push(Token {
                    id: None,
                    name: member_name.clone(),
                    signal: Signal::EndField,
                    encoding: Encoding::default(),
                    span: None,
                });
            }
        }
    }

    composite_tokens.push(structural(&name, Signal::EndComposite, Some(node.range())));

    registry
        .registry
        .insert(name.clone(), composite_tokens.clone());
    tokens.extend(composite_tokens);
    Ok(())
}

pub(crate) fn parse_enum(
    warn_state: &WarnState,
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
            deprecated: parse_deprecated_attr(node)?,
            description: collect_description(node),
            semantic_type,
            null_value: node
                .attribute("nullValue")
                .and_then(|s| parse_u64_val(s, Some(encoding_type))),
            ..Encoding::default()
        },
        span: None,
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
                span: None,
            });
        }
    }

    enum_tokens.push(structural(&name, Signal::EndEnum, Some(node.range())));

    registry.registry.insert(name, enum_tokens.clone());
    tokens.extend(enum_tokens);
    Ok(())
}

pub(crate) fn parse_set(
    warn_state: &WarnState,
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
            deprecated: parse_deprecated_attr(node)?,
            description: collect_description(node),
            ..Encoding::default()
        },
        span: None,
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
                span: None,
            });
        }
    }

    set_tokens.push(structural(&name, Signal::EndSet, Some(node.range())));

    registry.registry.insert(name, set_tokens.clone());
    tokens.extend(set_tokens);
    Ok(())
}
