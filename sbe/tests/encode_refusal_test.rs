//! What an encoder refuses, and that the refusal says where and why.
//!
//! `Some(v)` for an optional field whose `v` is the field's null value (NaN
//! for a float without another `nullValue`) would read back as `None`, so it
//! is refused: `EncodeError::SomeNullValue` from the DTO and from
//! `encode_from`, for `float` and `double`, at message level and in nested
//! groups, and `MappingError::SomeNullValue` from a composite mapping. A
//! domain conversion that refuses a value comes back as
//! `EncodeError::Conversion`, with the field's path (groups and entry
//! indices) and the conversion's own error, and a mapped setter's error
//! converts into `EncodeError` so `?` works in a `send` closure.
//!
//! The DTO and `encode_from` are fed the same values (the DTO is the
//! source), and must refuse them with equal errors.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::compile_and_run_modules;
use ergo_sbe::{
    CompositeMapping, ConversionSelector, DomainVarData, GenerationConfig, Generator, Schema, parse,
};

const REFUSAL_XML: &str = r#"<messageSchema package="refusal" id="21" version="0" byteOrder="littleEndian">
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
    <composite name="Leg">
      <type name="qty" primitiveType="uint32"/>
      <type name="ratio" primitiveType="double" presence="optional"/>
      <type name="weight" primitiveType="float" presence="optional"/>
    </composite>
    <composite name="Px">
      <type name="mantissa" primitiveType="int64"/>
      <type name="exponent" primitiveType="int8"/>
    </composite>
    <type name="OptF" primitiveType="float" presence="optional"/>
    <type name="OptD" primitiveType="double" presence="optional"/>
    <type name="OptNegD" primitiveType="double" presence="optional" nullValue="-1.0"/>
    <type name="OptU" primitiveType="uint32" presence="optional"/>
  </types>
  <message name="Order" id="1">
    <field name="id" id="1" type="uint64"/>
    <field name="f32v" id="2" type="OptF"/>
    <field name="f64v" id="3" type="OptD"/>
    <field name="neg" id="4" type="OptNegD"/>
    <field name="units" id="5" type="OptU"/>
    <field name="leg" id="6" type="Leg"/>
    <field name="px" id="7" type="Px"/>
    <field name="side" id="8" type="Side"/>
    <group name="fills" id="10" dimensionType="groupSizeEncoding">
      <field name="f64v" id="11" type="OptD"/>
      <field name="leg" id="12" type="Leg"/>
      <group name="allocs" id="13" dimensionType="groupSizeEncoding">
        <field name="weight" id="14" type="OptF"/>
        <field name="units" id="15" type="OptU"/>
        <field name="leg" id="16" type="Leg"/>
        <field name="px" id="17" type="Px"/>
        <field name="side" id="18" type="Side"/>
      </group>
    </group>
  </message>
</messageSchema>"#;

/// `Leg` maps to an application struct (so its refusal is a
/// `MappingError`), `Px` to a type whose conversion fails with a
/// `&'static str`, and `Side` to one whose conversion fails with its own
/// error type.
const APP: &str = r#"
pub mod app {
    use super::{Px, Side, TryFromSbe, TryToSbe};

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Leg { pub qty: u32, pub ratio: Option<f64>, pub weight: Option<f32> }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Money { pub cents: i64 }
    impl TryFromSbe<Px> for Money {
        type Error = &'static str;
        fn try_from_sbe(wire: Px) -> Result<Self, Self::Error> {
            Ok(Money { cents: wire.mantissa() })
        }
    }
    impl TryToSbe<Px> for Money {
        type Error = &'static str;
        fn try_to_sbe(&self) -> Result<Px, Self::Error> {
            if self.cents < 0 {
                return Err("negative money");
            }
            Ok(Px::new(self.cents, -2))
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Dir { Buy, Sell, Unknown }
    #[derive(Debug)]
    pub struct Problem;
    impl core::fmt::Display for Problem {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("no wire side")
        }
    }
    impl TryFromSbe<Side> for Dir {
        type Error = Problem;
        fn try_from_sbe(wire: Side) -> Result<Self, Self::Error> {
            match wire {
                Side::Buy => Ok(Dir::Buy),
                Side::Sell => Ok(Dir::Sell),
                Side::NullVal => Err(Problem),
            }
        }
    }
    impl TryToSbe<Side> for Dir {
        type Error = Problem;
        fn try_to_sbe(&self) -> Result<Side, Self::Error> {
            match self {
                Dir::Buy => Ok(Side::Buy),
                Dir::Sell => Ok(Side::Sell),
                Dir::Unknown => Err(Problem),
            }
        }
    }

