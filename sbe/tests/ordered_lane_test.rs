//! The ordered lane: one callback per tail, in wire order.
//!
//! A façade over the staged stages, so this file proves the façade preserves
//! what the staged lane guarantees — single traversal, compile-time tail order,
//! and identical values — plus the `EntryInfo` metadata that is its own reason
//! to exist.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, compile_and_run, compile_fails_with_diagnostics, generate};

const ENCODE: &str = r#"
    let mut storage = [0u8; 512];
    let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
        .fixed(&CarFixedFields {
            serial_number: 7,
            model_year: 2020,
            available: BooleanType::T,
            code: Model::A,
            some_numbers: [1, 2, 3, 4],
            vehicle_code: *b"ABCDEF",
            extras: OptionalExtras::default(),
            engine: Engine::new(1500, 4, *b"123", 0i8, BooleanType::F, Booster::new(BoostType::NITROUS, 150)),
        })
        .fuel_figures(2, |g| {
            g.add(|mut e| { e.speed(30).mpg(1.0); e.usage_description(b"aa") })?;
            g.add(|mut e| { e.speed(60).mpg(2.0); e.usage_description(b"bbb") })?;
            Ok(())
        })?
        .performance_figures(1, |g| {
            g.add(|mut e| {
                e.octane_rating(95);
                e.acceleration(3, |a| {
                    for mph in [10u16, 20, 30] {
                        a.add(|x| { x.mph(mph).seconds(1.0); Ok(()) })?;
                    }
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
"#;

/// Every tail is visited once, in wire order, and `EntryInfo` reports the
/// position the wire declares.
#[test]
fn ordered_lane_walks_every_tail_with_entry_info() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ordered_walk");
    assert!(
        !src.contains("__sbe_fuel_figures_dim") && !src.contains("__sbe_acceleration_dim"),
        "EntryInfo must come from the staged walk, not a discarded wrap"
    );
    compile_and_run(
        "ordered_walk",
        &src,
        &format!(
            r#"
        {ENCODE}
        let mut fuel = Vec::new();
        let mut accel = Vec::new();
        let mut text = Vec::new();
        let complete = CarDecoder::try_decode(encoded, 0)?
            .ordered()
            .fixed(|car| {{
                assert_eq!(car.serial_number(), 7);
                assert_eq!(car.model_year(), 2020);
                Ok(())
            }})?
            .fuel_figures(|e, info| {{
                assert_eq!(info.count, 2);
                assert_eq!(info.block_length, 6);
                assert_eq!(info.is_first(), info.index == 0);
                assert_eq!(info.is_last(), info.index == 1);
                assert_eq!(info.remaining(), 1 - info.index);
                fuel.push((info.index, e.speed()));
                Ok(e.ordered().usage_description(|_| Ok(()))?)
            }})?
            .performance_figures(|e, info| {{
                assert_eq!((info.index, info.count), (0, 1));
                assert_eq!(e.octane_rating(), 95);
                Ok(e.ordered().acceleration(|x, _| {{
                    accel.push(x.mph());
                    Ok(())
                }})?)
            }})?
            .manufacturer_as_str(|s| {{
                text.push(s.to_owned());
                Ok(())
            }})?
            .model(|b| {{
                text.push(String::from_utf8(b.to_vec()).unwrap());
                Ok(())
            }})?
            .activation_code(|b| {{
                text.push(String::from_utf8(b.to_vec()).unwrap());
                Ok(())
            }})?;

        assert_eq!(fuel, vec![(0usize, 30u16), (1, 60)]);
        assert_eq!(accel, vec![10u16, 20, 30]);
        assert_eq!(text, vec!["Honda".to_string(), "Civic".into(), "abc".into()]);
        assert_eq!(complete.encoded_length_with_header(), encoded.len());
        assert_eq!(complete.as_bytes_with_header(), encoded);
    "#
        ),
    );
    Ok(())
}

/// Empty groups invoke the callback zero times and still reach the next tail.
#[test]
fn ordered_lane_empty_group_invokes_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ordered_empty");
    compile_and_run(
        "ordered_empty",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1, model_year: 0, available: BooleanType::F,
                code: Model::NullVal, some_numbers: [0u32; 4], vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"")?
            .model(b"")?
            .activation_code(b"")?
            .encoded_length_with_header();
        let encoded = &storage[..len];

        let mut calls = 0usize;
        let done = CarDecoder::try_decode(encoded, 0)?
            .ordered()
            .fuel_figures(|e, _| -> Result<_, sbe_rt::DecodeError> {
                calls += 1;
                e.into_usage_description().map(|(_u, d)| d)
            })?
            .performance_figures(|e, _| -> Result<_, sbe_rt::DecodeError> {
                calls += 1;
                e.into_acceleration()?.finish()
            })?
            .manufacturer(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })?
            .model(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })?
            .activation_code(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })?
            .done();
        assert_eq!(calls, 0, "empty groups invoke the callback zero times");
        assert_eq!(done.encoded_length_with_header(), encoded.len());
    "#,
    );
    Ok(())
}

