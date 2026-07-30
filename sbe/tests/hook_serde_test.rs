//! End-to-end test: generate an enum + bitset with serde hooks, compile,
//! and verify serialize/deserialize round-trips.

mod common;
use common::compile_and_run_with_deps;

/// Schema with one enum (EventCode) and one bitset (OptionalFields).
const SCHEMA_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="serde_test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <enum name="EventCode" encodingType="uint32">
      <validValue name="Ok" description="Success">200</validValue>
      <validValue name="Error" description="Failure">400</validValue>
      <validValue name="Timeout">408</validValue>
    </enum>
    <set name="OptionalFields" encodingType="uint8">
      <choice name="hasPrice">0</choice>
      <choice name="hasQty">1</choice>
      <choice name="hasVenue">2</choice>
    </set>
  </types>
  <message name="Order" id="1" blockLength="16">
    <field name="price" id="1" type="int64"/>
    <field name="qty" id="2" type="int32" offset="8"/>
    <field name="code" id="3" type="EventCode" offset="12"/>
  </message>
</messageSchema>"#;

#[test]
fn serde_enum_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let ir = ergo_sbe::parse(SCHEMA_XML)?;
    let schema = ergo_sbe::Schema::from_ir(ir);

    let config = ergo_sbe::GenerationConfig::new("hook_serde")
        .enable_domain_objects(ergo_sbe::DomainVarData::Bytes)
        .with_hook(serde_hook);

    let modules = ergo_sbe::Generator::new(config).generate(&schema)?;
    let src = modules.modules().next().expect("one module").source.clone();

    let code = r##"
        // Serialize enum → JSON string → deserialize back
        let ok = EventCode::Ok;
        let json = serde_json::to_string(&ok)?;
        assert_eq!(json, "\"Ok\"");
        let back: EventCode = serde_json::from_str(&json)?;
        assert_eq!(back, ok);

        // Error variant round-trip
        let err = EventCode::Error;
        let json = serde_json::to_string(&err)?;
        assert_eq!(json, "\"Error\"");
        let back: EventCode = serde_json::from_str(&json)?;
        assert_eq!(back, err);

        // Unknown variant → error
        assert!(serde_json::from_str::<EventCode>("\"Bogus\"").is_err());
        // NullVal variant also serializes/deserializes
        let nv = EventCode::NullVal;
        let json = serde_json::to_string(&nv)?;
        assert_eq!(json, "\"NullVal\"");
        let back: EventCode = serde_json::from_str(&json)?;
        assert_eq!(back, nv);

        // Set: build one with has_price + has_venue
        let mut fields = OptionalFields::default();
        fields.has_price(true).has_venue(true);
        let json = serde_json::to_string(&fields)?;
        let arr: Vec<&str> = serde_json::from_str::<Vec<&str>>(&json)?;
        assert!(arr.contains(&"hasPrice"));
        assert!(!arr.contains(&"hasQty"));
        assert!(arr.contains(&"hasVenue"));

        // Deserialize set from JSON
        let back: OptionalFields = serde_json::from_str("[\"hasPrice\",\"hasQty\"]")?;
        assert!(back.is_has_price());
        assert!(back.is_has_qty());
        assert!(!back.is_has_venue());

        // Empty set from JSON
        let empty: OptionalFields = serde_json::from_str("[]")?;
        assert!(!empty.is_has_price() && !empty.is_has_qty() && !empty.is_has_venue());

        // Unknown choice → error
        assert!(serde_json::from_str::<OptionalFields>("[\"Bogus\"]").is_err());

        // ── Domain struct (DTO) round-trip ──
        let dto = OrderDomain {
            price: 12345,
            qty: 100,
            code: EventCode::Ok,
        };
        let json = serde_json::to_string(&dto)?;
        let back: OrderDomain = serde_json::from_str(&json)?;
        assert_eq!(back.price, 12345);
        assert_eq!(back.qty, 100);
        assert_eq!(back.code, EventCode::Ok);

        // ── DTO → encode → decoder → verify no panic ──
        let dto = OrderDomain { price: 999, qty: 50, code: EventCode::Error };
        let len = OrderEncoder::compute_length_with_header();
        let mut buf = vec![0u8; len];
        dto.encode(&mut buf)?;
        let dec = OrderDecoder::try_wrap_and_apply_header(&buf, 0)?;
        assert_eq!(dec.price(), 999);
        assert_eq!(dec.qty(), 50);
        assert_eq!(dec.code(), EventCode::Error);
    "##;

    compile_and_run_with_deps(
        "hook_serde",
        &src,
        code,
        "serde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\"\n",
    );

    Ok(())
}

