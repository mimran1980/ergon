//! sbe-tool 1.40.2 regression tests, ported: enum nulls from the encodingType
//! typedef (issue1116), float infinity constants and non-NaN float nulls
//! (floating-point-infinity-schema), and keyword-named set choices (issue1118).
//!
//! Each case compiles and runs the generated module: a wrong literal does not
//! compile, and a wrong null image fails the running program.
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

mod common;
use common::{Paths, compile_and_run_modules, generate};
use std::error::Error;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    Paths::sbe_dir()
        .join("tests")
        .join("fixtures")
        .join("schemas")
        .join(name)
}

fn generate_str(xml: &str, module: &str) -> Result<String, Box<dyn Error>> {
    let schema = ergo_sbe::Schema::from_ir(ergo_sbe::parse(xml)?);
    let modules =
        ergo_sbe::Generator::new(ergo_sbe::GenerationConfig::new(module)).generate(&schema)?;
    Ok(modules.modules().next().ok_or("no module")?.source.clone())
}

/// `nullValue="254"` on the enum's encodingType typedef is the enum's null:
/// the `NullVal` discriminant, both conversions, and the byte an optional
/// field's `None` writes. Java, C++, Go and sbe-tool 1.40.2 Rust all emit 254.
#[test]
fn enum_null_value_comes_from_the_encoding_type() -> Result<(), Box<dyn Error>> {
    let (_s, required) = generate(&fixture("issue1116.xml"), "issue1116");
    let xml = std::fs::read_to_string(fixture("issue1116.xml"))?;
    let optional_xml = xml.replace(
        r#"<field name="fooBar" id="1" type="FooBar"/>"#,
        r#"<field name="fooBar" id="1" type="FooBar" presence="optional"/>"#,
    );
    assert_ne!(optional_xml, xml, "fixture no longer has the fooBar field");
    let optional = generate_str(&optional_xml, "issue1116_opt")?;

    compile_and_run_modules(
        "issue1116_enum_null",
        &[("issue1116", &required), ("issue1116_opt", &optional)],
        r#"
        {
            use issue1116::*;
            assert_eq!(FooBar::NullVal as u8, 254);
            assert_eq!(u8::from(FooBar::NullVal), 254);
            assert_eq!(FooBar::from(254), FooBar::NullVal);
            assert_eq!(FooBar::from(255), FooBar::NullVal);

            let mut buf = [0u8; Issue1116Encoder::compute_length_with_header()];
            let len = Issue1116Encoder::wrap_and_apply_header(&mut buf, 0)
                .fixed(&Issue1116FixedFields { foo_bar: FooBar::NullVal })
                .encoded_length_with_header();
            assert_eq!(buf[8], 254, "the encoder must write the schema null");
            assert_eq!(Issue1116Decoder::try_from(&buf[..len])?.foo_bar(), FooBar::NullVal);
        }
        {
            use issue1116_opt::*;
            let mut buf = [0u8; Issue1116Encoder::compute_length_with_header()];
            Issue1116Encoder::wrap_and_apply_header(&mut buf, 0)
                .fixed(&Issue1116FixedFields { foo_bar: None });
            assert_eq!(buf[8], 254, "an optional enum's None must write the schema null");
        }
        "#,
    );
    Ok(())
}

