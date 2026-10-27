//! `with_source_traits`: a generated `{Message}Source` trait (and
//! `…EntrySource` per repeating group) that an application type implements
//! with getters, so the encoder can write it directly — fixed fields, nested
//! groups and var-data through the encoder's own type-state stages, with no
//! DTO and no allocation.
//!
//! The Car tests run in process against the checked-in golden codec, which
//! is generated with source traits: `CarDomain` implements `CarSource`, and
//! `encode_from` must write exactly the bytes the DTO's own `encode` writes.
//! The domain-mapped tests compile a scratch crate, because the mapped
//! application types have to live beside the generated module.

#![allow(unsafe_code)]
#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{
    Paths, compile_and_run_modules, compile_fails_with_diagnostics,
    compile_fails_with_diagnostics_excluding,
};
use ergo_sbe::{
    CompositeMapping, EnumMapping, GenerateError, GenerationConfig, Generator, Schema,
    SchemaSource, parse,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

// Keep the generated snapshot in generator formatting; stability_test compares
// its canonical syntax rather than rustfmt layout.
#[rustfmt::skip]
#[path = "golden/car_example.rs"]
#[allow(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    unused,
    non_camel_case_types,
    non_snake_case,
    missing_docs,
    unsafe_code
)]
mod car_example;
use car_example::*;

// ── allocation counting, for the allocation-free claim ─────────────────

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;

// SAFETY: forwards every call to the system allocator unchanged.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.with(Cell::get) {
            ALLOCATIONS.with(|n| n.set(n.get() + 1));
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn allocations_during<R>(f: impl FnOnce() -> R) -> (R, u64) {
    ALLOCATIONS.with(|n| n.set(0));
    COUNTING.with(|c| c.set(true));
    let out = f();
    COUNTING.with(|c| c.set(false));
    (out, ALLOCATIONS.with(Cell::get))
}

// ── CarDomain as a source: getters over its own fields ─────────────────

impl CarSource for CarDomain {
    fn serial_number(&self) -> u64 {
        self.serial_number
    }
    fn model_year(&self) -> u16 {
        self.model_year
    }
    fn available(&self) -> BooleanType {
        BooleanType::from(self.available)
    }
    fn code(&self) -> Model {
        self.code
    }
    fn some_numbers(&self) -> [u32; 4] {
        self.some_numbers
    }
    fn vehicle_code(&self) -> [u8; 6] {
        self.vehicle_code
    }
    fn extras(&self) -> OptionalExtras {
        self.extras
    }
    fn engine(&self) -> Engine {
        self.engine
    }
    fn fuel_figures(&self) -> impl ExactSizeIterator<Item = impl FuelFiguresEntrySource> {
        self.fuel_figures.iter()
    }
    fn performance_figures(
        &self,
    ) -> impl ExactSizeIterator<Item = impl PerformanceFiguresEntrySource> {
        self.performance_figures.iter()
    }
    fn manufacturer(&self) -> &[u8] {
        &self.manufacturer
    }
    fn model(&self) -> &[u8] {
        &self.model
    }
    fn activation_code(&self) -> &[u8] {
        &self.activation_code
    }
}

impl FuelFiguresEntrySource for CarFuelFiguresEntryDomain {
    fn speed(&self) -> u16 {
        self.speed
    }
    fn mpg(&self) -> f32 {
        self.mpg
    }
    fn usage_description(&self) -> &[u8] {
        &self.usage_description
    }
}

impl PerformanceFiguresEntrySource for CarPerformanceFiguresEntryDomain {
    fn octane_rating(&self) -> u8 {
        self.octane_rating
    }
    fn acceleration(
        &self,
    ) -> impl ExactSizeIterator<Item = impl PerformanceFiguresAccelerationEntrySource> {
        self.acceleration.iter()
    }
}

impl PerformanceFiguresAccelerationEntrySource
    for CarPerformanceFiguresEntryAccelerationEntryDomain
{
    fn mph(&self) -> u16 {
        self.mph
    }
    fn seconds(&self) -> f32 {
        self.seconds
    }
}

fn car() -> Result<CarDomain, Box<dyn std::error::Error>> {
    let frame = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/car_example_baseline_data.sbe"
    ))?;
    Ok(CarDomain::try_from_slice_with_header(&frame, 0)?)
}

/// An iterator whose `len()` is whatever the test says, to prove a source
/// that miscounts its entries is an error, not a malformed group.
struct Miscounted<I> {
    inner: I,
    claimed: usize,
}

impl<I: Iterator> Iterator for Miscounted<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.claimed, Some(self.claimed))
    }
}

impl<I: Iterator> ExactSizeIterator for Miscounted<I> {}

/// `CarDomain` whose fuel figures report `claimed` entries.
struct Miscounting {
    car: CarDomain,
    claimed: usize,
}

impl CarSource for Miscounting {
    fn serial_number(&self) -> u64 {
        self.car.serial_number()
    }
    fn model_year(&self) -> u16 {
        self.car.model_year()
    }
    fn available(&self) -> BooleanType {
        CarSource::available(&self.car)
    }
    fn code(&self) -> Model {
        self.car.code()
    }
    fn some_numbers(&self) -> [u32; 4] {
        self.car.some_numbers()
    }
    fn vehicle_code(&self) -> [u8; 6] {
        self.car.vehicle_code()
    }
    fn extras(&self) -> OptionalExtras {
        self.car.extras()
    }
    fn engine(&self) -> Engine {
        self.car.engine()
    }
    fn fuel_figures(&self) -> impl ExactSizeIterator<Item = impl FuelFiguresEntrySource> {
        Miscounted {
            inner: self.car.fuel_figures.iter(),
            claimed: self.claimed,
        }
    }
    fn performance_figures(
        &self,
    ) -> impl ExactSizeIterator<Item = impl PerformanceFiguresEntrySource> {
        self.car.performance_figures.iter()
    }
    fn manufacturer(&self) -> &[u8] {
        CarSource::manufacturer(&self.car)
    }
    fn model(&self) -> &[u8] {
        CarSource::model(&self.car)
    }
    fn activation_code(&self) -> &[u8] {
        CarSource::activation_code(&self.car)
    }
}

/// `encode_from` writes the bytes the DTO's own `encode` writes — into a
/// dirty buffer, so nothing can be left over — and sizes them exactly.
#[test]
fn encode_from_matches_the_dto_byte_for_byte() -> Result<(), Box<dyn std::error::Error>> {
    let car = car()?;
    let mut expected = [0u8; 512];
    let n = car.encode(&mut expected)?;

    let len = CarEncoder::encoded_length_with_header_from(&car)?;
    assert_eq!(len, n);

    let mut buf = [0xAAu8; 512];
    let written = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .encode_from(&car)?
        .encoded_length_with_header();
    assert_eq!(written, n);
    assert_eq!(&buf[..n], &expected[..n]);
    assert_eq!(CarDomain::try_from_slice_with_header(&buf[..n], 0)?, car);
    Ok(())
}

