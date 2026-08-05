//! `Display` / `Debug` impl codegen for message decoders.

use super::conversion_helpers::{find_domain_type, resolve_field_ident, DECODER_RESERVED};
use super::runtime::{to_pascal_case, to_snake_case};
use crate::ir::Presence;
use crate::structured_ir::{FieldType, MessageStructure};

/// Generate `Display` (via `Debug`) impls for a message decoder.
pub(crate) fn generate_decoder_display(
    msg: &MessageStructure,
    domain_types: &[(crate::ConversionSelector, String)],
) -> proc_macro2::TokenStream {
    let name = to_pascal_case(&msg.name);
    let decoder_ident =
        syn::Ident::new(&format!("{}Decoder", name), proc_macro2::Span::call_site());
    let type_name_lit =
        syn::LitStr::new(&format!("{}Decoder", name), proc_macro2::Span::call_site());
    let mut body = proc_macro2::TokenStream::new();
    let mut debug_body = proc_macro2::TokenStream::new();
    let display_header = format!("{} {{{{ ", name);
    body.extend(quote::quote! {
        write!(f, #display_header)?;
    });
    let mut out_idx = 0usize;
    for f in &msg.fields {
        let snake = to_snake_case(&f.name);
        // Domain-converted fields use `try_<name>` (HFT-003); Display must not
        // call the old infallible name.
        let wire_name = find_domain_type(f, domain_types).map(|_| format!("{snake}_wire"));
        // Shared list: inherent decoder methods only (not get_metadata placement).
        let f_ident = resolve_field_ident(&snake, &wire_name, DECODER_RESERVED);
        let domain_try = find_domain_type(f, domain_types)
            .map(|_| syn::Ident::new(&format!("try_{snake}"), proc_macro2::Span::call_site()));
        let sep = if out_idx == 0 { "" } else { ", " };
        let end_off = f.offset + f.field_type.size();
        let end_off_lit = syn::LitInt::new(&end_off.to_string(), proc_macro2::Span::call_site());
        // Only touch wire when the field's full range is in-buffer — Display must
        // not panic on truncated / invalid SBE.
        let in_bounds = quote::quote! {
            self.pos.saturating_add(#end_off_lit) <= self.buf.len()
                && #end_off_lit <= self.acting_block_length
        };
        match &f.field_type {
            FieldType::Primitive(_prim, length) => {
                if f.presence == Presence::Constant || length.is_some() {
                    continue;
                }
                let fmt_str = format!("{sep}{snake}: {{:?}}");
                let name_lit = syn::LitStr::new(&f.name, proc_macro2::Span::call_site());
                if let Some(try_ident) = &domain_try {
                    body.extend(quote::quote! {
                        if #in_bounds {
                            if let Ok(v) = self.#try_ident() {
                                write!(f, #fmt_str, v)?;
                            }
                        }
                    });
                    debug_body.extend(quote::quote! {
                        if #in_bounds {
                            if let Ok(v) = self.#try_ident() {
                                d.field(#name_lit, &v);
                            }
                        }
                    });
                } else {
                    body.extend(quote::quote! {
                        if #in_bounds {
                            let v = self.#f_ident();
                            write!(f, #fmt_str, v)?;
                        }
                    });
                    debug_body.extend(quote::quote! {
                        if #in_bounds {
                            let v = self.#f_ident();
                            d.field(#name_lit, &v);
                        }
                    });
                }
                out_idx += 1;
            }
            FieldType::Enum {
                name: enum_name, ..
            } => {
                if f.presence == Presence::Constant {
                    continue;
                }
                let fmt_str = format!("{sep}{snake}: {enum_name}::{{e:?}}");
                let name_lit = syn::LitStr::new(&f.name, proc_macro2::Span::call_site());
                if let Some(try_ident) = &domain_try {
                    // Domain enum (e.g. bool) — fallible try_*; show value only on Ok.
                    let fmt_dom = format!("{sep}{snake}: {{:?}}");
                    body.extend(quote::quote! {
                        if #in_bounds {
                            if let Ok(v) = self.#try_ident() {
                                write!(f, #fmt_dom, v)?;
                            }
                        }
                    });
                    debug_body.extend(quote::quote! {
                        if #in_bounds {
                            if let Ok(v) = self.#try_ident() {
                                d.field(#name_lit, &v);
                            }
                        }
                    });
                } else if f.since_version > 0 {
                    body.extend(quote::quote! {
                        if #in_bounds {
                            if let Some(e) = self.#f_ident() {
                                write!(f, #fmt_str)?;
                            }
                        }
                    });
                    debug_body.extend(quote::quote! {
                        if #in_bounds {
                            let v = self.#f_ident();
                            d.field(#name_lit, &v);
                        }
                    });
                } else {
                    body.extend(quote::quote! {
                        if #in_bounds {
                            let e = self.#f_ident();
                            write!(f, #fmt_str)?;
                        }
                    });
                    debug_body.extend(quote::quote! {
                        if #in_bounds {
                            let v = self.#f_ident();
                            d.field(#name_lit, &v);
                        }
                    });
                }
                out_idx += 1;
            }
            FieldType::Set { .. } => {
                // Bitset's own Display is pipe-separated flag names (A|B|C) —
                // reuse it via format_args! (Arguments: Debug delegates to
                // Display) so the message-level Debug shows readable flags
                // instead of raw bits, or silently omitting the field.
                if f.presence == Presence::Constant {
                    continue;
                }
                let name_lit = syn::LitStr::new(&f.name, proc_macro2::Span::call_site());
                if f.since_version > 0 {
                    debug_body.extend(quote::quote! {
                        if #in_bounds {
                            if let Some(v) = self.#f_ident() {
                                d.field(#name_lit, &format_args!("{}", v));
                            }
                        }
                    });
                } else {
                    debug_body.extend(quote::quote! {
                        if #in_bounds {
                            let v = self.#f_ident();
                            d.field(#name_lit, &format_args!("{}", v));
                        }
                    });
                }
            }
            FieldType::Composite { .. } => {
                if f.presence == Presence::Constant {
                    continue;
                }
                let name_lit = syn::LitStr::new(&f.name, proc_macro2::Span::call_site());
                // Domain-converted composites use fallible `try_*` (HFT-003).
                // Wire-only composites use the *_value() accessor.
                if find_domain_type(f, domain_types).is_some() {
                    let try_ident =
                        syn::Ident::new(&format!("try_{snake}"), proc_macro2::Span::call_site());
                    debug_body.extend(quote::quote! {
                        if #in_bounds {
                            if let Ok(v) = self.#try_ident() {
                                d.field(#name_lit, &format_args!("{}", v));
                            }
                        }
                    });
                } else {
                    let f_value = syn::Ident::new(
                        &format!("{}_value", &snake),
                        proc_macro2::Span::call_site(),
                    );
                    debug_body.extend(quote::quote! {
                        if #in_bounds {
                            let v = self.#f_value();
                            d.field(#name_lit, &v);
                        }
                    });
                }
            }
        }
    }
    for g in &msg.groups {
        let g_snake = to_snake_case(&g.name);
        let g_ident = syn::Ident::new(&g_snake, proc_macro2::Span::call_site());
        let sep = if out_idx == 0 { "" } else { ", " };
        let g_total_tail = g.groups.len() + g.var_data.len();
        if g_total_tail == 0 {
            let fmt_open = format!("{sep}{g_snake}: [");
            body.extend(quote::quote! {
                if let Ok(g) = self.#g_ident() {
                    write!(f, #fmt_open)?;
                    for (i, entry) in g.enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", entry)?;
                    }
                    write!(f, "]")?;
                }
            });
        } else {
            let fmt_open = format!("{sep}{g_snake}: [");
            body.extend(quote::quote! {
                if let Ok(g) = self.#g_ident() {
                    write!(f, #fmt_open)?;
                    for (i, result) in g.enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        match result {
                            Ok(entry) => write!(f, "{}", entry)?,
                            Err(_) => write!(f, "{{err}}")?,
                        }
                    }
                    write!(f, "]")?;
                }
            });
        }
        out_idx += 1;
        // Debug: format group entries as a Vec<String> via Display.
        let g_name_lit = syn::LitStr::new(&g.name, proc_macro2::Span::call_site());
        if g_total_tail == 0 {
            debug_body.extend(quote::quote! {
                if let Ok(_g) = self.#g_ident() {
                    let entries: Vec<String> = _g.map(|e| format!("{e}")).collect();
                    d.field(#g_name_lit, &entries);
                }
            });
        } else {
            debug_body.extend(quote::quote! {
                if let Ok(_g) = self.#g_ident() {
                    let entries: Vec<String> = _g.filter_map(|r| r.ok()).map(|e| format!("{e}")).collect();
                    d.field(#g_name_lit, &entries);
                }
            });
        }
    }
    for vd in &msg.var_data {
        let vd_snake = to_snake_case(&vd.name);
        let vd_ident = syn::Ident::new(&vd_snake, proc_macro2::Span::call_site());
        let sep = if out_idx == 0 { "" } else { ", " };
        let fmt_str = format!("{sep}{vd_snake}: {{}}");
        let err_fmt = format!("{sep}{vd_snake}: <{{}} bytes>");
        body.extend(quote::quote! {
            if let Ok(d) = self.#vd_ident() {
                match std::str::from_utf8(d) {
                    Ok(s) => write!(f, #fmt_str, s)?,
                    Err(_) => write!(f, #err_fmt, d.len())?,
                }
            }
        });
        let vd_name_lit = syn::LitStr::new(&vd.name, proc_macro2::Span::call_site());
        debug_body.extend(quote::quote! {
            if let Ok(_data) = self.#vd_ident() {
                match std::str::from_utf8(_data) {
                    Ok(_s) => d.field(#vd_name_lit, &_s),
                    Err(_) => d.field(#vd_name_lit, &format!("<{} bytes>", _data.len())),
                };
            }
        });
        out_idx += 1;
    }
    body.extend(quote::quote! {
        write!(f, " }}")
    });
    // Structural Debug never reads wire bytes — safe for truncated / invalid buffers.
    let ts = quote::quote! {
        impl<'a> core::fmt::Display for #decoder_ident<'a> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                // Display delegates to Debug — one impl, both {} and {:?} work.
                // {:?} gives debug_struct (compact), {:#?} gives pretty multi-line.
                core::fmt::Debug::fmt(self, f)
            }
        }

        impl<'a> core::fmt::Debug for #decoder_ident<'a> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let mut d = f.debug_struct(#type_name_lit);
                #debug_body
                d.finish()
            }
        }
    };
    ts
}
