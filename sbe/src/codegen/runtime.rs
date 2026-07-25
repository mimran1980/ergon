//! Inline sbe_rt emission, name helpers, composite/enum/set generation,
//! preamble, AnyMessage, FrameCursor, schema hash, and field metadata.

use crate::ir::{ByteOrder, Ir, Presence, PrimitiveType, Signal, Token};
use crate::structured_ir::*;
use proc_macro2::TokenStream;
use quote::format_ident;
use sha2::{Digest, Sha256};
use std::fmt::Write;

pub(crate) fn generate_sbe_rt_src() -> String {
    let module = quote::quote! {
        pub mod sbe_rt {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum DecodeError {
                BufferTooShort { field: &'static str, needed: usize, available: usize },
                WrongSchema { expected: u16, actual: u16, expected_name: &'static str },
                UnknownTemplateLength { template_id: u16 },
                InvalidVarDataLength { field: &'static str, length: u32, max_length: u32 },
                /// Field/group/data was added in a schema version later than the wire message.
                FieldNotInVersion { field: &'static str, wire_version: u16, since_version: u16 },
                InvalidUtf8 { field: &'static str, error: core::str::Utf8Error },
                InvalidAscii { field: &'static str },
            }

            impl core::fmt::Display for DecodeError {
                #[cold]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Self::BufferTooShort { field, needed, available } => write!(f, "field '{}': needed {} bytes, {} available", field, needed, available),
                        Self::WrongSchema { expected, actual, expected_name } => write!(f, "wrong schema: expected id {} ({}), got id {}", expected, expected_name, actual),
                        Self::UnknownTemplateLength { template_id } => write!(f, "unknown template id {}: SBE messages do not carry length. Use decode_frame() with an external frame length.", template_id),
                        Self::InvalidVarDataLength { field, length, max_length } => write!(f, "var data field '{}: length {} exceeds max {}", field, length, max_length),
                        Self::FieldNotInVersion { field, wire_version, since_version } => write!(f, "field '{}' not in wire version {} (added in version {})", field, wire_version, since_version),
                        Self::InvalidUtf8 { field, error } => write!(f, "field '{}': invalid UTF-8: {}", field, error),
                        Self::InvalidAscii { field } => write!(f, "field '{}': invalid ASCII", field),
                    }
                }
            }

            impl core::error::Error for DecodeError {}

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum EncodeError {
                BufferTooShort { needed: usize, available: usize },
                VarDataTooLong { field: &'static str, max_length: usize, actual: usize },
                GroupFull { declared: u32, attempted: u32 },
                /// Known-size group closure returned without adding enough entries.
                GroupCountMismatch { declared: u32, actual: u32 },
                /// Unknown-size group entry count does not fit in `numInGroup`.
                GroupCountOverflow { maximum: u32, actual: u32 },
                /// Checked arithmetic overflow in encoded length computation.
                EncodedLengthOverflow,
                Decode(DecodeError),
            }

            impl core::fmt::Display for EncodeError {
                #[cold]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Self::BufferTooShort { needed, available } => write!(f, "buffer too short: needed {}, available {}", needed, available),
                        Self::VarDataTooLong { field, max_length, actual } => write!(f, "var data too long for field {}: max {}, actual {}", field, max_length, actual),
                        Self::GroupFull { declared, attempted } => write!(f, "group full: declared count {}, attempted to write {}", declared, attempted),
                        Self::GroupCountMismatch { declared, actual } => write!(f, "group count mismatch: declared {declared}, wrote {actual}"),
                        Self::GroupCountOverflow { maximum, actual } => write!(f, "group count overflow: max {maximum}, actual {actual}"),
                        Self::EncodedLengthOverflow => write!(f, "encoded length computation overflowed"),
                        Self::Decode(e) => write!(f, "decode error: {e}"),
                    }
                }
            }

            impl core::error::Error for EncodeError {}

            impl From<DecodeError> for EncodeError {
                fn from(e: DecodeError) -> Self {
                    Self::Decode(e)
                }
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum VerifyError {
                HeaderTooShort,
                InvalidBlockLength { expected_min: usize, actual: usize },
                GroupDimOutOfBounds { field: &'static str, offset: usize },
                VarDataOutOfBounds { field: &'static str, offset: usize, length: u32 },
                MessageTooShort { needed: usize, available: usize },
            }

            impl core::fmt::Display for VerifyError {
                #[cold]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Self::HeaderTooShort => write!(f, "buffer too short to contain message header"),
                        Self::InvalidBlockLength { expected_min, actual } => write!(f, "invalid block length: expected at least {}, actual {}", expected_min, actual),
                        Self::GroupDimOutOfBounds { field, offset } => write!(f, "group dimension header for '{}' out of bounds at offset {}", field, offset),
                        Self::VarDataOutOfBounds { field, offset, length } => write!(f, "var-data for '{}' out of bounds at offset {} with length {}", field, offset, length),
                        Self::MessageTooShort { needed, available } => write!(f, "message too short: needed {} bytes, {} available", needed, available),
                    }
                }
            }

            impl core::error::Error for VerifyError {}

            #[diagnostic::on_unimplemented(
                message = "`{Self}` is not a generated SBE message type",
                note = "SbeMessage is a sealed trait — only types generated by `ergo_sbe::Generator` can implement it. Import the generated module and use the provided decoder/encoder types directly."
            )]
            pub trait SbeMessage {
                const TEMPLATE_ID: u16;
                const BLOCK_LENGTH: usize;
                const SCHEMA_ID: u16;
                const SCHEMA_VERSION: u16;
            }

            pub mod private {
                pub trait Sealed {}
            }

            /// Return type for group closures (`add`, `bids`, …).
            /// Closures return `Result<(), EncodeError>`; `?` just works.
            pub type GroupResult = Result<(), EncodeError>;

            /// Conversion trait for group-closure return values.
            /// Implemented for `()` and `Result<(), EncodeError>` so
            /// closures may use either return type.
            pub trait IntoGroupResult {
                fn into_group_result(self) -> GroupResult;
            }

            impl IntoGroupResult for () {
                fn into_group_result(self) -> GroupResult {
                    Ok(())
                }
            }

            impl IntoGroupResult for GroupResult {
                fn into_group_result(self) -> GroupResult {
                    self
                }
            }
        }
    };

    // Format the generated module through prettyplease for canonical output
    syn::parse_str::<syn::File>(&module.to_string())
        .map(|file| prettyplease::unparse(&file))
        .expect("generated SBE runtime must be valid Rust syntax")
}

pub(crate) fn to_pascal_case(s: &str) -> String {
    let mut res = String::new();
    let mut capitalize_next = true;
    let mut prev_is_lower = false;
    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            capitalize_next = true;
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                capitalize_next = true;
            }
            if capitalize_next {
                res.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                res.push(c);
            }
            prev_is_lower = false;
        } else {
            if capitalize_next {
                res.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                res.push(c);
            }
            prev_is_lower = true;
        }
    }
    res
}

