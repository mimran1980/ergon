//! The mutable ordered decoder lane is gone.
//!
//! Sequential decode is the staged `into_*` / `skip_*` chain.
//! Random-access and `.memoized()` remain. This file asserts the ordered
//! surface is not generated, then compile-and-runs the remaining lanes.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{
    Paths, compile_and_run, compile_fails_with_diagnostics, generate, generate_domain_with,
};
use ergo_sbe::{GenerationConfig, GenerationProfile, Generator, Schema, parse};

fn assert_ordered_lane_absent(src: &str) {
    assert!(
        !src.contains("OrderedDecoder"),
        "must not emit *OrderedDecoder types"
    );
    assert!(
        !src.contains("pub fn ordered(self)"),
        "must not emit Decoder::ordered()"
    );
    assert!(
        !src.contains("into_car_ordered") && !src.contains("_ordered("),
        "must not emit AnyMessage::into_*_ordered"
    );
    assert!(
        !src.contains("DecodeError::OutOfOrder"),
        "must not emit OutOfOrder checks"
    );
}

#[test]
fn ordered_lane_is_not_generated() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "no_ordered");
    assert_ordered_lane_absent(&src);
    compile_fails_with_diagnostics(
        "no_ordered_method",
        &src,
        r#"
        let buf = [0u8; 16];
        let _ = unsafe { CarDecoder::wrap_unchecked(&buf, 0, 45, 0) }.ordered();
        "#,
        &["no method named `ordered`"],
    );
    Ok(())
}

