//! Conversion and DTO helper functions shared by codegen paths.

use super::runtime::to_snake_case;
use crate::ir::{Presence, PrimitiveType};
use crate::structured_ir::{FieldType, MessageField, SchemaElements, rust_type};
use crate::{GenerationConfig, Schema};

/// Extract [`crate::FieldInfo`] entries from message fields (constant fields
/// excluded). Used by context builders and hook dispatch.
pub(crate) fn message_field_infos(
    fields: &[MessageField],
    domain_types: &[(crate::ConversionSelector, String)],
    elements: Option<&SchemaElements>,
) -> Vec<crate::FieldInfo> {
    fields
        .iter()
        .filter(|f| f.presence != Presence::Constant)
        .map(|f| {
            // Domain types apply to the DTO field only when the DTO
            // generation actually uses them: required scalar primitives
            // and boolean enums (the latter detected via is_bool_enum,
            // matching the unconditional bool emission in DTO generation).
            // Optional fields, arrays, composites/enums/sets without a
            // domain config all keep the wire type.
            let domain_ty = match &f.field_type {
                FieldType::Primitive(_, length) => {
                    if length.is_none() && f.presence == Presence::Required {
                        find_domain_type(f, domain_types)
                    } else {
                        None
                    }
                }
                FieldType::Enum {
                    name: enum_name, ..
                } => {
                    if elements.is_some_and(|el| crate::structured_ir::is_bool_enum(el, enum_name))
                    {
                        Some("bool")
                    } else {
                        find_domain_type(f, domain_types)
                    }
                }
                _ => None,
            };
            let rust_type = match domain_ty {
                Some(dt) => dt.to_string(),
                None => f.field_type.rust_type_name(),
            };
            crate::FieldInfo {
                name: to_snake_case(&f.name),
                rust_type,
                offset: Some(f.offset),
                since_version: f.since_version,
                semantic_type: f.semantic_type.clone(),
                presence: presence_str(f.presence),
                null_value: f.null_value,
                deprecated: f.deprecated,
                description: f.description.clone(),
            }
        })
        .collect()
}

pub(crate) fn presence_str(p: Presence) -> &'static str {
    match p {
        Presence::Required => "required",
        Presence::Optional => "optional",
        Presence::Constant => "constant",
    }
}

/// Resolve a field accessor name, appending `_field` when it clashes
/// with a reserved method name on the decoder/encoder.
pub(crate) fn resolve_field_ident(
    snake_name: &str,
    wire_name: &Option<String>,
    reserved: &[&str],
) -> syn::Ident {
    let method_name = wire_name.as_deref().unwrap_or(snake_name);
    let resolved: &str = match () {
        _ if wire_name.is_some() => method_name,
        _ if reserved.contains(&snake_name) => {
            // ponytail: allocate only on collision (rare, build-time only).
            Box::leak(format!("{snake_name}_field").into_boxed_str())
        }
        _ => snake_name,
    };
    syn::Ident::new(resolved, proc_macro2::Span::call_site())
}

/// Warn if a shared type has version-gated members (`sinceVersion > 0`).
///
/// Version numbers are per-schema. A shared type with members added in a later
/// version is ambiguous when imported by a schema at a different version — the
/// importer's `acting_version` may not match the type's evolution timeline.
/// Returns `Some(warning_string)` if the type carries version-gated members.
pub(crate) fn warn_version_gated(
    type_name: &str,
    tokens: &[crate::ir::Token],
    schema: &Schema,
) -> Option<String> {
    let max_since = tokens
        .iter()
        .filter_map(|t| {
            if t.signal == crate::ir::Signal::Encoding || t.signal == crate::ir::Signal::BeginField
            {
                if t.encoding.since_version > 0 {
                    Some(t.encoding.since_version)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .max()?;
    Some(format!(
        "warning: shared type `{}` (schema {} id {}) has members at sinceVersion={max_since}. \
         Version numbers are per-schema — importing schemas at different versions may decode \
         these members incorrectly. Consider keeping shared types at version 0.",
        type_name, schema.package, schema.id
    ))
}

pub(crate) fn field_has_conversion_free(
    field: &MessageField,
    conversions: &[crate::ConversionSelector],
) -> bool {
    let type_name = match &field.field_type {
        FieldType::Composite { name, .. } => name.clone(),
        FieldType::Enum { name, .. } => name.clone(),
        FieldType::Set { name, .. } => name.clone(),
        FieldType::Primitive(pt, _) => rust_type(*pt).to_string(),
    };
    conversions.iter().any(|sel| match sel {
        crate::ConversionSelector::NamedType(n) => n == &type_name,
        crate::ConversionSelector::SemanticType(st) => {
            field.semantic_type.as_deref() == Some(st.as_str())
        }
        _ => false,
    })
}

pub(crate) fn find_domain_type<'a>(
    field: &MessageField,
    domain_types: &'a [(crate::ConversionSelector, String)],
) -> Option<&'a str> {
    let type_name = match &field.field_type {
        FieldType::Composite { name, .. } => name.clone(),
        FieldType::Enum { name, .. } => name.clone(),
        FieldType::Set { name, .. } => name.clone(),
        FieldType::Primitive(pt, _) => rust_type(*pt).to_string(),
    };
    domain_types.iter().find_map(|(sel, ty)| match sel {
        crate::ConversionSelector::NamedType(n) if n == &type_name => Some(ty.as_str()),
        crate::ConversionSelector::SemanticType(st)
            if field.semantic_type.as_deref() == Some(st.as_str()) =>
        {
            Some(ty.as_str())
        }
        _ => None,
    })
}

/// Encoder setter name used by domain DTOs.
///
/// When a conversion is configured without a domain type, flyweight setters
/// are renamed to `*_wire` (concrete domain methods take the bare name).
/// Domain-object encode must call the same name.
pub(crate) fn domain_encode_setter_name(
    field: &MessageField,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    field_snake: &str,
) -> String {
    if field_has_conversion_free(field, conversions)
        && find_domain_type(field, domain_types).is_none()
    {
        format!("{field_snake}_wire")
    } else {
        field_snake.to_string()
    }
}