pub(crate) fn to_snake_case(s: &str) -> String {
    let mut res = String::new();
    let mut prev_is_lower = false;
    let mut _prev_is_upper = false;
    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            res.push('_');
            prev_is_lower = false;
            _prev_is_upper = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                res.push('_');
            }
            res.extend(c.to_lowercase());
            prev_is_lower = false;
            _prev_is_upper = true;
        } else {
            res.push(c);
            prev_is_lower = true;
            _prev_is_upper = false;
        }
    }
    let mut clean = String::new();
    for c in res.chars() {
        if c == '_' && clean.ends_with('_') {
            continue;
        }
        clean.push(c);
    }
    clean
}

pub(crate) fn to_upper_snake_case(s: &str) -> String {
    to_snake_case(s).to_uppercase()
}

pub(crate) fn constant_value_expr(prim: PrimitiveType, val: &str) -> String {
    match prim {
        // parser validates single-char constants, generator trusts it; add a debug_assert! if parser ever skips validation
        PrimitiveType::Char => format!("b'{}'", val),
        PrimitiveType::Float => {
            format!("{}f32", val)
        }
        PrimitiveType::Double => {
            format!("{}f64", val)
        }
        _ => {
            format!("{}", val)
        }
    }
}

/// Format a `u64` stored value as a valid Rust literal expression for the given type.
pub(crate) fn field_const_value_expr(val: u64, prim: PrimitiveType) -> String {
    match prim {
        PrimitiveType::Char | PrimitiveType::UInt8 => format!("{val}_u8"),
        PrimitiveType::UInt16 => format!("{val}_u16"),
        PrimitiveType::UInt32 => format!("{val}_u32"),
        PrimitiveType::UInt64 => format!("{val}_u64"),
        PrimitiveType::Int8 => format!("{}_i8", val as i8),
        PrimitiveType::Int16 => format!("{}_i16", val as i16),
        PrimitiveType::Int32 => format!("{}_i32", val as i32),
        PrimitiveType::Int64 => format!("{}_i64", val as i64),
        PrimitiveType::Float => format!("f32::from_bits({}u32)", val as u32),
        PrimitiveType::Double => format!("f64::from_bits({val})"),
    }
}

/// Emit `*_NULL`, `*_MIN`, `*_MAX` compile-time constants for a message field.
pub(crate) fn emit_field_consts(f: &MessageField) -> proc_macro2::TokenStream {
    let upper_name = to_upper_snake_case(&f.name);
    let mut _any = false;
    let mut tokens = proc_macro2::TokenStream::new();
    match &f.field_type {
        FieldType::Primitive(prim, _) => {
            let r_type = rust_type(*prim);
            let r_type_ty: syn::Type = syn::parse_str(r_type).unwrap();
            if let Some(val) = f.null_value {
                let name_ident = syn::Ident::new(
                    &format!("{upper_name}_NULL"),
                    proc_macro2::Span::call_site(),
                );
                let expr = field_const_value_expr(val, *prim);
                let expr_parsed: syn::Expr = syn::parse_str(&expr).unwrap();
                tokens.extend(quote::quote! {
                    pub const #name_ident: #r_type_ty = #expr_parsed;
                });
                _any = true;
            }
            if let Some(val) = f.min_value {
                let name_ident =
                    syn::Ident::new(&format!("{upper_name}_MIN"), proc_macro2::Span::call_site());
                let expr = field_const_value_expr(val, *prim);
                let expr_parsed: syn::Expr = syn::parse_str(&expr).unwrap();
                tokens.extend(quote::quote! {
                    pub const #name_ident: #r_type_ty = #expr_parsed;
                });
                _any = true;
            }
            if let Some(val) = f.max_value {
                let name_ident =
                    syn::Ident::new(&format!("{upper_name}_MAX"), proc_macro2::Span::call_site());
                let expr = field_const_value_expr(val, *prim);
                let expr_parsed: syn::Expr = syn::parse_str(&expr).unwrap();
                tokens.extend(quote::quote! {
                    pub const #name_ident: #r_type_ty = #expr_parsed;
                });
                _any = true;
            }
        }
        FieldType::Enum {
            name,
            encoding_type: _,
        } => {
            let target_name = to_pascal_case(name);
            let name_ident = syn::Ident::new(
                &format!("{upper_name}_NULL"),
                proc_macro2::Span::call_site(),
            );
            let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
            tokens.extend(quote::quote! {
                pub const #name_ident: #target_ident = #target_ident::NullVal;
            });
            _any = true;
        }
        FieldType::Composite { .. } | FieldType::Set { .. } => {}
    }
    tokens
}

pub(crate) fn find_matching_end(
    tokens: &[Token],
    start: usize,
    begin: Signal,
    end: Signal,
) -> usize {
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

pub(crate) fn generate_enum(src: &mut String, tokens: &[Token]) {
    let raw_name = &tokens[0].name;
    let name = to_pascal_case(raw_name);
    let encoding_type = tokens[0]
        .encoding
        .primitive_type
        .unwrap_or(PrimitiveType::UInt8);
    let r_type = rust_type(encoding_type);
    let is_char = encoding_type == PrimitiveType::Char;

    let name_ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
    let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();

    // Collect encoding variants
    struct Variant {
        variant_ident: syn::Ident,
        disc: proc_macro2::TokenStream,
    }

    let variants: Vec<Variant> = tokens
        .iter()
        .filter(|t| t.signal == Signal::Encoding)
        .filter_map(|t| {
            let val = t.encoding.constant_value.as_ref()?;
            let variant_ident =
                syn::Ident::new(&to_pascal_case(&t.name), proc_macro2::Span::call_site());
            let disc: proc_macro2::TokenStream = if is_char {
                let byte = val.as_bytes().first().copied().unwrap_or(0);
                let lit = syn::LitByte::new(byte, proc_macro2::Span::call_site());
                quote::quote! { #lit }
            } else {
                let lit = val
                    .parse::<u64>()
                    .ok()
                    .map(|v| syn::LitInt::new(&v.to_string(), proc_macro2::Span::call_site()))
                    .or_else(|| {
                        val.parse::<i64>().ok().map(|v| {
                            syn::LitInt::new(&v.to_string(), proc_macro2::Span::call_site())
                        })
                    })
                    .unwrap_or_else(|| syn::LitInt::new(val, proc_macro2::Span::call_site()));
                quote::quote! { #lit }
            };
            Some(Variant {
                variant_ident,
                disc,
            })
        })
        .collect();

    let variant_names: Vec<_> = variants.iter().map(|v| &v.variant_ident).collect();
    let variant_discs: Vec<_> = variants.iter().map(|v| &v.disc).collect();

    // Build From<r_type> arms: disc => Self::Variant, _ => Self::NullVal
    let from_raw_arms: Vec<_> = variants
        .iter()
        .map(|v| {
            let disc = &v.disc;
            let vname = &v.variant_ident;
            quote::quote! { #disc => Self::#vname }
        })
        .collect();

    // Detect boolean enum type (name convention or semanticType="Boolean")
    let is_bool = tokens[0].name == "BooleanType"
        || tokens[0].encoding.semantic_type.as_deref() == Some("Boolean");

    // Find TRUE/FALSE variant idents for From<bool>
    let (false_ident, true_ident) = if is_bool {
        let f = variants
            .iter()
            .find(|v| v.disc.to_string() == "0")
            .map(|v| v.variant_ident.clone());
        let t = variants
            .iter()
            .find(|v| v.disc.to_string() == "1")
            .map(|v| v.variant_ident.clone());
        (f, t)
    } else {
        (None, None)
    };

    // From<bool> / From<Name> for bool impls for boolean types
    let from_bool_impl = if let (Some(ref fv), Some(ref tv)) = (false_ident, true_ident) {
        quote::quote! {
            impl From<bool> for #name_ident {
                #[inline]
                fn from(val: bool) -> Self {
                    if val { Self::#tv } else { Self::#fv }
                }
            }

            impl From<#name_ident> for bool {
                #[inline]
                fn from(val: #name_ident) -> bool {
                    val as #r_type_ty != 0
                }
            }
        }
    } else {
        quote::quote! {}
    };

    // Emit enum rustdoc from the type's XML description.
    if let Some(ref desc) = tokens[0].encoding.description {
        push_description_doc(src, desc);
    }

    let tokens = quote::quote! {
        #[repr(#r_type_ty)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub enum #name_ident {
            #(#variant_names = #variant_discs,)*
            /// Unknown enum value — the wire discriminant did not match any known variant.
            NullVal,
        }

        impl #name_ident {
            pub fn raw(self) -> #r_type_ty {
                self as #r_type_ty
            }

            pub const fn from_raw(val: #r_type_ty) -> Self {
                match val {
                    #(#from_raw_arms,)*
                    _ => Self::NullVal,
                }
            }
        }

        impl From<#name_ident> for #r_type_ty {
            #[inline]
            fn from(val: #name_ident) -> Self {
                val as #r_type_ty
            }
        }

        impl From<#r_type_ty> for #name_ident {
            #[inline]
            fn from(val: #r_type_ty) -> Self {
                Self::from_raw(val)
            }
        }

        #from_bool_impl
    };

    let formatted = syn::parse_str::<syn::File>(&tokens.to_string())
        .map(|file| prettyplease::unparse(&file))
        .unwrap_or_else(|_| tokens.to_string());
    src.push_str(&formatted);
    src.push('\n');
}