/// Encoding from a source allocates nothing: groups are iterators and
/// var-data is borrowed.
#[test]
fn encode_from_does_not_allocate() -> Result<(), Box<dyn std::error::Error>> {
    let car = car()?;
    let mut buf = [0u8; 512];
    let (written, allocations) = allocations_during(|| -> Result<usize, sbe_rt::EncodeError> {
        let len = CarEncoder::encoded_length_with_header_from(&car)?;
        let enc = CarEncoder::try_wrap_and_apply_header(&mut buf[..len], 0)?;
        Ok(enc.encode_from(&car)?.encoded_length_with_header())
    });
    assert!(written? > 0);
    assert_eq!(allocations, 0, "encode_from allocated {allocations} times");
    Ok(())
}

/// `send` allocates nothing either: the claim guard, the length and header
/// checks and the commit, and an aborted claim on a length mismatch.
#[test]
fn send_does_not_allocate() -> Result<(), Box<dyn std::error::Error>> {
    let car = car()?;
    let len = CarEncoder::encoded_length_with_header_from(&car)?;
    let mut scratch = [0u8; 512];
    let mut published = 0usize;
    let ((committed, aborted), allocations) = allocations_during(|| {
        let committed = CarEncoder::send(
            sbe_rt::OfferClaim::new(&mut scratch[..len], |frame: &[u8]| -> Result<(), ()> {
                published += frame.len();
                Ok(())
            }),
            |enc| enc.encode_from(&car),
        );
        let aborted = CarEncoder::send(
            sbe_rt::OfferClaim::new(&mut scratch[..=len], |_: &[u8]| -> Result<(), ()> {
                Ok(())
            }),
            |enc| enc.encode_from(&car),
        );
        (committed, aborted)
    });
    assert_eq!(allocations, 0, "send allocated {allocations} times");
    assert_eq!(committed, Ok(()));
    assert_eq!(
        aborted,
        Err(sbe_rt::SendError::LengthMismatch {
            claimed: len + 1,
            frame: len
        })
    );
    assert_eq!(published, len);
    Ok(())
}

/// `encode_from` returns the terminal stage, so it is what a `send`
/// closure returns.
#[test]
fn encode_from_composes_with_send() -> Result<(), Box<dyn std::error::Error>> {
    let car = car()?;
    let mut expected = [0u8; 512];
    let n = car.encode(&mut expected)?;

    let len = CarEncoder::encoded_length_with_header_from(&car)?;
    let mut scratch = vec![0u8; len];
    let mut offered = Vec::new();
    let claim = sbe_rt::OfferClaim::new(&mut scratch, |frame: &[u8]| -> Result<(), ()> {
        offered = frame.to_vec();
        Ok(())
    });
    CarEncoder::send(claim, |enc| enc.encode_from(&car)).map_err(|e| format!("{e:?}"))?;
    assert_eq!(offered, expected[..n].to_vec());
    Ok(())
}

/// A source whose group iterator miscounts its entries is a typed error —
/// too few written, too many written, or more than `numInGroup` can hold —
/// never a group header that disagrees with its entries.
#[test]
fn a_miscounted_group_is_an_error() -> Result<(), Box<dyn std::error::Error>> {
    let car = car()?;
    let actual = car.fuel_figures.len();
    assert!(actual >= 2, "the fixture has several fuel figures");
    let mut buf = [0u8; 1024];

    let short = Miscounting {
        car: car.clone(),
        claimed: actual + 1,
    };
    let r = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?.encode_from(&short);
    assert!(
        matches!(
            r,
            Err(sbe_rt::EncodeError::GroupCountMismatch { declared, actual: wrote })
                if declared as usize == actual + 1 && wrote as usize == actual
        ),
        "{:?}",
        r.err()
    );

    let long = Miscounting {
        car: car.clone(),
        claimed: actual - 1,
    };
    let r = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?.encode_from(&long);
    assert!(
        matches!(r, Err(sbe_rt::EncodeError::GroupFull { declared, .. }) if declared as usize == actual - 1),
        "{:?}",
        r.err()
    );

    let huge = Miscounting {
        car,
        claimed: usize::from(u16::MAX) + 1,
    };
    let r = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?.encode_from(&huge);
    assert!(
        matches!(
            r,
            Err(sbe_rt::EncodeError::ValueOutOfRange {
                field: "fuelFigures",
                ..
            })
        ),
        "{:?}",
        r.err()
    );
    Ok(())
}

/// A value outside the field's schema range is refused, as the DTO's own
/// encode refuses it, in a group entry as at message level.
#[test]
fn a_value_outside_the_schema_range_is_an_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut car = car()?;
    let mut buf = [0u8; 1024];
    car.performance_figures[0].octane_rating = 200; // schema range 90..=110
    let r = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?.encode_from(&car);
    assert!(
        matches!(
            r,
            Err(sbe_rt::EncodeError::ValueOutOfRange {
                field: "octaneRating",
                min: 90,
                max: 110,
                actual: 200
            })
        ),
        "{:?}",
        r.err()
    );
    assert!(matches!(
        car.encode(&mut buf),
        Err(sbe_rt::EncodeError::ValueOutOfRange {
            field: "octaneRating",
            ..
        })
    ));
    Ok(())
}

/// Fields with no declared bound, one declared side, or both, at message
/// level and in a flat group entry (so `bulk_add_domain` is generated too).
const RANGE_XML: &str = r#"<messageSchema package="rng" id="76" version="0" byteOrder="littleEndian">
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
    <type name="Cap" primitiveType="int32" maxValue="100"/>
    <type name="Floor" primitiveType="uint32" minValue="10"/>
    <type name="Pct" primitiveType="uint8" minValue="1" maxValue="99"/>
    <type name="OptCount" primitiveType="uint32" presence="optional"/>
  </types>
  <message name="Row" id="1">
    <field name="source" id="1" type="uint64"/>
    <field name="session" id="2" type="int32"/>
    <field name="hdr" id="3" type="uint16"/>
    <field name="ch" id="4" type="char"/>
    <field name="cap" id="5" type="Cap"/>
    <field name="floor" id="6" type="Floor"/>
    <field name="pct" id="7" type="Pct"/>
    <field name="count" id="8" type="OptCount"/>
    <group name="legs" id="9" dimensionType="groupSizeEncoding">
      <field name="qty" id="10" type="uint64"/>
      <field name="legPct" id="11" type="Pct"/>
    </group>
  </message>