/// Remaining lanes still decode the same values after the ordered lane is gone.
#[test]
fn remaining_lanes_decode_identical_values() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "remaining_lanes");
    assert_ordered_lane_absent(&src);
    compile_and_run(
        "remaining_lanes",
        &src,
        r#"
        let mut storage = [0u8; 512];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [1, 2, 3, 4],
                vehicle_code: *b"abcdef",
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, *b"ABC", 1i8, BooleanType::T, Booster::new(BoostType::TURBO, 200)),
            })
            .fuel_figures(2, |g| {
                g.add(|mut e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban") })?;
                g.add(|mut e| { e.speed(55).mpg(49.0); e.usage_description(b"Hwy") })?;
                Ok(())
            })?
            .performance_figures(1, |g| {
                g.add(|mut e| {
                    e.octane_rating(95);
                    e.acceleration(2, |a| {
                        a.add(|row| { row.mph(30).seconds(4.0); Ok(()) })?;
                        a.add(|row| { row.mph(60).seconds(7.5); Ok(()) })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .manufacturer(b"Honda")?
            .model(b"Civic")?
            .activation_code(b"abc")?
            .encoded_length_with_header();
        let encoded = &storage[..len];

        let random = CarDecoder::try_decode(encoded, 0)?;
        let r_sn = random.serial_number();
        let r_year = random.model_year();
        let r_code = random.code();
        let r_eng = random.engine().capacity();
        let r_mfr = random.manufacturer_as_str()?.to_owned();
        let mut r_fuel = Vec::new();
        for e in random.fuel_figures()? {
            let e = e?;
            r_fuel.push((e.speed(), e.usage_description_as_str()?.to_owned()));
        }
        let mut r_octane = Vec::new();
        let mut r_acc = Vec::new();
        for e in random.performance_figures()? {
            let e = e?;
            r_octane.push(e.octane_rating());
            for a in e.acceleration()? {
                r_acc.push((a.mph(), a.seconds().to_bits()));
            }
        }
        let r_model = random.model_as_str()?.to_owned();
        let r_code_vd = random.activation_code_as_str()?.to_owned();

        let mut s_fuel = Vec::new();
        let mut s_octane = Vec::new();
        let mut s_acc = Vec::new();
        let staged = CarDecoder::try_decode(encoded, 0)?;
        let s_sn = staged.serial_number();
        let s_year = staged.model_year();
        let s_code = staged.code();
        let s_eng = staged.engine().capacity();
        let mut fuel = staged.into_fuel_figures()?;
        for entry in &mut fuel {
            let entry = entry?;
            let speed = entry.speed();
            let (usage, _) = entry.into_usage_description_as_str()?;
            s_fuel.push((speed, usage.to_owned()));
        }
        let mut perf = fuel.into_performance_figures()?;
        for entry in &mut perf {
            let entry = entry?;
            s_octane.push(entry.octane_rating());
            let mut acc = entry.into_acceleration()?;
            for a in &mut acc {
                let a = a?;
                s_acc.push((a.mph(), a.seconds().to_bits()));
            }
        }
        let (s_mfr, staged) = perf.into_manufacturer_as_str()?;
        let (s_model, staged) = staged.into_model_as_str()?;
        let (s_code_vd, _) = staged.into_activation_code_as_str()?;

        let memo = CarDecoder::try_decode(encoded, 0)?.memoized();
        let m_sn = memo.serial_number();
        let m_mfr = memo.manufacturer_as_str()?.to_owned();
        let mut m_fuel = Vec::new();
        for e in memo.fuel_figures()? {
            let e = e?;
            m_fuel.push((e.speed(), e.usage_description_as_str()?.to_owned()));
        }
        let m_model = memo.model_as_str()?.to_owned();

        assert_eq!((r_sn, s_sn, m_sn), (1234, 1234, 1234));
        assert_eq!(r_year, s_year);
        assert_eq!(r_code, s_code);
        assert_eq!(r_eng, s_eng);
        assert_eq!(r_fuel, s_fuel);
        assert_eq!(s_fuel, m_fuel);
        assert_eq!(r_octane, s_octane);
        assert_eq!(r_acc, s_acc);
        assert_eq!(r_mfr, s_mfr);
        assert_eq!(s_mfr, m_mfr);
        assert_eq!(r_model, s_model);
        assert_eq!(s_model, m_model);
        assert_eq!(r_code_vd, s_code_vd);
        assert_eq!(len, CarDecoder::try_decode(encoded, 0)?.encoded_length_with_header()?);
    "#,
    );
    Ok(())
}

/// A schema field named `ordered` keeps that name: the conversion method
/// that forced `ordered_field` is gone.
#[test]
fn schema_field_named_ordered_is_the_getter() -> Result<(), Box<dyn std::error::Error>> {
    const XML: &str = r#"<messageSchema package="ordclash" id="1" version="0" byteOrder="littleEndian">
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
  </types>
  <message name="Msg" id="1" blockLength="4">
    <field name="ordered" id="1" type="uint32" offset="0"/>
    <group name="legs" id="2" dimensionType="groupSizeEncoding" blockLength="4">
      <field name="qty" id="3" type="uint32" offset="0"/>
    </group>
  </message>
</messageSchema>"#;
    let schema = Schema::from_ir(parse(XML)?);
    let src = Generator::new(GenerationConfig::new("ordclash"))
        .generate(&schema)?
        .modules()
        .next()
        .expect("one module")
        .source
        .clone();
    assert!(
        src.contains("fn ordered("),
        "field named ordered is the getter"
    );
    assert!(
        !src.contains("fn ordered_field("),
        "no rename without ordered() conversion"
    );
    assert!(
        !src.contains("pub fn ordered(self)"),
        "lane conversion must not exist"
    );
    assert!(!src.contains("OrderedDecoder"));
    compile_and_run(
        "ordclash",
        &src,
        r#"
        let mut storage = [0u8; MsgEncoder::compute_length_with_header(1)];
        let len = MsgEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&MsgFixedFields { ordered: 7 })
            .legs(1, |legs| { legs.add(|l| { l.qty(3u32); Ok(()) })?; Ok(()) })?
            .encoded_length_with_header();
        assert_eq!(storage.len(), len);
        let dec = MsgDecoder::try_decode(&storage[..len], 0)?;
        assert_eq!(dec.ordered(), 7);
        assert_eq!(dec.acting_version(), 0);
        let mut n = 0u32;
        let mut legs = dec.into_legs()?;
        for e in &mut legs {
            let e = e?;
            n += e.qty();
        }
        assert_eq!(n, 3);
    "#,
    );
    Ok(())
}

/// Lean and Full both omit the mutable ordered lane.
#[test]
fn lean_and_full_profiles_omit_ordered() -> Result<(), Box<dyn std::error::Error>> {
    for (module, profile) in [
        ("mo_lean", GenerationProfile::Lean),
        ("mo_full", GenerationProfile::Full),
    ] {
        let (_schema, src) =
            generate_domain_with(&Paths::example_schema(), module, |c| c.profile(profile));
        assert!(
            !src.contains("pub fn ordered(self)"),
            "{module} must not emit ordered()"
        );
        assert!(
            !src.contains("OrderedDecoder"),
            "{module} must not emit OrderedDecoder"
        );
    }
    Ok(())
}

#[test]
fn random_access_and_iterator_still_work() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_compat");
    compile_and_run(
        "mo_compat",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 9,
                model_year: 2013,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(1, |g| {
                g.add(|mut e| { e.speed(10).mpg(1.0); e.usage_description(b"aa") })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let car = CarDecoder::try_decode(encoded, 0)?;
        assert_eq!(car.serial_number(), 9);
        assert_eq!(car.manufacturer()?, b"M");
        let mut n = 0usize;
        for e in car.fuel_figures()? {
            n += 1;
            assert_eq!(e?.speed(), 10);
        }
        assert_eq!(n, 1);
        let mut n2 = 0usize;
        let mut fuel = car.into_fuel_figures()?;
        for e in &mut fuel {
            let e = e?;
            n2 += 1;
            assert_eq!(e.speed(), 10);
            let _ = e.into_usage_description()?;
        }
        assert_eq!(n2, 1);
    "#,
    );
    Ok(())
}
