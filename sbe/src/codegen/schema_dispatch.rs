//! Opt-in dispatch over a configured set of sibling schema modules.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::{
    GenerateError, GeneratedModule, canonical_token_fingerprint, parse_composite_members,
    partition_tokens, to_pascal_case, to_snake_case,
};
use crate::Schema;
use crate::ir::Presence;
use crate::structured_ir::MemberType;

fn invalid(value: &str, reason: &str) -> GenerateError {
    GenerateError::InvalidConfiguration {
        option: "schema_dispatch".into(),
        value: value.into(),
        reason: reason.into(),
    }
}

fn validate(schemas: &[(&Schema, &str)], module_name: &str) -> Result<(), GenerateError> {
    if !crate::config::is_valid_module_ident(module_name) {
        return Err(invalid(
            module_name,
            "module name must be a Rust identifier",
        ));
    }
    if schemas.is_empty() {
        return Err(invalid(module_name, "at least one schema is required"));
    }
    let mut ids = HashSet::new();
    let mut modules = HashSet::from([module_name.to_owned()]);
    let mut variants = HashSet::from([
        "Self".to_owned(),
        "Other".to_owned(),
        "BufferTooShort".to_owned(),
        "InvalidHeader".to_owned(),
    ]);
    let mut fingerprint = None;
    for (schema, name) in schemas {
        if !crate::config::is_valid_module_ident(name) || !modules.insert((*name).to_owned()) {
            return Err(invalid(
                name,
                "schema module names must be unique Rust identifiers",
            ));
        }
        let variant = to_pascal_case(name);
        if !crate::config::is_valid_module_ident(&variant) || !variants.insert(variant) {
            return Err(invalid(
                name,
                "schema module names produce colliding or reserved variants",
            ));
        }
        if !ids.insert(schema.id) {
            return Err(invalid(name, "schema ids must be unique"));
        }
        let elements = partition_tokens(&schema.ir.tokens);
        let header = elements
            .composites
            .iter()
            .find(|c| c[0].name == schema.ir.header_type)
            .ok_or_else(|| invalid(name, "schema must declare a message header"))?;
        let members = parse_composite_members(header);
        let schema_member = members
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case("schemaId"));
        if !schema_member.is_some_and(|m| {
            matches!(
                m.member_type,
                MemberType::Primitive {
                    presence: Presence::Required,
                    ..
                }
            )
        }) {
            return Err(invalid(
                name,
                "schemaId must be encoded in the header, not constant",
            ));
        }
        for identity in ["schemaid", "templateid", "blocklength", "version"] {
            if members
                .iter()
                .filter(|m| m.name.to_lowercase().contains(identity))
                .count()
                != 1
            {
                return Err(invalid(
                    name,
                    "message header identity members must be unambiguous",
                ));
            }
        }
        let actual = canonical_token_fingerprint(header, schema.ir.byte_order);
        if let Some(expected) = &fingerprint {
            if expected != &actual {
                return Err(invalid(
                    name,
                    "schema headers must have identical layouts and byte order",
                ));
            }
        } else {
            fingerprint = Some(actual);
        }
    }
    Ok(())
}

fn types(modules: &[syn::Ident], variants: &[syn::Ident]) -> TokenStream {
    quote! {
        /// A decoded frame from any configured schema, or an unknown schema.
        #[non_exhaustive]
        pub enum AnySchemaMessage<'a> {
            #(#[doc = concat!("Messages from the ", stringify!(#modules), " schema.")]
              #variants(
                  super::#modules::AnyMessage<'a>,
                  /// Complete transport frame, including fields unknown to this codec.
                  &'a [u8],
              ),)*
            /// A complete header naming a schema outside the configured set.
            Other {
                /// Wire schema id.
                schema_id: u16,
                /// Wire template id.
                template_id: u16,
                /// Header and body, bounded by the caller's input slice.
                frame: &'a [u8],
            },
        }

        /// A header or schema decoder rejected the supplied frame.
        #[derive(Debug)]
        #[non_exhaustive]
        pub enum SchemaDecodeError {
            /// Fewer bytes remain than the common header requires.
            BufferTooShort {
                /// Required header bytes.
                needed: usize,
                /// Bytes remaining at the supplied offset.
                available: usize,
            },
            /// Header identity cannot be represented by an SBE schema/template id.
            InvalidHeader,
            #(#[doc = concat!("The ", stringify!(#modules), " schema rejected the frame.")]
              #variants(super::#modules::sbe_rt::DecodeError),)*
        }

        impl core::fmt::Display for SchemaDecodeError {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    Self::BufferTooShort { needed, available } =>
                        write!(f, "message header needs {needed} bytes, {available} available"),
                    Self::InvalidHeader => f.write_str("invalid message header identity"),
                    #(Self::#variants(err) => write!(f, "{}: {err}", stringify!(#modules)),)*
                }
            }
        }

        impl core::error::Error for SchemaDecodeError {
            fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
                match self {
                    #(Self::#variants(err) => Some(err),)*
                    Self::BufferTooShort { .. } | Self::InvalidHeader => None,
                }
            }
        }

        impl core::fmt::Debug for AnySchemaMessage<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    #(Self::#variants(..) => f.write_str(concat!(stringify!(#variants), "(..)")),)*
                    Self::Other { schema_id, template_id, .. } => f.debug_struct("Other")
                        .field("schema_id", schema_id).field("template_id", template_id).finish(),
                }
            }
        }
    }
}

