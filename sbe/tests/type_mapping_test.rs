//! Generated mappings between SBE types and application types:
//! `with_enum_mapping` (an SBE enum to an application enum, variant by
//! variant, with renames) and `with_composite_mapping` (an SBE composite to
//! an application struct, member by member, with renames; constant members
//! skipped, optional members as `Option`).
//!
//! The mappings are `TryFromSbe` / `TryToSbe` impls with an exhaustive
//! `match` / struct literal, so a mismatch between the two types is a compile
//! error in the generated module, never a silent default. `NullVal` — the
//! schema null, and every discriminant the schema does not declare, which
//! the decoder folds into `NullVal` — has no application value and decodes
//! as `sbe_rt::MappingError::NullEnum`. No mapping changes a wire byte: the
//! runtime test compares every encoded frame with the plain-codec encoding.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{compile_and_run_modules, compile_fails_with_diagnostics};
use ergo_sbe::{
    CompositeMapping, ConversionSelector, DomainVarData, EnumMapping, GenerateError,
    GenerationConfig, Generator, Schema, parse_file,
};

fn schema() -> Result<Schema, Box<dyn std::error::Error>> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/domain-mapping-schema.xml"
    );
    Ok(Schema::from_ir(parse_file(path)?))
}

fn generate(config: GenerationConfig) -> Result<String, Box<dyn std::error::Error>> {
    let set = Generator::new(config).generate(&schema()?)?;
    Ok(set
        .modules()
        .next()
        .ok_or("no module generated")?
        .source
        .clone())
}

/// The mapping every runtime test uses: `Buy`/`Sell` renamed, `sellShort`
/// by its generated name; `qty` renamed; `legVersion` (constant) skipped.
fn mapped(module: &str) -> GenerationConfig {
    // ANCHOR: mapping_config
    GenerationConfig::new(module)
        .with_enum_mapping(
            EnumMapping::new("Side", "app::Side")
                .rename("Buy", "Bid")
                .rename("Sell", "Ask"),
        )
        .with_composite_mapping(CompositeMapping::new("Price", "app::Px"))
        .with_composite_mapping(CompositeMapping::new("Leg", "app::Leg").rename("qty", "quantity"))
    // ANCHOR_END: mapping_config
}

const APP: &str = r#"
// ANCHOR: app_types
pub mod app {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Side { Bid, Ask, SellShort }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Px { pub mantissa: i64, pub exponent: i8 }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Leg {
        pub price: Px,
        pub side: Side,
        pub quantity: u32,
        pub min_qty: Option<u32>,
        pub ratio: Option<f64>,
        pub flags: super::Flags,
        pub venue: [u8; 4],
    }
}
// ANCHOR_END: app_types
"#;