</messageSchema>"#;

/// `RowDomain` as a `RowSource`: getters over its own fields.
const RANGE_SOURCE_IMPLS: &str = r#"
        impl rng::RowSource for rng::RowDomain {
            fn source(&self) -> u64 { self.source }
            fn session(&self) -> i32 { self.session }
            fn hdr(&self) -> u16 { self.hdr }
            fn ch(&self) -> u8 { self.ch }
            fn cap(&self) -> i32 { self.cap }
            fn floor(&self) -> u32 { self.floor }
            fn pct(&self) -> u8 { self.pct }
            fn count(&self) -> Option<u32> { self.count }
            fn legs(&self) -> impl ExactSizeIterator<Item = impl rng::LegsEntrySource> {
                self.legs.iter()
            }
        }
        impl rng::LegsEntrySource for rng::RowLegsEntryDomain {
            fn qty(&self) -> u64 { self.qty }
            fn leg_pct(&self) -> u8 { self.leg_pct }
        }
"#;

/// `encode_from`, the DTO's `encode` and `bulk_add_domain` check only the
/// bounds the schema declares. A value an undeclared side would have
/// refused under the SBE default range (`u64::MAX`, `i32::MIN`, `0xFFFF`,
/// a control `char`) is written as given, byte-for-byte what `fixed()`
/// writes, and reads back unchanged.
#[test]
fn undeclared_bounds_are_not_checked_on_encode() -> Result<(), Box<dyn std::error::Error>> {
    let source = generate_xml(
        RANGE_XML,
        GenerationConfig::new("rng")
            .with_domain_objects(ergo_sbe::DomainVarData::Bytes)
            .with_source_traits(true),
    )?;
    compile_and_run_modules(
        "source_trait_undeclared_bounds",
        &[("rng", &source)],
        &format!(
            "{RANGE_SOURCE_IMPLS}{}",
            r#"
        use rng::*;
        let legs = vec![
            RowLegsEntryDomain { qty: u64::MAX, leg_pct: 1 },
            RowLegsEntryDomain { qty: 0, leg_pct: 99 },
        ];
        let row = RowDomain {
            source: u64::MAX,
            session: i32::MIN,
            hdr: 0xFFFF,
            ch: 0x01,
            cap: i32::MIN,
            floor: u32::MAX,
            pct: 50,
            count: Some(u32::MAX - 1),
            legs: legs.clone(),
        };

        // Reference: the plain setters, which check nothing.
        const LEN: usize = RowEncoder::compute_length_with_header(2);
        let mut expected = [0u8; LEN];
        let n = RowEncoder::try_wrap_and_apply_header(&mut expected, 0)?
            .fixed(&RowFixedFields {
                source: u64::MAX,
                session: i32::MIN,
                hdr: 0xFFFF,
                ch: 0x01,
                cap: i32::MIN,
                floor: u32::MAX,
                pct: 50,
                count: Some(u32::MAX - 1),
            })
            .legs(2, |g| {
                g.add(|e| { e.qty(u64::MAX).leg_pct(1); Ok(()) })?;
                g.add(|e| { e.qty(0).leg_pct(99); Ok(()) })
            })?
            .encoded_length_with_header();
        assert_eq!(n, LEN);

        let mut dto_buf = [0xAAu8; LEN];
        assert_eq!(row.encode(&mut dto_buf)?, n);
        assert_eq!(dto_buf, expected, "DTO encode");

        let mut from_buf = [0xAAu8; LEN];
        let written = RowEncoder::try_wrap_and_apply_header(&mut from_buf, 0)?
            .encode_from(&row)?
            .encoded_length_with_header();
        assert_eq!(written, n);
        assert_eq!(from_buf, expected, "encode_from");

        let mut bulk_buf = [0xAAu8; LEN];
        RowEncoder::try_wrap_and_apply_header(&mut bulk_buf, 0)?
            .fixed(&RowFixedFields {
                source: u64::MAX,
                session: i32::MIN,
                hdr: 0xFFFF,
                ch: 0x01,
                cap: i32::MIN,
                floor: u32::MAX,
                pct: 50,
                count: Some(u32::MAX - 1),
            })
            .legs(2, |g| g.bulk_add_domain(&legs))?;
        assert_eq!(bulk_buf, expected, "bulk_add_domain");

        let back = RowDomain::try_from_slice_with_header(&dto_buf, 0)?;
        assert_eq!(back, row, "round trip");
        let dec = RowDecoder::decode(&dto_buf, 0)?;
        assert_eq!(
            (dec.source(), dec.session(), dec.hdr(), dec.ch(), dec.cap(), dec.floor()),
            (u64::MAX, i32::MIN, 0xFFFF, 0x01, i32::MIN, u32::MAX),
        );
        "#
        ),
    );
    Ok(())
}

