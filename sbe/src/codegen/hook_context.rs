//! Decoder-side hook contexts: block field lists with the generated accessor
//! names and return types, and the group-entry walk.
//!
//! Accessor *names* come from the same helpers the emitters call; accessor
//! *types* are read back from the generated decoder's token stream, so a hook
//! context cannot drift from the code it describes. Build time only, and only
//! when a hook is registered.

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::ToTokens;

use super::conversion_helpers::{
    DECODER_RESERVED, constant_converts, domain_accessor_ident, field_has_conversion_free,
    find_domain_type, message_field_infos, resolve_field_ident,
};
use super::runtime::{to_pascal_case, to_snake_case};
use crate::ir::Presence;
use crate::structured_ir::{MessageField, MessageGroup, MessageStructure, MessageVarData};

/// Return types of the generated public `&self` decoder methods that take no
/// other argument, keyed by (impl self type, method name).
pub(crate) struct AccessorTable(HashMap<(String, String), String>);

impl AccessorTable {
    /// Read every inherent `impl` in `decoder` (a message decoder and its
    /// group decoders). An unparsable stream gives an empty table: every
    /// `accessor_type` is then `None`, never guessed.
    pub(crate) fn from_tokens(decoder: &TokenStream) -> Self {
        let mut map = HashMap::new();
        let Ok(file) = syn::parse2::<syn::File>(decoder.clone()) else {
            return Self(map);
        };
        for item in &file.items {
            let syn::Item::Impl(imp) = item else { continue };
            if imp.trait_.is_some() {
                continue;
            }
            let syn::Type::Path(tp) = &*imp.self_ty else {
                continue;
            };
            let Some(owner) = tp.path.segments.last().map(|s| s.ident.to_string()) else {
                continue;
            };
            for it in &imp.items {
                let syn::ImplItem::Fn(f) = it else { continue };
                if !matches!(f.vis, syn::Visibility::Public(_)) || !f.sig.generics.params.is_empty()
                {
                    continue;
                }
                let mut inputs = f.sig.inputs.iter();
                let by_ref = matches!(
                    inputs.next(),
                    Some(syn::FnArg::Receiver(syn::Receiver {
                        kind: syn::ReceiverKind::Reference(_, _, None),
                        ..
                    }))
                );
                if !by_ref || inputs.next().is_some() {
                    continue;
                }
                let syn::ReturnType::Type(_, ty) = &f.sig.output else {
                    continue;
                };
                map.entry((owner.clone(), f.sig.ident.to_string()))
                    .or_insert_with(|| type_text(ty));
            }
        }
        Self(map)
    }

    fn get(&self, owner: &str, method: &str) -> Option<String> {
        self.0
            .get(&(owner.to_string(), method.to_string()))
            .cloned()
    }
}

/// A type as compact Rust source: `Option<u32>`, `[u8; 16]`, `&'a [u8]`.
fn type_text(ty: &syn::Type) -> String {
    const NO_SPACE_BEFORE: &[&str] = &["<", ">", ">>", ",", ";", "]", ")", "::"];
    const NO_SPACE_AFTER: &[&str] = &["<", "&", "[", "(", "::"];
    let raw = ty.to_token_stream().to_string();
    let mut out = String::with_capacity(raw.len());
    let mut prev = "";
    for piece in raw.split(' ').filter(|p| !p.is_empty()) {
        let space = !prev.is_empty()
            && (prev == ","
                || prev == ";"
                || !(NO_SPACE_BEFORE.contains(&piece) || NO_SPACE_AFTER.contains(&prev)));
        if space {
            out.push(' ');
        }
        out.push_str(piece);
        prev = piece;
    }
    out
}

/// What decides a decoder field's accessor name: the config's conversion and
/// domain-type selectors.
pub(crate) struct AccessorNaming<'a> {
    pub(crate) conv_sels: &'a [crate::ConversionSelector],
    pub(crate) domain_types: &'a [(crate::ConversionSelector, String)],
}

impl AccessorNaming<'_> {
    /// The decoder method that returns `f`: the domain getter when a domain
    /// type applies, else the raw accessor (`*_wire` under a conversion,
    /// `*_field` against `reserved`).
    fn fixed(&self, f: &MessageField, reserved: &[&str]) -> String {
        let snake = to_snake_case(&f.name);
        let domain = find_domain_type(f, self.domain_types).is_some()
            && (f.presence != Presence::Constant || constant_converts(f));
        if domain {
            return domain_accessor_ident(&snake, reserved).to_string();
        }
        let wire = field_has_conversion_free(f, self.conv_sels).then(|| format!("{snake}_wire"));
        resolve_field_ident(&snake, &wire, reserved).to_string()
    }
}

/// One block's members as hook rows: fixed fields (constants only when
/// `constants` is set), then groups, then var-data. `group_prefix` is the
/// prefix the generated child group types carry. With `decoder` set, each row
/// gets its accessor on that decoder type and the type `table` read for it.
pub(crate) struct Block<'a> {
    pub(crate) fields: &'a [MessageField],
    pub(crate) groups: &'a [MessageGroup],
    pub(crate) var_data: &'a [MessageVarData],
    pub(crate) group_prefix: &'a str,
}

/// Decoder context of one block: the type its methods live on, the names
/// they take, and the table their return types come from.
pub(crate) struct DecoderSide<'a> {
    pub(crate) owner: &'a str,
    pub(crate) reserved: &'a [&'a str],
    pub(crate) naming: &'a AccessorNaming<'a>,
    pub(crate) table: &'a AccessorTable,
}