/// The ordered lane decodes the same values the staged lane does.
#[test]
fn ordered_lane_matches_staged_lane() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ordered_parity");
    compile_and_run(
        "ordered_parity",
        &src,
        &format!(
            r#"
        {ENCODE}
        let mut staged = Vec::new();
        let (mfr_s, _) = CarDecoder::try_decode(encoded, 0)?
            .into_fuel_figures(|e| -> Result<_, sbe_rt::DecodeError> {{
                staged.push(e.speed());
                e.into_usage_description().map(|(_u, d)| d)
            }})?
            .into_performance_figures(|e| -> Result<_, sbe_rt::DecodeError> {{
                staged.push(e.octane_rating() as u16);
                e.into_acceleration()?.finish()
            }})?
            .into_manufacturer_as_str()?;

        let mut ord = Vec::new();
        let mut mfr_o = String::new();
        CarDecoder::try_decode(encoded, 0)?
            .ordered()
            .fuel_figures(|e, _| -> Result<_, sbe_rt::DecodeError> {{
                ord.push(e.speed());
                e.into_usage_description().map(|(_u, d)| d)
            }})?
            .performance_figures(|e, _| -> Result<_, sbe_rt::DecodeError> {{
                ord.push(e.octane_rating() as u16);
                e.into_acceleration()?.finish()
            }})?
            .manufacturer_as_str(|s| -> Result<(), sbe_rt::DecodeError> {{
                mfr_o.push_str(s);
                Ok(())
            }})?;

        assert_eq!(staged, ord, "both lanes decode identical values");
        assert_eq!(mfr_s, mfr_o);
    "#
        ),
    );
    Ok(())
}

/// Compile-fail: tail order is a type, here too — `manufacturer` is not on the
/// stage before the groups have been consumed.
#[test]
fn cf_ordered_lane_enforces_tail_order() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "cf_ordered_order");
    compile_fails_with_diagnostics(
        "cf_ordered_order",
        &src,
        r#"
        let buf = [0u8; 64];
        let dec = CarDecoder::wrap(&buf, 0, 0, 0);
        // ILLEGAL: manufacturer comes after both groups.
        let _ = dec.ordered().manufacturer(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) });
    "#,
        &["no method named `manufacturer`"],
    );
    Ok(())
}

/// A callback error consumes the lane and returns no continuation.
#[test]
fn ordered_lane_callback_error_propagates() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ordered_cb_err");
    compile_and_run(
        "ordered_cb_err",
        &src,
        &format!(
            r#"
        {ENCODE}
        #[derive(Debug)]
        enum E {{ Decode(sbe_rt::DecodeError), Mine }}
        impl From<sbe_rt::DecodeError> for E {{
            fn from(e: sbe_rt::DecodeError) -> Self {{ E::Decode(e) }}
        }}
        let r = CarDecoder::try_decode(encoded, 0)?
            .ordered()
            .try_fuel_figures(|_e, info| -> Result<_, E> {{
                if info.index == 1 {{ return Err(E::Mine); }}
                _e.into_usage_description().map(|(_u, d)| d).map_err(E::from)
            }});
        assert!(matches!(r, Err(E::Mine)), "callback error must propagate");
    "#
        ),
    );
    Ok(())
}