/// Accessors, the DTO and both trait directions with every mapping
/// configured, against a second module generated without mappings: the
/// bytes must be identical, and `NullVal` / unknown discriminants must be
/// errors (or `None` under `with_null_as_option`), never a default value.
#[test]
fn mapped_types_round_trip_without_changing_wire_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let mapped_src = generate(mapped("dm").with_domain_objects(DomainVarData::Strings))? + APP;
    let plain_src = generate(GenerationConfig::new("plain"))?;
    let optional_src =
        generate(mapped("dmo").with_null_as_option(ConversionSelector::named_type("Side")))? + APP;
    compile_and_run_modules(
        "type_mapping_runtime",
        &[
            ("dm", &mapped_src),
            ("plain", &plain_src),
            ("dmo", &optional_src),
        ],
        r#"
        use dm::{TryFromSbe, TryToSbe};

        // ── enum: every variant both ways, renames applied ────────────
        for (app_side, wire) in [
            (dm::app::Side::Bid, dm::Side::Buy),
            (dm::app::Side::Ask, dm::Side::Sell),
            (dm::app::Side::SellShort, dm::Side::SellShort),
        ] {
            assert_eq!(<dm::app::Side as TryToSbe<dm::Side>>::try_to_sbe(&app_side), Ok(wire));
            assert_eq!(<dm::app::Side as TryFromSbe<dm::Side>>::try_from_sbe(wire), Ok(app_side));
        }
        assert_eq!(
            <dm::app::Side as TryFromSbe<dm::Side>>::try_from_sbe(dm::Side::NullVal),
            Err(dm::sbe_rt::MappingError::NullEnum { enum_name: "Side" })
        );

        // ── composite: struct literal by member name ──────────────────
        let leg = dm::app::Leg {
            price: dm::app::Px { mantissa: 12_345, exponent: -2 },
            side: dm::app::Side::Ask,
            quantity: 10,
            min_qty: Some(4),
            ratio: Some(0.5),
            flags: dm::Flags(1),
            venue: *b"XLON",
        };
        let wire_leg = <dm::app::Leg as TryToSbe<dm::Leg>>::try_to_sbe(&leg)?;
        let plain_leg = plain::Leg::new(
            plain::Price::new(12_345, -2), plain::Side::Sell, 10, 4, 0.5, plain::Flags(1), *b"XLON",
        );
        assert_eq!(wire_leg.0, plain_leg.0, "mapping must not change the wire image");
        assert_eq!(wire_leg.leg_version(), 1, "constant member stays the schema constant");
        assert_eq!(<dm::app::Leg as TryFromSbe<dm::Leg>>::try_from_sbe(wire_leg), Ok(leg));

        // Optional members: None writes the null image and reads back None.
        let absent = dm::app::Leg { min_qty: None, ratio: None, ..leg };
        let wire_absent = <dm::app::Leg as TryToSbe<dm::Leg>>::try_to_sbe(&absent)?;
        assert_eq!((wire_absent.min_qty(), wire_absent.ratio()), (None, None));
        assert_eq!(<dm::app::Leg as TryFromSbe<dm::Leg>>::try_from_sbe(wire_absent), Ok(absent));

        // Some(null value) would read back as None: refused, not written.
        for (bad, member) in [
            (dm::app::Leg { min_qty: Some(u32::MAX), ..leg }, "minQty"),
            (dm::app::Leg { ratio: Some(f64::NAN), ..leg }, "ratio"),
        ] {
            assert_eq!(
                <dm::app::Leg as TryToSbe<dm::Leg>>::try_to_sbe(&bad),
                Err(dm::sbe_rt::MappingError::SomeNullValue { composite: "Leg", member })
            );
        }

        // A nested enum holding NullVal fails the whole composite.
        let null_side = dm::Leg::new(dm::Price::new(1, 0), dm::Side::NullVal, 1, 1, 0.5, dm::Flags(0), *b"XLON");
        assert_eq!(
            <dm::app::Leg as TryFromSbe<dm::Leg>>::try_from_sbe(null_side),
            Err(dm::sbe_rt::MappingError::NullEnum { enum_name: "Side" })
        );

        // ── message accessors and DTO, against the plain codec ────────
        let mut plain_buf = [0u8; 128];
        let plain_len = plain::OrderEncoder::try_wrap_and_apply_header(&mut plain_buf, 0)?
            .fixed(&plain::OrderFixedFields {
                order_id: 7,
                side: plain::Side::SellShort,
                leg: plain_leg,
                limit: Some(99),
            })
            .fills(0, |_| Ok(()))?
            .note(b"hi")?
            .encoded_length_with_header();

        let mut buf = [0u8; 128];
        let mut enc = dm::OrderEncoder::try_wrap_and_apply_header(&mut buf, 0)?.fixed(
            &dm::OrderFixedFields {
                order_id: 7,
                side: dm::Side::NullVal,
                leg: dm::Leg([0; 31]),
                limit: Some(99),
            },
        );
        enc.side(dm::app::Side::SellShort)?;
        enc.leg(leg)?;
        let len = enc.fills(0, |_| Ok(()))?.note(b"hi")?.encoded_length_with_header();
        assert_eq!(&buf[..len], &plain_buf[..plain_len], "mapped setters write the plain bytes");

        let dec = dm::OrderDecoder::decode(&buf, 0)?;
        assert_eq!(dec.side(), Ok(dm::app::Side::SellShort));
        assert_eq!(dec.leg(), Ok(leg));
        assert_eq!(dec.side_wire(), dm::Side::SellShort);

        let dto = dm::OrderDomain::try_from_slice_with_header(&buf[..len], 0)?;
        assert_eq!((dto.side, dto.leg), (dm::app::Side::SellShort, leg));
        let mut again = [0u8; 128];
        let n = dto.encode(&mut again)?;
        assert_eq!(&again[..n], &buf[..len], "DTO re-encode is byte-identical");

        // ── NullVal and unknown discriminants on the wire ─────────────
        // ANCHOR: null_and_unknown
        let side_at = dm::OrderEncoder::HEADER_LENGTH + 8;
        assert_eq!(buf[side_at], 5, "sellShort's discriminant");
        for raw in [0u8, 9] {
            // 0 is NullVal; 9 is undeclared and folds into NullVal on decode.
            buf[side_at] = raw;
            let dec = dm::OrderDecoder::decode(&buf, 0)?;
            assert_eq!(dec.side_wire(), dm::Side::NullVal);
            assert_eq!(dec.side(), Err(dm::sbe_rt::MappingError::NullEnum { enum_name: "Side" }));
            assert!(dm::OrderDomain::try_from_slice_with_header(&buf[..len], 0).is_err());

            // with_null_as_option: absent instead of an error — for the
            // undeclared 9 too, the folding limitation.
            let dec = dmo::OrderDecoder::decode(&buf, 0)?;
            assert_eq!(dec.side(), Ok(None));
        }
        buf[side_at] = 2;
        assert_eq!(dmo::OrderDecoder::decode(&buf, 0)?.side(), Ok(Some(dmo::app::Side::Ask)));
        // ANCHOR_END: null_and_unknown

        // Display names the type.
        let shown = dm::sbe_rt::MappingError::NullEnum { enum_name: "Side" }.to_string();
        assert!(shown.contains("Side") && shown.contains("NullVal"), "{shown}");
        "#,
    );
    Ok(())
}

