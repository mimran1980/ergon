//! Derives and prepared-callsite macros for `ergo-clickhouse-persist`.
//!
//! - `#[derive(Persistable)]` for handwritten DTOs: field order defines
//!   the storage schema; `Option<T>` fields map to nullable columns.
//! - `persist_table!` declares a typed callsite table at compile time.
//! - `persist_event!` / `persist_sbe!` / `persist_dto!` expand to a
//!   strict-lazy recording call: the enable word is read first, and when
//!   disabled none of the value expressions are evaluated.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Ident, LitStr, Type, parse_macro_input};

/// Derive `Persistable` for a plain struct.
///
/// Field types map to storage types: `bool`, `i8/i16/i32/i64`, `u8/u16/u32/u64`,
/// `f32/f64`, `String`/`&str`, `Vec<T>`/arrays for array columns, and any
/// type implementing `PersistAs` (e.g. exact-decimal wrappers).
#[proc_macro_derive(Persistable)]
pub fn derive_persistable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_persistable(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_persistable(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "Persistable requires a struct",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "Persistable requires named fields",
        ));
    };
    let field_names: Vec<&Ident> = fields
        .named
        .iter()
        .map(|f| f.ident.as_ref().unwrap())
        .collect();
    let field_types: Vec<&Type> = fields.named.iter().map(|f| &f.ty).collect();

    let schema_name = format_ident!("{}_PERSIST_SCHEMA", name.to_string().to_uppercase());

    // Schema entries: static ValueSchema per field type.
    let schema_entries = field_types.iter().map(|t| schema_entry(t));

    // encoded_len sum
    let len_terms = field_types
        .iter()
        .zip(&field_names)
        .map(|(t, n)| len_term(t, n));

    // encode body per field
    let encode_stmts = field_types
        .iter()
        .zip(&field_names)
        .enumerate()
        .map(|(i, (t, n))| encode_stmt(t, n, i));

    Ok(quote! {
        #[automatically_derived]
        impl ::ergo_clickhouse_persist::persist::Persistable for #name {
            fn schema() -> &'static ::ergo_clickhouse_persist::schema::RowSchema {
                static #schema_name: ::ergo_clickhouse_persist::schema::RowSchema =
                    ::ergo_clickhouse_persist::schema::RowSchema {
                        columns: &[ #(#schema_entries),* ],
                    };
                &#schema_name
            }
            fn encoded_len(&self) -> ::core::result::Result<usize, ::ergo_clickhouse_persist::persist::EncodeError> {
                // The null bitmap precedes the values and is sized by the
                // schema, not by the fields, so it is added once here. Omitting
                // it under-reported every row with an optional column by the
                // bitmap length, and `encoded_len` is what callers size their
                // buffers with.
                Ok(<Self as ::ergo_clickhouse_persist::persist::Persistable>::schema()
                    .nullmap_len()
                    #(+ #len_terms)*)
            }
            fn encode(&self, out: &mut ::ergo_clickhouse_persist::persist::RowWriter<'_>) -> ::core::result::Result<(), ::ergo_clickhouse_persist::persist::EncodeError> {
                #(#encode_stmts)*
                Ok(())
            }
        }
    })
}