/// A schema with var-data but **no groups** still gets an ordered lane, and it
/// must compile without `EntryInfo` — which is emitted only for schemas that
/// declare a group, because nothing else can reach it.
///
/// This is its own cell on purpose. Every other test here uses the Car schema,
/// which has groups, so it proves the opposite branch. "The group case compiles
/// so the group-less case compiles" is the inference that shipped fifteen
/// codegen defects; the two are separate branches.
#[test]
fn ordered_lane_without_groups_compiles_without_entry_info()
-> Result<(), Box<dyn std::error::Error>> {
    const XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
                   package="nogroups" id="901" version="0"
                   semanticVersion="1.0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="varStringEncoding">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
    </composite>
  </types>
  <sbe:message name="Flat" id="1">
    <field name="seq" id="10" type="uint32"/>
    <data name="label" id="20" type="varStringEncoding"/>
    <data name="note" id="21" type="varStringEncoding"/>
  </sbe:message>
</sbe:messageSchema>"#;
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
    let schema = Schema::from_ir(parse(XML)?);
    let src = Generator::new(GenerationConfig::new("nogroups"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();

    assert!(
        !src.contains("struct EntryInfo"),
        "a group-less schema must not carry EntryInfo as dead code"
    );
    assert!(
        src.contains("pub fn ordered(self)"),
        "the ordered lane is generated regardless of groups"
    );

    // Compilation is the assertion: the lane must walk fixed + both var-data
    // tails and reach done() with no EntryInfo in the module.
    compile_and_run(
        "nogroups",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = FlatEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&FlatFixedFields { seq: 9 })
            .label(b"abc")?
            .note(b"de")?
            .encoded_length_with_header();
        let encoded = &storage[..len];

        let mut seen: Vec<Vec<u8>> = Vec::new();
        let complete = FlatDecoder::try_decode(encoded, 0)?
            .ordered()
            .fixed(|d| {
                assert_eq!(d.seq(), 9);
                Ok(())
            })?
            .label(|b| { seen.push(b.to_vec()); Ok(()) })?
            .note(|b| { seen.push(b.to_vec()); Ok(()) })?
            .done();
        assert_eq!(seen, vec![b"abc".to_vec(), b"de".to_vec()]);
        assert_eq!(complete.encoded_length_with_header(), len);
    "#,
    );
    Ok(())
}

/// The ordered lane exists on **entry** decoders too, so a walk can stay in one
/// spelling past the entry boundary instead of switching to the staged API.
///
/// Two nested shapes, both compiled here because they are separate generator
/// branches: `fuelFigures` entries carry var-data, and `performanceFigures`
/// entries carry a fixed-stride nested group whose callback returns `()`
/// rather than a completion.
///
/// A nested `ordered()` walk returns `Ordered<{Entry}Complete>`, which the
/// parent visit accepts through `IntoEntryComplete` — no `.done()` on the
/// way out. The message chain ends on the extent helper, like encode.
#[test]
fn entry_ordered_lane_walks_nested_tails() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ordered_entry");
    assert!(
        src.matches("pub fn ordered(self)").count() >= 3,
        "entry decoders must gain an ordered() lane (message + fuelFigures + performanceFigures)"
    );

    let mut body = ENCODE.to_string();
    body.push_str(
        r#"
        // Message-level ordered lane, with each entry walked by the *entry*
        // ordered lane — one spelling all the way down.
        let mut usages: Vec<Vec<u8>> = Vec::new();
        let mut accels: Vec<(u16, usize, usize)> = Vec::new();
        let complete = CarDecoder::try_decode(encoded, 0)?
            .ordered()
            .fuel_figures(|entry, info| -> Result<_, sbe_rt::DecodeError> {
                assert_eq!(info.count, 2);
                Ok(entry.ordered().usage_description(|b| {
                    usages.push(b.to_vec());
                    Ok(())
                })?)
            })?
            .performance_figures(|entry, info| -> Result<_, sbe_rt::DecodeError> {
                assert_eq!(info.count, 1);
                assert!(info.is_first() && info.is_last());
                Ok(entry.ordered().acceleration(|a, ainfo| {
                    // Fixed-stride nested group: the callback returns (),
                    // and EntryInfo is filled from the iterator itself.
                    accels.push((a.mph(), ainfo.index, ainfo.count));
                    Ok(())
                })?)
            })?
            .manufacturer(|m| -> Result<(), sbe_rt::DecodeError> {
                assert_eq!(m, b"Honda");
                Ok(())
            })?
            .model(|m| -> Result<(), sbe_rt::DecodeError> {
                assert_eq!(m, b"Civic");
                Ok(())
            })?
            .activation_code(|c| -> Result<(), sbe_rt::DecodeError> {
                assert_eq!(c, b"abc");
                Ok(())
            })?;

        assert_eq!(usages, vec![b"aa".to_vec(), b"bbb".to_vec()]);
        assert_eq!(accels, vec![(10u16, 0, 3), (20, 1, 3), (30, 2, 3)]);
        assert_eq!(complete.encoded_length_with_header(), len);
    "#,
    );
    compile_and_run("ordered_entry", &src, &body);
    Ok(())
}

