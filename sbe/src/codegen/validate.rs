//! Opt-in `validate()` codegen ([`crate::GenerationConfig::with_validate`]).
//!
//! Each message decoder, group entry decoder and `AnyMessage` gets one extra
//! `impl` block, so no existing item changes and `try_decode` stays the
//! bounds-only hot path. The checks read the wire through `validate_read`,
//! independent of accessor shape (conversions, domain types,
//! `null_as_option`):
//!
//! | Wire value | Check |
//! |---|---|
//! | integer / char scalar, integer array element | declared `minValue` / `maxValue` only; an optional field's null passes |
//! | enum | a `validValue` or the enum's null |
//! | set | no bit outside the declared choices |
//! | fixed char array, float, var-data | none |

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};

use crate::ir::{ByteOrder, Presence, PrimitiveType, Signal, Token};
use crate::structured_ir::{
    FieldType, MessageField, MessageGroup, MessageStructure, SchemaElements,
    parse_composite_members, rust_type,
};

use super::runtime::{find_matching_end, to_pascal_case, to_snake_case};

/// The generated method's name.
const METHOD: &str = "validate";

/// Schema path of the first message member whose decoder accessor would be
/// named `validate` and so collide with the generated method.
pub(crate) fn name_clash(msg: &MessageStructure) -> Option<String> {
    let clash = |name: &str| to_snake_case(name) == METHOD;
    let tails = |groups: &[MessageGroup], var_data: &[crate::structured_ir::MessageVarData]| {
        groups
            .iter()
            .map(|g| g.name.clone())
            .chain(var_data.iter().map(|v| v.name.clone()))
            .collect::<Vec<_>>()
    };
    fn walk(
        scope: &str,
        fields: &[MessageField],
        tail_names: Vec<String>,
        groups: &[MessageGroup],
        clash: &dyn Fn(&str) -> bool,
    ) -> Option<String> {
        if let Some(f) = fields.iter().find(|f| clash(&f.name)) {
            return Some(format!("{scope}.{}", f.name));
        }
        if let Some(t) = tail_names.iter().find(|t| clash(t)) {
            return Some(format!("{scope}.{t}"));
        }
        groups.iter().find_map(|g| {
            let names = g
                .groups
                .iter()
                .map(|n| n.name.clone())
                .chain(g.var_data.iter().map(|v| v.name.clone()))
                .collect();
            walk(
                &format!("{scope}.{}", g.name),
                &g.fields,
                names,
                &g.groups,
                clash,
            )
        })
    }
    walk(
        &msg.name,
        &msg.fields,
        tails(&msg.groups, &msg.var_data),
        &msg.groups,
        &clash,
    )
}

/// Bounds-checked read used only by `validate()`: a short buffer is a
/// `ValidateError::Decode`, never a panic.
pub(crate) fn read_helper_tokens() -> TokenStream {
    quote! {
        /// Read `N` bytes at `offset` for `validate()`.
        #[inline]
        fn validate_read<const N: usize>(
            buf: &[u8],
            offset: usize,
            field: &'static str,
        ) -> Result<[u8; N], sbe_rt::ValidateError> {
            match offset.checked_add(N).and_then(|end| buf.get(offset..end)) {
                Some(bytes) => {
                    let mut out = [0u8; N];
                    out.copy_from_slice(bytes);
                    Ok(out)
                }
                None => {
                    core::hint::cold_path();
                    Err(sbe_rt::ValidateError::Decode(sbe_rt::DecodeError::BufferTooShort {
                        field,
                        needed: N,
                        available: buf.len().saturating_sub(offset),
                    }))
                }
            }
        }
    }
}