pub(crate) fn block_fields(
    block: &Block<'_>,
    decoder: Option<&DecoderSide<'_>>,
) -> Vec<crate::FieldInfo> {
    // Decoder contexts list constants (they have accessors); others do not.
    let mut rows = message_field_infos(block.fields, &[], None, decoder.is_some());
    if let Some(d) = decoder {
        for (row, f) in rows.iter_mut().zip(block.fields) {
            let acc = d.naming.fixed(f, d.reserved);
            row.accessor_type = d.table.get(d.owner, &acc);
            row.accessor = Some(acc);
        }
    }
    for g in block.groups {
        let mut row = group_row(g, block.group_prefix);
        if let Some(d) = decoder {
            accessor_row(&mut row, d);
        }
        rows.push(row);
    }
    for vd in block.var_data {
        let mut row = data_row(vd);
        if let Some(d) = decoder {
            accessor_row(&mut row, d);
        }
        rows.push(row);
    }
    rows
}

/// Groups and var-data are never renamed: their accessor is the snake name.
fn accessor_row(row: &mut crate::FieldInfo, d: &DecoderSide<'_>) {
    row.accessor_type = d.table.get(d.owner, &row.name);
    row.accessor = Some(row.name.clone());
}

fn group_row(g: &MessageGroup, prefix: &str) -> crate::FieldInfo {
    crate::FieldInfo {
        name: to_snake_case(&g.name),
        rust_type: format!("{prefix}{}Decoder", to_pascal_case(&g.name)),
        kind: crate::FieldKind::Group,
        offset: None,
        since_version: g.since_version,
        semantic_type: None,
        presence: "required",
        null_value: None,
        deprecated: g.deprecated,
        description: g.description.clone(),
        extensions: g.extensions.clone(),
        schema_name: g.name.clone(),
        accessor: None,
        accessor_type: None,
    }
}

fn data_row(vd: &MessageVarData) -> crate::FieldInfo {
    crate::FieldInfo {
        name: to_snake_case(&vd.name),
        rust_type: "&[u8]".to_string(),
        kind: crate::FieldKind::Data,
        offset: None,
        since_version: vd.since_version,
        semantic_type: None,
        presence: "required",
        null_value: None,
        deprecated: vd.deprecated,
        description: vd.description.clone(),
        extensions: vd.extensions.clone(),
        schema_name: vd.name.clone(),
        accessor: None,
        accessor_type: None,
    }
}

/// The prefix a message's generated group types carry: the message name in a
/// multi-message schema, nothing otherwise.
pub(crate) fn group_prefix(msg: &MessageStructure, multi_message: bool) -> String {
    if multi_message {
        to_pascal_case(&msg.name)
    } else {
        String::new()
    }
}

/// The message's own block.
pub(crate) fn message_block<'m>(msg: &'m MessageStructure, prefix: &'m str) -> Block<'m> {
    Block {
        fields: &msg.fields,
        groups: &msg.groups,
        var_data: &msg.var_data,
        group_prefix: prefix,
    }
}

/// Every group entry of `msg`, depth-first in schema order, as a
/// [`crate::ItemContext::GroupEntryDecoder`].
pub(crate) fn entry_contexts<'s>(
    schema: &'s crate::Schema,
    msg: &MessageStructure,
    multi_message: bool,
    naming: &AccessorNaming<'_>,
    table: &AccessorTable,
) -> Vec<crate::ItemContext<'s>> {
    let prefix = group_prefix(msg, multi_message);
    let block = message_block(msg, &prefix);
    let owner = format!("{}Decoder", to_pascal_case(&msg.name));
    let side = DecoderSide {
        owner: &owner,
        reserved: DECODER_RESERVED,
        naming,
        table,
    };
    let message = crate::BlockInfo {
        decoder: owner.clone(),
        fields: block_fields(&block, Some(&side)),
    };
    let mut out = Vec::new();
    let mut enclosing = vec![message];
    for g in &msg.groups {
        walk(
            schema,
            g,
            &format!("{prefix}{}", to_pascal_case(&g.name)),
            &mut enclosing,
            naming,
            table,
            &mut out,
        );
    }
    out
}

fn walk<'s>(
    schema: &'s crate::Schema,
    g: &MessageGroup,
    scoped: &str,
    enclosing: &mut Vec<crate::BlockInfo>,
    naming: &AccessorNaming<'_>,
    table: &AccessorTable,
    out: &mut Vec<crate::ItemContext<'s>>,
) {
    let name = format!("{scoped}EntryDecoder");
    let block = Block {
        fields: &g.fields,
        groups: &g.groups,
        var_data: &g.var_data,
        group_prefix: scoped,
    };
    // Group entries do not rename against the static reserved list.
    let side = DecoderSide {
        owner: &name,
        reserved: &[],
        naming,
        table,
    };
    let fields = block_fields(&block, Some(&side));
    out.push(crate::ItemContext::GroupEntryDecoder {
        schema,
        extensions: g.extensions.clone(),
        name: name.clone(),
        group_decoder: format!("{scoped}Decoder"),
        fields: fields.clone(),
        enclosing: enclosing.clone(),
    });
    enclosing.push(crate::BlockInfo {
        decoder: name,
        fields,
    });
    for ng in &g.groups {
        walk(
            schema,
            ng,
            &format!("{scoped}{}", to_pascal_case(&ng.name)),
            enclosing,
            naming,
            table,
            out,
        );
    }
    enclosing.pop();
}