/// Float/double constants spelled `Infinity` and `-Infinity` compile to the
/// exact IEEE images, and an optional field whose `nullValue` is not NaN
/// decodes its own `None` as `None` at message level, in a group entry and in
/// a composite member.
#[test]
fn float_infinity_constants_and_non_nan_nulls() -> Result<(), Box<dyn Error>> {
    let (_s, infinity) = generate(
        &fixture("floating-point-infinity-schema.xml"),
        "infinity_test",
    );
    let float_null = generate_str(FLOAT_NULL_SCHEMA, "float_null")?;

    compile_and_run_modules(
        "float_infinity_and_nulls",
        &[("infinity_test", &infinity), ("float_null", &float_null)],
        r#"
        {
            use infinity_test::*;
            let mut buf = [0u8; FloatConstantsEncoder::compute_length_with_header()];
            let len = FloatConstantsEncoder::wrap_and_apply_header(&mut buf, 0)
                .fixed(&FloatConstantsFixedFields {})
                .encoded_length_with_header();
            let c = FloatConstantsDecoder::try_from(&buf[..len])?;
            assert_eq!(c.positive().to_bits(), 0x7f80_0000);
            assert_eq!(c.negative().to_bits(), 0xff80_0000);

            let mut buf = [0u8; InfinityValuesEncoder::compute_length_with_header()];
            let len = InfinityValuesEncoder::wrap_and_apply_header(&mut buf, 0)
                .fixed(&InfinityValuesFixedFields {
                    float_positive_null: None,
                    float_negative_null: None,
                    double_positive_null: None,
                    double_negative_null: None,
                })
                .encoded_length_with_header();
            let v = InfinityValuesDecoder::try_from(&buf[..len])?;
            assert_eq!(v.float_positive_infinity().to_bits(), 0x7f80_0000);
            assert_eq!(v.float_negative_infinity().to_bits(), 0xff80_0000);
            assert_eq!(v.double_positive_infinity().to_bits(), 0x7ff0_0000_0000_0000);
            assert_eq!(v.double_negative_infinity().to_bits(), 0xfff0_0000_0000_0000);
            assert_eq!(&buf[8..12], &f32::INFINITY.to_le_bytes(), "None writes the schema null");
            assert!(v.float_positive_null().is_none());
            assert!(v.float_negative_null().is_none());
            assert!(v.double_positive_null().is_none());
            assert!(v.double_negative_null().is_none());

            let len = InfinityValuesEncoder::wrap_and_apply_header(&mut buf, 0)
                .fixed(&InfinityValuesFixedFields {
                    float_positive_null: Some(1.5),
                    float_negative_null: Some(-1.5),
                    double_positive_null: Some(f64::NAN),
                    double_negative_null: Some(0.25),
                })
                .encoded_length_with_header();
            let v = InfinityValuesDecoder::try_from(&buf[..len])?;
            assert_eq!(v.float_positive_null(), Some(1.5));
            assert_eq!(v.float_negative_null(), Some(-1.5));
            assert!(v.double_positive_null().is_some_and(f64::is_nan), "NaN is a value here");
            assert_eq!(v.double_negative_null(), Some(0.25));
        }
        {
            use float_null::*;
            let mut pad = [0u8; 128];
            let need = QuoteEncoder::try_compute_length_with_header(2)?;
            let buf = &mut pad[..need];
            let len = QuoteEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&QuoteFixedFields {
                    px: None,
                    qty: None,
                    mid: None,
                    bounds: Bounds::new(f32::NEG_INFINITY, -1.0),
                })
                .legs(2, |legs| {
                    legs.add(|_| Ok(()))?;
                    legs.add(|leg| {
                        leg.px(2.5).qty(3.0);
                        Ok(())
                    })?;
                    Ok(())
                })?
                .encoded_length_with_header();
            assert_eq!(len, need);

            let d = QuoteDecoder::try_from(&buf[..len])?;
            assert_eq!(d.px(), None, "message level, nullValue Infinity");
            assert_eq!(d.qty(), None, "message level, nullValue -1.0");
            assert_eq!(d.mid(), None, "message level, NaN null");
            assert_eq!(d.bounds().lo(), None, "composite member, nullValue -Infinity");
            assert_eq!(d.bounds().hi(), None, "composite member, nullValue -1.0");
            assert_eq!(d.bounds_value().lo(), None, "composite value, nullValue -Infinity");
            assert_eq!(d.bounds_value().hi(), None, "composite value, nullValue -1.0");
            let legs: Vec<_> = d.into_legs()?.map(|e| (e.px(), e.qty())).collect();
            assert_eq!(legs, vec![(None, None), (Some(2.5), Some(3.0))], "group entries");
        }
        "#,
    );
    Ok(())
}