    impl super::OrderSource for super::OrderDomain {
        fn id(&self) -> u64 { self.id }
        fn f32v(&self) -> Option<f32> { self.f32v }
        fn f64v(&self) -> Option<f64> { self.f64v }
        fn neg(&self) -> Option<f64> { self.neg }
        fn units(&self) -> Option<u32> { self.units }
        fn leg(&self) -> Leg { self.leg }
        fn px(&self) -> Money { self.px }
        fn side(&self) -> Dir { self.side }
        fn fills(&self) -> impl ExactSizeIterator<Item = impl super::FillsEntrySource> {
            self.fills.iter()
        }
    }
    impl super::FillsEntrySource for super::OrderFillsEntryDomain {
        fn f64v(&self) -> Option<f64> { self.f64v }
        fn leg(&self) -> Leg { self.leg }
        fn allocs(&self) -> impl ExactSizeIterator<Item = impl super::FillsAllocsEntrySource> {
            self.allocs.iter()
        }
    }
    impl super::FillsAllocsEntrySource for super::OrderFillsEntryAllocsEntryDomain {
        fn weight(&self) -> Option<f32> { self.weight }
        fn units(&self) -> Option<u32> { self.units }
        fn leg(&self) -> Leg { self.leg }
        fn px(&self) -> Money { self.px }
        fn side(&self) -> Dir { self.side }
    }
}
"#;

fn refusal_module() -> Result<String, Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(REFUSAL_XML)?);
    let source = Generator::new(
        GenerationConfig::new("rf")
            .with_domain_objects(DomainVarData::Strings)
            .with_source_traits(true)
            .with_composite_mapping(CompositeMapping::new("Leg", "app::Leg"))
            .with_domain_type(ConversionSelector::named_type("Px"), "app::Money")
            .with_domain_type(ConversionSelector::named_type("Side"), "app::Dir"),
    )
    .generate(&schema)?
    .modules()
    .next()
    .ok_or("no module")?
    .source
    .clone();
    Ok(source + APP)
}