pub(crate) fn generate_set(src: &mut String, tokens: &[Token]) {
    let raw_name = &tokens[0].name;
    let name = to_pascal_case(raw_name);
    let encoding_type = tokens[0]
        .encoding
        .primitive_type
        .unwrap_or(PrimitiveType::UInt8);
    let r_type = rust_type(encoding_type);

    let name_ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
    let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();

    let bits: Vec<proc_macro2::TokenStream> = tokens
        .iter()
        .filter(|t| t.signal == Signal::Encoding)
        .filter_map(|t| {
            let val = t.encoding.constant_value.as_ref()?;
            let bit_index: u8 = val.parse().unwrap_or(0);
            let bit_name = syn::Ident::new(&to_snake_case(&t.name), proc_macro2::Span::call_site());
            let set_bit_name = quote::format_ident!("set_{}", to_snake_case(&t.name));
            let bit_lit = syn::LitInt::new(&bit_index.to_string(), proc_macro2::Span::call_site());
            Some(quote::quote! {
                pub const fn #bit_name(self) -> bool {
                    (self.0 & (1 << #bit_lit)) != 0
                }

                pub fn #set_bit_name(&mut self, val: bool) {
                    if val {
                        self.0 |= 1 << #bit_lit;
                    } else {
                        self.0 &= !(1 << #bit_lit);
                    }
                }
            })
        })
        .collect();

    // Emit enum doc from the type's XML description (DECISIONS.md §9).
    if let Some(ref desc) = tokens[0].encoding.description {
        push_description_doc(src, desc);
    }

    // Emit set doc from the type's XML description.
    if let Some(ref desc) = tokens[0].encoding.description {
        push_description_doc(src, desc);
    }

    let tokens = quote::quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[repr(transparent)]
        pub struct #name_ident(pub #r_type_ty);

        impl #name_ident {
            pub const fn raw(self) -> #r_type_ty {
                self.0
            }

            pub const fn default() -> Self {
                Self(0)
            }

            #(#bits)*
        }

        impl From<#r_type_ty> for #name_ident {
            #[inline]
            fn from(val: #r_type_ty) -> Self {
                Self(val)
            }
        }

        impl From<#name_ident> for #r_type_ty {
            #[inline]
            fn from(val: #name_ident) -> Self {
                val.0
            }
        }
    };

    let formatted = syn::parse_str::<syn::File>(&tokens.to_string())
        .map(|file| prettyplease::unparse(&file))
        .unwrap_or_else(|_| tokens.to_string());
    src.push_str(&formatted);
    src.push('\n');
}