/// A global allocator that counts this thread's allocations while `count`
/// runs, appended to a generated module so a scratch crate can measure it.
const COUNTING_ALLOCATOR: &str = r#"
pub mod counting {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static ON: Cell<bool> = const { Cell::new(false) };
        static COUNT: Cell<u64> = const { Cell::new(0) };
    }

    pub struct Counting;

    // SAFETY: forwards every call to the system allocator unchanged.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            if ON.with(Cell::get) {
                COUNT.with(|n| n.set(n.get() + 1));
            }
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    #[global_allocator]
    static GLOBAL: Counting = Counting;

    pub fn count<R>(f: impl FnOnce() -> R) -> (R, u64) {
        COUNT.with(|n| n.set(0));
        ON.with(|c| c.set(true));
        let out = f();
        ON.with(|c| c.set(false));
        (out, COUNT.with(Cell::get))
    }
}
"#;

/// The mappings sit on the decode and encode paths (the mapped accessors
/// call them), so they must not allocate: both directions of both
/// mappings, the mapped setters and getters, and the `NullEnum` and
/// `SomeNullValue` refusals.
#[test]
fn mappings_do_not_allocate() -> Result<(), Box<dyn std::error::Error>> {
    let source = generate(mapped("dm"))? + APP + COUNTING_ALLOCATOR;
    compile_and_run_modules(
        "type_mapping_alloc",
        &[("dm", &source)],
        r#"
        use dm::{TryFromSbe, TryToSbe};
        let leg = dm::app::Leg {
            price: dm::app::Px { mantissa: 12_345, exponent: -2 },
            side: dm::app::Side::Ask,
            quantity: 10,
            min_qty: None,
            ratio: Some(0.5),
            flags: dm::Flags(1),
            venue: *b"XLON",
        };
        let mut buf = [0u8; 128];
        let (seen, allocations) = dm::counting::count(|| {
            let wire = <dm::app::Leg as TryToSbe<dm::Leg>>::try_to_sbe(&leg);
            let back = wire.map(<dm::app::Leg as TryFromSbe<dm::Leg>>::try_from_sbe);
            let null = <dm::app::Side as TryFromSbe<dm::Side>>::try_from_sbe(dm::Side::NullVal);
            let some_null = <dm::app::Leg as TryToSbe<dm::Leg>>::try_to_sbe(
                &dm::app::Leg { min_qty: Some(u32::MAX), ..leg },
            );
            let frame = dm::OrderEncoder::try_wrap_and_apply_header(&mut buf, 0).and_then(|enc| {
                let mut enc = enc.fixed(&dm::OrderFixedFields {
                    order_id: 7,
                    side: dm::Side::Buy,
                    leg: dm::Leg([0; 31]),
                    limit: None,
                });
                let mapped = enc.side(dm::app::Side::SellShort).is_ok() && enc.leg(leg).is_ok();
                Ok((mapped, enc.fills(0, |_| Ok(()))?.note(b"n")?.encoded_length_with_header()))
            });
            let decoded = frame.ok().and_then(|(mapped, len)| {
                let dec = dm::OrderDecoder::decode(&buf[..len], 0).ok()?;
                Some((mapped, dec.side(), dec.leg()))
            });
            (back, null, some_null.is_err(), decoded)
        });
        assert_eq!(allocations, 0, "mappings allocated {allocations} times");
        let (back, null, some_null_refused, decoded) = seen;
        assert_eq!(back, Ok(Ok(leg)));
        assert!(null.is_err() && some_null_refused);
        assert_eq!(decoded, Some((true, Ok(dm::app::Side::SellShort), Ok(leg))));
        "#,
    );
    Ok(())
}