/// An entry field named `ordered` keeps its accessor, and the entry-level lane
/// is not generated for that entry.
///
/// Group entries are **not** renamed against `DECODER_RESERVED`, unlike message
/// and memoized decoders, so `ordered()` on such an entry is already a field
/// getter. Emitting the lane there would be a duplicate method (E0592) — the
/// same defect class that shipped twice this cycle. The existing accessor wins,
/// exactly as `<group>_count` / `<field>_len` yield to a colliding sibling.
#[test]
fn entry_field_named_ordered_keeps_its_accessor() -> Result<(), Box<dyn std::error::Error>> {
    const XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
                   package="entryclash" id="902" version="0"
                   semanticVersion="1.0" byteOrder="littleEndian">
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
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
    </composite>
  </types>
  <sbe:message name="Msg" id="1">
    <group name="rows" id="10" dimensionType="groupSizeEncoding">
      <field name="ordered" id="11" type="uint32"/>
      <data name="tag" id="12" type="varStringEncoding"/>
    </group>
  </sbe:message>
</sbe:messageSchema>"#;
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
    let schema = Schema::from_ir(parse(XML)?);
    let src = Generator::new(GenerationConfig::new("entryclash"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();

    assert!(
        src.matches("pub fn ordered(self)").count() == 1,
        "the entry lane must yield to a field already named `ordered`"
    );
    assert!(
        src.contains("pub fn ordered(&self)"),
        "the entry's own `ordered` field accessor must survive unchanged"
    );
    // The message-level lane is unaffected: `Msg` has no field named `ordered`.
    assert!(
        src.contains("pub fn ordered(self)"),
        "message-level ordered() is independent of the entry collision"
    );

    // Compilation is the assertion: duplicate methods would be E0592 here.
    compile_and_run(
        "entryclash",
        &src,
        r#"
        let len = MsgEncodedLength::new()
            .rows(1)
            .tag(2)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; len];
        let actual = MsgEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&MsgFixedFields {})
            .rows(1, |g| {
                g.add(|mut e| { e.ordered(7u32); e.tag(b"hi") })?;
                Ok(())
            })?
            .encoded_length_with_header();
        assert_eq!(len, actual);

        let mut seen = Vec::new();
        let _c = MsgDecoder::try_decode(&storage[..actual], 0)?
            .into_rows(|e| -> Result<_, sbe_rt::DecodeError> {
                assert_eq!(e.ordered(), 7u32);
                let (tag, done) = e.into_tag()?;
                seen.push(tag.to_vec());
                Ok(done)
            })?;
        assert_eq!(seen, vec![b"hi".to_vec()]);
    "#,
    );
    Ok(())
}

const HEADER_TYPES: &str = r#"
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
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
    </composite>
"#;