pub(crate) fn generate_composite(src: &mut String, tokens: &[Token], byte_order: ByteOrder) {
    let raw_name = &tokens[0].name;
    let name = to_pascal_case(raw_name);
    let size = tokens[0].encoding.offset.unwrap_or(0);

    let members = parse_composite_members(tokens);

    let has_float = members.iter().any(|m| {
        matches!(
            &m.member_type,
            MemberType::Primitive {
                prim: PrimitiveType::Float | PrimitiveType::Double,
                ..
            }
        )
    });

    let name_ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
    let size_lit = syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());

    let derives = if has_float {
        quote::quote! { Clone, Copy, Debug, PartialEq, PartialOrd }
    } else {
        quote::quote! { Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash }
    };

    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };
    let from_method = syn::Ident::new(
        &format!("from_{order_suffix}_bytes"),
        proc_macro2::Span::call_site(),
    );
    let to_method = syn::Ident::new(
        &format!("to_{order_suffix}_bytes"),
        proc_macro2::Span::call_site(),
    );

    let mut getters = proc_macro2::TokenStream::new();
    let mut ctor_params = Vec::new();
    let mut ctor_body = proc_macro2::TokenStream::new();

    for m in &members {
        let field_name = to_snake_case(&m.name);
        let field_ident = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
        let offset_lit = syn::LitInt::new(&m.offset.to_string(), proc_macro2::Span::call_site());

        match &m.member_type {
            MemberType::Primitive {
                prim,
                length,
                presence,
                constant_value,
            } => {
                let r_type_str = rust_type(*prim);
                let r_type_ty: syn::Type = syn::parse_str(r_type_str).unwrap();
                let prim_size = prim.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                if *presence == Presence::Constant {
                    if let Some(val) = constant_value {
                        if *prim == PrimitiveType::Char && val.len() > 1 {
                            let val_lit = syn::LitStr::new(val, proc_macro2::Span::call_site());
                            getters.extend(quote::quote! {
                                #[inline]
                                pub const fn #field_ident(&self) -> &'static str {
                                    #val_lit
                                }
                            });
                        } else {
                            let expr_str = constant_value_expr(*prim, val);
                            let expr: syn::Expr = syn::parse_str(&expr_str).unwrap();
                            getters.extend(quote::quote! {
                                #[inline]
                                pub const fn #field_ident(&self) -> #r_type_ty {
                                    #expr
                                }
                            });
                        }
                    }
                    continue; // no ctor param for constants
                }

                if let Some(len) = length {
                    let len_lit =
                        syn::LitInt::new(&len.to_string(), proc_macro2::Span::call_site());
                    let array_ty: syn::Type =
                        syn::parse_str(&format!("[{}; {}]", r_type_str, len)).unwrap();
                    ctor_params.push(quote::quote! { #field_ident: #array_ty });

                    if *len > 0 {
                        getters.extend(quote::quote! {
                            #[inline]
                            pub fn #field_ident(&self) -> [#r_type_ty; #len_lit] {
                                let mut res = [0 as #r_type_ty; #len_lit];
                                let mut idx = 0;
                                while idx < #len_lit {
                                    let offset = #offset_lit + idx * #prim_size_lit;
                                    res[idx] = #r_type_ty::#from_method(
                                        read_bytes::<#prim_size_lit>(&self.0, offset)
                                    );
                                    idx += 1;
                                }
                                res
                            }
                        });

                        ctor_body.extend(quote::quote! {
                            let mut idx = 0;
                            while idx < #len_lit {
                                let val_bytes = #field_ident[idx].#to_method();
                                write_bytes::<#prim_size_lit>(&mut bytes, #offset_lit + idx * #prim_size_lit, &val_bytes);
                                idx += 1;
                            }
                        });
                    } else {
                        // zero-length array: return empty array immediately
                        let zero_ty: syn::Type =
                            syn::parse_str(&format!("[{}; 0]", r_type_str)).unwrap();
                        getters.extend(quote::quote! {
                            #[inline]
                            pub fn #field_ident(&self) -> #zero_ty {
                                []
                            }
                        });
                    }
                } else {
                    ctor_params.push(quote::quote! { #field_ident: #r_type_ty });

                    getters.extend(quote::quote! {
                        #[inline]
                        pub fn #field_ident(&self) -> #r_type_ty {
                            #r_type_ty::#from_method(read_bytes::<#prim_size_lit>(&self.0, #offset_lit))
                        }
                    });

                    ctor_body.extend(quote::quote! {
                        let val_bytes = #field_ident.#to_method();
                        write_bytes::<#prim_size_lit>(&mut bytes, #offset_lit, &val_bytes);
                    });
                }
            }
            MemberType::Composite {
                name: comp_name,
                size: comp_size,
            } => {
                let target_name = to_pascal_case(comp_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let comp_size_lit =
                    syn::LitInt::new(&comp_size.to_string(), proc_macro2::Span::call_site());

                ctor_params.push(quote::quote! { #field_ident: #target_ident });

                getters.extend(quote::quote! {
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        #target_ident(read_bytes::<#comp_size_lit>(&self.0, #offset_lit))
                    }
                });

                ctor_body.extend(quote::quote! {
                    write_bytes::<#comp_size_lit>(&mut bytes, #offset_lit, &#field_ident.0);
                });
            }
            MemberType::Enum {
                name: enum_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(enum_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                ctor_params.push(quote::quote! { #field_ident: #target_ident });

                getters.extend(quote::quote! {
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        #target_ident::from_raw(#r_type_ty::#from_method(
                            read_bytes::<#prim_size_lit>(&self.0, #offset_lit)
                        ))
                    }
                });

                ctor_body.extend(quote::quote! {
                    let val_bytes = (#field_ident as #r_type_ty).#to_method();
                    write_bytes::<#prim_size_lit>(&mut bytes, #offset_lit, &val_bytes);
                });
            }
            MemberType::Set {
                name: set_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(set_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                ctor_params.push(quote::quote! { #field_ident: #target_ident });

                getters.extend(quote::quote! {
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        #target_ident(#r_type_ty::#from_method(
                            read_bytes::<#prim_size_lit>(&self.0, #offset_lit)
                        ))
                    }
                });

                ctor_body.extend(quote::quote! {
                    let val_bytes = #field_ident.0.#to_method();
                    write_bytes::<#prim_size_lit>(&mut bytes, #offset_lit, &val_bytes);
                });
            }
        }
    }

    // manual String param parsing instead of syn::FnArg, refactor when generated code interface stabilises

    // Emit composite doc from the type's XML description.
    if let Some(ref desc) = tokens[0].encoding.description {
        push_description_doc(src, desc);
    }

    let ts = quote::quote! {
        #[derive(#derives)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[repr(transparent)]
        pub struct #name_ident(pub [u8; #size_lit]);

        impl #name_ident {
            #getters

            pub fn new(#(#ctor_params),*) -> Self {
                let mut bytes = [0u8; #size_lit];
                #ctor_body
                Self(bytes)
            }
        }

        // DECISIONS.md §10: compile-time proof that the Rust struct matches the
        // wire size — catches generator bugs at compile time, zero runtime cost.
        const _: () = assert!(core::mem::size_of::<#name_ident>() == #size_lit);
    };

    src.push_str(&ts.to_string());

    // MessageHeader convenience: peek methods + ENCODED_LENGTH so
    // callers don't re-copy the 8-byte header for dispatch.
    if raw_name == "messageHeader" {
        let extras = quote::quote! {
            /// Canonical wire size of the SBE message header (always 8 bytes).
            pub const MESSAGE_HEADER_ENCODED_LENGTH: usize = 8;

            impl #name_ident {
                /// Read `(template_id, schema_id)` from a frame without
                /// constructing a full `MessageHeader`. Returns `None`
                /// when the buffer is shorter than 8 bytes.
                #[inline]
                pub fn peek_header(data: &[u8]) -> Option<(u16, u16)> {
                    if data.len() < 8 {
                        return None;
                    }
                    let mut hdr = [0u8; 8];
                    hdr.copy_from_slice(&data[..8]);
                    let this = Self(hdr);
                    Some((this.template_id(), this.schema_id()))
                }

                /// Read `template_id` from a frame without constructing a full
                /// `MessageHeader`. Returns `None` when the buffer is shorter
                /// than the 8-byte header. For correct multi-schema dispatch,
                /// prefer [`Self::peek_header`] which also returns `schema_id`.
                #[inline]
                pub fn peek_template_id(data: &[u8]) -> Option<u16> {
                    if data.len() < 8 {
                        return None;
                    }
                    let mut hdr = [0u8; 8];
                    hdr.copy_from_slice(&data[..8]);
                    Some(Self(hdr).template_id())
                }

                /// Validate `schema_id` and return `template_id`. Returns
                /// `None` when the buffer is too short or the schema doesn't
                /// match. Use this for correct multi-schema dispatch.
                #[inline]
                pub fn peek_for_schema(data: &[u8], expected_schema_id: u16) -> Option<u16> {
                    let (tid, sid) = Self::peek_header(data)?;
                    if sid == expected_schema_id { Some(tid) } else { None }
                }
            }
        };
        src.push_str(&extras.to_string());
    }
    src.push('\n');

    // ── 5b. Composite decoder (flyweight / _lazy accessor) ──
    let mut decoder_getters = proc_macro2::TokenStream::new();
    for m in &members {
        let field_name = to_snake_case(&m.name);
        let field_ident = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
        let offset_lit = syn::LitInt::new(&m.offset.to_string(), proc_macro2::Span::call_site());

        match &m.member_type {
            MemberType::Primitive {
                prim,
                length,
                presence,
                constant_value,
            } => {
                let r_type_str = rust_type(*prim);
                let r_type_ty: syn::Type = syn::parse_str(r_type_str).unwrap();
                let prim_size = prim.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                if *presence == Presence::Constant {
                    if let Some(val) = constant_value {
                        if *prim == PrimitiveType::Char && val.len() > 1 {
                            let val_lit = syn::LitStr::new(val, proc_macro2::Span::call_site());
                            decoder_getters.extend(quote::quote! {
                                #[inline]
                                pub const fn #field_ident(&self) -> &'static str {
                                    #val_lit
                                }
                            });
                        } else {
                            let expr_str = constant_value_expr(*prim, val);
                            let expr: syn::Expr = syn::parse_str(&expr_str).unwrap();
                            decoder_getters.extend(quote::quote! {
                                #[inline]
                                pub const fn #field_ident(&self) -> #r_type_ty {
                                    #expr
                                }
                            });
                        }
                    }
                    continue;
                }

                if let Some(len) = length {
                    let len_lit =
                        syn::LitInt::new(&len.to_string(), proc_macro2::Span::call_site());
                    if *len > 0 {
                        decoder_getters.extend(quote::quote! {
                            #[inline]
                            pub fn #field_ident(&self) -> [#r_type_ty; #len_lit] {
                                let mut res = [0 as #r_type_ty; #len_lit];
                                let mut idx = 0;
                                while idx < #len_lit {
                                    let offset = self.pos + #offset_lit + idx * #prim_size_lit;
                                    res[idx] = #r_type_ty::#from_method(
                                        read_bytes::<#prim_size_lit>(self.buf, offset)
                                    );
                                    idx += 1;
                                }
                                res
                            }
                        });
                    } else {
                        let zero_ty: syn::Type =
                            syn::parse_str(&format!("[{}; 0]", r_type_str)).unwrap();
                        decoder_getters.extend(quote::quote! {
                            #[inline]
                            pub fn #field_ident(&self) -> #zero_ty {
                                []
                            }
                        });
                    }
                } else {
                    decoder_getters.extend(quote::quote! {
                        #[inline]
                        pub fn #field_ident(&self) -> #r_type_ty {
                            let offset = self.pos + #offset_lit;
                            #r_type_ty::#from_method(read_bytes::<#prim_size_lit>(self.buf, offset))
                        }
                    });
                }
            }
            MemberType::Composite {
                name: comp_name,
                size: comp_size,
            } => {
                let target_name = to_pascal_case(comp_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let comp_size_lit =
                    syn::LitInt::new(&comp_size.to_string(), proc_macro2::Span::call_site());

                decoder_getters.extend(quote::quote! {
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        let offset = self.pos + #offset_lit;
                        #target_ident(read_bytes::<#comp_size_lit>(self.buf, offset))
                    }
                });
            }
            MemberType::Enum {
                name: enum_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(enum_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                decoder_getters.extend(quote::quote! {
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        let offset = self.pos + #offset_lit;
                        #target_ident::from_raw(#r_type_ty::#from_method(read_bytes::<#prim_size_lit>(self.buf, offset)))
                    }
                });
            }
            MemberType::Set {
                name: set_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(set_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                decoder_getters.extend(quote::quote! {
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        let offset = self.pos + #offset_lit;
                        #target_ident(#r_type_ty::#from_method(read_bytes::<#prim_size_lit>(self.buf, offset)))
                    }
                });
            }
        }
    }

    let decoder_name = syn::Ident::new(&format!("{}Decoder", name), proc_macro2::Span::call_site());
    let decoder_ts = quote::quote! {
        #[derive(Clone, Copy)]
        pub struct #decoder_name<'a> {
            buf: &'a [u8],
            pos: usize,
        }

        impl<'a> #decoder_name<'a> {
            #decoder_getters
        }
    };
    src.push_str(&decoder_ts.to_string());
    src.push('\n');
}