/// A declared `minValue` or `maxValue` is still enforced on every encode
/// path, each side alone and both together, with the same
/// `ValueOutOfRange` it always returned. `Some` of an optional field's null
/// value is still `SomeNullValue`: it would read back as `None`.
#[test]
fn declared_bounds_are_still_checked_on_encode() -> Result<(), Box<dyn std::error::Error>> {
    let source = generate_xml(
        RANGE_XML,
        GenerationConfig::new("rng")
            .with_domain_objects(ergo_sbe::DomainVarData::Bytes)
            .with_source_traits(true),
    )?;
    compile_and_run_modules(
        "source_trait_declared_bounds",
        &[("rng", &source)],
        &format!(
            "{RANGE_SOURCE_IMPLS}{}",
            r#"
        use rng::*;
        use rng::sbe_rt::EncodeError;
        let ok = RowDomain {
            source: 0,
            session: 0,
            hdr: 0,
            ch: b'A',
            cap: 100,
            floor: 10,
            pct: 1,
            count: None,
            legs: vec![RowLegsEntryDomain { qty: 1, leg_pct: 99 }],
        };
        const LEN: usize = RowEncoder::compute_length_with_header(1);
        let mut buf = [0u8; LEN];
        ok.encode(&mut buf)?;

        // Both encode paths must refuse `row` with exactly `expect`.
        let refused = |row: &RowDomain, expect: EncodeError| {
            let mut b = [0u8; LEN];
            assert_eq!(row.encode(&mut b), Err(expect.clone()), "DTO encode");
            let r = RowEncoder::try_wrap_and_apply_header(&mut b, 0)
                .map_err(|e| format!("{e:?}"))
                .and_then(|enc| enc.encode_from(row).map(|_| ()).map_err(|e| format!("{e:?}")));
            assert_eq!(r, Err(format!("{expect:?}")), "encode_from");
        };

        // maxValue alone: the undeclared min reports int32's SBE default.
        refused(
            &RowDomain { cap: 101, ..ok.clone() },
            EncodeError::ValueOutOfRange { field: "cap", min: -2_147_483_647, max: 100, actual: 101 },
        );
        // minValue alone: the undeclared max reports uint32's SBE default.
        refused(
            &RowDomain { floor: 9, ..ok.clone() },
            EncodeError::ValueOutOfRange { field: "floor", min: 10, max: 4_294_967_294, actual: 9 },
        );
        // Both declared: each side.
        for actual in [0u8, 100] {
            refused(
                &RowDomain { pct: actual, ..ok.clone() },
                EncodeError::ValueOutOfRange { field: "pct", min: 1, max: 99, actual: i128::from(actual) },
            );
        }
        // In a group entry, through the DTO and through bulk_add_domain.
        let bad_legs = vec![RowLegsEntryDomain { qty: u64::MAX, leg_pct: 0 }];
        let expect = EncodeError::ValueOutOfRange { field: "legPct", min: 1, max: 99, actual: 0 };
        refused(&RowDomain { legs: bad_legs.clone(), ..ok.clone() }, expect.clone());
        let mut b = [0u8; LEN];
        let r = RowEncoder::try_wrap_and_apply_header(&mut b, 0)?
            .fixed(&RowFixedFields {
                source: 0, session: 0, hdr: 0, ch: b'A', cap: 0, floor: 10, pct: 1, count: None,
            })
            .legs(1, |g| g.bulk_add_domain(&bad_legs));
        assert_eq!(r.map(|_| ()).err(), Some(expect));

        // Some(null) on an optional field is not a range question.
        let mut b = [0u8; LEN];
        match (RowDomain { count: Some(u32::MAX), ..ok.clone() }).encode(&mut b) {
            Err(EncodeError::SomeNullValue { at }) => assert_eq!(at.to_string(), "count"),
            other => panic!("Some(null) must be refused, got {other:?}"),
        }
        "#
        ),
    );
    Ok(())
}

/// The other generated range-check sites: an optional field with declared
/// bounds, a `char` with declared bounds, a declared bound equal to the
/// Rust type's own limit (checks nothing), a versioned (`sinceVersion`)
/// field, and a nested group entry.
const RANGE_EDGE_XML: &str = r#"<messageSchema package="rng" id="77" version="1" byteOrder="littleEndian">
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
    <type name="Pct" primitiveType="uint8" minValue="1" maxValue="99"/>
    <type name="OptPct" primitiveType="uint8" presence="optional" minValue="1" maxValue="99"/>
    <type name="Full" primitiveType="uint8" minValue="1" maxValue="255"/>
    <type name="Wide" primitiveType="int8" minValue="-128"/>
    <type name="Letter" primitiveType="char" minValue="A" maxValue="Z"/>
  </types>
  <message name="Edge" id="1">
    <field name="opt" id="1" type="OptPct"/>
    <field name="full" id="2" type="Full"/>
    <field name="wide" id="3" type="Wide"/>
    <field name="letter" id="4" type="Letter"/>
    <field name="late" id="5" type="Pct" sinceVersion="1"/>
    <field name="lateFree" id="6" type="uint64" sinceVersion="1"/>
    <group name="outer" id="7" dimensionType="groupSizeEncoding">
      <field name="tag" id="8" type="uint32"/>
      <group name="inner" id="9" dimensionType="groupSizeEncoding">
        <field name="qty" id="10" type="uint64"/>
        <field name="pct" id="11" type="Pct"/>
      </group>
    </group>
  </message>
</messageSchema>"#;

/// `EdgeDomain` as an `EdgeSource`: getters over its own fields.
const RANGE_EDGE_SOURCE_IMPLS: &str = r#"
        impl rng::EdgeSource for rng::EdgeDomain {
            fn opt(&self) -> Option<u8> { self.opt }
            fn full(&self) -> u8 { self.full }
            fn wide(&self) -> i8 { self.wide }
            fn letter(&self) -> u8 { self.letter }
            fn late(&self) -> u8 { self.late.unwrap_or(1) }
            fn late_free(&self) -> u64 { self.late_free.unwrap_or(0) }
            fn outer(&self) -> impl ExactSizeIterator<Item = impl rng::OuterEntrySource> {
                self.outer.iter()
            }
        }
        impl rng::OuterEntrySource for rng::EdgeOuterEntryDomain {
            fn tag(&self) -> u32 { self.tag }
            fn inner(&self) -> impl ExactSizeIterator<Item = impl rng::OuterInnerEntrySource> {
                self.inner.iter()
            }
        }
        impl rng::OuterInnerEntrySource for rng::EdgeOuterEntryInnerEntryDomain {
            fn qty(&self) -> u64 { self.qty }
            fn pct(&self) -> u8 { self.pct }
        }
"#;