/// Every refusal through the DTO and through `encode_from`, with the path
/// and the source error each must carry; and the absent values those paths
/// must still write as null (over dirty bytes), so they read back `None`.
#[test]
fn refusals_name_the_field_and_keep_the_source_error() -> Result<(), Box<dyn std::error::Error>> {
    let module = refusal_module()?;
    compile_and_run_modules(
        "encode_refusal_paths",
        &[("rf", &module)],
        r#"
        use rf::*;
        use rf::app::{Dir, Leg, Money};
        use rf::sbe_rt::{ConversionError, EncodeError, MappingError};

        let leg = Leg { qty: 1, ratio: Some(0.5), weight: Some(0.25) };
        let alloc = OrderFillsEntryAllocsEntryDomain {
            weight: Some(1.5), units: Some(3), leg, px: Money { cents: 10 }, side: Dir::Buy,
        };
        let good = OrderDomain {
            id: 1,
            f32v: Some(2.5),
            f64v: Some(-3.5),
            neg: Some(f64::NAN),
            units: Some(4),
            leg,
            px: Money { cents: 99 },
            side: Dir::Sell,
            fills: vec![
                OrderFillsEntryDomain { f64v: Some(1.0), leg, allocs: vec![alloc.clone(); 3] },
                OrderFillsEntryDomain { f64v: None, leg, allocs: vec![alloc] },
            ],
        };

        // Both paths encode the same frame over a dirty buffer, or refuse
        // with the same error.
        fn both(dto: &OrderDomain) -> Result<Vec<u8>, EncodeError> {
            let mut a = [0xAAu8; 512];
            let mut b = [0x55u8; 512];
            let via_dto = dto.encode(&mut a);
            let via_source = OrderEncoder::try_wrap_and_apply_header(&mut b, 0)
                .and_then(|enc| enc.encode_from(dto))
                .map(|done| done.encoded_length_with_header());
            assert_eq!(via_dto, via_source, "the DTO and encode_from disagree");
            let n = via_dto?;
            assert_eq!(&a[..n], &b[..n]);
            Ok(a[..n].to_vec())
        }
        fn refused(dto: &OrderDomain) -> EncodeError {
            match both(dto) {
                Err(e) => e,
                Ok(_) => panic!("encoded what should be refused: {dto:?}"),
            }
        }
        fn some_null_at(e: EncodeError) -> String {
            match e {
                EncodeError::SomeNullValue { at } => at.to_string(),
                other => panic!("expected SomeNullValue, got {other:?}"),
            }
        }
        fn conversion_at(e: EncodeError) -> (String, ConversionError) {
            match e {
                EncodeError::Conversion { at: Some(at), error } => (at.to_string(), error),
                other => panic!("expected Conversion with a path, got {other:?}"),
            }
        }

        // ── the good frame, and absent values written as null ─────────
        let frame = both(&good)?;
        let back = OrderDomain::try_from_slice_with_header(&frame, 0)?;
        assert!(back.neg.is_some_and(f64::is_nan), "NaN is a value where -1.0 is the null");
        assert_eq!(back.fills, good.fills);

        let mut absent = good.clone();
        absent.f32v = None;
        absent.f64v = None;
        absent.neg = None;
        absent.units = None;
        absent.fills[0].f64v = None;
        absent.fills[0].allocs[1].weight = None;
        absent.fills[0].allocs[1].units = None;
        let frame = both(&absent)?;
        let back = OrderDomain::try_from_slice_with_header(&frame, 0)?;
        assert_eq!((back.f32v, back.f64v, back.neg, back.units), (None, None, None, None));
        assert_eq!(back.fills[0].f64v, None);
        assert_eq!((back.fills[0].allocs[1].weight, back.fills[0].allocs[1].units), (None, None));

        // ── Some(null value): f32, f64, a finite nullValue, an integer ─
        let mut bad = good.clone();
        bad.f32v = Some(f32::NAN);
        assert_eq!(some_null_at(refused(&bad)), "f32v");
        let mut bad = good.clone();
        bad.f64v = Some(-f64::NAN);
        assert_eq!(some_null_at(refused(&bad)), "f64v");
        let mut bad = good.clone();
        bad.neg = Some(-1.0);
        assert_eq!(some_null_at(refused(&bad)), "neg");
        let mut bad = good.clone();
        bad.units = Some(u32::MAX);
        assert_eq!(some_null_at(refused(&bad)), "units");

        // In groups the path carries the entry index at every level.
        let mut bad = good.clone();
        bad.fills[1].f64v = Some(f64::NAN);
        let e = refused(&bad);
        if let EncodeError::SomeNullValue { at } = e {
            assert_eq!((at.field(), at.entries()), ("fills.f64v", &[1u32][..]));
        }
        assert_eq!(some_null_at(e), "fills[1].f64v");
        let mut bad = good.clone();
        bad.fills[0].allocs[2].weight = Some(f32::NAN);
        let e = refused(&bad);
        if let EncodeError::SomeNullValue { at } = e {
            assert_eq!((at.field(), at.entries()), ("fills.allocs.weight", &[0u32, 2][..]));
        }
        assert_eq!(some_null_at(e), "fills[0].allocs[2].weight");
        let mut bad = good.clone();
        bad.fills[0].allocs[1].units = Some(u32::MAX);
        assert_eq!(some_null_at(refused(&bad)), "fills[0].allocs[1].units");

        // ── conversions: the mapping's own error, kept whole ──────────
        let nan_ratio = Leg { ratio: Some(f64::NAN), ..leg };
        let nan_weight = Leg { weight: Some(f32::NAN), ..leg };
        let mut bad = good.clone();
        bad.leg = nan_ratio;
        assert_eq!(
            conversion_at(refused(&bad)),
            ("leg".to_string(), ConversionError::Mapping(MappingError::SomeNullValue { composite: "Leg", member: "ratio" })),
        );
        let mut bad = good.clone();
        bad.fills[1].leg = nan_weight;
        assert_eq!(
            conversion_at(refused(&bad)),
            ("fills[1].leg".to_string(), ConversionError::Mapping(MappingError::SomeNullValue { composite: "Leg", member: "weight" })),
        );
        let mut bad = good.clone();
        bad.fills[1].allocs[0].leg = nan_ratio;
        assert_eq!(
            conversion_at(refused(&bad)),
            ("fills[1].allocs[0].leg".to_string(), ConversionError::Mapping(MappingError::SomeNullValue { composite: "Leg", member: "ratio" })),
        );

        // A `&'static str` error is kept as the reason.
        let mut bad = good.clone();
        bad.px = Money { cents: -1 };
        assert_eq!(conversion_at(refused(&bad)), ("px".to_string(), ConversionError::Reason("negative money")));
        let mut bad = good.clone();
        bad.fills[0].allocs[2].px = Money { cents: -1 };
        assert_eq!(
            conversion_at(refused(&bad)),
            ("fills[0].allocs[2].px".to_string(), ConversionError::Reason("negative money")),
        );

        // Any other error type cannot live in a `Copy` error: it is named.
        let mut bad = good.clone();
        bad.fills[1].allocs[0].side = Dir::Unknown;
        let (at, error) = conversion_at(refused(&bad));
        assert_eq!(at, "fills[1].allocs[0].side");
        assert!(
            matches!(error, ConversionError::Custom { error_type } if error_type.ends_with("app::Problem")),
            "{error:?}"
        );

        // Display and source carry the same.
        let mut bad = good.clone();
        bad.fills[0].allocs[2].leg = nan_weight;
        let e = refused(&bad);
        let shown = e.to_string();
        assert!(shown.contains("fills[0].allocs[2].leg") && shown.contains("weight"), "{shown}");
        let source = std::error::Error::source(&e).ok_or("no source")?;
        let mapping = std::error::Error::source(source).ok_or("no mapping source")?;
        assert_eq!(mapping.to_string(), MappingError::SomeNullValue { composite: "Leg", member: "weight" }.to_string());
        let mut bad = good.clone();
        bad.fills[1].f64v = Some(f64::NAN);
        let shown = refused(&bad).to_string();
        assert!(shown.contains("fills[1].f64v") && shown.contains("None"), "{shown}");

        // Neither error grows `EncodeError`, so no encode result grows.
        #[cfg(target_pointer_width = "64")]
        assert_eq!(core::mem::size_of::<EncodeError>(), 80);
        "#,
    );
    Ok(())
}