/// Core generator for concrete consuming tail stages (DECISIONS.md §3), shared
/// by message-level and entry-level tails. Emits non-`Copy` stage structs plus
/// `into_*`, `finish`, and `skip_remaining` methods. Additive: does not touch
/// the legacy `&self` random-access surface.
///
/// `initial_ident` is the existing decoder (e.g. `CarDecoder`, `BidsEntryDecoder`);
/// `stage_prefix` is its string form, used to name the `After*`/`Complete` stages.
/// `header_size` is the message header size for messages (0 for entries).
pub(crate) fn generate_prelude(
    src: &mut String,
    elements: &SchemaElements,
    messages: &[MessageStructure],
    schema_id: u16,
    schema_version: u16,
) {
    // Schema-level constants
    writeln!(src, "pub const SCHEMA_ID: u16 = {schema_id};").unwrap();
    writeln!(src, "pub const SCHEMA_VERSION: u16 = {schema_version};").unwrap();

    // Collect generated type names (module-level, not in sbe_rt)
    let mut gen_types: Vec<String> = Vec::new();

    // Composites: both value struct and decoder
    for ct in &elements.composites {
        let name = to_pascal_case(&ct[0].name);
        gen_types.push(name.clone());
        gen_types.push(format!("{name}Decoder"));
    }

    // Enums
    for et in &elements.enums {
        gen_types.push(to_pascal_case(&et[0].name));
    }

    // Sets
    for st in &elements.sets {
        gen_types.push(to_pascal_case(&st[0].name));
    }

    // Message decoders and encoders
    for msg in messages {
        gen_types.push(format!("{}Decoder", to_pascal_case(&msg.name)));
        gen_types.push(format!("{}Encoder", to_pascal_case(&msg.name)));
    }

    // Emit prelude
    // sbe_rt types (exported from super::sbe_rt)
    src.push_str("pub mod prelude {\n");
    src.push_str(
        "    pub use super::sbe_rt::{DecodeError, EncodeError, VerifyError, SbeMessage};\n",
    );

    // Module-level types (exported from super)
    src.push_str("    pub use super::{\n");
    // Built-in module-level types
    for ty in &[
        "AnyMessage",
        "DecodedFrame",
        "FrameCursor",
        "FramingPolicy",
        "MessageVisitor",
    ] {
        writeln!(src, "        {ty},").unwrap();
    }
    // Generated types (composites, enums, sets, messages)
    for ty in &gen_types {
        writeln!(src, "        {ty},").unwrap();
    }
    src.push_str("    };\n");
    src.push_str("}\n\n");
}

pub(crate) fn generate_schema_id_from_header(
    src: &mut String,
    elements: &SchemaElements,
    header_type: &str,
    byte_order: ByteOrder,
) {
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };

    let schema_id_offset = elements
        .composites
        .iter()
        .find(|c| c[0].name == header_type)
        .and_then(|comp| {
            parse_composite_members(comp)
                .iter()
                .find(|m| m.name.to_lowercase().contains("schemaid"))
                .map(|m| m.offset)
        })
        .unwrap_or(4);

    let sid = syn::Index::from(schema_id_offset);
    let order_fn = syn::Ident::new(
        &format!("from_{order_suffix}_bytes"),
        proc_macro2::Span::call_site(),
    );
    let ts = quote::quote! {
        #[inline]
        pub fn schema_id_from_header(buf: &[u8]) -> Option<u16> {
            if buf.len() < #sid + 2 {
                return None;
            }
            let bytes = read_bytes::<2>(buf, #sid);
            Some(u16::#order_fn(bytes))
        }
    };
    src.push_str(&ts.to_string());
    src.push('\n');
}

