//! Generate SBE codecs into `src/generated/` (gitignored) for IDE go-to-definition.

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
    // The three extension built-ins, keyed on the venue schema's own
    // attribute names. The other two sample schemas carry none of these
    // marks, so for them the knobs generate byte-identical output.
    let config = config
        .with_variant_labels("jsonValue", "as_str")
        .with_exponent_links("exponent")
        .with_field_extension_defaults("jsonDefaultValue", "jsonOmitNull")
        .with_hook(json_label_hook);
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