fn methods(
    schemas: &[(&Schema, &str)],
    modules: &[syn::Ident],
    variants: &[syn::Ident],
) -> Result<TokenStream, GenerateError> {
    let owner = &modules[0];
    let schema = schemas[0].0;
    let header = format_ident!("{}", to_pascal_case(&schema.ir.header_type));
    let elements = partition_tokens(&schema.ir.tokens);
    let tokens = elements
        .composites
        .iter()
        .find(|c| c[0].name == schema.ir.header_type)
        .ok_or_else(|| invalid(&schema.ir.header_type, "missing message header"))?;
    let size = tokens[0]
        .encoding
        .offset
        .ok_or_else(|| invalid(&schema.ir.header_type, "missing message header size"))?;
    let members = parse_composite_members(tokens);
    let getter = |name: &str| {
        members
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
            .map(|m| format_ident!("{}", to_snake_case(&m.name)))
            .ok_or_else(|| invalid(name, "missing header identity"))
    };
    let schema_id = getter("schemaId")?;
    let template_id = getter("templateId")?;
    let ids: Vec<_> = schemas.iter().map(|(schema, _)| schema.id).collect();
    Ok(quote! {
        impl<'a> AnySchemaMessage<'a> {
            /// Decode a single frame whose header starts at offset.
            ///
            /// Bound the input slice to one transport frame: an unknown schema
            /// or template retains every byte from offset to the end.
            ///
            /// # Errors
            ///
            /// A short or invalid header, or rejection by a known schema.
            #[inline]
            pub fn decode(buf: &'a [u8], offset: usize) -> Result<Self, SchemaDecodeError> {
                let frame = buf.get(offset..).unwrap_or(&[]);
                let needed = #size;
                if frame.len() < needed {
                    return Err(SchemaDecodeError::BufferTooShort { needed, available: frame.len() });
                }
                let mut bytes = [0u8; #size];
                bytes.copy_from_slice(&frame[..needed]);
                let header = super::#owner::#header(bytes);
                let schema_id = u16::try_from(header.#schema_id() as u64)
                    .map_err(|_| SchemaDecodeError::InvalidHeader)?;
                let template_id = u16::try_from(header.#template_id() as u64)
                    .map_err(|_| SchemaDecodeError::InvalidHeader)?;
                match schema_id {
                    #(#ids => super::#modules::AnyMessage::decode_frame(buf, offset, frame.len())
                        .map(|decoded| Self::#variants(decoded.message, frame))
                        .map_err(SchemaDecodeError::#variants),)*
                    schema_id => Ok(Self::Other { schema_id, template_id, frame }),
                }
            }

            /// The wire schema id.
            #[inline]
            #[must_use]
            pub fn schema_id(&self) -> u16 {
                match self {
                    #(Self::#variants(..) => #ids,)*
                    Self::Other { schema_id, .. } => *schema_id,
                }
            }

            /// Borrow the complete header and body for persistence or forwarding.
            ///
            /// The transport frame is retained even when its acting version has
            /// fields unknown to this codec. Variable fields remain checked when
            /// the typed decoder consumes them.
            #[inline]
            #[must_use]
            pub fn as_bytes(&self) -> &'a [u8] {
                match self {
                    #(Self::#variants(_, frame) => frame,)*
                    Self::Other { frame, .. } => frame,
                }
            }
        }
    })
}

pub(super) fn generate(
    schemas: &[(&Schema, &str)],
    module_name: &str,
) -> Result<GeneratedModule, GenerateError> {
    validate(schemas, module_name)?;
    let modules: Vec<_> = schemas
        .iter()
        .map(|(_, name)| format_ident!("{name}"))
        .collect();
    let variants: Vec<_> = schemas
        .iter()
        .map(|(_, name)| format_ident!("{}", to_pascal_case(name)))
        .collect();
    let types = types(&modules, &variants);
    let methods = methods(schemas, &modules, &variants)?;
    let tokens = quote! { #types #methods };
    let file = syn::parse2(tokens).map_err(|err| invalid(module_name, &err.to_string()))?;
    Ok(GeneratedModule {
        path: format!("{module_name}.rs"),
        source: prettyplease::unparse(&file),
    })
}