pub(crate) fn generate_any_message(
    messages: &[MessageStructure],
    elements: &SchemaElements,
    schema_id: u16,
    header_type: &str,
    schema_name: &str,
) -> proc_macro2::TokenStream {
    let header_size = elements
        .composites
        .iter()
        .find(|c| c[0].name == header_type)
        .and_then(|c| c[0].encoding.offset)
        .unwrap_or(8);

    let (header_bl, header_ti, header_si, header_vr) = {
        let mut bl = "block_length".to_string();
        let mut ti = "template_id".to_string();
        let mut si = "schema_id".to_string();
        let mut vr = "version".to_string();
        if let Some(comp) = elements
            .composites
            .iter()
            .find(|c| c[0].name == header_type)
        {
            let members = parse_composite_members(comp);
            for m in members {
                let lower = m.name.to_lowercase();
                if lower.contains("blocklength") {
                    bl = to_snake_case(&m.name);
                } else if lower.contains("templateid") {
                    ti = to_snake_case(&m.name);
                } else if lower.contains("schemaid") {
                    si = to_snake_case(&m.name);
                } else if lower.contains("version") {
                    vr = to_snake_case(&m.name);
                }
            }
        }
        (bl, ti, si, vr)
    };

    let span = proc_macro2::Span::call_site();
    let header_type_ident = syn::Ident::new(&to_pascal_case(header_type), span);
    let header_size_lit = syn::LitInt::new(&header_size.to_string(), span);
    let schema_id_lit = syn::LitInt::new(&schema_id.to_string(), span);
    let bl_ident = syn::Ident::new(&header_bl, span);
    let ti_ident = syn::Ident::new(&header_ti, span);
    let si_ident = syn::Ident::new(&header_si, span);
    let vr_ident = syn::Ident::new(&header_vr, span);

    let mut out = proc_macro2::TokenStream::new();

    // ── AnyMessage enum ─────────────────────────────────────────────────
    {
        let mut enum_variants = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            let decoder = quote::format_ident!("{}Decoder", to_pascal_case(&m.name));
            enum_variants.extend(quote::quote! {
                #name(#decoder<'a>),
            });
        }
        out.extend(quote::quote! {
            #[non_exhaustive]
            pub enum AnyMessage<'a> {
                #enum_variants
                Unknown {
                    header: #header_type_ident,
                    payload: &'a [u8],
                },
            }
        });
    }

    // ── DecodedFrame struct ──────────────────────────────────────────────
    out.extend(quote::quote! {
        pub struct DecodedFrame<'a> {
            pub message: AnyMessage<'a>,
            pub range: core::ops::Range<usize>,
            pub len: usize,
        }
    });

    // ── FramingPolicy enum ──────────────────────────────────────────────
    out.extend(quote::quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum FramingPolicy {
            LengthPrefixU32,
            LengthPrefixU16,
            Fixed(usize),
        }
    });

    // ── FrameCursor struct + Iterator impl ──────────────────────────────
    out.extend(quote::quote! {
        pub struct FrameCursor<'a> {
            buf: &'a [u8],
            pos: usize,
            framing: FramingPolicy,
        }

        impl<'a> FrameCursor<'a> {
            #[inline]
            pub const fn new(buf: &'a [u8], framing: FramingPolicy) -> Self {
                Self { buf, pos: 0, framing }
            }
        }

        impl<'a> Iterator for FrameCursor<'a> {
            type Item = Result<DecodedFrame<'a>, sbe_rt::DecodeError>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.pos >= self.buf.len() {
                    return None;
                }
                let (header_len, frame_len) = match self.framing {
                    FramingPolicy::LengthPrefixU32 => {
                        if self.pos + 4 > self.buf.len() {
                            return Some(Err(sbe_rt::DecodeError::BufferTooShort {
                                field: "length prefix",
                                needed: 4,
                                available: self.buf.len().saturating_sub(self.pos),
                            }));
                        }
                        let bytes: [u8; 4] = read_bytes::<4>(self.buf, self.pos);
                        let len = u32::from_le_bytes(bytes) as usize;
                        (4, len)
                    }
                    FramingPolicy::LengthPrefixU16 => {
                        if self.pos + 2 > self.buf.len() {
                            return Some(Err(sbe_rt::DecodeError::BufferTooShort {
                                field: "length prefix",
                                needed: 2,
                                available: self.buf.len().saturating_sub(self.pos),
                            }));
                        }
                        let bytes: [u8; 2] = read_bytes::<2>(self.buf, self.pos);
                        let len = u16::from_le_bytes(bytes) as usize;
                        (2, len)
                    }
                    FramingPolicy::Fixed(len) => (0, len),
                };

                if self.pos + header_len + frame_len > self.buf.len() {
                    return Some(Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "frame bounds",
                        needed: header_len + frame_len,
                        available: self.buf.len().saturating_sub(self.pos),
                    }));
                }
                let off = self.pos + header_len;
                let res = AnyMessage::decode_frame(self.buf, off, frame_len);
                match res {
                    Ok(frame) => {
                        self.pos += header_len + frame_len;
                        Some(Ok(frame))
                    }
                    Err(e) => Some(Err(e)),
                }
            }
        }
    });

    // ── decode() ────────────────────────────────────────────────────────
    {
        let mut decode_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            let decoder = quote::format_ident!("{}Decoder", to_pascal_case(&m.name));
            let id = syn::LitInt::new(&m.id.to_string(), span);
            decode_arms.extend(quote::quote! {
                #id => Ok(Self::#name(#decoder::wrap(buf, body_pos, block_length, version))),
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                #[inline]
                pub fn decode(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError> {
                    if pos + #header_size_lit > buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "message header",
                            needed: #header_size_lit,
                            available: buf.len().saturating_sub(pos),
                        });
                    }
                    let header_bytes = read_bytes::<#header_size_lit>(buf, pos);
                    let header = #header_type_ident(header_bytes);
                    let template_id = header.#ti_ident();
                    let schema_id = header.#si_ident();
                    let version = header.#vr_ident();
                    let block_length = header.#bl_ident() as usize;
                    let body_pos = pos + #header_size_lit;

                    if schema_id != #schema_id_lit {
                        return Err(sbe_rt::DecodeError::WrongSchema {
                            expected: #schema_id_lit,
                            actual: schema_id,
                            expected_name: #schema_name,
                        });
                    }

                    match template_id {
                        #decode_arms
                        _ => Err(sbe_rt::DecodeError::UnknownTemplateLength { template_id }),
                    }
                }
            }
        });
    }

    // ── decode_frame() ──────────────────────────────────────────────────
    {
        let mut decode_frame_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            let decoder = quote::format_ident!("{}Decoder", to_pascal_case(&m.name));
            let id = syn::LitInt::new(&m.id.to_string(), span);
            let field_name = &m.name;
            decode_frame_arms.extend(quote::quote! {
                #id => {
                    let decoder = #decoder::wrap(buf, body_pos, block_length, version);
                    let total_len = decoder.encoded_length_with_header()?;
                    if total_len > frame_len {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #field_name,
                            needed: total_len,
                            available: frame_len,
                        });
                    }
                    Ok(DecodedFrame {
                        message: Self::#name(decoder),
                        range: pos .. pos + total_len,
                        len: total_len,
                    })
                }
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                #[inline]
                pub fn decode_frame(buf: &'a [u8], pos: usize, frame_len: usize) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
                    // Trust boundary: always validate header fits
                    if pos + #header_size_lit > buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "message header",
                            needed: #header_size_lit,
                            available: buf.len().saturating_sub(pos),
                        });
                    }
                    let header_bytes: [u8; #header_size_lit] = read_bytes::<#header_size_lit>(buf, pos);
                    let header = #header_type_ident(header_bytes);
                    let template_id = header.#ti_ident();
                    let schema_id = header.#si_ident();
                    let version = header.#vr_ident();
                    let block_length = header.#bl_ident() as usize;
                    let body_pos = pos + #header_size_lit;

                    if schema_id != #schema_id_lit {
                        return Err(sbe_rt::DecodeError::WrongSchema {
                            expected: #schema_id_lit,
                            actual: schema_id,
                            expected_name: #schema_name,
                        });
                    }

                    match template_id {
                        #decode_frame_arms
                        _ => {
                            if pos + frame_len > buf.len() {
                                return Err(sbe_rt::DecodeError::BufferTooShort {
                                    field: "template body",
                                    needed: frame_len,
                                    available: buf.len().saturating_sub(pos),
                                });
                            }
                            let payload = &buf[pos .. pos + frame_len];
                            Ok(DecodedFrame {
                                message: Self::Unknown {
                                    header,
                                    payload,
                                },
                                range: pos .. pos + frame_len,
                                len: frame_len,
                            })
                        }
                    }
                }
            }
        });
    }

    // ── encoded_length_with_header() ────────────────────────────────────
    {
        let mut encoded_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            encoded_arms.extend(quote::quote! {
                Self::#name(d) => d.encoded_length_with_header(),
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                #[inline]
                pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::DecodeError> {
                    match self {
                        #encoded_arms
                        Self::Unknown { payload, .. } => Ok(payload.len()),
                    }
                }
            }
        });
    }

    // ── as_bytes() ──────────────────────────────────────────────────────
    {
        let mut as_bytes_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            as_bytes_arms.extend(quote::quote! {
                Self::#name(d) => d.as_bytes(),
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                #[inline]
                pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    match self {
                        #as_bytes_arms
                        Self::Unknown { payload, .. } => Ok(payload),
                    }
                }
            }
        });
    }

    // ── encode() ────────────────────────────────────────────────────────
    {
        let mut encode_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            encode_arms.extend(quote::quote! {
                Self::#name(d) => {
                    let len = d.encoded_length_with_header()?;
                    buf[..len].copy_from_slice(d.as_bytes()?);
                    Ok(len)
                }
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                #[inline]
                pub fn encode(&self, buf: &mut [u8]) -> Result<usize, sbe_rt::EncodeError> {
                    match self {
                        #encode_arms
                        Self::Unknown { payload, .. } => {
                            buf[..payload.len()].copy_from_slice(payload);
                            Ok(payload.len())
                        }
                    }
                }
            }
        });
    }

    // ── MessageVisitor trait + visit() ──────────────────────────────────
    {
        let mut visitor_methods = Vec::new();
        let mut visit_arms = Vec::new();
        for m in messages {
            let name_pascal = to_pascal_case(&m.name);
            let name_snake = to_snake_case(&m.name);
            let method_name = syn::Ident::new(
                &format!("visit_{name_snake}"),
                proc_macro2::Span::call_site(),
            );
            let decoder_ty: syn::Type =
                syn::parse_str(&format!("{name_pascal}Decoder<'_>")).unwrap();
            let variant = syn::Ident::new(&name_pascal, proc_macro2::Span::call_site());
            visitor_methods.push(quote::quote! {
                fn #method_name(&mut self, decoder: &#decoder_ty) -> Self::Output;
            });
            visit_arms.push(quote::quote! {
                Self::#variant(d) => visitor.#method_name(d),
            });
        }

        out.extend(quote::quote! {
            pub trait MessageVisitor {
                type Output;

                #(#visitor_methods)*

                /// Called for unknown template IDs (not in this schema).
                /// `header` is the raw 8-byte MessageHeader; `payload` is
                /// the bytes after the header. Default returns `unimplemented!()`.
                fn visit_unknown(
                    &mut self,
                    header: &#header_type_ident,
                    payload: &[u8],
                ) -> Self::Output {
                    unimplemented!("unknown template id {} in schema {}",
                        header.#ti_ident(), stringify!(#schema_name))
                }
            }

            impl<'a> AnyMessage<'a> {
                pub fn visit<V: MessageVisitor>(&self, visitor: &mut V) -> V::Output {
                    match self {
                        #(#visit_arms)*
                        Self::Unknown { header, payload } => visitor.visit_unknown(header, payload),
                    }
                }
            }
        });
    }

    out
}