/// Both directions are checked by the compiler: an application enum missing
/// a schema variant, one with a variant the schema lacks, a struct missing a
/// member and a struct with a field no member fills are all compile errors
/// in the generated impls.
#[test]
fn mismatched_application_types_do_not_compile() -> Result<(), Box<dyn std::error::Error>> {
    let enums = generate(
        GenerationConfig::new("e").with_enum_mapping(EnumMapping::new("Side", "app::Side")),
    )?;
    let structs = generate(
        GenerationConfig::new("s")
            .with_composite_mapping(CompositeMapping::new("Price", "app::Px")),
    )?;
    let source = format!(
        "pub mod missing_variant {{\n{enums}\npub mod app {{ #[derive(Debug)] pub enum Side {{ Buy, Sell }} }}\n}}\n\
         pub mod extra_variant {{\n{enums}\npub mod app {{ #[derive(Debug)] pub enum Side {{ Buy, Sell, SellShort, Cross }} }}\n}}\n\
         pub mod missing_field {{\n{structs}\npub mod app {{ #[derive(Debug)] pub struct Px {{ pub mantissa: i64 }} }}\n}}\n\
         pub mod extra_field {{\n{structs}\npub mod app {{ #[derive(Debug)] pub struct Px {{ pub mantissa: i64, pub exponent: i8, pub tick: u8 }} }}\n}}\n"
    );
    compile_fails_with_diagnostics(
        "type_mapping_mismatch",
        &source,
        "",
        &[
            "named `SellShort` found for enum",
            "Side::Cross` not covered",
            "has no field named `exponent`",
            "missing field `tick`",
        ],
    );
    Ok(())
}

/// On a `presence="optional"` field `NullVal` is not malformed: it is the
/// field being absent. A mapped enum has no variant for it, so such a field
/// would turn every valid frame without it into a decode error (the
/// accessor and the DTO alike). The mapping therefore needs
/// `with_null_as_option` on that enum, and says so at generate time; with it,
/// the absent field decodes as `None` and a present one as its variant, and
/// a DTO entry's `None` encodes as `NullVal` again.
#[test]
fn a_mapped_enum_on_an_optional_field_needs_null_as_option()
-> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<messageSchema package="opt" id="12" version="0" byteOrder="littleEndian">
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
    <enum name="Side" encodingType="uint8">
      <validValue name="Buy">1</validValue>
      <validValue name="Sell">2</validValue>
    </enum>
  </types>
  <message name="Order" id="1">
    <field name="side" id="1" type="Side"/>
  </message>
  <message name="Amend" id="2">
    <field name="id" id="1" type="uint32"/>
    <group name="legs" id="2" dimensionType="groupSizeEncoding">
      <field name="side" id="3" type="Side" presence="optional"/>
    </group>
  </message>
