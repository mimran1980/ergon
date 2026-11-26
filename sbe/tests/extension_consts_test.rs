//! `GenerationConfig::with_extension_consts`: every extension attribute and
//! foreign child element becomes an associated `&'static str` const next to the
//! item it is on, with no hook code.

#![allow(clippy::expect_used)]

mod common;
#[path = "common/ext_recipes.rs"]
mod ext_recipes;
use common::compile_and_run;
use ergo_sbe::{GenerateError, GenerationConfig, Generator, Schema, SchemaSource, parse};

const MARKED: &str = r#"<messageSchema xmlns:x="urn:acme:ext" xmlns:y="urn:other:ext" package="acme" id="7" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="groupSizeEncoding">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="numInGroup" primitiveType="uint16"/>
    </composite>
    <composite name="varStringEncoding">
      <type name="length" primitiveType="uint32"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
    <enum name="Venue" encodingType="uint8" x:registry="ISO 10383">
      <validValue name="Lse" x:mic="XLON">1</validValue>
      <validValue name="Xetra">2</validValue>
    </enum>
    <set name="Flags" encodingType="uint8">
      <choice name="fast" x:label="F">0</choice>
    </set>
    <composite name="Pair">
      <type name="lo" primitiveType="uint32" x:unit="cents"/>
      <type name="hi" primitiveType="uint32"/>
    </composite>
  </types>
  <message name="Quote" id="1" x:topic="quotes">
    <x:meta source="feed-a">primary<x:origin desk="ldn">feed</x:origin></x:meta>
    <field name="venue" id="1" type="Venue"/>
    <field name="priceScale" id="2" type="int8" tick="5"/>
    <field name="price" id="3" type="int64" x:scaleBy="priceScale" x:unit="USD"/>
    <field name="flags" id="4" type="Flags"/>
    <field name="pair" id="5" type="Pair"/>
    <group name="legs" id="20" dimensionType="groupSizeEncoding" x:omitNull="true">
      <field name="qty" id="1" type="int64" x:scaleBy="qtyScale"/>
      <field name="qtyScale" id="2" type="int8"/>
      <group name="fills" id="3" dimensionType="groupSizeEncoding">
        <field name="px" id="1" type="int64" x:default="-1"/>
      </group>
    </group>
    <data name="note" id="30" type="varStringEncoding" x:encoding="utf8"/>
  </message>
</messageSchema>"#;

fn generate(xml: &str, config: GenerationConfig) -> Result<String, GenerateError> {
    let schema = Schema::from_ir(parse(SchemaSource::xml(xml)).expect("schema parses"));
    let modules = Generator::new(config).generate(&schema)?;
    Ok(modules.modules().next().expect("one module").source.clone())
}

/// Every mark lands next to its item: the message element and its foreign
/// child on `{Name}Schema`, fields, groups and var-data on the owning
/// block's statics type (`{Name}Schema` or the group decoder), enum and set
/// values and composite members on their type.
#[test]
fn every_mark_becomes_a_const_next_to_its_item() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate(
        MARKED,
        GenerationConfig::new("ext_consts").with_extension_consts(true),
    )?;
    compile_and_run(
        "ext_consts",
        &src,
        r#"
        assert_eq!(QuoteSchema::TOPIC, "quotes");
        assert_eq!(QuoteSchema::META, "primary");
        assert_eq!(QuoteSchema::META_SOURCE, "feed-a");
        // A foreign element nested in another one extends its name.
        assert_eq!(QuoteSchema::META_ORIGIN, "feed");
        assert_eq!(QuoteSchema::META_ORIGIN_DESK, "ldn");
        assert_eq!(QuoteSchema::PRICE_SCALE_BY, "priceScale");
        // An unprefixed custom attribute is a mark like any other.
        assert_eq!(QuoteSchema::PRICE_SCALE_TICK, "5");
        assert_eq!(QuoteSchema::PRICE_UNIT, "USD");
        assert_eq!(QuoteSchema::LEGS_OMIT_NULL, "true");
        assert_eq!(QuoteSchema::NOTE_ENCODING, "utf8");
        assert_eq!(LegsDecoder::QTY_SCALE_BY, "qtyScale");
        assert_eq!(LegsFillsDecoder::PX_DEFAULT, "-1");
        assert_eq!(Venue::REGISTRY, "ISO 10383");
        assert_eq!(Venue::LSE_MIC, "XLON");
        assert_eq!(Flags::FAST_LABEL, "F");
        assert_eq!(Pair::LO_UNIT, "cents");
        "#,
    );
    Ok(())
}

