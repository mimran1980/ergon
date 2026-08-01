use crate::ir::PrimitiveType;
use crate::structured_ir::FieldType;

use super::runtime::to_pascal_case;

pub(crate) fn field_type_ident(ft: &FieldType, span: proc_macro2::Span) -> syn::Type {
    match ft {
        FieldType::Primitive(pt, Some(len)) => {
            let elem: syn::Type = field_type_ident(&FieldType::Primitive(*pt, None), span);
            let n = syn::LitInt::new(&len.to_string(), span);
            syn::parse_quote!([#elem; #n])
        }
        FieldType::Primitive(pt, None) => match pt {
            PrimitiveType::Char | PrimitiveType::UInt8 => syn::parse_quote!(u8),
            PrimitiveType::Int8 => syn::parse_quote!(i8),
            PrimitiveType::Int16 => syn::parse_quote!(i16),
            PrimitiveType::Int32 => syn::parse_quote!(i32),
            PrimitiveType::Int64 => syn::parse_quote!(i64),
            PrimitiveType::UInt16 => syn::parse_quote!(u16),
            PrimitiveType::UInt32 => syn::parse_quote!(u32),
            PrimitiveType::UInt64 => syn::parse_quote!(u64),
            PrimitiveType::Float => syn::parse_quote!(f32),
            PrimitiveType::Double => syn::parse_quote!(f64),
        },
        FieldType::Composite { name, .. } => {
            let ident = syn::Ident::new(&to_pascal_case(name), span);
            syn::parse_quote!(#ident)
        }
        FieldType::Enum { name, .. } => {
            let ident = syn::Ident::new(&to_pascal_case(name), span);
            syn::parse_quote!(#ident)
        }
        FieldType::Set { name, .. } => {
            let ident = syn::Ident::new(&to_pascal_case(name), span);
            syn::parse_quote!(#ident)
        }
    }
}
