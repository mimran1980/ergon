//! `*_as` / `*_from` / domain-type conversion method codegen for message
//! decoders and encoders (including nested group entries).

use super::conversion_helpers::{
    field_has_conversion_free, find_domain_selector, find_domain_type,
};
use super::runtime::{to_pascal_case, to_snake_case};
use crate::structured_ir::{FieldType, MessageField, MessageGroup, MessageStructure, rust_type};

/// A domain accessor is `Option`-wrapped exactly when the raw accessor it
/// delegates to is: a field gated by `sinceVersion` returns `Option` (absent
/// before its version), and a primitive/enum with `presence="optional"`
/// null-maps to `Option`. A *composite* with `presence="optional"` does NOT —
/// there is no null image for a composite, so the raw `_value()` accessor
/// stays plain and wrapping it here would produce a type mismatch
/// (`Some`/`None` on a non-`Option`). Shared by the message-level and
/// group-entry codegen paths so the rule can't drift between them.
pub(crate) fn is_optional_domain_field(
    f: &MessageField,
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
) -> bool {
    match &f.field_type {
        // A fixed array accessor is never `Option`: when the field predates the
        // acting version it returns a zero-filled array, not `None`.
        FieldType::Primitive(_, Some(_)) => false,
        // A scalar primitive null-maps to `Option` when declared optional.
        FieldType::Primitive(_, None) => {
            f.since_version > 0 || f.presence == crate::ir::Presence::Optional
        }
        // An enum carries absence *in band* — an optional enum has a
        // `NullVal` variant, so its accessor stays plain unless
        // `null_as_option` maps that variant to `None`.
        FieldType::Enum { name, .. } => {
            f.since_version > 0
                || super::conversion_helpers::enum_uses_null_as_option(
                    name,
                    null_as_option,
                    all_enums_as_option,
                )
        }
        // A composite has no null image; a set has no null variant.
        FieldType::Composite { .. } | FieldType::Set { .. } => f.since_version > 0,
    }
}