/// A message whose first tail is named `fixed` must still compile: the
/// callback yields, and `fixed()` on the ordered wrapper is the group visit.
#[test]
fn message_first_tail_named_fixed_compiles() -> Result<(), Box<dyn std::error::Error>> {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
                   package="fixedclash" id="910" version="0"
                   semanticVersion="1.0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <sbe:message name="Msg" id="1">
    <field name="seq" id="10" type="uint32"/>
    <group name="fixed" id="11" dimensionType="groupSizeEncoding">
      <field name="x" id="12" type="uint32"/>
    </group>
    <data name="note" id="13" type="varStringEncoding"/>
  </sbe:message>
</sbe:messageSchema>"#
    );
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
    let schema = Schema::from_ir(parse(&xml)?);
    let src = Generator::new(GenerationConfig::new("fixedclash"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();
    assert!(
        !src.contains("struct MsgDecoderFixedView"),
        "a first tail named `fixed` must suppress the ordered fixed callback"
    );
    compile_and_run(
        "fixedclash",
        &src,
        r#"
        let len = MsgEncoder::compute_length_with_header(1, 2);
        let mut storage = [0u8; 64];
        assert!(len <= storage.len());
        let buf = &mut storage[..len];
        let actual = MsgEncoder::try_wrap_and_apply_header(buf, 0)?
            .fixed(&MsgFixedFields { seq: 9 })
            .fixed(1, |g| { g.add(|e| { e.x(3u32); Ok(()) })?; Ok(()) })?
            .note(b"hi")?
            .encoded_length_with_header();
        assert_eq!(len, actual);
        let mut xs = Vec::new();
        let dec = MsgDecoder::try_decode(&storage[..actual], 0)?;
        assert_eq!(dec.seq(), 9);
        let complete = dec.ordered()
            .fixed(|e, info| {
                assert_eq!(info.count, 1);
                xs.push(e.x());
                Ok(())
            })?
            .note(|b| { assert_eq!(b, b"hi"); Ok(()) })?
            .done();
        assert_eq!(xs, vec![3u32]);
        assert_eq!(complete.encoded_length_with_header(), actual);
    "#,
    );
    Ok(())
}

/// A first tail named `tryFixed` must keep `try_fixed()` and suppress the
/// ordered `try_fixed` / `fixed` callbacks (same yield as a first tail named
/// `fixed`). Unconditional emission is E0592 (HFT review 2026-09-15).
#[test]
fn message_first_tail_named_try_fixed_compiles() -> Result<(), Box<dyn std::error::Error>> {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
                   package="tryfixedclash" id="912" version="0"
                   semanticVersion="1.0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <sbe:message name="Msg" id="1">
    <field name="seq" id="10" type="uint32"/>
    <data name="tryFixed" id="20" type="varStringEncoding"/>
    <data name="note" id="21" type="varStringEncoding"/>
  </sbe:message>
</sbe:messageSchema>"#
    );
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
    let schema = Schema::from_ir(parse(&xml)?);
    let src = Generator::new(GenerationConfig::new("tryfixedclash"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();
    assert!(
        !src.contains("struct MsgDecoderFixedView"),
        "a first tail named `tryFixed` must suppress the ordered fixed callback"
    );
    compile_and_run(
        "tryfixedclash",
        &src,
        r#"
        let len = MsgEncoder::compute_length_with_header(2, 2);
        let mut storage = [0u8; 64];
        assert!(len <= storage.len());
        let buf = &mut storage[..len];
        let actual = MsgEncoder::try_wrap_and_apply_header(buf, 0)?
            .fixed(&MsgFixedFields { seq: 9 })
            .try_fixed(b"hi")?
            .note(b"ok")?
            .encoded_length_with_header();
        assert_eq!(len, actual);
        let dec = MsgDecoder::try_decode(&storage[..actual], 0)?;
        assert_eq!(dec.seq(), 9);
        let complete = dec.ordered()
            .try_fixed(|b| { assert_eq!(b, b"hi"); Ok(()) })?
            .note(|b| { assert_eq!(b, b"ok"); Ok(()) })?
            .done();
        assert_eq!(complete.encoded_length_with_header(), actual);
        "#,
    );
    Ok(())
}

/// Last tail named `done` lives on the stage before complete; `done()` the
/// completer lives on `Ordered<Complete>`. Different types, so both exist.
#[test]
fn last_tail_named_done_compiles() -> Result<(), Box<dyn std::error::Error>> {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
                   package="doneclash" id="911" version="0"
                   semanticVersion="1.0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <sbe:message name="Msg" id="1">
    <field name="seq" id="10" type="uint32"/>
    <data name="done" id="11" type="varStringEncoding"/>
  </sbe:message>
</sbe:messageSchema>"#
    );
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
    let schema = Schema::from_ir(parse(&xml)?);
    let src = Generator::new(GenerationConfig::new("doneclash"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();
    compile_and_run(
        "doneclash",
        &src,
        r#"
        let len = MsgEncoder::compute_length_with_header(2);
        let mut storage = [0u8; 64];
        assert!(len <= storage.len());
        let buf = &mut storage[..len];
        let actual = MsgEncoder::try_wrap_and_apply_header(buf, 0)?
            .fixed(&MsgFixedFields { seq: 1 })
            .done(b"ok")?
            .encoded_length_with_header();
        assert_eq!(len, actual);
        let complete = MsgDecoder::try_decode(&storage[..actual], 0)?
            .ordered()
            .fixed(|d| { assert_eq!(d.seq(), 1); Ok(()) })?
            .done(|b| { assert_eq!(b, b"ok"); Ok(()) })?;
        assert_eq!(complete.encoded_length_with_header(), actual);
    "#,
    );
    Ok(())
}

/// A converted entry field named `ordered` is `ordered_wire` / `ordered_as`,
/// so the entry-level lane is free to exist.
#[test]
fn converted_entry_field_named_ordered_keeps_the_lane() -> Result<(), Box<dyn std::error::Error>> {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
                   package="ordconv" id="912" version="0"
                   semanticVersion="1.0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <sbe:message name="Msg" id="1">
    <group name="rows" id="10" dimensionType="groupSizeEncoding">
      <field name="ordered" id="11" type="uint32"/>
      <data name="tag" id="12" type="varStringEncoding"/>
    </group>
  </sbe:message>
</sbe:messageSchema>"#
    );
    use ergo_sbe::{ConversionSelector, GenerationConfig, Generator, Schema, parse};
    let schema = Schema::from_ir(parse(&xml)?);
    let src = Generator::new(
        GenerationConfig::new("ordconv")
            .with_conversion(ConversionSelector::field_path("Msg.rows.ordered")),
    )
    .generate(&schema)?
    .modules()
    .next()
    .ok_or("one module")?
    .source
    .clone();
    assert!(
        src.contains("fn ordered_wire("),
        "conversion must rename the wire getter"
    );
    assert!(
        src.matches("pub fn ordered(self)").count() >= 2,
        "message and entry ordered() lanes must both exist when the field is converted"
    );
    compile_and_run(
        "ordconv",
        &src,
        r#"
        let len = MsgEncodedLength::new().rows(1).tag(2)?.encoded_length_with_header();
        let mut storage = [0u8; 64];
        let buf = &mut storage[..len];
        let actual = MsgEncoder::try_wrap_and_apply_header(buf, 0)?
            .fixed(&MsgFixedFields {})
            .rows(1, |g| {
                g.add(|mut e| { e.ordered_wire(7u32); e.tag(b"hi") })?;
                Ok(())
            })?
            .encoded_length_with_header();
        assert_eq!(len, actual);
        let mut seen = Vec::new();
        let complete = MsgDecoder::try_decode(&storage[..actual], 0)?
            .ordered()
            .rows(|e, info| {
                assert_eq!(info.count, 1);
                assert_eq!(e.ordered_wire(), 7u32);
                Ok(e.ordered().tag(|b| { seen.push(b.to_vec()); Ok(()) })?)
            })?;
        assert_eq!(complete.encoded_length_with_header(), actual);
        assert_eq!(seen, vec![b"hi".to_vec()]);
    "#,
    );
    Ok(())
}

/// Version-absent tails still occupy a compile-time stage; the callback runs
/// zero times and peek count is 0.
#[test]
fn ordered_lane_version_absent_tails() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(
        &Paths::ordered_decoder_version_tails_schema(),
        "ordered_version_absent",
    );
    compile_and_run(
        "ordered_version_absent",
        &src,
        r#"
        let mut buf = [0u8; 64];
        buf[0..2].copy_from_slice(&4u16.to_le_bytes());
        buf[2..4].copy_from_slice(&1u16.to_le_bytes());
        buf[4..6].copy_from_slice(&77u16.to_le_bytes());
        buf[6..8].copy_from_slice(&0u16.to_le_bytes());
        buf[8..12].copy_from_slice(&7u32.to_le_bytes());
        buf[12..14].copy_from_slice(&2u16.to_le_bytes());
        buf[14..16].copy_from_slice(&1u16.to_le_bytes());
        buf[16..18].copy_from_slice(&30u16.to_le_bytes());
        buf[18] = 3;
        buf[19..22].copy_from_slice(b"urb");
        buf[22] = 2;
        buf[23..25].copy_from_slice(b"hi");
        let encoded = &buf[..25];

        let mut speeds = Vec::new();
        let mut extra_calls = 0usize;
        let complete = VersionedTailsDecoder::wrap(encoded, 0, 4, 0)
            .ordered()
            .fixed(|d| {
                assert_eq!(d.seq(), 7);
                assert_eq!(d.acting_version(), 0);
                Ok(())
            })?
            .figures(|e, info| {
                assert_eq!(info.count, 1);
                speeds.push(e.speed());
                Ok(e.ordered()
                    .extras(|_, _| Ok(()))?
                    .label(|b| { assert_eq!(b, b"urb"); Ok(()) })?)
            })?
            .extra_figures(|_, _| {
                extra_calls += 1;
                Ok(())
            })?
            .note(|b| { assert_eq!(b, b"hi"); Ok(()) })?
            .extra_note(|b| { assert!(b.is_empty()); Ok(()) })?;
        assert_eq!(speeds, vec![30u16]);
        assert_eq!(extra_calls, 0);
        assert_eq!(complete.encoded_length_with_header(), encoded.len());
    "#,
    );
    Ok(())
}

/// Three-level dynamic nesting: message → entry tails → nested entry tails.
/// A cell proven at message + one entry level does not prove this location.
#[test]
fn ordered_lane_three_level_dynamic() -> Result<(), Box<dyn std::error::Error>> {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
                   package="threelevel" id="913" version="0"
                   semanticVersion="1.0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <sbe:message name="Book" id="1">
    <group name="levels" id="10" dimensionType="groupSizeEncoding">
      <field name="px" id="11" type="uint64"/>
      <group name="orders" id="12" dimensionType="groupSizeEncoding">
        <field name="id" id="13" type="uint64"/>
        <data name="tag" id="14" type="varStringEncoding"/>
      </group>
    </group>
  </sbe:message>
</sbe:messageSchema>"#
    );
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
    let schema = Schema::from_ir(parse(&xml)?);
    let src = Generator::new(GenerationConfig::new("threelevel"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();
    compile_and_run(
        "threelevel",
        &src,
        r#"
        let len = BookEncoder::compute_length()
            .levels_ragged(1, |l| {
                l.add()?.orders(|o| { o.add()?.tag(2)?; Ok(()) })?;
                Ok(())
            })?
            .encoded_length_with_header();
        let mut storage = [0u8; 128];
        assert!(len <= storage.len());
        let buf = &mut storage[..len];
        let actual = BookEncoder::try_wrap_and_apply_header(buf, 0)?
            .fixed(&BookFixedFields {})
            .levels(1, |l| {
                l.add(|mut e| {
                    e.px(5u64);
                    e.orders(1, |o| {
                        o.add(|mut row| { row.id(9u64); row.tag(b"ab") })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        assert_eq!(len, actual);
        let mut tags = Vec::new();
        let complete = BookDecoder::try_decode(&storage[..actual], 0)?
            .ordered()
            .levels(|level, linfo| {
                assert_eq!((linfo.index, linfo.count, level.px()), (0, 1, 5u64));
                Ok(level.ordered().orders(|order, oinfo| {
                    assert_eq!((oinfo.index, oinfo.count, order.id()), (0, 1, 9u64));
                    Ok(order.ordered().tag(|b| { tags.push(b.to_vec()); Ok(()) })?)
                })?)
            })?;
        assert_eq!(tags, vec![b"ab".to_vec()]);
        assert_eq!(complete.encoded_length_with_header(), actual);
    "#,
    );
    Ok(())
}

/// Empty groups never deliver EntryInfo; count is still observable on the
/// ordered stage that is about to consume the group.
#[test]
fn ordered_lane_empty_group_count_is_peekable() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "ordered_empty_count");
    compile_and_run(
        "ordered_empty_count",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1, model_year: 0, available: BooleanType::F,
                code: Model::NullVal, some_numbers: [0u32; 4], vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"")?
            .model(b"")?
            .activation_code(b"")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let ord = CarDecoder::try_decode(encoded, 0)?.ordered();
        assert_eq!(ord.fuel_figures_count()?, 0);
        let done = ord
            .fuel_figures(|e, _| {
                panic!("empty");
                #[allow(unreachable_code)]
                Ok(e.ordered().usage_description(|_| Ok(()))?)
            })?
            .performance_figures(|e, _| {
                panic!("empty");
                #[allow(unreachable_code)]
                Ok(e.ordered().acceleration(|_, _| Ok(()))?)
            })?
            .manufacturer(|_| Ok(()))?
            .model(|_| Ok(()))?
            .activation_code(|_| Ok(()))?
            .done();
        assert_eq!(done.encoded_length_with_header(), encoded.len());
    "#,
    );
    Ok(())
}

/// The `fixed` callback receives a view without tail accessors.
#[test]
fn cf_ordered_fixed_view_has_no_tail_getters() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "cf_ordered_view");
    compile_fails_with_diagnostics(
        "cf_ordered_view",
        &src,
        r#"
        let buf = [0u8; 64];
        let dec = CarDecoder::wrap(&buf, 0, 0, 0);
        let _ = dec.ordered().fixed(|view| -> Result<(), sbe_rt::DecodeError> {
            let _ = view.fuel_figures();
            Ok(())
        });
    "#,
        &["no method named `fuel_figures`"],
    );
    Ok(())
}

/// An entry whose first (and only) tail is a var-data field literally named
/// `fixed` must still compile: nothing synthesizes a competing `fixed()` on
/// the entry-level ordered lane, so the field's own accessor is the only
/// `fixed()` emitted.
#[test]
fn entry_first_tail_named_fixed_compiles() -> Result<(), Box<dyn std::error::Error>> {
    const XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
                   package="entryfixedclash" id="903" version="0"
                   semanticVersion="1.0" byteOrder="littleEndian">
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
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
    </composite>
  </types>
  <sbe:message name="Msg" id="1">
    <group name="rows" id="10" dimensionType="groupSizeEncoding">
      <field name="tag" id="11" type="uint32"/>
      <data name="fixed" id="12" type="varStringEncoding"/>
    </group>
  </sbe:message>
</sbe:messageSchema>"#;
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
    let schema = Schema::from_ir(parse(XML)?);
    let src = Generator::new(GenerationConfig::new("entryfixedclash"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();

    // Compilation is the assertion: a synthesized entry-level `fixed()`
    // colliding with the field's own accessor would be E0592 here.
    compile_and_run(
        "entryfixedclash",
        &src,
        r#"
        let len = MsgEncodedLength::new()
            .rows(1)
            .fixed(3)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; len];
        let actual = MsgEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&MsgFixedFields {})
            .rows(1, |g| {
                g.add(|mut e| { e.tag(7u32); e.fixed(b"abc") })?;
                Ok(())
            })?
            .encoded_length_with_header();
        assert_eq!(len, actual);

        let mut seen = Vec::new();
        let _c = MsgDecoder::try_decode(&storage[..actual], 0)?
            .into_rows(|e| -> Result<_, sbe_rt::DecodeError> {
                assert_eq!(e.tag(), 7u32);
                let (fixed, done) = e.into_fixed()?;
                seen.push(fixed.to_vec());
                Ok(done)
            })?;
        assert_eq!(seen, vec![b"abc".to_vec()]);
    "#,
    );
    Ok(())
}