#[test]
fn declared_only_bounds_hold_at_every_encode_site() -> Result<(), Box<dyn std::error::Error>> {
    let source = generate_xml(
        RANGE_EDGE_XML,
        GenerationConfig::new("rng")
            .with_domain_objects(ergo_sbe::DomainVarData::Bytes)
            .with_source_traits(true),
    )?;
    compile_and_run_modules(
        "source_trait_range_edges",
        &[("rng", &source)],
        &format!(
            "{RANGE_EDGE_SOURCE_IMPLS}{}",
            r#"
        use rng::*;
        use rng::sbe_rt::EncodeError;
        let entry = |qty: u64, pct: u8| EdgeOuterEntryDomain {
            tag: u32::MAX,
            inner: vec![EdgeOuterEntryInnerEntryDomain { qty, pct }],
        };
        // Every undeclared side and every type-limit bound at its extreme.
        let ok = EdgeDomain {
            opt: Some(99),
            full: 255,
            wide: i8::MIN,
            letter: b'A',
            late: Some(1),
            late_free: Some(u64::MAX),
            outer: vec![entry(u64::MAX, 99)],
        };
        let mut buf = [0xAAu8; 256];
        let n = ok.encode(&mut buf)?;
        let mut from_buf = [0x55u8; 256];
        let m = EdgeEncoder::try_wrap_and_apply_header(&mut from_buf, 0)?
            .encode_from(&ok)?
            .encoded_length_with_header();
        assert_eq!((n, &buf[..n]), (m, &from_buf[..m]), "DTO encode vs encode_from");
        assert_eq!(EdgeDomain::try_from_slice_with_header(&buf[..n], 0)?, ok, "round trip");
        let wide_max = EdgeDomain { wide: i8::MAX, letter: b'Z', ..ok.clone() };
        wide_max.encode(&mut buf)?;

        // Both encode paths must refuse `row` with exactly `expect`.
        let refused = |row: &EdgeDomain, expect: EncodeError| {
            let mut b = [0u8; 256];
            assert_eq!(row.encode(&mut b), Err(expect.clone()), "DTO encode");
            let r = EdgeEncoder::try_wrap_and_apply_header(&mut b, 0)
                .map_err(|e| format!("{e:?}"))
                .and_then(|enc| enc.encode_from(row).map(|_| ()).map_err(|e| format!("{e:?}")));
            assert_eq!(r, Err(format!("{expect:?}")), "encode_from");
        };
        let out = |field: &'static str, min: i128, max: i128, actual: i128| {
            EncodeError::ValueOutOfRange { field, min, max, actual }
        };
        refused(&EdgeDomain { opt: Some(0), ..ok.clone() }, out("opt", 1, 99, 0));
        refused(&EdgeDomain { opt: Some(100), ..ok.clone() }, out("opt", 1, 99, 100));
        refused(&EdgeDomain { full: 0, ..ok.clone() }, out("full", 1, 255, 0));
        refused(&EdgeDomain { letter: b'@', ..ok.clone() }, out("letter", 65, 90, 64));
        refused(&EdgeDomain { letter: b'[', ..ok.clone() }, out("letter", 65, 90, 91));
        refused(&EdgeDomain { late: Some(0), ..ok.clone() }, out("late", 1, 99, 0));
        refused(&EdgeDomain { late: Some(100), ..ok.clone() }, out("late", 1, 99, 100));
        refused(&EdgeDomain { outer: vec![entry(u64::MAX, 0)], ..ok.clone() }, out("pct", 1, 99, 0));
        refused(&EdgeDomain { outer: vec![entry(0, 100)], ..ok.clone() }, out("pct", 1, 99, 100));
        "#
        ),
    );
    Ok(())
}

fn mapping_schema() -> Result<Schema, Box<dyn std::error::Error>> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/domain-mapping-schema.xml"
    );
    Ok(Schema::from_ir(parse(SchemaSource::file(path))?))
}

fn source_of(config: GenerationConfig) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Generator::new(config)
        .generate(&mapping_schema()?)?
        .modules()
        .next()
        .ok_or("no module")?
        .source
        .clone())
}

/// With enum and composite mappings the getters return the application
/// types, and the frame is the one the plain codec writes; an absent
/// optional field writes its null value over a dirty buffer, and var-data
/// over its schema maximum is refused by both the sizing and the encode.
#[test]
fn mapped_source_writes_the_plain_frame() -> Result<(), Box<dyn std::error::Error>> {
    let mapped = source_of(
        GenerationConfig::new("dm")
            .with_source_traits(true)
            .with_enum_mapping(
                EnumMapping::new("Side", "app::Side")
                    .rename("Buy", "Bid")
                    .rename("Sell", "Ask"),
            )
            .with_composite_mapping(CompositeMapping::new("Price", "app::Px"))
            .with_composite_mapping(
                CompositeMapping::new("Leg", "app::Leg").rename("qty", "quantity"),
            ),
    )? + r#"
pub mod app {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Side { Bid, Ask, SellShort }
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Px { pub mantissa: i64, pub exponent: i8 }
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Leg {
        pub price: Px, pub side: Side, pub quantity: u32, pub min_qty: Option<u32>,
        pub ratio: Option<f64>, pub flags: super::Flags, pub venue: [u8; 4],
    }

    pub struct Fill { pub px: i64, pub qty: u32, pub side: Side, pub accounts: Vec<u32>, pub venue: String }
    pub struct Order { pub id: u64, pub side: Side, pub leg: Leg, pub limit: Option<i64>, pub fills: Vec<Fill>, pub note: String }

    impl super::OrderSource for Order {
        fn order_id(&self) -> u64 { self.id }
        fn side(&self) -> Side { self.side }
        fn leg(&self) -> Leg { self.leg }
        fn limit(&self) -> Option<i64> { self.limit }
        fn fills(&self) -> impl ExactSizeIterator<Item = impl super::OrderFillsEntrySource> { self.fills.iter() }
        fn note(&self) -> &[u8] { self.note.as_bytes() }
    }
    impl super::OrderFillsEntrySource for Fill {
        fn px(&self) -> i64 { self.px }
        fn qty(&self) -> u32 { self.qty }
        fn side(&self) -> Side { self.side }
        fn allocs(&self) -> impl ExactSizeIterator<Item = impl super::OrderFillsAllocsEntrySource> {
            self.accounts.iter().copied().map(Account)
        }
        fn venue_ref(&self) -> &[u8] { self.venue.as_bytes() }
    }
    pub struct Account(pub u32);
    impl super::OrderFillsAllocsEntrySource for Account {
        fn account(&self) -> u32 { self.0 }
    }
    impl super::HeartbeatSource for Order {
        fn seq(&self) -> u64 { self.id }
        fn side(&self) -> Side { self.side }
    }
}
"#;
    let plain = source_of(GenerationConfig::new("plain"))?;
    compile_and_run_modules(
        "source_trait_mapped",
        &[("dm", &mapped), ("plain", &plain)],
        r#"
        use dm::app;
        let leg = app::Leg {
            price: app::Px { mantissa: 12_345, exponent: -2 },
            side: app::Side::Ask,
            quantity: 10,
            min_qty: None,
            ratio: Some(0.5),
            flags: dm::Flags(1),
            venue: *b"XLON",
        };
        let mut order = app::Order {
            id: 7,
            side: app::Side::SellShort,
            leg,
            limit: None,
            fills: vec![
                app::Fill { px: 101, qty: 2, side: app::Side::Bid, accounts: vec![3, 4], venue: "V1".into() },
                app::Fill { px: 102, qty: 5, side: app::Side::Ask, accounts: vec![], venue: String::new() },
            ],
            note: "hello".into(),
        };

        let mut expected = [0u8; 256];
        let n = plain::OrderEncoder::try_wrap_and_apply_header(&mut expected, 0)?
            .fixed(&plain::OrderFixedFields {
                order_id: 7,
                side: plain::Side::SellShort,
                leg: plain::Leg::new(
                    plain::Price::new(12_345, -2), plain::Side::Sell, 10, u32::MAX, 0.5,
                    plain::Flags(1), *b"XLON",
                ),
                limit: None,
            })
            .fills(2, |g| {
                g.add(|e| {
                    let mut e = e;
                    e.px(101).qty(2).side(plain::Side::Buy);
                    e.allocs(2, |a| {
                        a.add(|x| { x.account(3); Ok(()) })?;
                        a.add(|x| { x.account(4); Ok(()) })
                    })?
                    .venue_ref(b"V1")
                })?;
                g.add(|e| {
                    let mut e = e;
                    e.px(102).qty(5).side(plain::Side::Sell);
                    e.allocs(0, |_| Ok(()))?.venue_ref(b"")
                })
            })?
            .note(b"hello")?
            .encoded_length_with_header();

        assert_eq!(dm::OrderEncoder::encoded_length_with_header_from(&order)?, n);
        let mut buf = [0xAAu8; 256];
        let written = dm::OrderEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .encode_from(&order)?
            .encoded_length_with_header();
        assert_eq!(&buf[..written], &expected[..n], "mapped source, plain bytes");
        assert_eq!(dm::OrderDecoder::decode(&buf, 0)?.limit(), None, "null image over dirty bytes");

        // Fixed-only message: encode_from ends at the fixed stage.
        let mut hb = [0u8; dm::HeartbeatEncoder::ENCODED_LENGTH];
        let stage = dm::HeartbeatEncoder::try_wrap_and_apply_header(&mut hb, 0)?
            .encode_from(&order)?;
        assert_eq!(stage.encoded_length_with_header(), dm::HeartbeatEncoder::ENCODED_LENGTH);
        assert_eq!(dm::HeartbeatEncoder::encoded_length_with_header_from(&order)?, hb.len());
        let dec = dm::HeartbeatDecoder::decode(&hb, 0)?;
        assert_eq!((dec.seq(), dec.side()), (7, Ok(app::Side::SellShort)));

        // Var-data over its schema maximum (64): refused by both.
        order.note = "x".repeat(65);
        let expect = dm::sbe_rt::EncodeError::VarDataTooLong { field: "note", max_length: 64, actual: 65 };
        assert_eq!(dm::OrderEncoder::encoded_length_with_header_from(&order), Err(expect));
        let r = dm::OrderEncoder::try_wrap_and_apply_header(&mut buf, 0)?.encode_from(&order);
        assert!(matches!(r, Err(dm::sbe_rt::EncodeError::VarDataTooLong { field: "note", .. })));
        "#,
    );
    Ok(())
}