/// A composite mapping refuses `Some(null value)` for a `float` member as it
/// does for a `double` one, with the same `MappingError::SomeNullValue`.
/// A mapped setter's error converts into `EncodeError`, so `?` works inside
/// a `send` closure, and the claim is aborted.
#[test]
fn mapped_setters_convert_into_encode_errors() -> Result<(), Box<dyn std::error::Error>> {
    let module = refusal_module()?;
    compile_and_run_modules(
        "encode_refusal_setters",
        &[("rf", &module)],
        r#"
        use rf::*;
        use rf::app::Leg;
        use rf::sbe_rt::{ConversionError, EncodeError, MappingError, SendError};

        for (leg, member) in [
            (Leg { qty: 1, ratio: Some(f64::NAN), weight: None }, "ratio"),
            (Leg { qty: 1, ratio: None, weight: Some(f32::NAN) }, "weight"),
        ] {
            assert_eq!(
                <Leg as TryToSbe<rf::Leg>>::try_to_sbe(&leg),
                Err(MappingError::SomeNullValue { composite: "Leg", member })
            );
        }

        let fixed = OrderFixedFields {
            id: 1,
            f32v: None,
            f64v: None,
            neg: None,
            units: None,
            leg: rf::Leg::new(1, 0.5, 0.25),
            px: Px::new(1, -2),
            side: Side::Buy,
        };
        let bad = Leg { qty: 1, ratio: None, weight: Some(f32::NAN) };
        // The closure fails before the frame's length is checked.
        let mut scratch = vec![0u8; 128];
        let mut offered = 0;
        let r = OrderEncoder::send(
            sbe_rt::OfferClaim::new(&mut scratch, |_: &[u8]| -> Result<(), ()> {
                offered += 1;
                Ok(())
            }),
            |enc| {
                let mut enc = enc.fixed(&fixed);
                enc.leg(bad)?;
                enc.fills(0, |_| Ok(()))
            },
        );
        let expected = EncodeError::Conversion {
            at: None,
            error: ConversionError::Mapping(MappingError::SomeNullValue { composite: "Leg", member: "weight" }),
        };
        assert_eq!(r, Err(SendError::Encode(expected)));
        assert_eq!(offered, 0, "a refused frame is never offered");
        assert_eq!(EncodeError::from(MappingError::NullEnum { enum_name: "Side" }), EncodeError::Conversion {
            at: None,
            error: ConversionError::Mapping(MappingError::NullEnum { enum_name: "Side" }),
        });
        "#,
    );
    Ok(())
}

/// Four groups deep: `books` > `levels` > `orders` > `fills`, with an
/// optional `px` in each fill.
const DEEP_XML: &str = r#"<messageSchema package="deep" id="22" version="0" byteOrder="littleEndian">
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
    <type name="OptD" primitiveType="double" presence="optional"/>
  </types>
  <message name="Deep" id="1">
    <group name="books" id="1" dimensionType="groupSizeEncoding">
      <field name="k" id="2" type="uint8"/>
      <group name="levels" id="3" dimensionType="groupSizeEncoding">
        <field name="k" id="4" type="uint8"/>
        <group name="orders" id="5" dimensionType="groupSizeEncoding">
          <field name="k" id="6" type="uint8"/>
          <group name="fills" id="7" dimensionType="groupSizeEncoding">
            <field name="px" id="8" type="OptD"/>
          </group>
        </group>
      </group>
    </group>
  </message>
