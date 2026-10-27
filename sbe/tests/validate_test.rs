//! Opt-in `validate()` (`GenerationConfig::with_validate(true)`).
//!
//! `try_decode` proves bounds only. `validate()` also checks what a foreign
//! or older producer can get wrong on the wire:
//!
//! | Check | Error |
//! |---|---|
//! | value outside a declared `minValue` / `maxValue` | `ValueOutOfRange` |
//! | enum discriminant the schema does not declare | `UnknownEnum` |
//! | set bit no choice declares | `UnknownSetBits` |
//!
//! Fixed char arrays are not checked (NUL padding is valid SBE), and a bound
//! the schema does not declare is not checked. The runtime test builds frames
//! byte by byte (and with `fixed()`, which checks no range), so each failure
//! names the exact wire byte that causes it.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

mod common;
use common::{compile_and_run_modules, impl_for_type_defines_method};
use ergo_sbe::{GenerateError, GenerationConfig, Generator, Schema, SchemaSource, parse};

const SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="checks" id="91" version="1" byteOrder="littleEndian">
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
    <type name="Pct" primitiveType="uint8" minValue="0" maxValue="100"/>
    <type name="Qty" primitiveType="int32" minValue="1" maxValue="1000"/>
    <type name="Cap" primitiveType="uint16" maxValue="500"/>
    <type name="Level" primitiveType="int16" presence="optional" nullValue="-32768" minValue="-5" maxValue="5"/>
    <type name="Code" primitiveType="char" length="4"/>
    <type name="Lots" primitiveType="uint16" length="2" maxValue="10"/>
    <enum name="Side" encodingType="uint8">
      <validValue name="Buy">1</validValue>
      <validValue name="Sell">2</validValue>
    </enum>
    <set name="Flags" encodingType="uint8">
      <choice name="A">0</choice>
      <choice name="C">2</choice>
    </set>
    <composite name="Px">
      <type name="mantissa" primitiveType="int64" minValue="-1000000" maxValue="1000000"/>
      <type name="exponent" primitiveType="int8" presence="constant">-2</type>
    </composite>
  </types>
  <sbe:message name="Order" id="1">
    <field name="pct" id="1" type="Pct"/>
    <field name="qty" id="2" type="Qty"/>
    <field name="cap" id="3" type="Cap"/>
    <field name="level" id="4" type="Level"/>
    <field name="code" id="5" type="Code"/>
    <field name="side" id="6" type="Side"/>
    <field name="flags" id="7" type="Flags"/>
    <field name="px" id="8" type="Px"/>
    <field name="lots" id="9" type="Lots"/>
    <field name="late" id="10" type="Pct" sinceVersion="1"/>
    <group name="legs" id="20" dimensionType="groupSizeEncoding">
      <field name="legQty" id="21" type="Qty"/>
      <field name="legSide" id="22" type="Side"/>
    </group>
    <group name="fills" id="30" dimensionType="groupSizeEncoding">
      <field name="fillSide" id="31" type="Side"/>
      <data name="note" id="32" type="varStringEncoding"/>
    </group>
  </sbe:message>
  <sbe:message name="Quote" id="2">
    <field name="pct" id="1" type="Pct"/>
    <field name="qty" id="2" type="Qty"/>
    <field name="seq" id="3" type="uint32"/>
    <field name="delta" id="4" type="int32"/>
  </sbe:message>
</sbe:messageSchema>"#;

fn generate_from(xml: &str, config: GenerationConfig) -> Result<String, GenerateError> {
    let schema = parse(SchemaSource::xml(xml))
        .map(Schema::from_ir)
        .map_err(|e| GenerateError::InvalidConversion {
            selector: "fixture".into(),
            reason: e.to_string(),
        })?;
    let set = Generator::new(config).generate(&schema)?;
    Ok(set
        .modules()
        .next()
        .map(|m| m.source.clone())
        .unwrap_or_default())
}

fn generate(config: GenerationConfig) -> Result<String, GenerateError> {
    generate_from(SCHEMA, config)
}