/// An optional scalar keeps its wire type even when a domain selector matches
/// it (see `dto_domain_type`): the DTO field is `Option<i64>`, so the Source
/// getter must be too, and `encode_from` must write through the raw setter.
/// Asking for the domain type here is the drift `dto_domain_type` exists to
/// prevent — the trait would demand `Option<i128>` while the write takes `i64`.
#[test]
fn an_optional_field_with_a_domain_selector_keeps_the_wire_type()
-> Result<(), Box<dyn std::error::Error>> {
    const XML: &str = r#"<messageSchema package="opt" id="5" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="OptPx" primitiveType="int64" presence="optional" semanticType="ScaledPrice"/>
  </types>
  <message name="Quote" id="1">
    <field name="px" id="1" type="OptPx"/>
    <field name="qty" id="2" type="uint32"/>
  </message>
</messageSchema>"#;

    let source = generate_xml(
        XML,
        GenerationConfig::new("optq")
            .with_source_traits(true)
            .with_manual_domain_type(
                ergo_sbe::ConversionSelector::semantic_type("ScaledPrice"),
                "i128",
            ),
    )?;
    // The wire type, not the configured domain type: `OptPx` is optional, so
    // `dto_domain_type` keeps `i64`. A regression here says `Option<i128>`.
    assert!(
        source.contains("fn px(&self) -> Option<i64>"),
        "optional source getter must keep the wire type:\n{source}"
    );

    compile_and_run_modules(
        "source_trait_opt_domain",
        &[("optq", &source)],
        r#"
        use optq::{TryFromSbe, TryToSbe};

        // The decoder's `px()` domain accessor is generated because a selector
        // matches ScaledPrice; it needs these even though the DTO and the
        // Source getter keep the wire type for an optional.
        impl TryFromSbe<i64> for i128 {
            type Error = &'static str;
            fn try_from_sbe(wire: i64) -> Result<Self, Self::Error> { Ok(i128::from(wire)) }
        }
        impl TryToSbe<i64> for i128 {
            type Error = &'static str;
            fn try_to_sbe(&self) -> Result<i64, Self::Error> {
                i64::try_from(*self).map_err(|_| "out of i64 range")
            }
        }

        struct Src { px: Option<i64>, qty: u32 }
        impl optq::QuoteSource for Src {
            fn px(&self) -> Option<i64> { self.px }
            fn qty(&self) -> u32 { self.qty }
        }

        // `Some(i64::MIN)` is the SBE default null for an optional int64, so
        // writing it would read back as `None`: refused, not lost.
        let mut buf = [0u8; optq::QuoteEncoder::ENCODED_LENGTH];
        let r = optq::QuoteEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .encode_from(&Src { px: Some(i64::MIN), qty: 1 });
        match r {
            Err(optq::sbe_rt::EncodeError::SomeNullValue { at }) => {
                assert_eq!(at.to_string(), "px")
            }
            other => panic!("Some(null) must be refused, got {other:?}"),
        }

        // `Some(0)` is an ordinary value; `None` writes the null image.
        for (px, expect) in [(Some(0i64), Some(0)), (None, None)] {
            let mut b = [0u8; optq::QuoteEncoder::ENCODED_LENGTH];
            optq::QuoteEncoder::try_wrap_and_apply_header(&mut b, 0)?
                .encode_from(&Src { px, qty: 7 })?;
            assert_eq!(optq::QuoteDecoder::decode(&b, 0)?.px_wire(), expect);
        }
        "#,
    );
    Ok(())
}

/// The trait is exhaustive: a source that leaves out a field does not
/// compile, so a schema that gains a field breaks every source until it
/// provides it.
#[test]
fn a_source_missing_a_field_does_not_compile() -> Result<(), Box<dyn std::error::Error>> {
    let source = source_of(GenerationConfig::new("hb").with_source_traits(true))?;
    compile_fails_with_diagnostics(
        "source_trait_incomplete",
        &source,
        r#"
        struct OnlySeq;
        impl HeartbeatSource for OnlySeq {
            fn seq(&self) -> u64 { 1 }
        }
        "#,
        &["not all trait items implemented, missing: `side`"],
    );
    Ok(())
}

