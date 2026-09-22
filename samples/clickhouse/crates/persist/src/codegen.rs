//! Ergo SBE hook factory: generates `Persistable` implementations for
//! domain DTOs as tokens appended after each generated item.
//!
//! Used from `build.rs` files: `config.with_hook(persist_hook())`. The
//! generated impls cover fixed scalar fields and fixed scalar arrays;
//! composites, enums, groups, and var-data are recorded through raw-SBE
//! layouts instead (the typed row stores the flat scalar projection).

use ergo_sbe::config::{FieldInfo, FieldKind, ItemContext};
use quote::{format_ident, quote};

/// Hook type shared with `GenerationConfig::with_hook`.
pub type PersistHookFn = dyn Fn(&ItemContext<'_>) -> Vec<proc_macro2::TokenStream> + Send + Sync;

/// Build the persist hook for `GenerationConfig::with_hook`. Clone the
/// returned `Arc` to register it with multiple generation configs.
#[must_use]
pub fn persist_hook() -> std::sync::Arc<PersistHookFn> {
    std::sync::Arc::new(|ctx: &ItemContext<'_>| match ctx {
        ItemContext::DomainStruct { name, fields, .. } => {
            vec![domain_struct_persistable(name, fields)]
        }
        _ => Vec::new(),
    })
}

