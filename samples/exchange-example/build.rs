//! Generate SBE codecs into `src/generated/` (gitignored) for IDE go-to-definition.

// The venue-neutral example hooks ergo-sbe's tests and book share.
#[path = "../../sbe/tests/common/ext_recipes.rs"]
mod ext_recipes;

// ANCHOR: json_label_hook
/// Emit `json_label()` for enums whose validValues all carry a `jsonValue`
/// extension mark; enums with partial or no coverage are left untouched, so
/// schemas without marks generate byte-identical code.
fn json_label_hook(ctx: &ergo_sbe::ItemContext) -> Vec<proc_macro2::TokenStream> {
    let ergo_sbe::ItemContext::Enum { name, variants, .. } = ctx else {
        return Vec::new();
    };
    let mut arms = Vec::new();
    for v in variants {
        let Some(label) = v.extensions.attr("jsonValue") else {
            return Vec::new(); // partial coverage: skip the whole enum
        };
        let var = quote::format_ident!("{}", v.name);
        arms.push(quote::quote! { Self::#var => #label, });
    }
    if arms.is_empty() {
        return Vec::new();
    }
    let ident = quote::format_ident!("{name}");
    // Generated enums carry a `NullVal` variant, hence the wildcard arm.
    vec![quote::quote! {
        impl #ident {
            pub fn json_label(&self) -> &'static str {
                match self { #(#arms)* _ => "UNKNOWN" }
            }
        }
    }]
}
// ANCHOR_END: json_label_hook

// ANCHOR: json_hook
/// Responses the JSON renderer covers: the public market-data endpoints. A
/// message outside this list gets no `write_json`, so a shape the hook does
/// not handle (floats, embedded messages) never reaches the generated code.
const JSON_MESSAGES: &[&str] = &[
    "ServerTimeResponse",
    "DepthResponse",
    "TradesResponse",
    "AggTradesResponse",
    "KlinesResponse",
    "AveragePriceResponse",
    "PriceTickerSymbolResponse",
    "PriceTickerResponse",
    "BookTickerSymbolResponse",
    "BookTickerResponse",
];

/// JSON the venue sends that its schema has no field for: each kline row
/// ends in an unused `"0"` ("Unused field. Ignore." in the REST docs).
const JSON_TRAILERS: &[(&str, &str)] = &[("KlinesResponseKlinesEntryDecoder", "\"0\"")];

/// Schema names every `exponent` mark in `message` points at. Those fields
/// exist only to scale a mantissa, so the JSON never shows them.
fn exponent_targets(schema: &ergo_sbe::Schema, message: &str) -> Vec<String> {
    use ergo_sbe::Signal;
    schema
        .ir
        .tokens
        .iter()
        .skip_while(|t| !(t.signal == Signal::BeginMessage && t.name == message))
        .take_while(|t| t.signal != Signal::EndMessage)
        .filter_map(|t| t.extensions.attr("exponent").map(str::to_string))
        .collect()
}

/// `write_json` on every covered response decoder and its group entries, built
/// only from the marks and the decoder accessors the hook context names:
/// `jsonPath` picks the key (`[]`: array position, `..`: unwrap the group),
/// `exponent` turns a mantissa into an exact decimal string (the exponent may
/// sit on an enclosing block), `jsonDefaultValue` fills an absent optional.
fn json_hook(ctx: &ergo_sbe::ItemContext) -> Result<Vec<proc_macro2::TokenStream>, String> {
    use ergo_sbe::ItemContext;
    use quote::{format_ident, quote};
    let (schema, owner, fields, enclosing) = match ctx {
        ItemContext::MessageDecoder {
            schema,
            name,
            fields,
            ..
        } => (*schema, name, fields, &[][..]),
        ItemContext::GroupEntryDecoder {
            schema,
            name,
            fields,
            enclosing,
            ..
        } => (*schema, name, fields, enclosing.as_slice()),
        _ => return Ok(vec![]),
    };
    let message = enclosing
        .first()
        .map_or(owner.as_str(), |b| b.decoder.as_str())
        .trim_end_matches("Decoder");
    if !JSON_MESSAGES.contains(&message) {
        return Ok(vec![]);
    }
    let hidden = exponent_targets(schema, message);
    let params: Vec<_> = (0..enclosing.len())
        .map(|i| format_ident!("e{i}"))
        .collect();
    let param_types: Vec<_> = enclosing
        .iter()
        .map(|b| ext_recipes::ident(&b.decoder))
        .collect::<Result<_, _>>()?;
    let mut keyed = Vec::new();
    let mut positional = Vec::new();
    let mut unwrapped = Vec::new();
    for f in fields {
        if f.kind() == ergo_sbe::FieldKind::Fixed && hidden.contains(&f.schema_name) {
            continue;
        }
        let value = json_value(f, fields, enclosing, &params)?;
        match f.extensions.attr("jsonPath") {
            Some("[]") => positional.push(quote! { sep(out); #value }),
            Some("..") => unwrapped.push(value),
            path => {
                let key = format!("\"{}\":", path.unwrap_or(&f.schema_name));
                keyed.push(quote! { sep(out); out.push_str(#key); #value });
            }
        }
    }
    let trailer = JSON_TRAILERS
        .iter()
        .find(|(o, _)| o == owner)
        .map(|(_, t)| quote! { out.push(','); out.push_str(#t); });
    let body = match (keyed.len(), positional.len(), unwrapped.len()) {
        (0, 0, 1) => unwrapped.remove(0),
        (_, 0, 0) => quote! {
            out.push('{');
            #(#keyed)*
            out.push('}');
        },
        (0, _, 0) => quote! {
            out.push('[');
            #(#positional)*
            #trailer
            out.push(']');
        },
        _ => return Err(format!("{owner}: mixed jsonPath shapes")),
    };
    // Comma-separate members: each value is preceded by one unless first.
    let body = quote! {
        let mut first = true;
        let mut sep = |out: &mut String| {
            if !core::mem::take(&mut first) { out.push(','); }
        };
        #body
    };
    let owner_ty = ext_recipes::ident(owner)?;
    let mut out = vec![quote! {
        impl<'a> #owner_ty<'a> {
            /// This block as the JSON the venue's REST API returns.
            #[allow(unused_mut, unused_variables, clippy::all)]
            pub fn write_json(
                &self,
                #(#params: &#param_types<'a>,)*
                out: &mut String,
            ) -> Result<(), crate::sbe_json::JsonError> {
                #body
                Ok(())
            }
        }
    }];
    if let ItemContext::GroupEntryDecoder { group_decoder, .. } = ctx {
        // Entries with var-data or nested groups iterate as `Result`.
        let fallible = fields
            .iter()
            .any(|f| f.kind() != ergo_sbe::FieldKind::Fixed);
        let entry = if fallible {
            quote! { entry? }
        } else {
            quote! { entry }
        };
        let group = ext_recipes::ident(group_decoder)?;
        out.push(quote! {
            impl<'a> #group<'a> {
                /// Every entry of this group as a JSON array.
                pub fn write_json(
                    self,
                    #(#params: &#param_types<'a>,)*
                    out: &mut String,
                ) -> Result<(), crate::sbe_json::JsonError> {
                    out.push('[');
                    for (i, entry) in self.enumerate() {
                        if i > 0 { out.push(','); }
                        #entry.write_json(#(#params,)* out)?;
                    }
                    out.push(']');
                    Ok(())
                }
            }
        });
    }
    Ok(out)
}

/// Tokens that write one member's JSON value.
fn json_value(
    f: &ergo_sbe::FieldInfo,
    fields: &[ergo_sbe::FieldInfo],
    enclosing: &[ergo_sbe::BlockInfo],
    params: &[proc_macro2::Ident],
) -> Result<proc_macro2::TokenStream, String> {
    use quote::quote;
    let acc = ext_recipes::ident(f.accessor.as_deref().ok_or("no accessor")?)?;
    let ty = f.accessor_type.as_deref().ok_or("no accessor type")?;
    let value = match f.kind() {
        ergo_sbe::FieldKind::Group => quote! { self.#acc()?.write_json(#(#params,)* self, out)?; },
        ergo_sbe::FieldKind::Data => quote! { crate::sbe_json::string(out, self.#acc()?)?; },
        ergo_sbe::FieldKind::Fixed => {
            let optional = ty.starts_with("Option<");
            let inner = ty.trim_start_matches("Option<").trim_end_matches('>');
            let write = if let Some(target) = f.extensions.attr("exponent") {
                let exponent = if let Some(t) = fields.iter().find(|t| t.schema_name == target) {
                    let ta = ext_recipes::ident(t.accessor.as_deref().ok_or("no accessor")?)?;
                    quote! { self.#ta() }
                } else {
                    let (i, t) = enclosing
                        .iter()
                        .enumerate()
                        .rev()
                        .find_map(|(i, b)| {
                            b.fields
                                .iter()
                                .find(|t| t.schema_name == target)
                                .map(|t| (i, t))
                        })
                        .ok_or_else(|| format!("{}: exponent {target} not found", f.schema_name))?;
                    let ta = ext_recipes::ident(t.accessor.as_deref().ok_or("no accessor")?)?;
                    let p = &params[i];
                    quote! { #p.#ta() }
                };
                quote! { crate::sbe_json::decimal(out, v, #exponent) }
            } else {
                match inner {
                    "BoolEnum" => {
                        quote! { out.push_str(if v == BoolEnum::True { "true" } else { "false" }) }
                    }
                    "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" => {
                        quote! { crate::sbe_json::number(out, v) }
                    }
                    other => return Err(format!("{}: no JSON rule for {other}", f.schema_name)),
                }
            };
            if optional {
                let absent = match f.extensions.attr("jsonDefaultValue") {
                    Some(d) if f.extensions.attr("exponent").is_some() => {
                        let d: i64 = d.parse().map_err(|e| format!("{}: {e}", f.schema_name))?;
                        let v = quote! { #d };
                        quote! { { let v = #v; #write } }
                    }
                    Some(d) => quote! { out.push_str(#d) },
                    None => quote! { out.push_str("null") },
                };
                quote! { match self.#acc() { Some(v) => #write, None => #absent } }
            } else {
                quote! { { let v = self.#acc(); #write } }
            }
        }
    };
    Ok(quote! { #value; })
}
// ANCHOR_END: json_hook

fn generate_schema(
    xml_path: &str,
    module_name: &str,
    decimal: bool,
    out: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if !std::path::Path::new(xml_path).exists() {
        println!("cargo:warning=schema not found: {xml_path}");
        return Ok(());
    }
    // ANCHOR: with_conversion_config
    let mut config = ergo_sbe::GenerationConfig::new(module_name);
    if decimal {
        config = config.with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));
    }
    // ANCHOR_END: with_conversion_config
    // ANCHOR: extension_config
    // Extension marks through the generic API only, keyed on the venue
    // schema's own attribute names: labels from `json_label_hook`,
    // mantissa/exponent pairs from the `pairs` recipe (group mantissas with
    // message-level exponents included), and every mark (`jsonPath`,
    // `jsonDefaultValue`, `jsonOmitNull`, ...) as a const. The other two
    // sample schemas carry no marks, so their output does not change.
    let config = config
        .with_hook(json_label_hook)
        .with_try_hook(ext_recipes::pairs("exponent", "with_exponent"))
        .with_try_hook(json_hook)
        .with_extension_consts(true);
    // ANCHOR_END: extension_config
    ergo_sbe::generate(ergo_sbe::SchemaSource::file(xml_path), config)?.write_to(out)?;
    Ok(())
}

// ANCHOR: build_with_conversion
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generated_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    generate_schema(
        "schemas/normalized-app.xml",
        "normalized_app",
        true,
        &generated_dir,
    )?;
    generate_schema(
        "schemas/bitget-spot.xml",
        "bitget_spot",
        false,
        &generated_dir,
    )?;
    generate_schema(
        "schemas/binance-spot.xml",
        "binance_spot",
        false,
        &generated_dir,
    )?;
    Ok(())
}
// ANCHOR_END: build_with_conversion