fn schema_entry(ty: &Type) -> TokenStream2 {
    let s = ty_string(ty);
    match s.as_str() {
        "bool" => {
            quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(::ergo_clickhouse_persist::schema::TypeCode::Bool) }
        }
        "i8" => {
            quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(::ergo_clickhouse_persist::schema::TypeCode::I8) }
        }
        "i16" => {
            quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(::ergo_clickhouse_persist::schema::TypeCode::I16) }
        }
        "i32" => {
            quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(::ergo_clickhouse_persist::schema::TypeCode::I32) }
        }
        "i64" => {
            quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(::ergo_clickhouse_persist::schema::TypeCode::I64) }
        }
        "u8" => {
            quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(::ergo_clickhouse_persist::schema::TypeCode::U8) }
        }
        "u16" => {
            quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(::ergo_clickhouse_persist::schema::TypeCode::U16) }
        }
        "u32" => {
            quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(::ergo_clickhouse_persist::schema::TypeCode::U32) }
        }
        "u64" => {
            quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(::ergo_clickhouse_persist::schema::TypeCode::U64) }
        }
        "f32" => {
            quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(::ergo_clickhouse_persist::schema::TypeCode::F32) }
        }
        "f64" => {
            quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(::ergo_clickhouse_persist::schema::TypeCode::F64) }
        }
        _ => {
            if s.starts_with("Option<") {
                let inner = s.trim_start_matches("Option<").trim_end_matches('>');
                let code = scalar_code(inner);
                quote! { ::ergo_clickhouse_persist::schema::ValueSchema::optional(::ergo_clickhouse_persist::schema::TypeCode::#code) }
            } else if s.starts_with("Vec<") || s.contains('[') {
                let inner = s.trim_start_matches("Vec<").trim_end_matches('>');
                let code = scalar_code(inner);
                quote! { ::ergo_clickhouse_persist::schema::ValueSchema::array(::ergo_clickhouse_persist::schema::TypeCode::#code) }
            } else if s == "String" {
                quote! { ::ergo_clickhouse_persist::schema::ValueSchema::scalar(::ergo_clickhouse_persist::schema::TypeCode::Utf8) }
            } else if s == "Vec<u8>"
                || s == "Vec<i64>"
                || s == "Vec<u64>"
                || s == "Vec<i32>"
                || s == "Vec<u32>"
            {
                let code = scalar_code(s.trim_start_matches("Vec<").trim_end_matches('>'));
                quote! { ::ergo_clickhouse_persist::schema::ValueSchema::array(::ergo_clickhouse_persist::schema::TypeCode::#code) }
            } else {
                // PersistAs wrapper: adopt its declared value schema.
                quote! { *<#ty as ::ergo_clickhouse_persist::persist::PersistAs>::value_schema() }
            }
        }
    }
}

fn scalar_code(s: &str) -> TokenStream2 {
    match s {
        "bool" => quote! { Bool },
        "i8" => quote! { I8 },
        "i16" => quote! { I16 },
        "i32" => quote! { I32 },
        "i64" => quote! { I64 },
        "u8" => quote! { U8 },
        "u16" => quote! { U16 },
        "u32" => quote! { U32 },
        "u64" => quote! { U64 },
        "f32" => quote! { F32 },
        "f64" => quote! { F64 },
        _ => quote! { U64 },
    }
}

fn ty_string(ty: &Type) -> String {
    quote!(#ty).to_string().replace(' ', "")
}

fn len_term(ty: &Type, name: &Ident) -> TokenStream2 {
    let s = ty_string(ty);
    match s.as_str() {
        "bool" | "i8" | "u8" => quote! { 1usize },
        "i16" | "u16" => quote! { 2usize },
        "i32" | "u32" | "f32" => quote! { 4usize },
        "i64" | "u64" | "f64" => quote! { 8usize },
        _ => {
            if let Some(inner_s) = s.strip_prefix("Option<").map(|r| r.trim_end_matches('>')) {
                let inner: Type = syn::parse_str(inner_s).expect("type");
                if inner_s == "String" {
                    quote! { 4usize + self.#name.as_ref().map_or(0, ::std::string::String::len) }
                } else {
                    quote! { <#inner as ::ergo_clickhouse_persist::persist::ScalarWidth>::WIDTH }
                }
            } else if let Some(inner_s) = s.strip_prefix("Vec<").map(|r| r.trim_end_matches('>')) {
                let inner: Type = syn::parse_str(inner_s).expect("type");
                if inner_s == "String" {
                    quote! {{ let mut n = 4usize; for v in &self.#name { n += 4 + v.len(); } n }}
                } else {
                    quote! { 4usize + self.#name.len() * <#inner as ::ergo_clickhouse_persist::persist::ScalarWidth>::WIDTH }
                }
            } else if s == "String" {
                quote! { 4usize + self.#name.len() }
            } else {
                quote! { <#ty as ::ergo_clickhouse_persist::persist::PersistAs>::encoded_len(&self.#name)? }
            }
        }
    }
}

fn encode_stmt(ty: &Type, name: &Ident, _index: usize) -> TokenStream2 {
    let s = ty_string(ty);
    match s.as_str() {
        "bool" => quote! { out.write_bool(self.#name)?; },
        "i8" => quote! { out.write_i8(self.#name)?; },
        "i16" => quote! { out.write_i16(self.#name)?; },
        "i32" => quote! { out.write_i32(self.#name)?; },
        "i64" => quote! { out.write_i64(self.#name)?; },
        "u8" => quote! { out.write_u8(self.#name)?; },
        "u16" => quote! { out.write_u16(self.#name)?; },
        "u32" => quote! { out.write_u32(self.#name)?; },
        "u64" => quote! { out.write_u64(self.#name)?; },
        "f32" => quote! { out.write_f32(self.#name)?; },
        "f64" => quote! { out.write_f64(self.#name)?; },
        _ => {
            if let Some(inner_s) = s.strip_prefix("Option<").map(|r| r.trim_end_matches('>')) {
                match inner_s {
                    "bool" => quote! { out.write_bool_opt(self.#name)?; },
                    "i64" => quote! { out.write_i64_opt(self.#name)?; },
                    "u64" => quote! { out.write_u64_opt(self.#name)?; },
                    "String" => quote! { out.write_str_opt(self.#name.as_deref())?; },
                    "i8" | "i16" | "i32" | "u8" | "u16" | "u32" | "f32" | "f64" => {
                        // `write_opt_le` takes its width from the declared
                        // column type, so the encoder and the schema agree.
                        // Going through `write_i64_opt` declared a `Nullable(
                        // Int8)` column but wrote 8 bytes, which the writer
                        // rejected as a type mismatch — every `Option<i8>` and
                        // its siblings failed to encode at all.
                        quote! {{
                            match self.#name {
                                Some(v) => out.write_opt_le(Some(&v.to_le_bytes()))?,
                                None => out.write_opt_le(None)?,
                            }
                        }}
                    }
                    _ => {
                        let e = format!("unsupported Option inner type: {inner_s}");
                        quote! { ::core::compile_error!(#e); }
                    }
                }
            } else if let Some(inner_s) = s.strip_prefix("Vec<").map(|r| r.trim_end_matches('>')) {
                if inner_s == "String" {
                    quote! {{
                        // length-prefixed sequence of strings (Bytes column)
                        let mut blob = ::std::vec::Vec::new();
                        for v in &self.#name { blob.extend_from_slice(v.as_bytes()); }
                        out.write_bytes(&blob)?;
                    }}
                } else {
                    let code = scalar_code(inner_s);
                    let width: usize = match inner_s {
                        "i8" | "u8" | "bool" => 1,
                        "i16" | "u16" => 2,
                        "i32" | "u32" | "f32" => 4,
                        _ => 8,
                    };
                    quote! {{
                        let mut le = ::std::vec::Vec::new();
                        for v in &self.#name { le.extend_from_slice(&v.to_le_bytes()); }
                        out.write_array(::ergo_clickhouse_persist::schema::TypeCode::#code, #width, &le, self.#name.len())?;
                    }}
                }
            } else if s == "String" {
                quote! { out.write_str(&self.#name)?; }
            } else if s == "Vec<u8>" {
                quote! { out.write_bytes(&self.#name)?; }
            } else {
                quote! { out.write_persist_as(&self.#name)?; }
            }
        }
    }
}

/// Declare a typed callsite table.
///
/// The table name, policy (`temporary` or `permanent`) and column list are
/// fixed at compile time; `prepare` then resolves them against the session's
/// catalog once, on the control thread.
///
/// ```
/// use ergo_clickhouse_persist_derive::persist_table;
///
/// persist_table! {
///     static BOOK_DEBUG: temporary "book_debug" {
///         instrument: u32,
///         update_id: u64,
///     }
/// }
/// # let _ = BOOK_DEBUG;
/// ```
#[proc_macro]
pub fn persist_table(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as TableDecl);
    expand_table(input).into()
}

struct TableDecl {
    static_name: Ident,
    policy: Ident,
    table: LitStr,
    fields: Vec<(Ident, Type)>,
}

impl syn::parse::Parse for TableDecl {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        input.parse::<Token![static]>()?;
        let static_name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let policy: Ident = input.parse()?;
        let table: LitStr = input.parse()?;
        let content;
        syn::braced!(content in input);
        let mut fields = Vec::new();
        while !content.is_empty() {
            let name: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let ty: Type = content.parse()?;
            fields.push((name, ty));
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        Ok(Self {
            static_name,
            policy,
            table,
            fields,
        })
    }
}

use syn::Token;

fn expand_table(d: TableDecl) -> TokenStream2 {
    let static_name = &d.static_name;
    let table = &d.table;
    let field_types: Vec<&Type> = d.fields.iter().map(|(_, t)| t).collect();
    let schema_name = format_ident!("{}__SCHEMA", static_name);
    let schema_entries = field_types.iter().map(|t| schema_entry(t));
    let is_temporary = d.policy == "temporary";
    let type_name = format_ident!("{}Type", static_name);
    quote! {
        #[doc = concat!("Prepared-callsite table declaration for `", #table, "`.")]
        pub static #schema_name: ::ergo_clickhouse_persist::schema::RowSchema =
            ::ergo_clickhouse_persist::schema::RowSchema {
                columns: &[ #(#schema_entries),* ],
            };

        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy)]
        pub struct #type_name;

        impl ::ergo_clickhouse_persist::persist::Persistable for #type_name {
            fn schema() -> &'static ::ergo_clickhouse_persist::schema::RowSchema { &#schema_name }
            fn encoded_len(&self) -> ::core::result::Result<usize, ::ergo_clickhouse_persist::persist::EncodeError> { Ok(0) }
            fn encode(&self, _out: &mut ::ergo_clickhouse_persist::persist::RowWriter<'_>) -> ::core::result::Result<(), ::ergo_clickhouse_persist::persist::EncodeError> { Ok(()) }
        }

        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy)]
        pub struct #static_name {
            _m: [(); 0],
        }

        impl #static_name {
            pub const TABLE: &'static str = #table;
            #[allow(clippy::new_without_default)]
            pub const fn new() -> Self {
                Self { _m: [] }
            }
            /// Declared lifetime class.
            pub const TEMPORARY: bool = #is_temporary;
            /// Prepare the callsite against a session (control thread),
            /// defaulting to the declared policy.
            pub fn prepare(
                &self,
                session: &mut ::ergo_clickhouse_persist::registration::RecorderSession,
            ) -> ::core::result::Result<::ergo_clickhouse_persist::recorder::PreparedTable<#type_name>, ::ergo_clickhouse_persist::registration::RegistrationError> {
                let policy = if Self::TEMPORARY {
                    ::ergo_clickhouse_persist::protocol::Policy::Temporary
                } else {
                    ::ergo_clickhouse_persist::protocol::Policy::Permanent
                };
                session.table_schema_typed::<#type_name>(#table, policy, &#schema_name)
            }
        }

        /// Callsite value used by `persist_event!` / `persist_sbe!` /
        /// `persist_dto!`.
        #[allow(non_upper_case_globals)]
        pub const #static_name: #static_name = #static_name::new();
    }
}

/// Strict-lazy event recording: the enable word is read first; when
/// disabled, no value expression is evaluated.
///
/// The handle's enable word is loaded once; on the disabled path the payload
/// expressions are not evaluated at all, so an expensive computed column
/// costs nothing while recording is switched off.
///
/// ```
/// use ergo_clickhouse_persist::persist::RecordOutcome;
/// use ergo_clickhouse_persist::registration::{RecorderConfig, RecorderSession, TransportConfig};
/// use ergo_clickhouse_persist_derive::{persist_event, persist_table};
///
/// persist_table! {
///     static DEBUG: temporary "debug" {
///         instrument: u32,
///         buffered: u64,
///     }
/// }
///
/// let mut session = RecorderSession::connect(RecorderConfig {
///     process: "example".into(),
///     instance: "doc-0".into(),
///     build: "doc".into(),
///     permanent: TransportConfig::Memory { slots: 8, slot_bytes: 4096 },
///     diagnostics: TransportConfig::Memory { slots: 8, slot_bytes: 4096 },
///     ..RecorderConfig::default()
/// })?;
/// session.declare_session_start()?;
/// let handle = DEBUG.prepare(&mut session)?;
/// let mut writer = session.writer()?;
///
/// let mut expensive_calls = 0u32;
/// let mut expensive = |calls: &mut u32| { *calls += 1; 7u64 };
///
/// // Disabled: `expensive` is never called.
/// let outcome = persist_event!(
///     writer,
///     handle,
///     at = 1,
///     instrument = 42u32,
///     buffered = expensive(&mut expensive_calls),
/// );
/// assert_eq!(outcome, RecordOutcome::Disabled);
/// assert_eq!(expensive_calls, 0, "the disabled path must not evaluate payloads");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[proc_macro]
pub fn persist_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as EventCall);
    expand_event(input).into()
}

struct EventCall {
    writer: Ident,
    table: Ident,
    at: syn::Expr,
    fields: Vec<(Ident, syn::Expr)>,
}

impl syn::parse::Parse for EventCall {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let writer: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let table: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let at_key: Ident = input.parse()?;
        if at_key != "at" {
            return Err(syn::Error::new(at_key.span(), "expected `at = <expr>`"));
        }
        input.parse::<Token![=]>()?;
        let at: syn::Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let mut fields = Vec::new();
        while !input.is_empty() {
            let name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: syn::Expr = input.parse()?;
            fields.push((name, value));
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self {
            writer,
            table,
            at,
            fields,
        })
    }
}

fn expand_event(e: EventCall) -> TokenStream2 {
    let writer = &e.writer;
    let table = &e.table;
    let at = &e.at;
    let field_writes = e.fields.iter().enumerate().map(|(i, (_, v))| {
        // Field order is the declared callsite order; values evaluate only
        // on the enabled path.
        quote! { ::ergo_clickhouse_persist::persist::EventField::write_field(&#v, out, #i)?; }
    });
    quote! {{
        let __handle = &#table;
        if !::ergo_clickhouse_persist::recorder::EnableSlot::is_enabled_word(__handle.slot().load()) {
            ::ergo_clickhouse_persist::persist::RecordOutcome::Disabled
        } else {
            let __at: u64 = #at;
            #writer.record_with(__handle, __at, |out| {
                #(#field_writes)*
                Ok(())
            })
        }
    }}
}

/// Strict-lazy raw-SBE recording with typed extras.
#[proc_macro]
pub fn persist_sbe(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as SbeCall);
    let writer = &input.writer;
    let table = &input.table;
    let at = &input.at;
    let message = &input.message;
    let extra = &input.extra;
    quote! {{
        let __handle = &#table;
        if !::ergo_clickhouse_persist::recorder::EnableSlot::is_enabled_word(__handle.slot().load()) {
            ::ergo_clickhouse_persist::persist::RecordOutcome::Disabled
        } else {
            let __at: u64 = #at;
            let __msg: &[u8] = &{ #message };
            let __extra = #extra;
            #writer.record_sbe(__handle, __msg, &__extra, __at)
        }
    }}.into()
}

struct SbeCall {
    writer: Ident,
    table: Ident,
    at: syn::Expr,
    message: syn::Expr,
    extra: syn::Expr,
}

impl syn::parse::Parse for SbeCall {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let writer: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let table: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let mut keys: Vec<Ident> = Vec::new();
        let mut exprs: Vec<syn::Expr> = Vec::new();
        while !input.is_empty() {
            let k: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let v: syn::Expr = input.parse()?;
            keys.push(k);
            exprs.push(v);
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        if keys.len() != 3 || keys[0] != "at" || keys[1] != "message" || keys[2] != "extra" {
            return Err(syn::Error::new(
                keys.first()
                    .map(|k| k.span())
                    .unwrap_or(proc_macro2::Span::call_site()),
                "expected at = ..., message = ..., extra = ...",
            ));
        }
        Ok(Self {
            writer,
            table,
            at: exprs[0].clone(),
            message: exprs[1].clone(),
            extra: exprs[2].clone(),
        })
    }
}

/// Strict-lazy DTO recording (borrowed, no Debug serialization).
#[proc_macro]
pub fn persist_dto(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DtoCall);
    let writer = &input.writer;
    let table = &input.table;
    let at = &input.at;
    let value = &input.value;
    quote! {{
        let __handle = &#table;
        if !::ergo_clickhouse_persist::recorder::EnableSlot::is_enabled_word(__handle.slot().load()) {
            ::ergo_clickhouse_persist::persist::RecordOutcome::Disabled
        } else {
            let __at: u64 = #at;
            let __value = #value;
            #writer.record_with(__handle, __at, |out| ::ergo_clickhouse_persist::persist::Persistable::encode(&__value, out))
        }
    }}.into()
}

struct DtoCall {
    writer: Ident,
    table: Ident,
    at: syn::Expr,
    value: syn::Expr,
}

impl syn::parse::Parse for DtoCall {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let writer: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let table: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let mut keys: Vec<Ident> = Vec::new();
        let mut exprs: Vec<syn::Expr> = Vec::new();
        while !input.is_empty() {
            let k: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let v: syn::Expr = input.parse()?;
            keys.push(k);
            exprs.push(v);
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        if keys.len() != 2 || keys[0] != "at" || keys[1] != "value" {
            return Err(syn::Error::new(
                keys.first()
                    .map(|k| k.span())
                    .unwrap_or(proc_macro2::Span::call_site()),
                "expected at = ..., value = ...",
            ));
        }
        Ok(Self {
            writer,
            table,
            at: exprs[0].clone(),
            value: exprs[1].clone(),
        })
    }
}