#[test]
fn validate_is_opt_in() -> Result<(), GenerateError> {
    let plain = generate(GenerationConfig::new("plain"))?;
    for ty in [
        "OrderDecoder",
        "QuoteDecoder",
        "OrderLegsEntryDecoder",
        "AnyMessage",
    ] {
        assert!(
            !impl_for_type_defines_method(&plain, ty, "validate"),
            "{ty}::validate must not exist without with_validate(true)"
        );
    }

    let checked = generate(GenerationConfig::new("checked").with_validate(true))?;
    for ty in [
        "OrderDecoder",
        "QuoteDecoder",
        "OrderLegsEntryDecoder",
        "OrderFillsEntryDecoder",
        "AnyMessage",
    ] {
        assert!(
            impl_for_type_defines_method(&checked, ty, "validate"),
            "{ty}::validate missing with with_validate(true)"
        );
    }
    Ok(())
}

/// `validate()` only adds items: every item of the plain module, including
/// `try_decode` and every getter, is in the checked module token for token.
#[test]
fn validate_adds_items_and_changes_none() -> Result<(), Box<dyn std::error::Error>> {
    let plain = syn::parse_file(&generate(GenerationConfig::new("m"))?)?;
    let checked = syn::parse_file(&generate(GenerationConfig::new("m").with_validate(true))?)?;
    let render = |item: &syn::Item| quote::quote!(#item).to_string();
    let checked_items: std::collections::HashSet<String> =
        checked.items.iter().map(render).collect();
    for item in &plain.items {
        assert!(
            checked_items.contains(&render(item)),
            "with_validate(true) changed an existing item:\n{}",
            render(item).chars().take(300).collect::<String>()
        );
    }
    assert!(checked.items.len() > plain.items.len());
    Ok(())
}

#[test]
fn a_member_named_validate_is_rejected_only_when_enabled() -> Result<(), GenerateError> {
    for (label, xml) in [
        (
            "field",
            SCHEMA.replace(r#"name="cap" id="3""#, r#"name="validate" id="3""#),
        ),
        (
            "group",
            SCHEMA.replace(r#"name="legs" id="20""#, r#"name="validate" id="20""#),
        ),
        (
            "entry field",
            SCHEMA.replace(r#"name="legQty" id="21""#, r#"name="validate" id="21""#),
        ),
        (
            "entry data",
            SCHEMA.replace(r#"name="note" id="32""#, r#"name="validate" id="32""#),
        ),
    ] {
        generate_from(&xml, GenerationConfig::new("m"))?;
        match generate_from(&xml, GenerationConfig::new("m").with_validate(true)) {
            Err(GenerateError::InvalidConfiguration { option, value, .. }) => {
                assert_eq!(option, "validate", "{label}");
                assert!(value.ends_with(".validate"), "{label}: {value}");
            }
            other => panic!("{label}: expected InvalidConfiguration, got {other:?}"),
        }
    }
    Ok(())
}

#[test]
fn validate_reports_each_wire_violation() -> Result<(), GenerateError> {
    let src = generate(GenerationConfig::new("vt").with_validate(true))?;
    compile_and_run_modules(
        "validate_runtime",
        &[("vt", &src)],
        r#"
        use vt::*;
        use vt::sbe_rt::{DecodeError, ValidateError};
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::sync::atomic::{AtomicUsize, Ordering};

        // Zero heap allocations inside validate().
        struct Counting;
        static ALLOCS: AtomicUsize = AtomicUsize::new(0);
        unsafe impl GlobalAlloc for Counting {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                ALLOCS.fetch_add(1, Ordering::SeqCst);
                unsafe { System.alloc(layout) }
            }
            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                unsafe { System.dealloc(ptr, layout) }
            }
        }
        #[global_allocator]
        static GLOBAL: Counting = Counting;

        // DecodeError keeps its size: the new checks live in ValidateError.
        assert_eq!(core::mem::size_of::<DecodeError>(), 40);

        // Order body (block length 28 at version 1, 27 at version 0):
        // pct@0 u8, qty@1 i32, cap@5 u16, level@7 i16 (optional),
        // code@9 char[4], side@13 u8, flags@14 u8, px.mantissa@15 i64,
        // lots@23 u16[2], late@27 u8 (sinceVersion 1).
        fn body() -> Vec<u8> {
            let mut b = Vec::new();
            b.push(50u8);
            b.extend_from_slice(&10i32.to_le_bytes());
            b.extend_from_slice(&500u16.to_le_bytes());
            b.extend_from_slice(&i16::MIN.to_le_bytes()); // level null
            b.extend_from_slice(b"AB\0\0"); // NUL-padded fixed char array
            b.push(1); // Side::Buy
            b.push(0b101); // A | C
            b.extend_from_slice(&12_345i64.to_le_bytes());
            b.extend_from_slice(&3u16.to_le_bytes());
            b.extend_from_slice(&10u16.to_le_bytes());
            b.push(7); // late
            assert_eq!(b.len(), 28);
            b
        }
        fn frame(version: u16, body: &[u8], legs: &[(i32, u8)], fills: &[u8]) -> Vec<u8> {
            let mut f = Vec::new();
            for v in [body.len() as u16, 1, 91, version] {
                f.extend_from_slice(&v.to_le_bytes());
            }
            f.extend_from_slice(body);
            f.extend_from_slice(&5u16.to_le_bytes());
            f.extend_from_slice(&(legs.len() as u16).to_le_bytes());
            for (q, s) in legs {
                f.extend_from_slice(&q.to_le_bytes());
                f.push(*s);
            }
            f.extend_from_slice(&1u16.to_le_bytes());
            f.extend_from_slice(&(fills.len() as u16).to_le_bytes());
            for s in fills {
                f.push(*s);
                f.extend_from_slice(&2u32.to_le_bytes());
                f.extend_from_slice(b"ok");
            }
            f
        }
        fn check(f: &[u8]) -> Result<(), ValidateError> {
            // try_decode is unchanged: bounds only, so every frame here decodes.
            let d = OrderDecoder::try_decode(f, 0).expect("bounds-valid frame");
            let before = ALLOCS.load(Ordering::SeqCst);
            let r = d.validate();
            assert_eq!(ALLOCS.load(Ordering::SeqCst), before, "validate allocated");
            // AnyMessage reaches the same check.
            let any = AnyMessage::decode(f, 0).expect("dispatch");
            assert_eq!(any.validate(), r);
            r
        }
        let range = |field: &'static str, min: i128, max: i128, actual: i128| {
            Err(ValidateError::ValueOutOfRange { field, min, max, actual })
        };

        // A valid frame passes: null optional, NUL-padded chars, NullVal-free enums.
        let good = frame(1, &body(), &[(5, 2), (1000, 1)], &[1, 2]);
        assert_eq!(check(&good), Ok(()));

        // Declared ranges, both bounds, max-only, optional and composite members.
        let mut b = body(); b[0] = 101;
        assert_eq!(check(&frame(1, &b, &[], &[])), range("Order.pct", 0, 100, 101));
        let mut b = body(); b[1..5].copy_from_slice(&0i32.to_le_bytes());
        assert_eq!(check(&frame(1, &b, &[], &[])), range("Order.qty", 1, 1000, 0));
        let mut b = body(); b[5..7].copy_from_slice(&501u16.to_le_bytes());
        assert_eq!(check(&frame(1, &b, &[], &[])), range("Order.cap", 0, 500, 501));
        let mut b = body(); b[5..7].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(check(&frame(1, &b, &[], &[])), Ok(()), "only maxValue is declared");
        let mut b = body(); b[7..9].copy_from_slice(&6i16.to_le_bytes());
        assert_eq!(check(&frame(1, &b, &[], &[])), range("Order.level", -5, 5, 6));
        let mut b = body(); b[7..9].copy_from_slice(&(-5i16).to_le_bytes());
        assert_eq!(check(&frame(1, &b, &[], &[])), Ok(()));
        let mut b = body(); b[15..23].copy_from_slice(&2_000_000i64.to_le_bytes());
        assert_eq!(
            check(&frame(1, &b, &[], &[])),
            range("Order.px.mantissa", -1_000_000, 1_000_000, 2_000_000)
        );
        let mut b = body(); b[25..27].copy_from_slice(&11u16.to_le_bytes());
        assert_eq!(check(&frame(1, &b, &[], &[])), range("Order.lots", 0, 10, 11));

        // Fixed char arrays are excluded: any byte passes.
        let mut b = body(); b[9..13].copy_from_slice(&[0, 0xff, 0x01, 0x7f]);
        assert_eq!(check(&frame(1, &b, &[], &[])), Ok(()));

        // Enum discriminants: undeclared fails, NullVal (255) is declared.
        let mut b = body(); b[13] = 3;
        assert_eq!(
            check(&frame(1, &b, &[], &[])),
            Err(ValidateError::UnknownEnum { field: "Order.side", discriminant: 3 })
        );
        let mut b = body(); b[13] = 255;
        assert_eq!(check(&frame(1, &b, &[], &[])), Ok(()));

        // Set bits: bit 1 has no choice.
        let mut b = body(); b[14] = 0b111;
        assert_eq!(
            check(&frame(1, &b, &[], &[])),
            Err(ValidateError::UnknownSetBits { field: "Order.flags", bits: 0b010 })
        );

        // sinceVersion: `late` is checked at version 1 and absent at version 0.
        let mut b = body(); b[27] = 200;
        assert_eq!(check(&frame(1, &b, &[], &[])), range("Order.late", 0, 100, 200));
        assert_eq!(check(&frame(0, &b[..27], &[], &[])), Ok(()));

        // Group entries: fixed-stride and dynamic (var-data) groups.
        assert_eq!(
            check(&frame(1, &body(), &[(5, 1), (0, 1)], &[])),
            range("Order.legs.legQty", 1, 1000, 0)
        );
        assert_eq!(
            check(&frame(1, &body(), &[(5, 1)], &[1, 9])),
            Err(ValidateError::UnknownEnum { field: "Order.fills.fillSide", discriminant: 9 })
        );
        let bad_leg = frame(1, &body(), &[(5, 7)], &[]);
        let d = OrderDecoder::try_decode(&bad_leg, 0)?;
        let entry = d.legs()?.next().expect("one leg");
        assert_eq!(
            entry.validate(),
            Err(ValidateError::UnknownEnum { field: "Order.legs.legSide", discriminant: 7 })
        );

        // A truncated tail surfaces as the decode error that walking it hits.
        let cut = &good[..good.len() - 1];
        assert!(matches!(check(cut), Err(ValidateError::Decode(_))));

        // fixed() writes without a range check; validate() catches it.
        let mut qbuf = [0u8; QuoteEncoder::ENCODED_LENGTH];
        let _ = QuoteEncoder::wrap_and_apply_header(&mut qbuf, 0)
            .fixed(&QuoteFixedFields { pct: 101, qty: 5, seq: 0, delta: 0 });
        let q = QuoteDecoder::try_decode(&qbuf, 0)?;
        assert_eq!(q.pct(), 101);
        assert_eq!(q.validate(), range("Quote.pct", 0, 100, 101));
        assert_eq!(
            AnyMessage::decode(&qbuf, 0)?.validate(),
            range("Quote.pct", 0, 100, 101)
        );

        // No declared bound, no check: the SBE default range (which excludes
        // each type's null) is not applied, so u32::MAX and i32::MIN pass.
        let mut ubuf = [0u8; QuoteEncoder::ENCODED_LENGTH];
        let _ = QuoteEncoder::wrap_and_apply_header(&mut ubuf, 0)
            .fixed(&QuoteFixedFields { pct: 50, qty: 5, seq: u32::MAX, delta: i32::MIN });
        assert_eq!(QuoteDecoder::try_decode(&ubuf, 0)?.validate(), Ok(()));

        // An unknown template is not silently accepted.
        let mut other = qbuf;
        other[2..4].copy_from_slice(&77u16.to_le_bytes());
        assert_eq!(
            AnyMessage::decode_frame(&other, 0, other.len())?.message.validate(),
            Err(ValidateError::UnknownTemplate { template_id: 77 })
        );

        // Display and source.
        let e = q.validate().expect_err("out of range");
        assert_eq!(e.to_string(), "field 'Quote.pct': value 101 outside schema range [0, 100]");
        let decode = ValidateError::from(DecodeError::InvalidAscii { field: "x" });
        assert!(std::error::Error::source(&decode).is_some());
        println!("PASS: validate_reports_each_wire_violation");
        "#,
    );
    Ok(())
}

/// Shared schema for the `with_shared` tests: a bounded composite member, an
/// enum and a set, none declared by the consumer.
const SHARED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="vshared" id="92" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="Px">
      <type name="mantissa" primitiveType="int64" minValue="-1000000" maxValue="1000000"/>
      <type name="exponent" primitiveType="int8" presence="constant">-2</type>
    </composite>
    <enum name="Side" encodingType="uint8">
      <validValue name="Buy">1</validValue>
      <validValue name="Sell">2</validValue>
    </enum>
    <set name="Marks" encodingType="uint8">
      <choice name="Low">0</choice>
      <choice name="High">2</choice>
    </set>
  </types>
</sbe:messageSchema>"#;

const CONSUMER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="vconsumer" id="93" version="0" byteOrder="littleEndian" headerType="messageHeader">
  <sbe:message name="Order" id="1">
    <field name="px" id="1" type="Px"/>
    <field name="side" id="2" type="Side"/>
    <field name="marks" id="3" type="Marks"/>
  </sbe:message>
</sbe:messageSchema>"#;

/// Runtime checks on the consumer's `Order`; `{rt}` is the module that owns
/// `sbe_rt`.
const SHARED_CHECKS: &str = r#"
        use vconsumer::*;
        use {rt}::sbe_rt::ValidateError;

        // Order body: px.mantissa@0 i64 (shared Px), side@8 u8 (shared Side),
        // marks@9 u8 (shared Marks).
        fn frame(mantissa: i64, side: u8, marks: u8) -> Vec<u8> {
            let mut f = Vec::new();
            for v in [10u16, 1, 93, 0] {
                f.extend_from_slice(&v.to_le_bytes());
            }
            f.extend_from_slice(&mantissa.to_le_bytes());
            f.push(side);
            f.push(marks);
            f
        }
        let check = |f: &[u8]| OrderDecoder::try_decode(f, 0).expect("bounds-valid frame").validate();

        assert_eq!(check(&frame(12_345, 1, 0b101)), Ok(()));
        assert_eq!(
            check(&frame(2_000_000, 1, 0)),
            Err(ValidateError::ValueOutOfRange {
                field: "Order.px.mantissa",
                min: -1_000_000,
                max: 1_000_000,
                actual: 2_000_000,
            })
        );
        assert_eq!(
            check(&frame(0, 3, 0)),
            Err(ValidateError::UnknownEnum { field: "Order.side", discriminant: 3 })
        );
        assert_eq!(
            check(&frame(0, 1, 0b010)),
            Err(ValidateError::UnknownSetBits { field: "Order.marks", bits: 0b010 })
        );
"#;

/// A consumer parsed `with_shared` and generated through `generate_multi`
/// checks the shared composites, enums and sets its fields use: the shared
/// schema's types travel with the consumer's IR, so `validate()` finds their
/// bounds.
#[test]
fn validate_checks_shared_types_in_a_consumer_module() -> Result<(), Box<dyn std::error::Error>> {
    let shared = parse(SchemaSource::xml(SHARED))?;
    let consumer = parse(SchemaSource::xml(CONSUMER).with_shared(&shared))?;
    let (shared, consumer) = (Schema::from_ir(shared), Schema::from_ir(consumer));
    let set = Generator::new(
        GenerationConfig::new("vshared")
            .with_shared_module("vshared")
            .with_validate(true),
    )
    .generate_multi(&[(&shared, "vshared"), (&consumer, "vconsumer")])?;
    let source = |path: &str| {
        set.modules()
            .find(|m| m.path == path)
            .map(|m| m.source.clone())
            .ok_or_else(|| format!("no {path}"))
    };
    compile_and_run_modules(
        "validate_shared_types",
        &[
            ("vshared", &source("vshared.rs")?),
            ("vconsumer", &source("vconsumer.rs")?),
        ],
        &SHARED_CHECKS.replace("{rt}", "vshared"),
    );
    Ok(())
}

/// The same consumer generated on its own (`ergo_sbe::generate`) emits the
/// shared types itself, and its `validate()` checks them the same way.
#[test]
fn validate_checks_shared_types_in_a_self_contained_consumer()
-> Result<(), Box<dyn std::error::Error>> {
    let shared = parse(SchemaSource::xml(SHARED))?;
    let set = ergo_sbe::generate(
        SchemaSource::xml(CONSUMER).with_shared(&shared),
        GenerationConfig::new("vconsumer").with_validate(true),
    )?;
    let source = &set.modules().next().ok_or("one module")?.source;
    compile_and_run_modules(
        "validate_shared_self_contained",
        &[("vconsumer", source)],
        &SHARED_CHECKS.replace("{rt}", "vconsumer"),
    );
    Ok(())
}