/// Make schema XML descriptions safe for rustdoc doctests.
///
/// Multi-line descriptions often carry indented ASCII protocol diagrams or
/// XML-comment prose (e.g. Aeron cluster codecs). Rustdoc treats 4-space
/// indented blocks as Rust doctests, which then fail `cargo test --doc`.
/// Fence multi-line content as `text` so it stays documentation only.
pub(crate) fn sanitize_description_for_doc(desc: &str) -> String {
    let desc = desc.trim_end_matches(['\r', '\n']);
    if !desc.contains('\n') {
        return desc.to_string();
    }
    let fence = if desc.contains("```") { "````" } else { "```" };
    format!("{fence}text\n{desc}\n{fence}")
}

/// `#[doc = "..."]` token for a schema description (doctest-safe).
pub(crate) fn doc_attr_tokens(desc: &str) -> proc_macro2::TokenStream {
    let lit = syn::LitStr::new(
        &sanitize_description_for_doc(desc),
        proc_macro2::Span::call_site(),
    );
    quote::quote! { #[doc = #lit] }
}

/// Append `///` rustdoc lines for a schema description (doctest-safe).
///
/// Single-line style is `///Text` (no forced space) so existing provenance
/// tests that match `///Description…` keep passing. Multi-line content is
/// first fenced as `text` by [`sanitize_description_for_doc`].
pub(crate) fn push_description_doc(src: &mut String, desc: &str) {
    for line in sanitize_description_for_doc(desc).lines() {
        src.push_str("///");
        src.push_str(line);
        src.push('\n');
    }
}

/// Compute a deterministic 64-bit hash of the schema identity.
///
/// Uses FNV-1a over `package` bytes, `id` (LE), and `version` (LE).
/// This is a simple compile-time-expressible hash for schema identity
/// verification — not a cryptographic hash.
pub(crate) fn compute_schema_hash(package: &str, id: u16, version: u16) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for &b in package.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    for &b in &id.to_le_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    for &b in &version.to_le_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Compute SHA-256 hash of the canonical schema IR.
pub(crate) fn compute_schema_sha256(ir: &Ir) -> [u8; 32] {
    let canonical = canonical_schema_bytes(ir);
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    let result = hasher.finalize();
    result.into()
}

/// Serialize the schema IR to a canonical byte sequence for hashing.
/// The output is deterministic for the same IR content.
pub(crate) fn canonical_schema_bytes(ir: &Ir) -> Vec<u8> {
    let mut buf = Vec::new();

    // Schema identity
    extend_str(&mut buf, &ir.package);
    buf.extend_from_slice(&ir.id.to_le_bytes());
    buf.extend_from_slice(&ir.version.to_le_bytes());
    buf.push(match ir.byte_order {
        ByteOrder::LittleEndian => 0,
        ByteOrder::BigEndian => 1,
    });
    extend_opt_str(&mut buf, ir.description.as_deref());
    extend_opt_str(&mut buf, ir.semantic_version.as_deref());
    extend_str(&mut buf, &ir.header_type);

    // Tokens
    for token in &ir.tokens {
        buf.push(token.signal as u8);
        extend_str(&mut buf, &token.name);
        match token.id {
            Some(id) => {
                buf.push(1);
                buf.extend_from_slice(&id.to_le_bytes());
            }
            None => buf.push(0),
        }

        // Encoding
        match token.encoding.primitive_type {
            Some(pt) => {
                buf.push(1);
                buf.push(pt as u8);
            }
            None => buf.push(0),
        }
        buf.push(token.encoding.presence as u8);
        buf.extend_from_slice(&token.encoding.since_version.to_le_bytes());
        match token.encoding.null_value {
            Some(nv) => {
                buf.push(1);
                buf.extend_from_slice(&nv.to_le_bytes());
            }
            None => buf.push(0),
        }
        extend_opt_str(&mut buf, token.encoding.character_encoding.as_deref());
        extend_opt_str(&mut buf, token.encoding.semantic_type.as_deref());
        match token.encoding.min_value {
            Some(mv) => {
                buf.push(1);
                buf.extend_from_slice(&mv.to_le_bytes());
            }
            None => buf.push(0),
        }
        match token.encoding.max_value {
            Some(mv) => {
                buf.push(1);
                buf.extend_from_slice(&mv.to_le_bytes());
            }
            None => buf.push(0),
        }
        extend_opt_str(&mut buf, token.encoding.description.as_deref());
        extend_opt_str(&mut buf, token.encoding.constant_value.as_deref());
        match token.encoding.length {
            Some(len) => {
                buf.push(1);
                buf.extend_from_slice(&(len as u64).to_le_bytes());
            }
            None => buf.push(0),
        }
        match token.encoding.offset {
            Some(off) => {
                buf.push(1);
                buf.extend_from_slice(&(off as u64).to_le_bytes());
            }
            None => buf.push(0),
        }
    }

    buf
}

/// Append a null-terminated string to the canonical hash input.
pub(crate) fn extend_str(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
}

/// Append an optional null-terminated string (presence-tagged).
pub(crate) fn extend_opt_str(buf: &mut Vec<u8>, s: Option<&str>) {
    match s {
        Some(s) => {
            buf.push(1);
            extend_str(buf, s);
        }
        None => buf.push(0),
    }
}

/// Generate a `field_meta` module for a message, exposing field metadata
/// as a compile-time constant slice.
///
/// Emits:
/// ```ignore
/// pub mod car_field_meta {
///     pub struct FieldInfo {
///         pub name: &'static str,
///         pub id: u16,
///         pub offset: usize,
///         pub since_version: u16,
///         pub field_type: &'static str,
///     }
///     pub const FIELDS: &[FieldInfo] = &[
///         FieldInfo { name: "serialNumber", id: 1, offset: 0, since_version: 0, field_type: "uint64" },
///         ...
///     ];
/// }
/// ```
pub(crate) fn generate_message_field_meta(src: &mut String, msg: &MessageStructure) {
    let mod_name = syn::Ident::new(
        &format!("{}_field_meta", to_snake_case(&msg.name)),
        proc_macro2::Span::call_site(),
    );

    let fields: Vec<proc_macro2::TokenStream> = msg
        .fields
        .iter()
        .map(|f| {
            let name_lit = syn::LitStr::new(&f.name, proc_macro2::Span::call_site());
            let id = f.id.unwrap_or(0);
            let id_lit = syn::LitInt::new(&id.to_string(), proc_macro2::Span::call_site());
            let offset_lit =
                syn::LitInt::new(&f.offset.to_string(), proc_macro2::Span::call_site());
            let sv_lit = syn::LitInt::new(
                &f.since_version.to_string(),
                proc_macro2::Span::call_site(),
            );
            let field_type_str = match &f.field_type {
                FieldType::Primitive(prim, _) => rust_type(*prim).to_string(),
                FieldType::Composite { name, .. } => to_pascal_case(name),
                FieldType::Enum { name, .. } => to_pascal_case(name),
                FieldType::Set { name, .. } => to_pascal_case(name),
            };
            let field_type_lit =
                syn::LitStr::new(&field_type_str, proc_macro2::Span::call_site());
            let presence_str = match f.presence {
                Presence::Required => "required",
                Presence::Optional => "optional",
                Presence::Constant => "constant",
            };
            let presence_lit =
                syn::LitStr::new(presence_str, proc_macro2::Span::call_site());
            let null_val = f.null_value.map(|v| {
                let s = v.to_string();
                let lit = syn::LitStr::new(&s, proc_macro2::Span::call_site());
                quote::quote! { Some(#lit) }
            }).unwrap_or(quote::quote! { None });
            let sem_type = f
                .semantic_type
                .as_deref()
                .map(|v| {
                    let lit = syn::LitStr::new(v, proc_macro2::Span::call_site());
                    quote::quote! { Some(#lit) }
                })
                .unwrap_or(quote::quote! { None });
            let desc = f
                .description
                .as_deref()
                .map(|v| {
                    let lit = syn::LitStr::new(v, proc_macro2::Span::call_site());
                    quote::quote! { Some(#lit) }
                })
                .unwrap_or(quote::quote! { None });

            quote::quote! {
                FieldInfo { name: #name_lit, id: #id_lit, offset: #offset_lit, since_version: #sv_lit, field_type: #field_type_lit, presence: #presence_lit, null_value: #null_val, semantic_type: #sem_type, description: #desc },
            }
        })
        .collect();

    let tokens = quote::quote! {
        pub mod #mod_name {
            pub struct FieldInfo {
                pub name: &'static str,
                pub id: u16,
                pub offset: usize,
                pub since_version: u16,
                pub field_type: &'static str,
                pub presence: &'static str,
                pub null_value: Option<&'static str>,
                pub semantic_type: Option<&'static str>,
                pub description: Option<&'static str>,
            }
            pub const FIELDS: &[FieldInfo] = &[
                #(#fields)*
            ];
        }
    };

    let formatted = syn::parse_str::<syn::File>(&tokens.to_string())
        .map(|file| prettyplease::unparse(&file))
        .unwrap_or_else(|_| tokens.to_string());
    src.push_str(&formatted);
    src.push('\n');
}
