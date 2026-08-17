//! `*_as` / `*_from` / domain-type conversion method codegen for message
//! decoders and encoders (including nested group entries).

use super::conversion_helpers::{field_has_conversion_free, find_domain_type};
use super::runtime::{to_pascal_case, to_snake_case};
use crate::structured_ir::{FieldType, MessageGroup, MessageStructure, rust_type};

/// Generate `*_as`/`*_from` conversion methods for fields matching the
/// configured conversion selectors. Also emits raw `*_wire` aliases if the
/// field would otherwise shadow them.
pub(crate) fn generate_converter_impls(
    msg: &MessageStructure,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    _multi_message: bool,
) -> String {
    let span = proc_macro2::Span::call_site();
    let msg_name = to_pascal_case(&msg.name);
    let decoder_ident = syn::Ident::new(&format!("{msg_name}Decoder"), span);
    let encoder_ident = syn::Ident::new(&format!("{msg_name}Encoder"), span);

    let mut decoder_methods = proc_macro2::TokenStream::new();
    let mut encoder_methods = proc_macro2::TokenStream::new();

    for f in &msg.fields {
        // Determine if this field has a conversion, and what the wire type is.
        let (type_name, wire_type_ident): (String, syn::Ident) = match &f.field_type {
            FieldType::Composite { name, .. } => {
                (name.clone(), syn::Ident::new(&to_pascal_case(name), span))
            }
            FieldType::Enum { name, .. } => {
                (name.clone(), syn::Ident::new(&to_pascal_case(name), span))
            }
            FieldType::Set { name, .. } => {
                (name.clone(), syn::Ident::new(&to_pascal_case(name), span))
            }
            FieldType::Primitive(pt, _) => {
                let rust_name = rust_type(*pt);
                (rust_name.to_string(), syn::Ident::new(rust_name, span))
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
            let is_optional = f.presence == crate::ir::Presence::Optional;
            if is_optional {
                decoder_methods.extend(quote::quote! {
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

            if is_optional {
                encoder_methods.extend(quote::quote! {
                    #[inline]
                    pub fn #try_ident(
                        &mut self,
                        val: Option<#dt_ty>,
                    ) -> Result<&mut Self, <#dt_ty as TryToSbe<#wire_type_ident>>::Error> {
                        let wire = match val {
                            Some(v) => Some(<#dt_ty as TryToSbe<#wire_type_ident>>::try_to_sbe(&v)?),
                            None => None,
                        };
                        self.#wire_setter(wire);
                        Ok(self)
                    }
                });
            } else {
                encoder_methods.extend(quote::quote! {
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
            }
        } else {
            let as_ident = syn::Ident::new(&format!("{field_snake}_as"), span);
            let from_ident = syn::Ident::new(&format!("{field_snake}_from"), span);

            decoder_methods.extend(quote::quote! {
                #[inline]
                #[must_use]
                pub fn #as_ident<T: TryFromSbe<#wire_type_ident>>(&self) -> Result<T, T::Error> {
                    T::try_from_sbe(self.#raw_decoder_getter())
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
        out: &mut String,
    ) {
        let span = proc_macro2::Span::call_site();
        let scoped = format!("{scope}{}", to_pascal_case(&g.name));
        let entry_dec_ident = syn::Ident::new(&format!("{scoped}EntryDecoder"), span);
        let entry_enc_ident = syn::Ident::new(&format!("{scoped}EntryEncoder"), span);
        let mut dec_methods = proc_macro2::TokenStream::new();
        let mut enc_methods = proc_macro2::TokenStream::new();
        for f in &g.fields {
            if !field_has_conversion_free(f, conversions) {
                continue;
            }
            let field_snake = to_snake_case(&f.name);
            let wire_type_ident = match &f.field_type {
                FieldType::Composite { name, .. } => syn::Ident::new(&to_pascal_case(name), span),
                FieldType::Enum { name, .. } => syn::Ident::new(&to_pascal_case(name), span),
                FieldType::Set { name, .. } => syn::Ident::new(&to_pascal_case(name), span),
                FieldType::Primitive(pt, _) => syn::Ident::new(rust_type(*pt), span),
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
                dec_methods.extend(quote::quote! {
                    #[inline]
                    #[must_use]
                    pub fn #as_ident<T: TryFromSbe<#wire_type_ident>>(&self) -> Result<T, T::Error> {
                        T::try_from_sbe(self.#raw_decoder_getter())
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
            emit_group_entry_impls(&scoped, ng, &conversions, domain_types, out);
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