const FLOAT_NULL_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
    package="float_null" id="77" version="0" byteOrder="littleEndian">
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
        <composite name="Bounds">
            <type name="lo" primitiveType="float" presence="optional" nullValue="-Infinity"/>
            <type name="hi" primitiveType="double" presence="optional" nullValue="-1.0"/>
        </composite>
        <type name="PosInfF" primitiveType="float" presence="optional" nullValue="Infinity"/>
        <type name="NegOneD" primitiveType="double" presence="optional" nullValue="-1.0"/>
        <type name="NanD" primitiveType="double" presence="optional"/>
    </types>
    <sbe:message name="Quote" id="1">
        <field name="px" id="1" type="PosInfF"/>
        <field name="qty" id="2" type="NegOneD"/>
        <field name="mid" id="3" type="NanD"/>
        <field name="bounds" id="4" type="Bounds"/>
        <group name="legs" id="10" dimensionType="groupSizeEncoding">
            <field name="px" id="11" type="PosInfF"/>
            <field name="qty" id="12" type="NegOneD"/>
        </group>
    </sbe:message>
</sbe:messageSchema>
"#;

/// Choices named after Rust keywords (`Type`, `Match`) get usable accessors
/// and round-trip. ergo's names (`type_`, `is_match_`) differ from sbe-tool's
/// `get_`/`set_` by design, so this checks compilation and behaviour only.
#[test]
fn keyword_named_set_choices_round_trip() -> Result<(), Box<dyn Error>> {
    let (_s, src) = generate(&fixture("issue1118.xml"), "issue1118");
    compile_and_run_modules(
        "issue1118_keyword_choices",
        &[("issue1118", &src)],
        r#"
        use issue1118::*;
        let mut flags = Flags::default();
        flags.type_(true);
        assert!(flags.is_type_() && !flags.is_match_() && !flags.is_ordinary());
        flags.match_(true).type_(false);
        assert!(!flags.is_type_() && flags.is_match_());
        flags.match_(false);
        assert_eq!(flags.raw(), 0);

        let mut buf = [0u8; Issue1118Encoder::compute_length_with_header()];
        let mut sent = Flags::default();
        sent.match_(true);
        let len = Issue1118Encoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&Issue1118FixedFields { flags: sent })
            .encoded_length_with_header();
        let got = Issue1118Decoder::try_from(&buf[..len])?.flags();
        assert!(got.is_match_() && !got.is_type_());
        "#,
    );
    Ok(())
}

/// `inf`, `infinity` and `nan` in any case other than Java's `Infinity` and
/// `NaN` are rejected, for null/min/max values and for constants alike.
#[test]
fn non_java_float_spellings_are_rejected() -> Result<(), Box<dyn Error>> {
    let xml = std::fs::read_to_string(fixture("floating-point-infinity-schema.xml"))?;
    ergo_sbe::parse(&xml)?;
    for (from, to) in [
        (r#"nullValue="Infinity""#, r#"nullValue="inf""#),
        (r#"nullValue="-Infinity""#, r#"nullValue="-infinity""#),
        (r#"maxValue="1.5""#, r#"maxValue="nan""#),
        (">Infinity</type>", ">inf</type>"),
        (">-Infinity</type>", ">-INFINITY</type>"),
    ] {
        let bad = xml.replacen(from, to, 1);
        assert_ne!(bad, xml, "fixture no longer contains {from}");
        assert!(ergo_sbe::parse(&bad).is_err(), "{to} must be rejected");
    }
    let field_constant = FLOAT_NULL_SCHEMA.replace(
        r#"<field name="mid" id="3" type="NanD"/>"#,
        r#"<field name="mid" id="3" type="double" presence="constant" constantValue="nan"/>"#,
    );
    assert!(
        ergo_sbe::parse(&field_constant).is_err(),
        "field constantValue nan must be rejected"
    );
    let accepted = field_constant.replace(r#"constantValue="nan""#, r#"constantValue="+Infinity""#);
    ergo_sbe::parse(&accepted)?;
    Ok(())
}