/// Source traits that cannot be generated are rejected up front: below the
/// schema version (members would be dropped), or when a trait name is
/// already a schema type.
#[test]
fn unusable_source_traits_are_rejected_at_generate_time() -> Result<(), Box<dyn std::error::Error>>
{
    let extension = Schema::from_ir(parse(SchemaSource::file(Paths::extension_schema()))?);
    let r = Generator::new(
        GenerationConfig::new("ext")
            .with_source_traits(true)
            .with_encode_version(extension.ir.version - 1),
    )
    .generate(&extension);
    assert!(
        matches!(&r, Err(GenerateError::InvalidConfiguration { option, reason, .. })
            if option == "encode_version" && reason.contains("source traits")),
        "{r:?}"
    );

    let clash = Schema::from_ir(parse(SchemaSource::xml(
        r#"<messageSchema package="clash" id="3" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="PingSource">
      <type name="x" primitiveType="uint8"/>
    </composite>
  </types>
  <message name="Ping" id="1">
    <field name="src" id="1" type="PingSource"/>
  </message>
</messageSchema>"#,
    ))?);
    let r =
        Generator::new(GenerationConfig::new("clash").with_source_traits(true)).generate(&clash);
    assert!(
        matches!(&r, Err(GenerateError::InvalidConfiguration { option, reason, .. })
            if option == "source_traits" && reason.contains("`PingSource`")),
        "{r:?}"
    );
    // Without source traits the same schema generates.
    Generator::new(GenerationConfig::new("clash")).generate(&clash)?;
    Ok(())
}

/// `encoded_length_with_header_from` is an associated function on the
/// encoder, so a first var-data (or group) of that name, itself a method on
/// the encoder, would make the path ambiguous. The sizing helper yields: the
/// tail keeps its path, and `encode_from` still writes the source.
#[test]
fn a_first_tail_named_like_the_sizing_helper_keeps_its_path()
-> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<messageSchema package="sized" id="5" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="varDataEncoding">
      <type name="length" primitiveType="uint16"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="Note" id="1">
    <field name="id" id="1" type="uint32"/>
    <data name="encodedLengthWithHeaderFrom" id="2" type="varDataEncoding"/>
  </message>
</messageSchema>"#;
    let schema = Schema::from_ir(parse(SchemaSource::xml(xml))?);
    let sized = Generator::new(GenerationConfig::new("sized").with_source_traits(true))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("no module")?
        .source
        .clone();
    compile_and_run_modules(
        "source_trait_first_tail",
        &[("sized", &sized)],
        r#"
        use sized::*;
        struct Note;
        impl NoteSource for Note {
            fn id(&self) -> u32 { 4 }
            fn encoded_length_with_header_from(&self) -> &[u8] { b"abc" }
        }

        let mut a = [0u8; 32];
        let stage = NoteEncoder::try_wrap_and_apply_header(&mut a, 0)?.fixed(&NoteFixedFields { id: 4 });
        let n = NoteEncoder::encoded_length_with_header_from(stage, b"abc")?.encoded_length_with_header();

        let mut b = [0u8; 32];
        let m = NoteEncoder::try_wrap_and_apply_header(&mut b, 0)?
            .encode_from(&Note)?
            .encoded_length_with_header();
        assert_eq!(&a[..n], &b[..m]);
        "#,
    );
    Ok(())
}

/// Messages whose fields own the source helpers' names: `Ping` a field
/// `encodeFrom`, `Pong` a field `encodedLengthWithHeaderFrom`; `Pair` has
/// both helpers.
const SOURCE_FIELDS_XML: &str = r#"<messageSchema package="named" id="4" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Ping" id="1">
    <field name="encodeFrom" id="1" type="uint32"/>
  </message>
  <message name="Pong" id="2">
    <field name="encodedLengthWithHeaderFrom" id="1" type="uint32"/>
  </message>
  <message name="Pair" id="3">
    <field name="a" id="1" type="uint32"/>
  </message>
</messageSchema>"#;

fn generate_xml(xml: &str, config: GenerationConfig) -> Result<String, Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(SchemaSource::xml(xml))?);
    Ok(Generator::new(config)
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("no module")?
        .source
        .clone())
}

/// A field named `encode_from` or `encoded_length_with_header_from` keeps
/// its accessor whether or not source traits are generated: the field wins,
/// as it does over `tail_accessor_ident`'s convenience methods, and only the
/// clashing helper is left out of that message.
#[test]
fn fields_named_like_source_helpers_keep_their_names() -> Result<(), Box<dyn std::error::Error>> {
    let with = generate_xml(
        SOURCE_FIELDS_XML,
        GenerationConfig::new("with").with_source_traits(true),
    )?;
    let without = generate_xml(SOURCE_FIELDS_XML, GenerationConfig::new("without"))?;
    compile_and_run_modules(
        "source_trait_field_wins",
        &[("with", &with), ("without", &without)],
        r#"
        struct Src;
        impl with::PingSource for Src {
            fn encode_from(&self) -> u32 { 1 }
        }
        impl with::PongSource for Src {
            fn encoded_length_with_header_from(&self) -> u32 { 2 }
        }
        impl with::PairSource for Src {
            fn a(&self) -> u32 { 3 }
        }

        // The same setters with and without source traits.
        let mut a = [0u8; with::PingEncoder::ENCODED_LENGTH];
        with::PingEncoder::try_wrap_and_apply_header(&mut a, 0)?.encode_from(1);
        let mut b = [0u8; without::PingEncoder::ENCODED_LENGTH];
        without::PingEncoder::try_wrap_and_apply_header(&mut b, 0)?.encode_from(1);
        assert_eq!(a, b);
        let mut c = [0u8; with::PongEncoder::ENCODED_LENGTH];
        with::PongEncoder::try_wrap_and_apply_header(&mut c, 0)?.encoded_length_with_header_from(2);
        let mut d = [0u8; without::PongEncoder::ENCODED_LENGTH];
        without::PongEncoder::try_wrap_and_apply_header(&mut d, 0)?.encoded_length_with_header_from(2);
        assert_eq!(c, d);

        // Only the clashing helper yields.
        assert_eq!(with::PingEncoder::encoded_length_with_header_from(&Src)?, a.len());
        let mut e = [0u8; with::PongEncoder::ENCODED_LENGTH];
        let _ = with::PongEncoder::try_wrap_and_apply_header(&mut e, 0)?.encode_from(&Src)?;
        assert_eq!(c, e);
        let mut f = [0u8; with::PairEncoder::ENCODED_LENGTH];
        assert_eq!(with::PairEncoder::encoded_length_with_header_from(&Src)?, f.len());
        let _ = with::PairEncoder::try_wrap_and_apply_header(&mut f, 0)?.encode_from(&Src)?;
        assert_eq!(with::PairDecoder::decode(&f, 0)?.a(), 3);
        "#,
    );
    Ok(())
}

