//! er͏go-clickhouse-persist-derive — todo 04.
//!
//! Proc-macro for `#[derive(Persist)]`.
//! Generates `Persist` trait impls from annotated struct definitions.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Data, DeriveInput, Field, Fields, GenericArgument, GenericParam, Ident, LitStr, PathArguments,
    Type, parse_macro_input,
};

// ── Parsed annotations ───────────────────────────────────────────────────────

#[derive(Default)]
struct StructAttrs {
    order_by: Option<String>,
}

#[derive(Default)]
#[allow(clippy::struct_excessive_bools)]
struct FieldAttrs {
    skip: bool,
    json: bool,
    array: bool,
    flatten: bool,
    custom_name: Option<String>,
    type_override: Option<String>,
}

// ── Attribute parsing ────────────────────────────────────────────────────────

fn parse_struct_attrs(attrs: &[syn::Attribute]) -> syn::Result<StructAttrs> {
    let mut result = StructAttrs::default();
    for attr in attrs {
        if !attr.path().is_ident("persist") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("order_by") {
                let value: LitStr = meta.value()?.parse()?;
                result.order_by = Some(value.value());
            } else {
                return Err(meta.error("unknown persist struct attribute; expected `order_by`"));
            }
            Ok(())
        })?;
    }
    Ok(result)
}

fn parse_field_attrs(attrs: &[syn::Attribute]) -> syn::Result<FieldAttrs> {
    let mut result = FieldAttrs::default();
    for attr in attrs {
        if !attr.path().is_ident("persist") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                result.skip = true;
            } else if meta.path.is_ident("flatten") {
                result.flatten = true;
            } else if meta.path.is_ident("json") {
                result.json = true;
            } else if meta.path.is_ident("array") {
                result.array = true;
            } else if meta.path.is_ident("name") {
                let value: LitStr = meta.value()?.parse()?;
                result.custom_name = Some(value.value());
            } else if meta.path.is_ident("type") {
                let value: LitStr = meta.value()?.parse()?;
                result.type_override = Some(value.value());
            } else {
                return Err(
                    meta.error("unknown persist attribute; expected `skip`, `flatten`, `json`, `array`, `name`, or `type`"),
                );
            }
            Ok(())
        })?;
    }
    Ok(result)
}

// ── Type introspection helpers ───────────────────────────────────────────────

/// Return the top-level type identifier, e.g. `"Option"` from `Option<u64>`.
fn type_ident(ty: &Type) -> Option<&Ident> {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.first().map(|s| &s.ident)
    } else {
        None
    }
}

/// Extract the first generic argument from a type like `Option<u64>`.
fn first_generic_arg(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner);
    }
    None
}

fn is_option(ty: &Type) -> bool {
    type_ident(ty).is_some_and(|id| id == "Option")
}

fn is_vec(ty: &Type) -> bool {
    type_ident(ty).is_some_and(|id| id == "Vec")
}

/// Checks if a type is `Vec<u8>` specifically.
fn is_vec_u8(ty: &Type) -> bool {
    if !is_vec(ty) {
        return false;
    }
    first_generic_arg(ty).is_some_and(|inner| type_ident(inner).is_some_and(|id| id == "u8"))
}

fn option_inner(ty: &Type) -> Option<&Type> {
    if is_option(ty) {
        first_generic_arg(ty)
    } else {
        None
    }
}

fn vec_inner(ty: &Type) -> Option<&Type> {
    if is_vec(ty) {
        first_generic_arg(ty)
    } else {
        None
    }
}

/// Check whether `ty` references any type parameter from `generics`.
fn type_uses_generics(ty: &Type, generics: &syn::Generics) -> bool {
    match ty {
        Type::Path(type_path) => {
            for segment in &type_path.path.segments {
                if generics
                    .params
                    .iter()
                    .any(|p| matches!(p, GenericParam::Type(tp) if tp.ident == segment.ident))
                {
                    return true;
                }
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    for arg in &args.args {
                        if let GenericArgument::Type(inner_ty) = arg
                            && type_uses_generics(inner_ty, generics)
                        {
                            return true;
                        }
                    }
                }
            }
            false
        }
        _ => false,
    }
}

// ── ColumnType expression generation ────────────────────────────────────────