</messageSchema>"#;
    let schema = Schema::from_ir(ergo_sbe::parse(xml)?);
    let mapping = || EnumMapping::new("Side", "app::Side");
    let config = || {
        GenerationConfig::new("opt")
            .with_domain_objects(DomainVarData::Bytes)
            .with_enum_mapping(mapping())
    };

    match Generator::new(config()).generate(&schema) {
        Err(GenerateError::InvalidConfiguration { option, reason, .. }) => {
            assert_eq!(option, "enum_mapping");
            assert!(
                reason.contains("`Amend.legs.side`") && reason.contains("with_null_as_option"),
                "{reason}"
            );
        }
        Ok(_) => return Err("expected InvalidConfiguration, but the schema generated".into()),
        Err(e) => return Err(format!("expected InvalidConfiguration, got {e:?}").into()),
    }

    let source = Generator::new(
        config().with_null_as_option(ConversionSelector::named_type("Side")),
    )
    .generate(&schema)?
    .modules()
    .next()
    .ok_or("no module")?
    .source
    .clone()
        + "pub mod app { #[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Side { Buy, Sell } }\n";
    compile_and_run_modules(
        "type_mapping_optional",
        &[("opt", &source)],
        r#"
        let mut buf = [0u8; 64];
        let len = opt::AmendEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&opt::AmendFixedFields { id: 1 })
            .legs(2, |g| {
                g.add(|e| { e.side_wire(opt::Side::NullVal); Ok(()) })?;
                g.add(|e| { e.side_wire(opt::Side::Sell); Ok(()) })
            })?
            .encoded_length_with_header();
        let dto = opt::AmendDomain::try_from_slice_with_header(&buf[..len], 0)?;
        let sides: Vec<_> = dto.legs.iter().map(|l| l.side).collect();
        assert_eq!(sides, vec![None, Some(opt::app::Side::Sell)]);

        // Re-encoded over bytes that each read as `Sell`, `None` still
        // writes `NullVal`: the entry has no `apply_nulls` to rely on.
        let mut again = [2u8; 64];
        let n = dto.encode(&mut again)?;
        assert_eq!(&again[..n], &buf[..len]);
        "#,
    );
    Ok(())
}

/// A mapping the schema cannot satisfy is a typed `GenerateError` naming
/// the problem, raised before any code is generated.
#[test]
fn invalid_mappings_are_rejected_at_generate_time() -> Result<(), Box<dyn std::error::Error>> {
    let schema = schema()?;
    let reject =
        |config: GenerationConfig, needle: &str| -> Result<(), Box<dyn std::error::Error>> {
            match Generator::new(config).generate(&schema) {
                Err(GenerateError::InvalidConfiguration { option, reason, .. }) => {
                    let text = format!("{option}: {reason}");
                    assert!(text.contains(needle), "expected {needle:?} in {text:?}");
                    Ok(())
                }
                other => Err(format!(
                    "expected InvalidConfiguration containing {needle:?}, got {other:?}"
                )
                .into()),
            }
        };
    let m = || GenerationConfig::new("m");
    reject(
        m().with_enum_mapping(EnumMapping::new("Sides", "app::Side")),
        "no enum named `Sides`",
    )?;
    reject(
        m().with_enum_mapping(EnumMapping::new("Price", "app::Px")),
        "no enum named `Price`",
    )?;
    reject(
        m().with_enum_mapping(EnumMapping::new("Side", "app::Side").rename("Hold", "Wait")),
        "no variant `Hold`",
    )?;
    reject(
        m().with_enum_mapping(EnumMapping::new("Side", "app::Side").rename("Buy", "not a variant")),
        "`not a variant` is not a Rust identifier",
    )?;
    reject(
        m().with_enum_mapping(EnumMapping::new("Side", "app::Side").rename("Buy", "Sell")),
        "`Buy` and `Sell` both map to `Sell`",
    )?;
    reject(
        m().with_enum_mapping(EnumMapping::new("Side", "app::Side<")),
        "not a valid Rust type",
    )?;
    reject(
        m().with_enum_mapping(EnumMapping::new("Side", "bool")),
        "application enum",
    )?;
    reject(
        m().with_enum_mapping(EnumMapping::new("Side", "app::Side"))
            .with_domain_type(ConversionSelector::named_type("Side"), "other::Side"),
        "mapped to `app::Side` but its domain type is `other::Side`",
    )?;
    reject(
        m().with_composite_mapping(CompositeMapping::new("Side", "app::S")),
        "no composite named `Side`",
    )?;
    reject(
        m().with_composite_mapping(CompositeMapping::new("Leg", "app::Leg").rename("size", "n")),
        "no member `size`",
    )?;
    reject(
        m().with_composite_mapping(
            CompositeMapping::new("Leg", "app::Leg").rename("legVersion", "v"),
        ),
        "`legVersion` is constant",
    )?;
    reject(
        m().with_composite_mapping(
            CompositeMapping::new("Leg", "app::Leg").rename("qty", "min_qty"),
        ),
        "`qty` and `minQty` both map to `min_qty`",
    )?;

    // A later rename of the same variant or member replaces an earlier one
    // whichever spelling names it (schema or generated): here the later one
    // collides, so only it being applied makes the mapping invalid.
    reject(
        m().with_enum_mapping(
            EnumMapping::new("Side", "app::Side")
                .rename("Buy", "Bid")
                .rename("sellShort", "Short")
                .rename("SellShort", "Bid"),
        ),
        "`Buy` and `sellShort` both map to `Bid`",
    )?;
    reject(
        m().with_composite_mapping(
            CompositeMapping::new("Leg", "app::Leg")
                .rename("minQty", "lower")
                .rename("min_qty", "qty"),
        ),
        "`qty` and `minQty` both map to `qty`",
    )?;

    // The last mapping registered for a type wins, as with_domain_type does.
    let src = generate(
        m().with_enum_mapping(EnumMapping::new("Side", "first::Side"))
            .with_enum_mapping(EnumMapping::new("Side", "app::Side")),
    )?;
    assert!(src.contains("for app::Side"));
    assert!(!src.contains("first::Side"));
    Ok(())
}