/// A generated named type (composite/enum/set) as a `syn::Type`.
fn pascal_type(name: &str, span: proc_macro2::Span) -> syn::Type {
    let ident = syn::Ident::new(&to_pascal_case(name), span);
    syn::parse_quote!(#ident)
}

/// The wire type a converter converts from/to. A fixed array field's wire type
/// is `[T; N]`, not the element type — generating converters against `T` for an
/// array field produces a module that does not compile.
fn primitive_wire_type(rust_name: &str, length: Option<usize>) -> syn::Type {
    let elem = syn::Ident::new(rust_name, proc_macro2::Span::call_site());
    match length {
        Some(n) => {
            let n = syn::LitInt::new(&n.to_string(), proc_macro2::Span::call_site());
            syn::parse_quote!([#elem; #n])
        }
        None => syn::parse_quote!(#elem),
    }
}

/// Generate `*_as`/`*_from` conversion methods for fields matching the
/// configured conversion selectors. Also emits raw `*_wire` aliases if the
/// field would otherwise shadow them.
pub(crate) fn generate_converter_impls(
    msg: &MessageStructure,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    manual_impl_snippets: &[(crate::ConversionSelector, String)],
    _multi_message: bool,
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
) -> String {
    let span = proc_macro2::Span::call_site();
    let msg_name = to_pascal_case(&msg.name);
    let decoder_ident = syn::Ident::new(&format!("{msg_name}Decoder"), span);
    let encoder_ident = syn::Ident::new(&format!("{msg_name}Encoder"), span);

    let mut decoder_methods = proc_macro2::TokenStream::new();
    let mut encoder_methods = proc_macro2::TokenStream::new();

    for f in &msg.fields {
        // A constant field has no wire storage: no `*_wire` setter and no
        // raw getter to convert through, and it is excluded from the DTO. A
        // selector that happens to match its type must not generate accessors
        // against setters that do not exist.
        if f.presence == crate::ir::Presence::Constant {
            continue;
        }
        // Determine if this field has a conversion, and what the wire type is.
        let (type_name, wire_type_ident): (String, syn::Type) = match &f.field_type {
            FieldType::Composite { name, .. } => (name.clone(), pascal_type(name, span)),
            FieldType::Enum { name, .. } => (name.clone(), pascal_type(name, span)),
            FieldType::Set { name, .. } => (name.clone(), pascal_type(name, span)),
            FieldType::Primitive(pt, length) => {
                let rust_name = rust_type(*pt);
                (
                    rust_name.to_string(),
                    primitive_wire_type(rust_name, *length),
                )
            }
        };
        let has_conversion = field_has_conversion_free(f, conversions);
        if !has_conversion {
            continue;
        }

        let field_snake = to_snake_case(&f.name);
        let domain_type_path = find_domain_type(f, domain_types);

        // Determine which raw accessor to call. Composites have _value()
        // for the owned wire value; everything else uses the _wire getter.
        let raw_decoder_getter = if matches!(f.field_type, FieldType::Composite { .. }) {
            syn::Ident::new(&format!("{field_snake}_value"), span)
        } else {
            syn::Ident::new(&format!("{field_snake}_wire"), span)
        };
        let wire_setter = syn::Ident::new(&format!("{field_snake}_wire"), span);

        if let Some(dt) = domain_type_path {
            let dt_ty: syn::Type =
                syn::parse_str(dt).unwrap_or_else(|_| panic!("invalid domain type path: {dt}"));
            let domain_ident = syn::Ident::new(&field_snake, span);

            // checked domain accessors are fallible — no `.expect`.
            let try_ident = syn::Ident::new(&format!("try_{field_snake}"), span);
            let is_optional = is_optional_domain_field(f, null_as_option, all_enums_as_option);
            // DomainImpl::Manual doc comment: a ready-to-paste starting point,
            // shown on both the decoder and encoder accessor (IDE hover /
            // `cargo doc`) so a missing-impl compile error has somewhere to
            // point the caller.
            let manual_impl_doc = find_domain_selector(f, domain_types)
                .and_then(|sel| manual_impl_snippets.iter().find(|(s, _)| s == sel))
                .map(|(_, snippet)| {
                    let doc = format!(
                        "This field uses `DomainImpl::Manual` — provide these impls \
                         yourself (starting point, copy-paste and adjust):\n\n\
                         ```text\n{snippet}```"
                    );
                    quote::quote! { #[doc = #doc] }
                })
                .unwrap_or_default();
            if is_optional {
                decoder_methods.extend(quote::quote! {
                    #manual_impl_doc
                    #[inline]
                    pub fn #try_ident(
                        &self,
                    ) -> Result<Option<#dt_ty>, <#dt_ty as TryFromSbe<#wire_type_ident>>::Error> {
                        match self.#raw_decoder_getter() {
                            Some(wire) => <#dt_ty as TryFromSbe<#wire_type_ident>>::try_from_sbe(wire).map(Some),
                            None => Ok(None),
                        }
                    }
                });
            } else {
                decoder_methods.extend(quote::quote! {
                    #manual_impl_doc
                    #[inline]
                    pub fn #try_ident(
                        &self,
                    ) -> Result<#dt_ty, <#dt_ty as TryFromSbe<#wire_type_ident>>::Error> {
                        <#dt_ty as TryFromSbe<#wire_type_ident>>::try_from_sbe(
                            self.#raw_decoder_getter()
                        )
                    }
                });
            }

            // The encoder setter always takes the domain value directly. The
            // raw `*_wire` setter it delegates to takes the plain wire type —
            // `Option`/null handling lives in `fixed(&FixedFields)`, which
            // writes the schema null image for `None`. Wrapping the domain
            // value in `Option` here produced a type mismatch on optional
            // primitives (`self.ts_wire(Option<u64>)` against `ts_wire(u64)`).
            // Absence is expressed on the raw path, not the domain setter.
            encoder_methods.extend(quote::quote! {
                #manual_impl_doc
                #[inline]
                pub fn #try_ident(
                    &mut self,
                    value: #dt_ty,
                ) -> Result<&mut Self, <#dt_ty as TryToSbe<#wire_type_ident>>::Error> {
                    let wire = <#dt_ty as TryToSbe<#wire_type_ident>>::try_to_sbe(&value)?;
                    self.#wire_setter(wire);
                    Ok(self)
                }
            });
        } else {
            let as_ident = syn::Ident::new(&format!("{field_snake}_as"), span);
            let from_ident = syn::Ident::new(&format!("{field_snake}_from"), span);

            // Same `Option` rule as the domain `try_*` path: when the raw
            // getter yields `Option<W>` the converter must unwrap it, or the
            // generated body feeds `Option<W>` to `TryFromSbe<W>`.
            decoder_methods.extend(if is_optional_domain_field(f, null_as_option, all_enums_as_option) {
                quote::quote! {
                    #[inline]
                    #[must_use]
                    pub fn #as_ident<T: TryFromSbe<#wire_type_ident>>(&self) -> Result<Option<T>, T::Error> {
                        match self.#raw_decoder_getter() {
                            Some(wire) => T::try_from_sbe(wire).map(Some),
                            None => Ok(None),
                        }
                    }
                }
            } else {
                quote::quote! {
                    #[inline]
                    #[must_use]
                    pub fn #as_ident<T: TryFromSbe<#wire_type_ident>>(&self) -> Result<T, T::Error> {
                        T::try_from_sbe(self.#raw_decoder_getter())
                    }
                }
            });

            encoder_methods.extend(quote::quote! {
                #[inline]
                #[must_use]
                pub fn #from_ident<T: TryToSbe<#wire_type_ident>>(&mut self, value: &T) -> Result<&mut Self, T::Error> {
                    let wire = value.try_to_sbe()?;
                    self.#wire_setter(wire);
                    Ok(self)
                }
            });
        }
    }

    // Group entries (recursively): concrete methods when domain type is
    // configured, generic *_as/*_from otherwise.
    fn emit_group_entry_impls(
        scope: &str,
        g: &MessageGroup,
        conversions: &[crate::ConversionSelector],
        domain_types: &[(crate::ConversionSelector, String)],
        null_as_option: &[crate::ConversionSelector],
        all_enums_as_option: bool,
        out: &mut String,
    ) {
        let span = proc_macro2::Span::call_site();
        let scoped = format!("{scope}{}", to_pascal_case(&g.name));
        let entry_dec_ident = syn::Ident::new(&format!("{scoped}EntryDecoder"), span);
        let entry_enc_ident = syn::Ident::new(&format!("{scoped}EntryEncoder"), span);
        let mut dec_methods = proc_macro2::TokenStream::new();
        let mut enc_methods = proc_macro2::TokenStream::new();
        for f in &g.fields {
            // A constant field has no wire storage: no `*_wire` setter and no
            // raw getter to convert through, and it is excluded from the DTO. A
            // selector that happens to match its type must not generate accessors
            // against setters that do not exist.
            if f.presence == crate::ir::Presence::Constant {
                continue;
            }
            if !field_has_conversion_free(f, conversions) {
                continue;
            }
            let field_snake = to_snake_case(&f.name);
            let wire_type_ident: syn::Type = match &f.field_type {
                FieldType::Composite { name, .. } => pascal_type(name, span),
                FieldType::Enum { name, .. } => pascal_type(name, span),
                FieldType::Set { name, .. } => pascal_type(name, span),
                FieldType::Primitive(pt, length) => primitive_wire_type(rust_type(*pt), *length),
            };
            let raw_decoder_getter = if matches!(f.field_type, FieldType::Composite { .. }) {
                syn::Ident::new(&format!("{field_snake}_value"), span)
            } else {
                syn::Ident::new(&format!("{field_snake}_wire"), span)
            };
            let wire_setter = syn::Ident::new(&format!("{field_snake}_wire"), span);

            if let Some(dt) = find_domain_type(f, domain_types) {
                let dt_ty: syn::Type =
                    syn::parse_str(dt).unwrap_or_else(|_| panic!("invalid domain type path: {dt}"));
                let domain_ident = syn::Ident::new(&field_snake, span);
                let try_ident = syn::Ident::new(&format!("try_{field_snake}"), span);
                let is_optional = is_optional_domain_field(f, null_as_option, all_enums_as_option);
                if is_optional {
                    dec_methods.extend(quote::quote! {
                        #[inline]
                        pub fn #try_ident(
                            &self,
                        ) -> Result<Option<#dt_ty>, <#dt_ty as TryFromSbe<#wire_type_ident>>::Error> {
                            match self.#raw_decoder_getter() {
                                Some(wire) => <#dt_ty as TryFromSbe<#wire_type_ident>>::try_from_sbe(wire).map(Some),
                                None => Ok(None),
                            }
                        }
                    });
                } else {
                    dec_methods.extend(quote::quote! {
                        #[inline]
                        pub fn #try_ident(
                            &self,
                        ) -> Result<#dt_ty, <#dt_ty as TryFromSbe<#wire_type_ident>>::Error> {
                            <#dt_ty as TryFromSbe<#wire_type_ident>>::try_from_sbe(
                                self.#raw_decoder_getter()
                            )
                        }
                    });
                }
                enc_methods.extend(quote::quote! {
                    #[inline]
                    pub fn #try_ident(
                        &mut self,
                        value: #dt_ty,
                    ) -> Result<&mut Self, <#dt_ty as TryToSbe<#wire_type_ident>>::Error> {
                        let wire = <#dt_ty as TryToSbe<#wire_type_ident>>::try_to_sbe(&value)?;
                        self.#wire_setter(wire);
                        Ok(self)
                    }
                });
            } else {
                let as_ident = syn::Ident::new(&format!("{field_snake}_as"), span);
                let from_ident = syn::Ident::new(&format!("{field_snake}_from"), span);
                dec_methods.extend(if is_optional_domain_field(f, null_as_option, all_enums_as_option) {
                    quote::quote! {
                        #[inline]
                        #[must_use]
                        pub fn #as_ident<T: TryFromSbe<#wire_type_ident>>(&self) -> Result<Option<T>, T::Error> {
                            match self.#raw_decoder_getter() {
                                Some(wire) => T::try_from_sbe(wire).map(Some),
                                None => Ok(None),
                            }
                        }
                    }
                } else {
                    quote::quote! {
                        #[inline]
                        #[must_use]
                        pub fn #as_ident<T: TryFromSbe<#wire_type_ident>>(&self) -> Result<T, T::Error> {
                            T::try_from_sbe(self.#raw_decoder_getter())
                        }
                    }
                });
                enc_methods.extend(quote::quote! {
                    #[inline]
                    #[must_use]
                    pub fn #from_ident<T: TryToSbe<#wire_type_ident>>(&mut self, value: &T) -> Result<&mut Self, T::Error> {
                        let wire = value.try_to_sbe()?;
                        let _ = self.#wire_setter(wire);
                        Ok(self)
                    }
                });
            }
        }
        if !dec_methods.is_empty() {
            let ts = quote::quote! {
                impl<'a> #entry_dec_ident<'a> {
                    #dec_methods
                }
                impl<'a> #entry_enc_ident<'a> {
                    #enc_methods
                }
            };
            out.push_str(&ts.to_string());
        }
        for ng in &g.groups {
            emit_group_entry_impls(
                &scoped,
                ng,
                &conversions,
                domain_types,
                null_as_option,
                all_enums_as_option,
                out,
            );
        }
    }

    let mut entry_impls = String::new();
    let group_scope = if _multi_message {
        msg_name.clone()
    } else {
        String::new()
    };
    for g in &msg.groups {
        emit_group_entry_impls(
            &group_scope,
            g,
            &conversions,
            domain_types,
            null_as_option,
            all_enums_as_option,
            &mut entry_impls,
        );
    }

    if decoder_methods.is_empty() && entry_impls.is_empty() {
        return String::new();
    }

    let mut out = if decoder_methods.is_empty() {
        String::new()
    } else {
        // Generic over H so body-only wrap (`HeaderAbsent`) gets conversion
        // setters. Emitted once per *concrete* fields phase, mirroring the
        // ordinary field setters: these methods delegate to the raw `*_wire`
        // setters, which live on concrete impls (a generic `impl<H, F>` cannot
        // resolve them), and the fixed-phase copy is what gives a domain-typed
        // field a route to a terminal method — `fixed()` accepts wire values
        // only, so conversions cannot go through it.
        quote::quote! {
            impl<'a> #decoder_ident<'a> {
                #decoder_methods
            }
            impl<'a, H: sbe_rt::HeaderState> #encoder_ident<'a, H, sbe_rt::FieldsUnfixed> {
                #encoder_methods
            }
            impl<'a, H: sbe_rt::HeaderState> #encoder_ident<'a, H, sbe_rt::FieldsFixed> {
                #encoder_methods
            }
        }
        .to_string()
    };
    out.push_str(&entry_impls);
    out
}