/// Parse a `ClickHouse` type string (from `#[persist(type = "...")]`) into a
/// `ColumnType` expression.
fn parse_column_type_string(s: &str) -> TokenStream2 {
    let map_err = |s: &str| -> TokenStream2 {
        quote! {
            compile_error!(concat!("invalid ClickHouse type in #[persist(type = "...")]: ", #s))
        }
    };

    match s {
        "Int8" => return quote! { ergo_clickhouse_persist::ColumnType::Int8 },
        "Int16" => return quote! { ergo_clickhouse_persist::ColumnType::Int16 },
        "Int32" => return quote! { ergo_clickhouse_persist::ColumnType::Int32 },
        "Int64" => return quote! { ergo_clickhouse_persist::ColumnType::Int64 },
        "UInt8" => return quote! { ergo_clickhouse_persist::ColumnType::UInt8 },
        "UInt16" => return quote! { ergo_clickhouse_persist::ColumnType::UInt16 },
        "UInt32" => return quote! { ergo_clickhouse_persist::ColumnType::UInt32 },
        "UInt64" => return quote! { ergo_clickhouse_persist::ColumnType::UInt64 },
        "Float32" => return quote! { ergo_clickhouse_persist::ColumnType::Float32 },
        "Float64" => return quote! { ergo_clickhouse_persist::ColumnType::Float64 },
        "String" => return quote! { ergo_clickhouse_persist::ColumnType::String },
        "Bool" => return quote! { ergo_clickhouse_persist::ColumnType::Bool },
        "Json" => return quote! { ergo_clickhouse_persist::ColumnType::Json },
        "Date" => return quote! { ergo_clickhouse_persist::ColumnType::Date },
        "Interval" => return quote! { ergo_clickhouse_persist::ColumnType::Interval },
        _ => {}
    }

    if let Some(inner) = s
        .strip_prefix("Nullable(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let inner_ts = parse_column_type_string(inner);
        return quote! {
            ergo_clickhouse_persist::ColumnType::Nullable(Box::new(#inner_ts))
        };
    }

    if let Some(inner) = s.strip_prefix("Array(").and_then(|s| s.strip_suffix(')')) {
        let inner_ts = parse_column_type_string(inner);
        return quote! {
            ergo_clickhouse_persist::ColumnType::Array(Box::new(#inner_ts))
        };
    }

    if let Some(rest) = s.strip_prefix("Decimal(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = rest.splitn(2, ',').collect();
        if parts.len() == 2 {
            let p: u8 = match parts[0].trim().parse() {
                Ok(n) => n,
                Err(_) => return map_err(s),
            };
            let scale: u8 = match parts[1].trim().parse() {
                Ok(n) => n,
                Err(_) => return map_err(s),
            };
            return quote! {
                ergo_clickhouse_persist::ColumnType::Decimal { precision: #p, scale: #scale }
            };
        }
        return map_err(s);
    }

    if let Some(rest) = s
        .strip_prefix("DateTime(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return rest.trim().parse::<u8>().map_or_else(
            |_| map_err(s),
            |p| quote! { ergo_clickhouse_persist::ColumnType::DateTime(#p) },
        );
    }

    if let Some(rest) = s
        .strip_prefix("DateTime64(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return rest.trim().parse::<u8>().map_or_else(
            |_| map_err(s),
            |p| quote! { ergo_clickhouse_persist::ColumnType::DateTime64(#p) },
        );
    }

    if let Some(rest) = s
        .strip_prefix("FixedString(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return rest.trim().parse::<usize>().map_or_else(
            |_| map_err(s),
            |n| quote! { ergo_clickhouse_persist::ColumnType::FixedString(#n) },
        );
    }

    map_err(s)
}

/// Generate a `ColumnType` expression for the given type and field attributes.
///
/// Resolution chain:
/// 1. `#[persist(type = "...")]` — literal override
/// 2. `#[persist(json)]` — Json
/// 3. `Option<T>` → `Nullable(...)` (recurse)
/// 4. Known primitives (u64, String, etc.)
/// 5. `Vec<u8>` → String
/// 6. Generic type parameters → `default_column_type::<T>()` (resolvable for any 'static T)
/// 7. Concrete non-primitive types → `<T as PersistAs>::column_type()`
#[allow(clippy::only_used_in_recursion)]
fn column_type_expr(
    ty: &Type,
    attrs: &FieldAttrs,
    field_name: &str,
    generics: &syn::Generics,
) -> TokenStream2 {
    // 1. #[persist(type = "...")]
    if let Some(ref override_type) = attrs.type_override {
        return parse_column_type_string(override_type);
    }
    // 2. #[persist(json)]
    if attrs.json {
        return quote! { ergo_clickhouse_persist::ColumnType::Json };
    }
    // 3. Option<T> → Nullable(...)
    if let Some(inner) = option_inner(ty) {
        let inner_ct = column_type_expr(inner, &FieldAttrs::default(), field_name, generics);
        return quote! {
            ergo_clickhouse_persist::ColumnType::Nullable(Box::new(#inner_ct))
        };
    }
    // 4. Known primitives
    if let Some(ident) = type_ident(ty) {
        let known = match ident.to_string().as_str() {
            "u8" => Some(quote! { ergo_clickhouse_persist::ColumnType::UInt8 }),
            "u16" => Some(quote! { ergo_clickhouse_persist::ColumnType::UInt16 }),
            "u32" => Some(quote! { ergo_clickhouse_persist::ColumnType::UInt32 }),
            "u64" => Some(quote! { ergo_clickhouse_persist::ColumnType::UInt64 }),
            "i8" => Some(quote! { ergo_clickhouse_persist::ColumnType::Int8 }),
            "i16" => Some(quote! { ergo_clickhouse_persist::ColumnType::Int16 }),
            "i32" => Some(quote! { ergo_clickhouse_persist::ColumnType::Int32 }),
            "i64" => Some(quote! { ergo_clickhouse_persist::ColumnType::Int64 }),
            "f32" => Some(quote! { ergo_clickhouse_persist::ColumnType::Float32 }),
            "f64" => Some(quote! { ergo_clickhouse_persist::ColumnType::Float64 }),
            "bool" => Some(quote! { ergo_clickhouse_persist::ColumnType::Bool }),
            "String" | "str" => Some(quote! { ergo_clickhouse_persist::ColumnType::String }),
            _ => None,
        };
        if let Some(ts) = known {
            return ts;
        }
    }
    // 5. Vec<u8> → String
    if is_vec_u8(ty) {
        return quote! { ergo_clickhouse_persist::ColumnType::String };
    }
    // 6. If the type references a generic parameter, use default_column_type
    //    (handles primitives at runtime, falls back to Json for unknown types).
    if type_uses_generics(ty, generics) {
        return quote! { ergo_clickhouse_persist::default_column_type::<#ty>() };
    }
    // 7. Concrete non-primitive → use PersistAs impl (chrono, rust_decimal, etc.)
    quote! { <#ty as ergo_clickhouse_persist::PersistAs>::column_type() }
}

/// Generate the column type for the inner element of a `#[persist(array)]` field.
#[allow(clippy::only_used_in_recursion)]
fn array_inner_column_type_expr(
    inner: &Type,
    inner_attrs: &FieldAttrs,
    generics: &syn::Generics,
) -> TokenStream2 {
    // If inner is Option<T>, wrap in Nullable
    if let Some(inner_inner) = option_inner(inner) {
        let inner_ts = array_inner_column_type_expr(inner_inner, inner_attrs, generics);
        return quote! {
            ergo_clickhouse_persist::ColumnType::Nullable(Box::new(#inner_ts))
        };
    }
    // If inner is a known primitive, return directly
    if let Some(ident) = type_ident(inner) {
        let known = match ident.to_string().as_str() {
            "u8" => Some(quote! { ergo_clickhouse_persist::ColumnType::UInt8 }),
            "u16" => Some(quote! { ergo_clickhouse_persist::ColumnType::UInt16 }),
            "u32" => Some(quote! { ergo_clickhouse_persist::ColumnType::UInt32 }),
            "u64" => Some(quote! { ergo_clickhouse_persist::ColumnType::UInt64 }),
            "i8" => Some(quote! { ergo_clickhouse_persist::ColumnType::Int8 }),
            "i16" => Some(quote! { ergo_clickhouse_persist::ColumnType::Int16 }),
            "i32" => Some(quote! { ergo_clickhouse_persist::ColumnType::Int32 }),
            "i64" => Some(quote! { ergo_clickhouse_persist::ColumnType::Int64 }),
            "f32" => Some(quote! { ergo_clickhouse_persist::ColumnType::Float32 }),
            "f64" => Some(quote! { ergo_clickhouse_persist::ColumnType::Float64 }),
            "bool" => Some(quote! { ergo_clickhouse_persist::ColumnType::Bool }),
            "String" | "str" => Some(quote! { ergo_clickhouse_persist::ColumnType::String }),
            _ => None,
        };
        if let Some(ts) = known {
            return ts;
        }
    }
    // If Vec<u8>, that's a string-like array — use String
    if is_vec_u8(inner) {
        return quote! { ergo_clickhouse_persist::ColumnType::String };
    }
    // For struct types, use default_column_type (handles primitives at runtime,
    // falls back to Json — array inner column doesn't need PersistAs).
    quote! { ergo_clickhouse_persist::default_column_type::<#inner>() }
}

// ── Schema column generation ─────────────────────────────────────────────────

struct FieldInfo {
    ident: Ident,
    field_type: Type,
    attrs: FieldAttrs,
}

impl FieldInfo {
    fn from_field(field: &Field) -> syn::Result<Self> {
        let ident = field
            .ident
            .clone()
            .ok_or_else(|| syn::Error::new_spanned(field, "Persist requires named fields"))?;
        let attrs = parse_field_attrs(&field.attrs)?;
        Ok(Self {
            ident,
            field_type: field.ty.clone(),
            attrs,
        })
    }
}

/// Generate schema column-push statements for the field list, wrapped in a
/// block that returns `columns: Vec<ergo_clickhouse_persist::ColumnDef>`.
fn generate_schema_body(
    fields: &[FieldInfo],
    struct_attrs: &StructAttrs,
    generics: &syn::Generics,
) -> TokenStream2 {
    let col_stmts: Vec<TokenStream2> = fields
        .iter()
        .flat_map(|f| field_to_schema_columns(f, generics))
        .collect();

    let order_by = struct_attrs.order_by.as_ref().map_or_else(
        || quote! { vec![] },
        |ob| {
            let parts: Vec<TokenStream2> = ob
                .split(',')
                .map(|s| {
                    let trimmed = s.trim();
                    quote! { #trimmed.into() }
                })
                .collect();
            quote! { vec![#(#parts),*] }
        },
    );

    quote! {{
        let mut columns: Vec<ergo_clickhouse_persist::ColumnDef> = Vec::new();
        #(#col_stmts)*
        ergo_clickhouse_persist::TableSchema::new(columns, #order_by)
    }}
}

/// Convert one field into one or more `ColumnDef` push statements.
fn field_to_schema_columns(field: &FieldInfo, generics: &syn::Generics) -> Vec<TokenStream2> {
    // ── skip ──
    if field.attrs.skip {
        return vec![];
    }

    let default_name = field.ident.to_string();
    let col_name = field.attrs.custom_name.as_deref().unwrap_or(&default_name);

    // ── flatten ──
    if field.attrs.flatten {
        let inner_ty = &field.field_type;
        let prefix = &field.ident.to_string();
        return vec![quote! {{
            let inner = <#inner_ty as ergo_clickhouse_persist::Persist>::table_schema();
            for col in inner.columns {
                // Skip _persist_time — it is auto-added by TableSchema::new.
                if col.name == "_persist_time" { continue; }
                columns.push(ergo_clickhouse_persist::ColumnDef {
                    name: format!("{}_{}", #prefix, col.name),
                    col_type: col.col_type,
                });
            }
        }}];
    }

    // ── array ──
    if field.attrs.array {
        if let Some(inner) = vec_inner(&field.field_type) {
            // If inner is a known scalar type, single Array column
            // If inner is a known scalar type or Option<T> wrapping a known scalar
            let is_scalar = is_known_primitive(inner)
                || is_vec_u8(inner)
                || option_inner(inner).is_some_and(is_known_primitive);
            if is_scalar {
                // single column
                let inner_ct =
                    array_inner_column_type_expr(inner, &FieldAttrs::default(), generics);
                return vec![quote! {{
                    columns.push(ergo_clickhouse_persist::ColumnDef {
                        name: #col_name.into(),
                        col_type: ergo_clickhouse_persist::ColumnType::Array(Box::new(#inner_ct)),
                    });
                }}];
            }
            // Inner is a struct — generate one Array column per field of inner
            return vec![quote! {{
                let inner_schema = <#inner as ergo_clickhouse_persist::Persist>::table_schema();
                for col in inner_schema.columns {
                    columns.push(ergo_clickhouse_persist::ColumnDef {
                        name: format!("{}_{}", #col_name, col.name),
                        col_type: ergo_clickhouse_persist::ColumnType::Array(Box::new(col.col_type)),
                    });
                }
            }}];
        }
        // #[persist(array)] on a non-Vec field — error
        return vec![quote! {{
            compile_error!(concat!("#[persist(array)] requires a Vec field"));
        }}];
    }

    // ── Normal scalar field ──
    let ct = column_type_expr(
        &field.field_type,
        &field.attrs,
        &field.ident.to_string(),
        generics,
    );
    vec![quote! {{
        columns.push(ergo_clickhouse_persist::ColumnDef {
            name: #col_name.into(),
            col_type: #ct,
        });
    }}]
}

fn is_known_primitive(ty: &Type) -> bool {
    type_ident(ty).is_some_and(|id| {
        matches!(
            id.to_string().as_str(),
            "u8" | "u16"
                | "u32"
                | "u64"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "f32"
                | "f64"
                | "bool"
                | "String"
                | "str"
        )
    })
}

// ── Encode-row generation ────────────────────────────────────────────────────

fn generate_encode_body(fields: &[FieldInfo]) -> TokenStream2 {
    // Note: `_persist_time` is just cloned here; the main function adds an
    // override with `chrono::Utc::now()` if the field exists.
    let stmts: Vec<TokenStream2> = fields
        .iter()
        .filter(|f| !f.attrs.skip)
        .map(|f| {
            let ident = &f.ident;
            quote! { row.#ident = self.#ident.clone(); }
        })
        .collect();

    quote! {{
        #(#stmts)*
    }}
}

// ── Main entry point ─────────────────────────────────────────────────────────

/// Derive the [`Persist`] trait for a struct.
///
/// Generates `table_schema()` and `encode_row()` for the annotated struct.
///
/// # Attributes
///
/// - `#[persist(order_by = "col1, col2")]` — struct-level ORDER BY
/// - `#[persist(name = "custom")]` — override column name
/// - `#[persist(skip)]` — exclude field
/// - `#[persist(json)]` — JSON column type
/// - `#[persist(flatten)]` — inline nested struct fields
/// - `#[persist(array)]` — Vec<T> → Array columns
/// - `#[persist(type = "Decimal(18,2)")]` — override `ClickHouse` type
#[proc_macro_derive(Persist, attributes(persist))]
pub fn derive_persist(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Must be a struct
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return syn::Error::new_spanned(name, "Persist requires named fields")
                    .to_compile_error()
                    .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(name, "Persist can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    // Parse attributes
    let struct_attrs = match parse_struct_attrs(&input.attrs) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    let field_infos: Vec<FieldInfo> = match fields.iter().map(FieldInfo::from_field).collect() {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    // Check if struct has a _persist_time field so we can auto-set it
    let has_persist_time = field_infos
        .iter()
        .any(|f| !f.attrs.skip && f.ident == "_persist_time");

    // Generate bodies
    let schema_body = generate_schema_body(&field_infos, &struct_attrs, &input.generics);
    let encode_body = generate_encode_body(&field_infos);
    let persist_time_assignment = if has_persist_time {
        quote! {
            row._persist_time = chrono::Utc::now();
        }
    } else {
        TokenStream2::new()
    };

    let expanded = quote! {
        impl #impl_generics ergo_clickhouse_persist::Persist for #name #ty_generics #where_clause {
            fn table_schema() -> ergo_clickhouse_persist::TableSchema {
                #schema_body
            }

            fn encode_row(&self, row: &mut Self) {
                #encode_body
                #persist_time_assignment
            }
        }
    };

    expanded.into()
}