</messageSchema>"#;

/// A path keeps the entry index of the outermost `FieldPath::MAX_ENTRIES`
/// (3) groups and shows a deeper one as `[?]`, the same through the DTO and
/// through `encode_from`.
#[test]
fn a_path_deeper_than_its_entries_shows_the_rest_as_unknown()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(DEEP_XML)?);
    let module = Generator::new(
        GenerationConfig::new("deep")
            .with_domain_objects(DomainVarData::Bytes)
            .with_source_traits(true),
    )
    .generate(&schema)?
    .modules()
    .next()
    .ok_or("no module")?
    .source
    .clone()
        + r#"
pub mod app {
    use super::*;
    impl DeepSource for DeepDomain {
        fn books(&self) -> impl ExactSizeIterator<Item = impl BooksEntrySource> { self.books.iter() }
    }
    impl BooksEntrySource for DeepBooksEntryDomain {
        fn k(&self) -> u8 { self.k }
        fn levels(&self) -> impl ExactSizeIterator<Item = impl BooksLevelsEntrySource> { self.levels.iter() }
    }
    impl BooksLevelsEntrySource for DeepBooksEntryLevelsEntryDomain {
        fn k(&self) -> u8 { self.k }
        fn orders(&self) -> impl ExactSizeIterator<Item = impl BooksLevelsOrdersEntrySource> { self.orders.iter() }
    }
    impl BooksLevelsOrdersEntrySource for DeepBooksEntryLevelsEntryOrdersEntryDomain {
        fn k(&self) -> u8 { self.k }
        fn fills(&self) -> impl ExactSizeIterator<Item = impl BooksLevelsOrdersFillsEntrySource> { self.fills.iter() }
    }
    impl BooksLevelsOrdersFillsEntrySource for DeepBooksEntryLevelsEntryOrdersEntryFillsEntryDomain {
        fn px(&self) -> Option<f64> { self.px }
    }
}
"#;
    compile_and_run_modules(
        "encode_refusal_deep_path",
        &[("deep", &module)],
        r#"
        use deep::*;
        use deep::sbe_rt::EncodeError;

        // Two books, three levels, four orders, five fills; with `bad`, the
        // last fill of the last order of the last level of the last book
        // holds NaN.
        let fills = |bad: bool| {
            (0..5)
                .map(|i| DeepBooksEntryLevelsEntryOrdersEntryFillsEntryDomain {
                    px: Some(if bad && i == 4 { f64::NAN } else { 1.5 }),
                })
                .collect::<Vec<_>>()
        };
        let orders = |bad: bool| {
            (0..4)
                .map(|i| DeepBooksEntryLevelsEntryOrdersEntryDomain { k: 3, fills: fills(bad && i == 3) })
                .collect::<Vec<_>>()
        };
        let levels = |bad: bool| {
            (0..3)
                .map(|i| DeepBooksEntryLevelsEntryDomain { k: 2, orders: orders(bad && i == 2) })
                .collect::<Vec<_>>()
        };
        let deep = |bad: bool| DeepDomain {
            books: (0..2).map(|i| DeepBooksEntryDomain { k: 1, levels: levels(bad && i == 1) }).collect(),
        };

        let mut buf = vec![0u8; 8192];
        let n = deep(false).encode(&mut buf)?;
        assert_eq!(DeepDomain::try_from_slice_with_header(&buf[..n], 0)?, deep(false));

        let bad = deep(true);
        let via_dto = bad.encode(&mut buf);
        let via_source = DeepEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .and_then(|enc| enc.encode_from(&bad))
            .map(|done| done.encoded_length_with_header());
        assert_eq!(via_dto, via_source, "the DTO and encode_from disagree");
        match via_dto {
            Err(EncodeError::SomeNullValue { at }) => {
                assert_eq!(at.field(), "books.levels.orders.fills.px");
                assert_eq!(at.entries(), &[1u32, 2, 3][..]);
                assert_eq!(at.to_string(), "books[1].levels[2].orders[3].fills[?].px");
            }
            other => panic!("expected SomeNullValue, got {other:?}"),
        }
        "#,
    );
    Ok(())
}