fn type_code_ident(rust_type: &str) -> Option<proc_macro2::TokenStream> {
    let code = match rust_type {
        "bool" => "Bool",
        "i8" => "I8",
        "i16" => "I16",
        "i32" => "I32",
        "i64" => "I64",
        "u8" => "U8",
        "u16" => "U16",
        "u32" => "U32",
        "u64" => "U64",
        "f32" => "F32",
        "f64" => "F64",
        _ => return None,
    };
    let ident = format_ident!("{code}");
    Some(quote! { ::ergo_clickhouse_persist::schema::TypeCode::#ident })
}

fn field_schema(f: &FieldInfo) -> Option<proc_macro2::TokenStream> {
    match f.kind() {
        FieldKind::Fixed => {}
        _ => return None,
    }
    // Scalar (or scalar-array) storage only; optional-presence fields are
    // `Option<T>` on the DTO and nullable in storage.
    let optional = f.presence == "optional";
    let bare = if f.rust_type.starts_with("Option<") {
        f.rust_type
            .trim_start_matches("Option<")
            .trim_end_matches('>')
            .to_string()
    } else {
        f.rust_type.clone()
    };
    if let Some(code) = type_code_ident(&bare) {
        return Some(match optional {
            true => quote! { ::ergo_clickhouse_persist::schema::ValueSchema::optional(#code) },
            false => quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(#code) },
        });
    }
    if let Some(elem) = scalar_array_elem(&bare) {
        let code = type_code_ident(&elem)?;
        return Some(quote! { ::ergo_clickhouse_persist::schema::ValueSchema::array(#code) });
    }
    None
}

/// `"[u8; 6]"` → `"u8"`.
fn scalar_array_elem(rust_type: &str) -> Option<String> {
    let inner = rust_type.trim_start_matches('[').split(';').next()?.trim();
    let is_array = rust_type.starts_with('[') && rust_type.contains(';');
    if is_array {
        Some(inner.to_string())
    } else {
        None
    }
}

fn array_dims(rust_type: &str) -> Option<(String, usize)> {
    if !rust_type.starts_with('[') || !rust_type.contains(';') {
        return None;
    }
    let inner = rust_type.trim_start_matches('[');
    let (elem, n) = inner.split_once(';')?;
    let n: usize = n.trim().trim_end_matches(']').trim().parse().ok()?;
    Some((elem.trim().to_string(), n))
}

fn field_width(rust_type: &str) -> Option<usize> {
    Some(match rust_type {
        "bool" | "i8" | "u8" => 1,
        "i16" | "u16" => 2,
        "i32" | "u32" | "f32" => 4,
        "i64" | "u64" | "f64" => 8,
        _ => return None,
    })
}

fn encode_stmt(f: &FieldInfo) -> Option<proc_macro2::TokenStream> {
    let fname = format_ident!("{}", f.name);
    let optional = f.presence == "optional";
    let bare = if f.rust_type.starts_with("Option<") {
        f.rust_type
            .trim_start_matches("Option<")
            .trim_end_matches('>')
            .to_string()
    } else {
        f.rust_type.clone()
    };

    if optional {
        let le = match bare.as_str() {
            "bool" => quote! { u8::from(*v).to_le_bytes() },
            _ => quote! { v.to_le_bytes() },
        };
        return Some(quote! {
            match self.#fname {
                ::core::option::Option::Some(v) => {
                    let b = #le;
                    out.write_opt_le(::core::option::Option::Some(&b))?;
                }
                ::core::option::Option::None => out.write_opt_le(::core::option::Option::None)?,
            }
        });
    }

    let scalar_write = || {
        let setter = match bare.as_str() {
            "bool" => format_ident!("write_bool"),
            "i8" => format_ident!("write_i8"),
            "i16" => format_ident!("write_i16"),
            "i32" => format_ident!("write_i32"),
            "i64" => format_ident!("write_i64"),
            "u8" => format_ident!("write_u8"),
            "u16" => format_ident!("write_u16"),
            "u32" => format_ident!("write_u32"),
            "u64" => format_ident!("write_u64"),
            "f32" => format_ident!("write_f32"),
            "f64" => format_ident!("write_f64"),
            _ => return None,
        };
        Some(quote! { out.#setter(self.#fname)?; })
    };

    if let Some(w) = scalar_write() {
        return Some(w);
    }
    if let Some((elem, n)) = array_dims(&f.rust_type) {
        let code = type_code_ident(&elem)?;
        let width = field_width(&elem)?;
        let total = n.checked_mul(width)?;
        return Some(quote! {{
            let mut le = [0u8; #total];
            for (i, v) in self.#fname.iter().enumerate() {
                let b = v.to_le_bytes();
                le[i * #width..i * #width + #width].copy_from_slice(&b[..#width]);
            }
            out.write_array(#code, #width, &le, #n)?;
        }});
    }
    None
}

fn domain_struct_persistable(name: &str, fields: &[FieldInfo]) -> proc_macro2::TokenStream {
    let struct_ident = format_ident!("{name}");
    let schema_ident = format_ident!("{}_PERSIST_SCHEMA", name.to_uppercase());

    let stored: Vec<&FieldInfo> = fields
        .iter()
        .filter(|f| field_schema(f).is_some())
        .collect();
    if stored.is_empty() {
        // Nothing stored in the typed row: no Persistable impl generated.
        return quote! {};
    }

    let schemas: Vec<_> = stored.iter().filter_map(|f| field_schema(f)).collect();
    let width_terms: Vec<_> = stored
        .iter()
        .map(|f| {
            if let Some(w) = field_width(&f.rust_type) {
                quote! { #w }
            } else if let Some((elem, n)) = array_dims(&f.rust_type) {
                let w = field_width(&elem).unwrap_or(0);
                let total = n * w;
                quote! { #total }
            } else {
                quote! { 0 }
            }
        })
        .collect();
    let encode_stmts: Vec<_> = stored.iter().filter_map(|f| encode_stmt(f)).collect();

    quote! {
        #[doc = concat!("Ordered storage schema of the `", stringify!(#struct_ident), "` domain DTO (scalar projection).")]
        pub static #schema_ident: ::ergo_clickhouse_persist::schema::RowSchema =
            ::ergo_clickhouse_persist::schema::RowSchema {
                columns: &[ #(#schemas),* ],
            };

        #[automatically_derived]
        impl ::ergo_clickhouse_persist::persist::Persistable for #struct_ident {
            #[inline]
            fn schema() -> &'static ::ergo_clickhouse_persist::schema::RowSchema {
                &#schema_ident
            }
            #[inline]
            fn encoded_len(&self) -> ::core::result::Result<usize, ::ergo_clickhouse_persist::persist::EncodeError> {
                ::core::result::Result::Ok(
                    #schema_ident.nullmap_len() #(+ #width_terms)*
                )
            }
            #[inline]
            fn encode(&self, out: &mut ::ergo_clickhouse_persist::persist::RowWriter<'_>) -> ::core::result::Result<(), ::ergo_clickhouse_persist::persist::EncodeError> {
                #(#encode_stmts)*
                ::core::result::Result::Ok(())
            }
        }
    }
}