/// Off by default, and on with no marks: byte-identical output.
#[test]
fn no_switch_or_no_marks_means_no_change() -> Result<(), Box<dyn std::error::Error>> {
    let plain = generate(MARKED, GenerationConfig::new("ext_c"))?;
    assert!(!plain.contains("PRICE_UNIT"), "off by default");
    let stripped = MARKED
        .replace(r#" x:registry="ISO 10383""#, "")
        .replace(r#" x:mic="XLON""#, "")
        .replace(r#" x:label="F""#, "")
        .replace(r#" x:unit="cents""#, "")
        .replace(r#" x:topic="quotes""#, "")
        .replace(
            r#"<x:meta source="feed-a">primary<x:origin desk="ldn">feed</x:origin></x:meta>"#,
            "",
        )
        .replace(r#" x:scaleBy="priceScale" x:unit="USD""#, "")
        .replace(r#" x:omitNull="true""#, "")
        .replace(r#" x:scaleBy="qtyScale""#, "")
        .replace(r#" x:default="-1""#, "")
        .replace(r#" x:encoding="utf8""#, "")
        .replace(r#" tick="5""#, "");
    assert!(!stripped.contains("x:"), "every mark stripped: {stripped}");
    let on = generate(
        &stripped,
        GenerationConfig::new("ext_c").with_extension_consts(true),
    )?;
    let off = generate(&stripped, GenerationConfig::new("ext_c"))?;
    assert_eq!(on, off);
    Ok(())
}

/// Two marks that spell one const name (same local name, two namespaces)
/// fail generation instead of emitting a duplicate.
#[test]
fn colliding_marks_fail_generation() {
    let xml = MARKED.replace(r#"x:unit="USD""#, r#"x:unit="USD" y:unit="EUR""#);
    let err = generate(
        &xml,
        GenerationConfig::new("ext_c").with_extension_consts(true),
    )
    .expect_err("a duplicate const name must fail");
    assert!(
        matches!(&err, GenerateError::InvalidConfiguration { option, value, .. }
            if option == "with_extension_consts" && value.ends_with("PRICE_UNIT")),
        "{err:?}"
    );
}

/// A mark that spells a const the generator already emits on that type
/// (`TEMPLATE_ID`, `BLOCK_LENGTH` on `{Name}Schema`) fails generation rather
/// than the consumer's compile.
#[test]
fn marks_that_repeat_a_generated_const_fail_generation() {
    // `x:blockLength` is a mark, not the grammar attribute (sbe-tool reads
    // grammar attributes only without a prefix): it leaves the block length
    // alone and still spells `BLOCK_LENGTH`.
    for (mark, name) in [
        ("templateId", "TEMPLATE_ID"),
        ("blockLength", "BLOCK_LENGTH"),
    ] {
        let xml = MARKED.replace(
            r#"x:topic="quotes""#,
            &format!(r#"x:topic="quotes" x:{mark}="64""#),
        );
        let err = generate(
            &xml,
            GenerationConfig::new("ext_c").with_extension_consts(true),
        )
        .expect_err("a mark that repeats a generated const must fail");
        assert!(
            matches!(&err, GenerateError::InvalidConfiguration { option, value, .. }
                if option == "with_extension_consts" && *value == format!("QuoteSchema::{name}")),
            "{mark}: {err:?}"
        );
    }
}

/// The duplicate check sees only real clashes: no fixture schema hits it
/// without the switch, and every one that generates without the switch also
/// generates with it.
#[test]
fn consts_switch_generates_every_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/schemas");
    let mut generated = 0;
    for entry in std::fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "xml") {
            continue;
        }
        let Ok(ir) = parse(SchemaSource::file(&path)) else {
            continue;
        };
        let schema = Schema::from_ir(ir);
        match Generator::new(GenerationConfig::new("all")).generate(&schema) {
            Ok(_) => {}
            Err(GenerateError::NameClash { name, .. }) => {
                return Err(format!("{}: {name}", path.display()).into());
            }
            Err(_) => continue,
        }
        Generator::new(GenerationConfig::new("all").with_extension_consts(true))
            .generate(&schema)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        generated += 1;
    }
    assert!(
        generated > 100,
        "only {generated} fixture schemas generated"
    );
    Ok(())
}

const WIRE_MARKED: &str = r#"<messageSchema xmlns:x="urn:acme:ext" package="acme" id="7" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="groupSizeEncoding">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="numInGroup" primitiveType="uint16"/>
    </composite>
    <composite name="varStringEncoding">
      <type name="length" primitiveType="uint32"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
    <enum name="Venue" encodingType="uint8">
      <validValue name="Lse" x:mic="XLON">1</validValue>
      <validValue name="Xetra">2</validValue>
    </enum>
    <set name="Flags" encodingType="uint8">
      <choice name="fast" x:label="F">0</choice>
    </set>
    <type name="id4" primitiveType="uint8" length="4"/>
  </types>
  <message name="Quote" id="1" x:topic="quotes">
    <field name="venue" id="1" type="Venue" x:unit="mic"/>
    <field name="priceScale" id="2" type="int8"/>
    <field name="price" id="3" type="int64" x:scaleBy="priceScale" scale="2"/>
    <field name="maybe" id="4" type="int32" presence="optional" x:default="-1"/>
    <field name="flags" id="5" type="Flags"/>
    <field name="key" id="6" type="id4" x:kind="bytes"/>
    <group name="legs" id="20" dimensionType="groupSizeEncoding" x:omitNull="true">
      <field name="qty" id="1" type="int64" x:scaleBy="priceScale"/>
    </group>
    <data name="note" id="30" type="varStringEncoding" x:encoding="utf8"/>
  </message>
</messageSchema>"#;

/// `xml` with every ` x:name="…"` and ` scale="…"` attribute and the `x`
/// namespace declaration removed.
fn strip_marks(xml: &str) -> String {
    let mut out = xml.replace(r#" xmlns:x="urn:acme:ext""#, "");
    for prefix in [" x:", " scale=\""] {
        while let Some(start) = out.find(prefix) {
            let value = out[start..].find("=\"").map(|i| start + i + 2);
            let end = value.and_then(|v| out[v..].find('"').map(|i| v + i + 1));
            let Some(end) = end else { break };
            out.replace_range(start..end, "");
        }
    }
    out
}

/// Marks never reach the wire: the same message encoded by a module generated
/// from the marked schema (with the consts switch and both example hooks)
/// and by one from the same schema with every mark removed is byte-identical.
#[test]
fn marks_never_reach_the_wire() -> Result<(), Box<dyn std::error::Error>> {
    let marked = generate(
        WIRE_MARKED,
        GenerationConfig::new("marked")
            .with_extension_consts(true)
            .with_try_hook(ext_recipes::labels("mic", "mic"))
            .with_try_hook(ext_recipes::pairs("scaleBy", "scaled")),
    )?;
    let stripped_xml = strip_marks(WIRE_MARKED);
    assert!(!stripped_xml.contains("x:"), "{stripped_xml}");
    let stripped = generate(&stripped_xml, GenerationConfig::new("stripped"))?;
    assert!(marked.contains("PRICE_SCALE_BY") && marked.contains("fn price_scaled"));
    common::compile_and_run_two_modules(
        "ext_wire",
        "marked",
        &marked,
        "stripped",
        &stripped,
        r#"
        macro_rules! encode {
            ($m:ident) => {{
                let mut buf = vec![0u8; 256];
                let done = $m::QuoteEncoder::try_wrap_and_apply_header(&mut buf, 0)
                    .expect("wrap")
                    .fixed(&$m::QuoteFixedFields {
                        venue: $m::Venue::Lse,
                        price_scale: -2,
                        price: 12_345,
                        maybe: Some(7),
                        flags: $m::Flags(1),
                        key: [1, 2, 3, 4],
                    })
                    .legs(1, |g| {
                        g.add(|e| {
                            e.qty(9);
                            Ok(())
                        })?;
                        Ok(())
                    })
                    .expect("legs")
                    .note(b"hi")
                    .expect("note");
                done.as_bytes_with_header().to_vec()
            }};
        }
        assert_eq!(encode!(marked), encode!(stripped));
        "#,
    );
    Ok(())
}

/// A hook method that collides with a generated accessor fails loudly in
/// the consumer's crate (E0592), never silently shadows it.
#[test]
fn hook_method_colliding_with_an_accessor_fails_to_compile()
-> Result<(), Box<dyn std::error::Error>> {
    let src = generate(
        WIRE_MARKED,
        GenerationConfig::new("ext_clash").with_hook(|ctx: &ergo_sbe::ItemContext| match ctx {
            ergo_sbe::ItemContext::MessageDecoder { name, .. } => {
                let owner = quote::format_ident!("{name}");
                vec![quote::quote! {
                    impl<'a> #owner<'a> { pub fn price(&self) -> i64 { 0 } }
                }]
            }
            _ => vec![],
        }),
    )?;
    common::compile_fails_with_diagnostics("ext_clash", &src, "", &["E0592"]);
    Ok(())
}

/// `WIRE_MARKED` with its marks removed but the `x` namespace still declared.
fn unmarked_with_namespace() -> String {
    strip_marks(WIRE_MARKED).replacen(
        "<messageSchema ",
        r#"<messageSchema xmlns:x="urn:acme:ext" "#,
        1,
    )
}

/// sbe-tool reads grammar attributes only without a namespace
/// (`getNamedItemNS(null, …)`), and so does ergo-sbe: a prefixed grammar name
/// (`x:offset`, `x:blockLength`) is a mark and never changes layout or code,
/// even when it comes before the real attribute.
#[test]
fn namespaced_grammar_names_are_marks_and_change_nothing() -> Result<(), Box<dyn std::error::Error>>
{
    let base = unmarked_with_namespace();
    let baseline = generate(&base, GenerationConfig::new("ns"))?;
    for (anchor, injected) in [
        (
            "<messageSchema ",
            r#"x:package="other" x:id="99" x:version="9" x:byteOrder="bigEndian" x:headerType="groupSizeEncoding" x:description="no" "#,
        ),
        ("<types", ""),
        (r#"<composite name="groupSizeEncoding">"#, ""),
        (
            "<message ",
            r#"x:name="Other" x:id="2" x:blockLength="64" x:sinceVersion="1" x:semanticType="S" x:deprecated="0" x:description="no" "#,
        ),
        (
            r#"<field name="price""#,
            r#"x:name="p2" x:id="9" x:type="int8" x:offset="16" x:length="2" x:presence="optional" x:nullValue="0" x:minValue="1" x:maxValue="2" x:sinceVersion="1" x:deprecated="0" x:semanticType="Price" x:epoch="unix" x:timeUnit="ns" x:description="no" "#,
        ),
        (
            r#"<field name="maybe""#,
            r#"x:presence="required" x:nullValue="5" "#,
        ),
        (
            r#"<field name="venue""#,
            r#"x:presence="constant" x:valueRef="Venue.Xetra" x:constantValue="2" "#,
        ),
        (
            r#"<group name="legs""#,
            r#"x:blockLength="64" x:dimensionType="varStringEncoding" x:id="5" x:sinceVersion="1" x:description="no" "#,
        ),
        (r#"<field name="qty""#, r#"x:offset="16" x:type="int8" "#),
        (
            r#"<data name="note""#,
            r#"x:type="groupSizeEncoding" x:id="3" x:characterEncoding="ASCII" "#,
        ),
        (
            r#"<type name="id4""#,
            r#"x:primitiveType="uint64" x:length="8" x:presence="constant" x:characterEncoding="ASCII" "#,
        ),
        (
            r#"<type name="numInGroup""#,
            r#"x:primitiveType="uint32" x:offset="4" x:nullValue="7" "#,
        ),
        (
            r#"<enum name="Venue""#,
            r#"x:encodingType="uint16" x:nullValue="9" x:description="no" "#,
        ),
        (
            r#"<validValue name="Lse""#,
            r#"x:name="Other" x:sinceVersion="3" x:description="no" "#,
        ),
        (r#"<set name="Flags""#, r#"x:encodingType="uint32" "#),
        (
            r#"<choice name="fast""#,
            r#"x:name="slow" x:sinceVersion="2" "#,
        ),
    ] {
        let element = anchor.split_whitespace().next().unwrap_or(anchor);
        let at = base.find(anchor).ok_or(anchor)? + element.len();
        let mut xml = base.clone();
        let injected = if injected.is_empty() {
            r#"x:description="no" x:name="other""#
        } else {
            injected
        };
        // Before the element's own attributes: the first match in document
        // order would win if prefixed names were read as grammar.
        xml.insert_str(at, &format!(" {}", injected.trim_end()));
        let out =
            generate(&xml, GenerationConfig::new("ns")).map_err(|e| format!("{anchor}: {e}"))?;
        assert!(
            out == baseline,
            "{anchor} with {injected} changed the output"
        );
    }
    Ok(())
}

/// Order does not matter either: a prefixed `x:offset` before or after the
/// real `offset` leaves the explicit offset in force.
#[test]
fn namespaced_offset_never_moves_a_field() -> Result<(), Box<dyn std::error::Error>> {
    let base = unmarked_with_namespace();
    let explicit = base.replacen(
        r#"<field name="price""#,
        r#"<field offset="8" name="price""#,
        1,
    );
    let expected = generate(&explicit, GenerationConfig::new("ns"))?;
    for attrs in [r#"offset="8" x:offset="16""#, r#"x:offset="16" offset="8""#] {
        let xml = base.replacen(
            r#"<field name="price""#,
            &format!(r#"<field {attrs} name="price""#),
            1,
        );
        assert!(
            generate(&xml, GenerationConfig::new("ns"))? == expected,
            "{attrs} moved the field"
        );
    }
    assert!(expected.contains("PRICE_ENCODING_OFFSET: usize = 8"));
    Ok(())
}

/// A field whose const spells one every message already has (`template` ->
/// `TEMPLATE_ID`, `schema` -> `SCHEMA_ID`) fails generation with an error
/// naming the clash, with or without the consts switch, and never blames
/// extension marks.
#[test]
fn a_field_whose_const_repeats_a_generated_one_fails_generation() {
    let base = unmarked_with_namespace();
    for (field, name) in [("template", "TEMPLATE_ID"), ("schema", "SCHEMA_ID")] {
        let xml = base.replacen(
            r#"<field name="flags""#,
            &format!(
                r#"<field name="{field}" id="40" type="int8"/>
    <field name="flags""#
            ),
            1,
        );
        for consts in [false, true] {
            let err = generate(
                &xml,
                GenerationConfig::new("clash").with_extension_consts(consts),
            )
            .map(drop)
            .expect_err("a repeated const must fail generation");
            assert!(
                matches!(&err, GenerateError::NameClash { name: n, .. }
                    if n.ends_with(&format!("::{name}"))),
                "{field} (consts {consts}): {err:?}"
            );
            assert!(!err.to_string().contains("extension mark"), "{err}");
        }
    }
}

/// A hook const that repeats a const the extension marks generate on the same
/// type fails generation naming the clash.
#[test]
fn a_hook_const_clashing_with_a_mark_const_fails_generation() {
    let err = generate(
        MARKED,
        GenerationConfig::new("hook_mark_clash")
            .with_extension_consts(true)
            .with_hook(move |ctx: &ergo_sbe::ItemContext| match ctx {
                ergo_sbe::ItemContext::MessageDecoder { schema_marker, .. } => vec![
                    format!("impl {schema_marker} {{ pub const TOPIC: &'static str = \"hook\"; }}")
                        .parse()
                        .expect("hook items parse"),
                ],
                _ => vec![],
            }),
    )
    .map(drop)
    .expect_err("a hook const repeating a mark const must fail generation");
    assert!(
        matches!(&err,
            GenerateError::InvalidConfiguration { option, value, .. }
                if option == "with_extension_consts" && value == "QuoteSchema::TOPIC"),
        "{err:?}"
    );
}

/// `WIRE_MARKED` generated with a hook that emits `items` once, on the
/// message decoder, with `MARKER` replaced by the `{Name}Schema` type.
fn generate_with_hook_items(items: &'static str) -> Result<String, GenerateError> {
    generate(
        WIRE_MARKED,
        GenerationConfig::new("hook_consts").with_hook(
            move |ctx: &ergo_sbe::ItemContext| match ctx {
                ergo_sbe::ItemContext::MessageDecoder { schema_marker, .. } => vec![
                    items
                        .replace("MARKER", schema_marker)
                        .parse()
                        .expect("hook items parse"),
                ],
                _ => vec![],
            },
        ),
    )
}

/// A hook may repeat a const name where rustc accepts it: on impls under
/// mutually exclusive `cfg`s, on consts under them in one impl, and on impls
/// of distinct instantiations of one generic type (`HookG<u8>`, `HookG<u16>`),
/// generic impls included (`impl<'a> HookP<'a, u8>`, `impl<T> HookQ<T, u8>`),
/// with a defaulted parameter left out (`HookD<T>` is `HookD<T, u8>`) and with
/// one parameter in two places (`HookR<T, T>` is never `HookR<u8, u16>`).
#[test]
fn hook_consts_that_rustc_accepts_generate() -> Result<(), Box<dyn std::error::Error>> {
    let src = generate_with_hook_items(
        r#"
        #[cfg(test)] impl MARKER { pub const HOOK_MODE: &'static str = "test"; }
        #[cfg(not(test))] impl MARKER { pub const HOOK_MODE: &'static str = "build"; }
        impl MARKER {
            #[cfg(test)] pub const HOOK_SIDE: u8 = 1;
            #[cfg(not(test))] pub const HOOK_SIDE: u8 = 2;
        }
        pub struct HookG<T>(pub T);
        impl HookG<u8> { pub const WIDTH: usize = 1; }
        impl HookG<u16> { pub const WIDTH: usize = 2; }
        pub struct HookP<'a, T>(pub &'a T);
        impl<'a> HookP<'a, u8> { pub const WIDTH: usize = 1; }
        impl<'a> HookP<'a, u16> { pub const WIDTH: usize = 2; }
        pub struct HookQ<T, U>(pub T, pub U);
        impl<T> HookQ<T, u8> { pub const WIDTH: usize = 1; }
        impl<T> HookQ<T, u16> { pub const WIDTH: usize = 2; }
        pub struct HookD<T, U = u8>(pub T, pub U);
        impl<T> HookD<T> { pub const WIDTH: usize = 1; }
        impl<T> HookD<T, u16> { pub const WIDTH: usize = 2; }
        pub struct HookR<T, U>(pub T, pub U);
        impl<T> HookR<T, T> { pub const WIDTH: usize = 1; }
        impl HookR<u8, u16> { pub const WIDTH: usize = 2; }
        "#,
    )?;
    compile_and_run(
        "hook_consts",
        &src,
        r#"
        assert_eq!(QuoteSchema::HOOK_MODE, "build");
        assert_eq!(QuoteSchema::HOOK_SIDE, 2);
        assert_eq!(HookG::<u8>::WIDTH + HookG::<u16>::WIDTH, 3);
        assert_eq!(<HookP<'static, u8>>::WIDTH + <HookP<'static, u16>>::WIDTH, 3);
        assert_eq!(<HookQ<i8, u8>>::WIDTH + <HookQ<i8, u16>>::WIDTH, 3);
        assert_eq!(<HookD<i8>>::WIDTH + <HookD<i8, u16>>::WIDTH, 3);
        assert_eq!(<HookR<i8, i8>>::WIDTH + <HookR<u8, u16>>::WIDTH, 3);
        "#,
    );
    Ok(())
}

/// A hook const that rustc would reject as a duplicate (E0592) still fails
/// generation with `NameClash`: one instantiation twice, a generic impl and
/// a concrete one, a lifetime-generic impl and an elided one, generic impls
/// that meet at one instantiation (`HookQ<T, u8>` and `HookQ<u16, u8>`; a
/// defaulted parameter left out; one parameter in two places), one `cfg`
/// twice, and a `cfg`-gated const beside an ungated one.
#[test]
fn hook_consts_that_rustc_rejects_fail_generation() {
    for items in [
        "pub struct HookG<T>(pub T);
         impl HookG<u8> { pub const C: u8 = 1; }
         impl HookG<u8> { pub const C: u8 = 2; }",
        "pub struct HookG<T>(pub T);
         impl<T> HookG<T> { pub const C: u8 = 1; }
         impl HookG<u8> { pub const C: u8 = 2; }",
        "pub struct HookL<'a>(pub &'a u8);
         impl<'a> HookL<'a> { pub const C: u8 = 1; }
         impl HookL<'_> { pub const C: u8 = 2; }",
        "pub struct HookQ<T, U>(pub T, pub U);
         impl<T> HookQ<T, u8> { pub const C: u8 = 1; }
         impl HookQ<u16, u8> { pub const C: u8 = 2; }",
        "pub struct HookD<T, U = u8>(pub T, pub U);
         impl<T> HookD<T> { pub const C: u8 = 1; }
         impl HookD<u16, u8> { pub const C: u8 = 2; }",
        "pub struct HookR<T, U>(pub T, pub U);
         impl<T> HookR<T, T> { pub const C: u8 = 1; }
         impl HookR<u8, u8> { pub const C: u8 = 2; }",
        "#[cfg(test)] impl MARKER { pub const C: u8 = 1; }
         #[cfg(test)] impl MARKER { pub const C: u8 = 2; }",
        "impl MARKER { pub const C: u8 = 1; }
         impl MARKER { #[cfg(test)] pub const C: u8 = 2; }",
    ] {
        let err = generate_with_hook_items(items)
            .map(drop)
            .expect_err("a repeated hook const must fail generation");
        assert!(
            matches!(&err, GenerateError::NameClash { name, .. } if name.ends_with("::C")),
            "{items}: {err:?}"
        );
    }
}

/// A group field mark that spells a const the generator emits on the group
/// decoder (`entryBlock` + `x:length` -> `ENTRY_BLOCK_LENGTH`) fails
/// generation, though the marks sit on `impl LegsDecoder<'_>` and the
/// generated const on `impl<'a> LegsDecoder<'a, Detached>`.
#[test]
fn a_group_mark_that_spells_a_generated_const_fails_generation() {
    let xml = WIRE_MARKED.replacen(
        r#"<field name="qty" id="1" type="int64" x:scaleBy="priceScale"/>"#,
        r#"<field name="qty" id="1" type="int64" x:scaleBy="priceScale"/>
      <field name="entryBlock" id="2" type="int8" x:length="9"/>"#,
        1,
    );
    let err = generate(
        &xml,
        GenerationConfig::new("ext_c").with_extension_consts(true),
    )
    .map(drop)
    .expect_err("a mark that spells a generated const must fail");
    assert!(
        matches!(&err, GenerateError::InvalidConfiguration { option, value, .. }
            if option == "with_extension_consts" && value == "LegsDecoder::ENTRY_BLOCK_LENGTH"),
        "{err:?}"
    );
}

/// A mark whose name has no ASCII letter or digit maps to no const name and
/// fails generation instead of emitting `pub const _`.
#[test]
fn a_mark_with_no_identifier_fails_generation() {
    for (from, to) in [
        (r#"x:topic="quotes""#, r#"x:topic="quotes" x:é="1""#),
        (
            r#"<x:meta source="feed-a">"#,
            r#"<x:é>v</x:é><x:meta source="feed-a">"#,
        ),
        (
            r#"<x:meta source="feed-a">"#,
            r#"<x:meta source="feed-a" é="1">"#,
        ),
    ] {
        let xml = MARKED.replacen(from, to, 1);
        let err = generate(
            &xml,
            GenerationConfig::new("ext_c").with_extension_consts(true),
        )
        .map(drop)
        .expect_err("a mark with no identifier must fail");
        assert!(
            matches!(&err, GenerateError::InvalidConfiguration { option, value, .. }
                if option == "with_extension_consts" && value.contains('é')),
            "{to}: {err:?}"
        );
    }
}

/// An enum-level mark that spells a variant (`x:x` beside `<validValue
/// name="X">`) fails generation: `Venue::X` would always mean the variant.
#[test]
fn an_enum_mark_that_spells_a_variant_fails_generation() {
    let xml = MARKED
        .replacen(
            r#"<validValue name="Xetra">"#,
            r#"<validValue name="X">"#,
            1,
        )
        .replacen(
            r#"x:registry="ISO 10383""#,
            r#"x:registry="ISO 10383" x:x="1""#,
            1,
        );
    let err = generate(
        &xml,
        GenerationConfig::new("ext_c").with_extension_consts(true),
    )
    .map(drop)
    .expect_err("a mark that spells a variant must fail");
    assert!(
        matches!(&err, GenerateError::InvalidConfiguration { option, value, .. }
            if option == "with_extension_consts" && value == "Venue::X"),
        "{err:?}"
    );
}