/// `validate()` for a message decoder and, recursively, its group entries.
pub(crate) fn message_validate_tokens(
    msg: &MessageStructure,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    multi_message: bool,
) -> TokenStream {
    let name = to_pascal_case(&msg.name);
    let decoder = format_ident!("{name}Decoder");
    let mut out = TokenStream::new();

    let fixed = fields_checks(&msg.fields, elements, byte_order);
    let mut groups = TokenStream::new();
    for g in &msg.groups {
        let scoped = if multi_message {
            format!("{name}{}", to_pascal_case(&g.name))
        } else {
            to_pascal_case(&g.name)
        };
        groups.extend(group_walk(g, &quote! { self }));
        out.extend(entry_validate_tokens(g, &scoped, elements, byte_order));
    }
    // A tailed message proves every group and var-data header first, so a
    // truncated tail is reported even when no group is checked.
    let tails = if msg.has_tails() {
        quote! { self.encoded_length()?; }
    } else {
        TokenStream::new()
    };
    let doc = doc(&format!(
        "Check this `{}` frame against the schema beyond the bounds that \
         `try_decode` proves: declared value ranges, enum discriminants and \
         set bits, in the fixed block and every group entry. Not a hot-path \
         call; it reads every field.",
        msg.name
    ));
    out.extend(quote! {
        impl<'a> #decoder<'a> {
            #doc
            pub fn validate(&self) -> Result<(), sbe_rt::ValidateError> {
                #tails
                #fixed
                #groups
                Ok(())
            }
        }
    });
    out
}

fn entry_validate_tokens(
    g: &MessageGroup,
    scoped: &str,
    elements: &SchemaElements,
    byte_order: ByteOrder,
) -> TokenStream {
    let entry = format_ident!("{scoped}EntryDecoder");
    let fixed = fields_checks(&g.fields, elements, byte_order);
    let mut nested = TokenStream::new();
    let mut out = TokenStream::new();
    for ng in &g.groups {
        let nested_scoped = format!("{scoped}{}", to_pascal_case(&ng.name));
        nested.extend(group_walk(ng, &quote! { self }));
        out.extend(entry_validate_tokens(
            ng,
            &nested_scoped,
            elements,
            byte_order,
        ));
    }
    let doc = doc(&format!(
        "Check this `{}` entry: declared value ranges, enum discriminants and \
         set bits, then every nested group entry.",
        g.name
    ));
    out.extend(quote! {
        impl<'a> #entry<'a> {
            #doc
            pub fn validate(&self) -> Result<(), sbe_rt::ValidateError> {
                #fixed
                #nested
                Ok(())
            }
        }
    });
    out
}

/// Iterate a group through its `&self` accessor and validate each entry.
fn group_walk(g: &MessageGroup, owner: &TokenStream) -> TokenStream {
    let accessor = syn::Ident::new(&to_snake_case(&g.name), Span::call_site());
    let each = if g.has_fixed_stride() {
        quote! {
            for entry in #owner.#accessor()? {
                entry.validate()?;
            }
        }
    } else {
        quote! {
            for entry in #owner.#accessor()? {
                entry?.validate()?;
            }
        }
    };
    if g.since_version > 0 {
        let since = lit(g.since_version);
        quote! {
            if self.acting_version >= #since {
                #each
            }
        }
    } else {
        each
    }
}