/// A constant field of a mapped enum: `Quote.side` at message level and
/// `levels.dir` in a group entry.
const CONSTANTS_XML: &str = r#"<messageSchema package="constants" id="31" version="0" byteOrder="littleEndian">
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
    <enum name="Side" encodingType="uint8">
      <validValue name="Buy">1</validValue>
      <validValue name="Sell">2</validValue>
    </enum>
  </types>
  <message name="Quote" id="1">
    <field name="side" id="1" type="Side" presence="constant" valueRef="Side.Sell"/>
    <field name="px" id="2" type="int64"/>
    <group name="levels" id="3" dimensionType="groupSizeEncoding">
      <field name="dir" id="4" type="Side" presence="constant" valueRef="Side.Buy"/>
      <field name="qty" id="5" type="uint32"/>
    </group>
  </message>
</messageSchema>"#;

/// A constant field of a mapped enum gets the same domain getter a
/// non-constant one has, beside its `*_wire` getter, on the decoder, the
/// group entry decoder, the memoized decoder and the ordered fixed view. A
/// constant has no wire bytes and no setter, so the encoder and the DTO are
/// unchanged.
#[test]
fn a_constant_field_of_a_mapped_enum_has_a_domain_getter() -> Result<(), Box<dyn std::error::Error>>
{
    let schema = Schema::from_ir(ergo_sbe::parse(CONSTANTS_XML)?);
    let source = Generator::new(
        GenerationConfig::new("cq")
            .with_enum_mapping(
                EnumMapping::new("Side", "app::Side")
                    .rename("Buy", "Bid")
                    .rename("Sell", "Ask"),
            )
            .with_domain_objects(DomainVarData::Bytes),
    )
    .generate(&schema)?
    .modules()
    .next()
    .ok_or("no module generated")?
    .source
    .clone()
        + r#"
pub mod app {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Side { Bid, Ask }
}
"#;
    compile_and_run_modules(
        "type_mapping_constant",
        &[("cq", &source)],
        r#"
        use cq::*;

        let mut buf = [0u8; 64];
        let n = QuoteEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&QuoteFixedFields { px: 5 })
            .levels(1, |g| g.add(|e| { e.qty(3); Ok(()) }))?
            .encoded_length_with_header();

        let dec = QuoteDecoder::decode(&buf[..n], 0)?;
        assert_eq!(dec.side_wire(), Side::Sell);
        assert_eq!(dec.side(), Ok(app::Side::Ask));
        let level = dec.levels()?.next().ok_or("no level")?;
        assert_eq!(level.dir_wire(), Side::Buy);
        assert_eq!(level.dir(), Ok(app::Side::Bid));

        let memo = QuoteDecoder::decode(&buf[..n], 0)?.memoized();
        assert_eq!(memo.side(), Ok(app::Side::Ask));
        assert_eq!(memo.side_wire(), Side::Sell);

        let _ = QuoteDecoder::decode(&buf[..n], 0)?.ordered().fixed(|view| {
            assert_eq!(view.side(), Ok(app::Side::Ask));
            assert_eq!(view.side_wire(), Side::Sell);
            Ok(())
        })?;

        let dto = QuoteDomain::try_from_slice_with_header(&buf[..n], 0)?;
        let mut again = [0u8; 64];
        let m = dto.encode(&mut again)?;
        assert_eq!(&again[..m], &buf[..n]);
        "#,
    );
    Ok(())
}