/// Messages whose first tail owns `encode_from`: `Batch` a group, `Note` a
/// var-data. `Quote`'s `encodeFrom` is a later tail, on another stage type.
const SOURCE_TAILS_XML: &str = r#"<messageSchema package="tails" id="6" version="0" byteOrder="littleEndian">
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
    <composite name="varDataEncoding">
      <type name="length" primitiveType="uint16"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="Batch" id="1">
    <field name="id" id="1" type="uint32"/>
    <group name="encodeFrom" id="2" dimensionType="groupSizeEncoding">
      <field name="px" id="3" type="int64"/>
    </group>
  </message>
  <message name="Note" id="2">
    <field name="id" id="1" type="uint32"/>
    <data name="encodeFrom" id="2" type="varDataEncoding"/>
  </message>
  <message name="Quote" id="3">
    <field name="id" id="1" type="uint32"/>
    <group name="legs" id="2" dimensionType="groupSizeEncoding">
      <field name="px" id="3" type="int64"/>
    </group>
    <data name="encodeFrom" id="4" type="varDataEncoding"/>
  </message>
</messageSchema>"#;

/// A first group or var-data named `encode_from` is a method on the encoder
/// type itself, so with an `encode_from` helper beside it every
/// `XEncoder::encode_from` path would be ambiguous. The tail keeps its full
/// path, the helper is left out of that message (its sizing helper stays),
/// and a later tail of that name leaves the helper in place.
#[test]
fn a_first_tail_named_encode_from_keeps_its_path() -> Result<(), Box<dyn std::error::Error>> {
    let tails = generate_xml(
        SOURCE_TAILS_XML,
        GenerationConfig::new("tails").with_source_traits(true),
    )?;
    compile_and_run_modules(
        "source_trait_first_tail_encode_from",
        &[("tails", &tails)],
        r#"
        use tails::*;
        struct Src;
        struct Px(i64);
        impl BatchEncodeFromEntrySource for Px {
            fn px(&self) -> i64 { self.0 }
        }
        impl QuoteLegsEntrySource for Px {
            fn px(&self) -> i64 { self.0 }
        }
        impl BatchSource for Src {
            fn id(&self) -> u32 { 1 }
            fn encode_from(&self) -> impl ExactSizeIterator<Item = impl BatchEncodeFromEntrySource> {
                [Px(5)].into_iter()
            }
        }
        impl NoteSource for Src {
            fn id(&self) -> u32 { 2 }
            fn encode_from(&self) -> &[u8] { b"hi" }
        }
        impl QuoteSource for Src {
            fn id(&self) -> u32 { 3 }
            fn legs(&self) -> impl ExactSizeIterator<Item = impl QuoteLegsEntrySource> {
                [Px(6)].into_iter()
            }
            fn encode_from(&self) -> &[u8] { b"q" }
        }

        // The tails keep their full paths.
        let mut batch = [0u8; 64];
        let stage = BatchEncoder::try_wrap_and_apply_header(&mut batch, 0)?
            .fixed(&BatchFixedFields { id: 1 });
        let n = BatchEncoder::encode_from(stage, 1, |g| g.add(|e| { e.px(5); Ok(()) }))?
            .encoded_length_with_header();
        assert_eq!(BatchEncoder::encoded_length_with_header_from(&Src)?, n);
        let mut note = [0u8; 64];
        let stage = NoteEncoder::try_wrap_and_apply_header(&mut note, 0)?
            .fixed(&NoteFixedFields { id: 2 });
        let m = NoteEncoder::encode_from(stage, b"hi")?.encoded_length_with_header();
        assert_eq!(NoteEncoder::encoded_length_with_header_from(&Src)?, m);

        // A later tail of that name: the helper stays.
        let mut quote = [0u8; 64];
        let q = QuoteEncoder::try_wrap_and_apply_header(&mut quote, 0)?
            .fixed(&QuoteFixedFields { id: 3 })
            .legs(1, |g| g.add(|e| { e.px(6); Ok(()) }))?
            .encode_from(b"q")?
            .encoded_length_with_header();
        let mut again = [0u8; 64];
        let r = QuoteEncoder::try_wrap_and_apply_header(&mut again, 0)?
            .encode_from(&Src)?
            .encoded_length_with_header();
        assert_eq!(&quote[..q], &again[..r]);
        "#,
    );
    Ok(())
}

/// The yielded helpers are gone, not shadowed: `encode_from(&src)` does not
/// resolve on `Ping`, `Batch` or `Note`, and `Pong` has no sizing helper (its
/// path is ambiguous between the field setter's two stage impls, and only
/// those: a helper would be a third candidate).
#[test]
fn messages_that_own_encode_from_have_no_helper() -> Result<(), Box<dyn std::error::Error>> {
    let fields = generate_xml(
        SOURCE_FIELDS_XML,
        GenerationConfig::new("fields").with_source_traits(true),
    )?;
    let tails = generate_xml(
        SOURCE_TAILS_XML,
        GenerationConfig::new("tails").with_source_traits(true),
    )?;
    let module = format!("pub mod fields {{\n{fields}\n}}\npub mod tails {{\n{tails}\n}}\n");
    compile_fails_with_diagnostics_excluding(
        "source_trait_helper_yields",
        &module,
        r#"
        fn ping<S: fields::PingSource>(src: &S, buf: &mut [u8]) {
            let _ = fields::PingEncoder::try_wrap_and_apply_header(buf, 0).map(|e| e.encode_from(src));
        }
        fn pong<S: fields::PongSource>(src: &S) {
            let _ = fields::PongEncoder::encoded_length_with_header_from(src);
        }
        fn batch<S: tails::BatchSource>(src: &S, buf: &mut [u8]) {
            let _ = tails::BatchEncoder::try_wrap_and_apply_header(buf, 0).map(|e| e.encode_from(src));
        }
        fn note<S: tails::NoteSource>(src: &S, buf: &mut [u8]) {
            let _ = tails::NoteEncoder::try_wrap_and_apply_header(buf, 0).map(|e| e.encode_from(src));
        }
        "#,
        &[
            "expected `u32`, found `&S`",
            "multiple `encoded_length_with_header_from` found",
            "pub fn encoded_length_with_header_from(&mut self, val: u32) -> &mut Self",
            "no method named `encode_from` found for struct `BatchEncoder",
            "no method named `encode_from` found for struct `NoteEncoder",
        ],
        &["note: candidate #3", "duplicate definitions"],
    );
    Ok(())
}