/// Checks for every non-constant fixed field, each gated on the field being
/// present at the acting version and inside the acting block.
fn fields_checks(
    fields: &[MessageField],
    elements: &SchemaElements,
    byte_order: ByteOrder,
) -> TokenStream {
    let mut out = TokenStream::new();
    for f in fields {
        if f.presence == Presence::Constant {
            continue;
        }
        let at = f.offset;
        let checks = match &f.field_type {
            FieldType::Primitive(prim, length) => primitive_checks(
                &f.path,
                *prim,
                *length,
                at,
                &Bounds {
                    min: f.min_value,
                    max: f.max_value,
                    min_declared: f.min_value_declared,
                    max_declared: f.max_value_declared,
                    null: (f.presence == Presence::Optional)
                        .then_some(f.null_value)
                        .flatten(),
                },
                byte_order,
            ),
            FieldType::Enum {
                name,
                encoding_type,
            } => enum_check(&f.path, name, *encoding_type, at, byte_order),
            FieldType::Set {
                name,
                encoding_type,
            } => set_check(&f.path, name, *encoding_type, at, elements, byte_order),
            FieldType::Composite { name, .. } => {
                composite_checks(&f.path, name, at, elements, byte_order)
            }
        };
        if checks.is_empty() {
            continue;
        }
        let end = lit(at + f.field_type.size());
        let present = if f.since_version > 0 {
            let since = lit(f.since_version);
            quote! { self.acting_version >= #since && #end <= self.acting_block_length }
        } else {
            quote! { #end <= self.acting_block_length }
        };
        out.extend(quote! {
            if #present {
                #checks
            }
        });
    }
    out
}

/// Value range and null of one primitive (scalar or array element).
struct Bounds {
    min: Option<u64>,
    max: Option<u64>,
    min_declared: bool,
    max_declared: bool,
    /// The null value when the field is optional: it always passes.
    null: Option<u64>,
}

/// Wire bit pattern (as stored in the IR) to its signed value.
pub(super) fn to_i128(prim: PrimitiveType, v: u64) -> i128 {
    match prim {
        PrimitiveType::Int8 => i128::from(v as i8),
        PrimitiveType::Int16 => i128::from(v as i16),
        PrimitiveType::Int32 => i128::from(v as i32),
        PrimitiveType::Int64 => i128::from(v as i64),
        PrimitiveType::Char | PrimitiveType::UInt8 => i128::from(v as u8),
        PrimitiveType::UInt16 => i128::from(v as u16),
        PrimitiveType::UInt32 => i128::from(v as u32),
        PrimitiveType::UInt64 | PrimitiveType::Float | PrimitiveType::Double => i128::from(v),
    }
}

/// The Rust type's full range; a declared bound equal to it checks nothing.
pub(super) fn type_range(prim: PrimitiveType) -> (i128, i128) {
    match prim {
        PrimitiveType::Int8 => (i128::from(i8::MIN), i128::from(i8::MAX)),
        PrimitiveType::Int16 => (i128::from(i16::MIN), i128::from(i16::MAX)),
        PrimitiveType::Int32 => (i128::from(i32::MIN), i128::from(i32::MAX)),
        PrimitiveType::Int64 => (i128::from(i64::MIN), i128::from(i64::MAX)),
        PrimitiveType::Char | PrimitiveType::UInt8 => (0, i128::from(u8::MAX)),
        PrimitiveType::UInt16 => (0, i128::from(u16::MAX)),
        PrimitiveType::UInt32 => (0, i128::from(u32::MAX)),
        PrimitiveType::UInt64 | PrimitiveType::Float | PrimitiveType::Double => {
            (0, i128::from(u64::MAX))
        }
    }
}

/// Range check for a scalar, or for each element of a non-char array.
/// Empty when nothing is declared, the bound is the type's own limit, or the
/// type is a float.
fn primitive_checks(
    path: &str,
    prim: PrimitiveType,
    length: Option<usize>,
    offset: usize,
    bounds: &Bounds,
    byte_order: ByteOrder,
) -> TokenStream {
    if matches!(prim, PrimitiveType::Float | PrimitiveType::Double) {
        return TokenStream::new();
    }
    if prim == PrimitiveType::Char && length.is_some() {
        return TokenStream::new();
    }
    let (Some(min), Some(max)) = (bounds.min, bounds.max) else {
        return TokenStream::new();
    };
    let (type_min, type_max) = type_range(prim);
    let (min, max) = (to_i128(prim, min), to_i128(prim, max));
    let check_min = bounds.min_declared && min > type_min;
    let check_max = bounds.max_declared && max < type_max;
    if !check_min && !check_max {
        return TokenStream::new();
    }
    let span = Span::call_site();
    let min_lit = syn::LitInt::new(&min.to_string(), span);
    let max_lit = syn::LitInt::new(&max.to_string(), span);
    let out_of_range = match (check_min, check_max) {
        (true, true) => quote! { v < #min_lit || v > #max_lit },
        (true, false) => quote! { v < #min_lit },
        _ => quote! { v > #max_lit },
    };
    let out_of_range = match bounds.null {
        Some(null) => {
            let null_lit = syn::LitInt::new(&to_i128(prim, null).to_string(), span);
            quote! { v != #null_lit && (#out_of_range) }
        }
        None => out_of_range,
    };
    let field = syn::LitStr::new(path, span);
    let size = prim.size();
    let read = |at: TokenStream| read_value(prim, size, at, &field, byte_order);
    let check = |value: TokenStream| {
        quote! {
            let v = #value as i128;
            if #out_of_range {
                core::hint::cold_path();
                return Err(sbe_rt::ValidateError::ValueOutOfRange {
                    field: #field,
                    min: #min_lit,
                    max: #max_lit,
                    actual: v,
                });
            }
        }
    };
    let off = lit(offset);
    match length {
        None => check(read(quote! { self.offset + #off })),
        Some(0) => TokenStream::new(),
        Some(n) => {
            let n = lit(n);
            let size = lit(size);
            let body = check(read(quote! { self.offset + #off + i * #size }));
            quote! {
                for i in 0..#n {
                    #body
                }
            }
        }
    }
}

/// `<rt>::from_{le,be}_bytes(validate_read::<N>(self.buf, at, field)?)`.
fn read_value(
    prim: PrimitiveType,
    size: usize,
    at: TokenStream,
    field: &syn::LitStr,
    byte_order: ByteOrder,
) -> TokenStream {
    let rt: syn::Type = syn::parse_str(rust_type(prim)).unwrap_or_else(|_| syn::parse_quote!(u8));
    let from = match byte_order {
        ByteOrder::LittleEndian => format_ident!("from_le_bytes"),
        ByteOrder::BigEndian => format_ident!("from_be_bytes"),
    };
    let size = lit(size);
    quote! { #rt::#from(validate_read::<#size>(self.buf, #at, #field)?) }
}

fn enum_check(
    path: &str,
    enum_name: &str,
    encoding: PrimitiveType,
    offset: usize,
    byte_order: ByteOrder,
) -> TokenStream {
    let field = syn::LitStr::new(path, Span::call_site());
    let ty = format_ident!("{}", to_pascal_case(enum_name));
    let rt: syn::Type =
        syn::parse_str(rust_type(encoding)).unwrap_or_else(|_| syn::parse_quote!(u8));
    let off = lit(offset);
    let raw = read_value(
        encoding,
        encoding.size(),
        quote! { self.offset + #off },
        &field,
        byte_order,
    );
    // `from_raw` maps every undeclared discriminant to `NullVal`, so an
    // undeclared value is a `NullVal` whose raw differs from the null.
    quote! {
        let raw = #raw;
        if matches!(#ty::from_raw(raw), #ty::NullVal) && raw != #ty::NullVal as #rt {
            core::hint::cold_path();
            return Err(sbe_rt::ValidateError::UnknownEnum {
                field: #field,
                discriminant: raw as u64,
            });
        }
    }
}

fn set_check(
    path: &str,
    set_name: &str,
    encoding: PrimitiveType,
    offset: usize,
    elements: &SchemaElements,
    byte_order: ByteOrder,
) -> TokenStream {
    let Some(set) = elements
        .sets
        .iter()
        .find(|tokens| tokens.first().is_some_and(|t| t.name == set_name))
    else {
        return TokenStream::new();
    };
    let declared = set
        .iter()
        .filter(|t| t.signal == Signal::Encoding)
        .filter_map(|t| t.encoding.constant_value.as_deref()?.parse::<u32>().ok())
        .filter(|&bit| bit < 64)
        .fold(0u64, |mask, bit| mask | (1u64 << bit));
    let width = encoding.size() * 8;
    let all = if width >= 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    };
    let undeclared = all & !declared;
    if undeclared == 0 {
        return TokenStream::new();
    }
    let field = syn::LitStr::new(path, Span::call_site());
    let rt: syn::Type =
        syn::parse_str(rust_type(encoding)).unwrap_or_else(|_| syn::parse_quote!(u8));
    let mask = syn::LitInt::new(&format!("{undeclared}u64"), Span::call_site());
    let off = lit(offset);
    let raw = read_value(
        encoding,
        encoding.size(),
        quote! { self.offset + #off },
        &field,
        byte_order,
    );
    quote! {
        let bits = #raw & (#mask as #rt);
        if bits != 0 {
            core::hint::cold_path();
            return Err(sbe_rt::ValidateError::UnknownSetBits {
                field: #field,
                bits: bits as u64,
            });
        }
    }
}

/// Checks for every member of composite `name` at `base` (relative to the
/// owner's body), nested composites included.
fn composite_checks(
    path: &str,
    name: &str,
    base: usize,
    elements: &SchemaElements,
    byte_order: ByteOrder,
) -> TokenStream {
    let Some(tokens) = elements.composites.iter().find(|c| c[0].name == name) else {
        return TokenStream::new();
    };
    let mut out = TokenStream::new();
    let last = tokens.len().saturating_sub(1);
    let mut i = 1;
    while i < last {
        if tokens[i].signal != Signal::BeginField {
            i += 1;
            continue;
        }
        let member: &Token = &tokens[i];
        let end = find_matching_end(tokens, i, Signal::BeginField, Signal::EndField);
        let at = base + member.encoding.offset.unwrap_or(0);
        let member_path = format!("{path}.{}", member.name);
        let next = tokens.get(i + 1).filter(|_| i + 1 < end);
        let checks = if member.encoding.presence == Presence::Constant {
            TokenStream::new()
        } else {
            match next.map(|t| t.signal) {
                Some(Signal::BeginComposite) => next.map_or_else(TokenStream::new, |t| {
                    composite_checks(&member_path, &t.name, at, elements, byte_order)
                }),
                Some(Signal::BeginEnum) => next.map_or_else(TokenStream::new, |t| {
                    enum_check(
                        &member_path,
                        &t.name,
                        t.encoding.primitive_type.unwrap_or(PrimitiveType::UInt8),
                        at,
                        byte_order,
                    )
                }),
                Some(Signal::BeginSet) => next.map_or_else(TokenStream::new, |t| {
                    set_check(
                        &member_path,
                        &t.name,
                        t.encoding.primitive_type.unwrap_or(PrimitiveType::UInt8),
                        at,
                        elements,
                        byte_order,
                    )
                }),
                _ => member
                    .encoding
                    .primitive_type
                    .map_or_else(TokenStream::new, |prim| {
                        primitive_checks(
                            &member_path,
                            prim,
                            member.encoding.length,
                            at,
                            &Bounds {
                                min: member.encoding.min_value,
                                max: member.encoding.max_value,
                                min_declared: member.encoding.min_value_declared,
                                max_declared: member.encoding.max_value_declared,
                                null: (member.encoding.presence == Presence::Optional)
                                    .then_some(member.encoding.null_value)
                                    .flatten(),
                            },
                            byte_order,
                        )
                    }),
            }
        };
        if !checks.is_empty() {
            if member.encoding.since_version > 0 {
                let since = lit(member.encoding.since_version);
                out.extend(quote! {
                    if self.acting_version >= #since {
                        #checks
                    }
                });
            } else {
                out.extend(quote! { { #checks } });
            }
        }
        i = end + 1;
    }
    out
}

/// `AnyMessage::validate`: each known variant's `validate()`;
/// `Unknown` is `ValidateError::UnknownTemplate`.
pub(crate) fn any_message_validate_tokens(
    messages: &[MessageStructure],
    elements: &SchemaElements,
    header_type: &str,
) -> TokenStream {
    let mut arms = TokenStream::new();
    for m in messages {
        let variant = format_ident!("{}", to_pascal_case(&m.name));
        arms.extend(quote! { Self::#variant(d) => d.validate(), });
    }
    let template_id = elements
        .composites
        .iter()
        .find(|c| c[0].name == header_type)
        .and_then(|c| {
            parse_composite_members(c)
                .into_iter()
                .find(|m| m.name.to_lowercase().contains("templateid"))
        })
        .map_or_else(|| "template_id".to_string(), |m| to_snake_case(&m.name));
    let template_id = syn::Ident::new(&template_id, Span::call_site());
    quote! {
        impl<'a> AnyMessage<'a> {
            /// [`Self`]'s message `validate()`. An `Unknown` frame has no
            /// schema message to check against and is
            /// `ValidateError::UnknownTemplate`.
            pub fn validate(&self) -> Result<(), sbe_rt::ValidateError> {
                match self {
                    #arms
                    Self::Unknown { header, .. } => Err(sbe_rt::ValidateError::UnknownTemplate {
                        template_id: sbe_rt::checked_header_u16(
                            "templateId",
                            header.#template_id() as u64,
                        )?,
                    }),
                }
            }
        }
    }
}

fn lit(v: impl std::fmt::Display) -> syn::LitInt {
    syn::LitInt::new(&v.to_string(), Span::call_site())
}

fn doc(text: &str) -> TokenStream {
    super::runtime::doc_lines_tokens(text)
}