/// The hook that adds serde Serialize + Deserialize for enums and sets.
fn serde_hook(ctx: &ergo_sbe::ItemContext) -> Vec<proc_macro2::TokenStream> {
    use ergo_sbe::ItemContext;
    use quote::format_ident;

    match ctx {
        ItemContext::Enum { name, variants, .. } => {
            let ident = format_ident!("{name}");
            let var_names: Vec<_> = variants.iter().map(|v| format_ident!("{}", v.name)).collect();
            let var_labels: Vec<_> = variants.iter().map(|v| v.label.clone()).collect();
            vec![quote::quote! {
                impl serde::Serialize for #ident {
                    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                        let label = match self {
                            #(Self::#var_names => #var_labels,)*
                            _ => "NullVal",
                        };
                        s.serialize_str(label)
                    }
                }

                impl<'de> serde::Deserialize<'de> for #ident {
                    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                        let s = String::deserialize(d)?;
                        match s.as_str() {
                            #(#var_labels => Ok(Self::#var_names),)*
                            "NullVal" => Ok(Self::NullVal),
                            _ => Err(serde::de::Error::unknown_variant(
                                &s, &[#(#var_labels),*])),
                        }
                    }
                }
            }]
        }
        ItemContext::Set { name, choices, .. } => {
            let ident = format_ident!("{name}");
            let c_is_idents: Vec<_> = choices.iter().map(|c| format_ident!("is_{}", c.snake_name)).collect();
            let c_labels: Vec<_> = choices.iter().map(|c| c.label.clone()).collect();
            let c_bits: Vec<_> = choices.iter().map(|c| c.bit_position).collect();
            vec![quote::quote! {
                impl serde::Serialize for #ident {
                    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                        let mut names = Vec::new();
                        #(if self.#c_is_idents() { names.push(#c_labels); })*
                        names.serialize(s)
                    }
                }

                impl<'de> serde::Deserialize<'de> for #ident {
                    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                        let names: Vec<String> = Vec::deserialize(d)?;
                        let mut value = 0u8;
                        for name in &names {
                            match name.as_str() {
                                #(#c_labels => value |= 1u8 << #c_bits,)*
                                other => return Err(serde::de::Error::unknown_variant(
                                    other, &[#(#c_labels),*])),
                            }
                        }
                        Ok(Self(value))
                    }
                }
            }]
        }
        ItemContext::MessageDecoder { name, fields, .. } => {
            let ident = format_ident!("{name}");
            let f_names: Vec<_> = fields.iter().map(|f| format_ident!("{}", f.name)).collect();
            let f_strs: Vec<_> = fields.iter().map(|f| f.name.clone()).collect();
            let n = fields.len();
            vec![quote::quote! {
                impl serde::Serialize for #ident<'_> {
                    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                        use serde::ser::SerializeStruct;
                        let mut st = s.serialize_struct(stringify!(#ident), #n)?;
                        #(st.serialize_field(#f_strs, &self.#f_names())?;)*
                        st.end()
                    }
                }
            }]
        }
        ItemContext::DomainStruct { name, fields } => {
            let ident = format_ident!("{name}");
            let f_names: Vec<_> = fields.iter().map(|f| format_ident!("{}", f.name)).collect();
            let f_strs: Vec<_> = fields.iter().map(|f| f.name.clone()).collect();
            let n = fields.len();
            vec![quote::quote! {
                impl serde::Serialize for #ident {
                    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                        use serde::ser::SerializeStruct;
                        let mut st = s.serialize_struct(stringify!(#ident), #n)?;
                        #(st.serialize_field(#f_strs, &self.#f_names)?;)*
                        st.end()
                    }
                }

                impl<'de> serde::Deserialize<'de> for #ident {
                    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                        let m: std::collections::HashMap<String, serde_json::Value> =
                            std::collections::HashMap::deserialize(d)?;
                        Ok(Self {
                            #(#f_names: serde_json::from_value(
                                m.get(#f_strs).cloned().unwrap_or(serde_json::Value::Null)
                            ).map_err(|e| serde::de::Error::custom(e))?,)*
                        })
                    }
                }
            }]
        }
        _ => vec![],
    }
}