/// `Quote` with a `uint32` constant `ver` (semantic type `Version`) at
/// message level and in a group entry, and a constant `side` of an enum.
const CONSTANT_CONVERSIONS_XML: &str = r#"<messageSchema package="cc" id="13" version="0" byteOrder="littleEndian">
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
    <enum name="Side" encodingType="uint8">
      <validValue name="Buy">1</validValue>
      <validValue name="Sell">2</validValue>
    </enum>
    <type name="Ver" primitiveType="uint32" presence="constant">7</type>
  </types>
  <message name="Quote" id="1">
    <field name="ver" id="1" type="Ver" semanticType="Version"/>
    <field name="side" id="2" type="Side" presence="constant" valueRef="Side.Sell"/>
    <field name="px" id="3" type="int64"/>
    <group name="levels" id="4" dimensionType="groupSizeEncoding">
      <field name="ver" id="5" type="Ver" semanticType="Version"/>
      <field name="qty" id="6" type="uint32"/>
    </group>
  </message>
</messageSchema>"#;

/// Not only a generated mapping: a constant field under any conversion gets
/// the decoder side a non-constant field has, converting what its `*_wire`
/// getter returns. `ver` is under `with_conversion`, so it has
/// `ver_as::<T>()`; `side` is under `with_manual_domain_type`, so `side()`
/// returns the application type. Decoder, entry decoder, memoized decoder
/// and ordered fixed view alike.
#[test]
fn constant_fields_get_the_decoder_side_of_any_conversion() -> Result<(), Box<dyn std::error::Error>>
{
    let schema = Schema::from_ir(ergo_sbe::parse(CONSTANT_CONVERSIONS_XML)?);
    let source = Generator::new(
        GenerationConfig::new("cc")
            .with_conversion(ConversionSelector::semantic_type("Version"))
            .with_manual_domain_type(ConversionSelector::named_type("Side"), "app::Dir"),
    )
    .generate(&schema)?
    .modules()
    .next()
    .ok_or("no module generated")?
    .source
    .clone()
        + r#"
pub mod app {
    use super::{Side, TryFromSbe, TryToSbe};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Version(pub u32);
    impl TryFromSbe<u32> for Version {
        type Error = &'static str;
        fn try_from_sbe(wire: u32) -> Result<Self, Self::Error> { Ok(Version(wire)) }
    }
    impl TryToSbe<u32> for Version {
        type Error = &'static str;
        fn try_to_sbe(&self) -> Result<u32, Self::Error> { Ok(self.0) }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Dir { Bid, Ask }
    impl TryFromSbe<Side> for Dir {
        type Error = &'static str;
        fn try_from_sbe(wire: Side) -> Result<Self, Self::Error> {
            match wire {
                Side::Buy => Ok(Dir::Bid),
                Side::Sell => Ok(Dir::Ask),
                Side::NullVal => Err("no side"),
            }
        }
    }
    impl TryToSbe<Side> for Dir {
        type Error = &'static str;
        fn try_to_sbe(&self) -> Result<Side, Self::Error> {
            Ok(match self { Dir::Bid => Side::Buy, Dir::Ask => Side::Sell })
        }
    }
}
"#;
    compile_and_run_modules(
        "type_mapping_constant_conversions",
        &[("cc", &source)],
        r#"
        use cc::*;

        let mut buf = [0u8; 64];
        let n = QuoteEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&QuoteFixedFields { px: 5 })
            .levels(1, |g| g.add(|e| { e.qty(3); Ok(()) }))?
            .encoded_length_with_header();

        let dec = QuoteDecoder::decode(&buf[..n], 0)?;
        assert_eq!(dec.ver_wire(), 7);
        assert_eq!(dec.ver_as::<app::Version>(), Ok(app::Version(7)));
        assert_eq!(dec.side_wire(), Side::Sell);
        assert_eq!(dec.side(), Ok(app::Dir::Ask));
        let level = dec.levels()?.next().ok_or("no level")?;
        assert_eq!(level.ver_wire(), 7);
        assert_eq!(level.ver_as::<app::Version>(), Ok(app::Version(7)));

        let memo = QuoteDecoder::decode(&buf[..n], 0)?.memoized();
        assert_eq!(memo.ver_as::<app::Version>(), Ok(app::Version(7)));
        assert_eq!(memo.side(), Ok(app::Dir::Ask));

        let _ = QuoteDecoder::decode(&buf[..n], 0)?.ordered().fixed(|view| {
            assert_eq!(view.ver_as::<app::Version>(), Ok(app::Version(7)));
            assert_eq!(view.side(), Ok(app::Dir::Ask));
            Ok(())
        })?;
        "#,
    );
    Ok(())
}
