//! ErgoSBE feature tour — live demos of generated APIs.
//!
//! Schema: [`schemas/feature-tour.xml`](../schemas/feature-tour.xml)  
//! Generated codecs: `src/generated/feature_tour.rs` after `cargo build`
//! (gitignored — open that path for go-to-definition / browsing).
//!
//! # Features covered
//!
//! | Demo | Feature |
//! |------|---------|
//! | [`demo_fixed_heartbeat`] | Fixed message + `compute_length_with_header()` |
//! | [`demo_car_size_and_encode`] | Staged `CarEncodedLength` + exact buffer encode |
//! | [`demo_car_decode_stages`] | Staged decoder lane (`into_*` groups → var-data) |
//! | [`demo_car_visit_entries`] | Staged one-pass `visit_entries` + `remaining_entries` |
//! | [`demo_car_random_access`] | Random-access lane (any-order dynamic getters) |
//! | [`demo_car_mutable_ordered`] | Mutable ordered lane (`ordered()` + runtime order checks) |
//! | [`demo_car_domain_dto`] | Owned `CarDomain` DTO + re-encode round-trip |
//! | [`demo_any_message`] | Multi-template `AnyMessage` dispatch |
//! | [`demo_try_vs_trusted`] | `try_decode` / `try_from` / `wrap` + full-tail `verify` |
//! | [`demo_display_debug`] | Diagnostic `Display` / `Debug` (not a wire format) |
//! | [`demo_conversion_only`] | **`with_conversion` only** — generic `price_as` / `price_from` (no domain type on field) |
//! | [`demo_domain_type_manual_impl`] | **`with_manual_domain_type(..)`** — concrete `try_manual_price(...)?`, app-supplied impl |
//! | [`demo_bulk_add`] | `bulk_add` on the fixed-stride nested `acceleration` group |
//! | [`run_all`] | Runs every demo; used by `main` and tests |

#![allow(
    dead_code,
    unused_imports,
    non_camel_case_types,
    non_snake_case,
    clippy::all
)]

// Real file path (not OUT_DIR include) so rust-analyzer can jump into impls.
// Created by build.rs; listed in root `.gitignore` as `**/src/generated/`.
#[allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    unused_must_use,
    non_camel_case_types,
    non_snake_case,
    clippy::all,
    warnings
)]
// ANCHOR: include_build_dep_only
#[path = "generated/feature_tour.rs"]
pub mod feature_tour;
pub use feature_tour::*;
// ANCHOR_END: include_build_dep_only

use chrono::{DateTime, Utc};
use rust_decimal::Decimal as Rd;

// ─── 1. Fixed-only message ─────────────────────────────────────────────────

/// Heartbeat is fixed-block only: size with `HeartbeatEncoder::compute_length_with_header()`,
/// no staged length builder.
///
// ANCHOR: demo_fixed_heartbeat
pub fn demo_fixed_heartbeat() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Const length → stack array (no heap).
    let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
    let nanos: i64 = 1_720_000_000_000_000_000;
    // Buffer pre-sized via const compute_length_with_header; try_* still validates extent.
    let written = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&HeartbeatFixedFields {
            sequence: 7,
            timestamp: nanos as u64,
        })
        .encoded_length_with_header();

    let dec = HeartbeatDecoder::try_decode(&buf[..written], 0)?;
    assert_eq!(dec.sequence(), 7);
    let decoded_ts: DateTime<Utc> = dec.try_timestamp()?;
    assert_eq!(decoded_ts.timestamp_nanos_opt(), Some(nanos));
    Ok(buf[..written].to_vec())
}
// ANCHOR_END: demo_fixed_heartbeat

// ─── 2. EncodedLength + encode ─────────────────────────────────────────────

/// Compute exact wire length for a Car with known group shapes, allocate once,
/// encode with `wrap_and_apply_header` + fixed phase + consuming tails.
// ANCHOR: demo_car_size_and_encode
pub fn demo_car_size_and_encode() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Fuel: 2 entries with usage ASCII lengths 5 and 7.
    // Performance: 1 entry with 2 nested acceleration rows (fixed-only entries).
    // Message var-data: manufacturer / model / activationCode lengths.
    let complete_len = CarEncoder::compute_length()
        .fuel_figures_ragged(2, |ff| {
            ff.add()?.usage_description(5)?; // "Urban"
            ff.add()?.usage_description(7)?; // "Highway"
            Ok(())
        })?
        .performance_figures_ragged(1, |pf| {
            pf.add()?.acceleration(|acc| {
                acc.uniform(2)?;
                Ok(())
            })?;
            Ok(())
        })?
        .manufacturer(5)? // "Honda"
        .model(9)? // "Civic VTi"
        .activation_code(6)? // "abcdef"
        .encoded_length_with_header();

    // Exact size from compute_length → stack pad (this demo fits well under 512).
    const CAR_PAD: usize = 512;
    assert!(
        complete_len <= CAR_PAD,
        "sample car length {complete_len} exceeds stack pad {CAR_PAD}"
    );
    let mut storage = [0u8; CAR_PAD];
    let written = encode_sample_car(&mut storage[..complete_len])?;
    assert_eq!(
        written, complete_len,
        "CarEncodedLength must equal encoder-produced length"
    );
    Ok(storage[..written].to_vec())
}
/// Encode the canonical sample car into `buf` (must be pre-sized).
// ANCHOR: encode_sample_car
pub fn encode_sample_car(buf: &mut [u8]) -> Result<usize, sbe_rt::EncodeError> {
    let mut extras = OptionalExtras::default();
    extras.cruise_control(true).sports_pack(true);

    // Buffer pre-sized from EncodedLength; try_* still validates extent.
    let len = CarEncoder::try_wrap_and_apply_header(buf, 0)
        .unwrap()
        .fixed(&CarFixedFields {
            serial_number: 1234,
            model_year: 2013,
            available: true.into(),
            code: Model::A,
            some_numbers: [10, 20, 30, 40],
            vehicle_code: [b'A', b'B', b'C', b'D', b'E', b'F'],
            extras,
            engine: Engine::new(
                2000,
                4,
                [b'1', b'2', b'3'],
                0i8,
                false.into(),
                Booster::new(BoostType::TURBO, 210),
            ),
        })
        .fuel_figures(2, |g| {
            g.add(|mut e| {
                e.speed(30).mpg(35.9);
                e.usage_description(b"Urban")
            })?;
            g.add(|mut e| {
                e.speed(60).mpg(25.0);
                e.usage_description(b"Highway")
            })?;
            Ok(())
        })?
        .performance_figures(1, |g| {
            g.add(|mut e| {
                e.octane_rating(95);
                e.acceleration(2, |a| {
                    a.add(|x| {
                        x.mph(30).seconds(4.0);
                        Ok(())
                    })?;
                    a.add(|x| {
                        x.mph(60).seconds(7.5);
                        Ok(())
                    })
                })
            })?;
            Ok(())
        })?
        .manufacturer(b"Honda")?
        .model(b"Civic VTi")?
        .activation_code(b"abcdef")?
        .encoded_length_with_header();

    Ok(len)
}
// ANCHOR_END: encode_sample_car
// ANCHOR_END: demo_car_size_and_encode

/// Nested `acceleration` rows are a fixed-stride leaf group — `bulk_add`
/// writes every entry after one region check.
// ANCHOR: demo_bulk_add
pub fn demo_bulk_add() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut extras = OptionalExtras::default();
    extras.cruise_control(true);
    let rows = [
        CarPerformanceFiguresAccelerationEntry {
            mph: 30,
            seconds: 4.0,
        },
        CarPerformanceFiguresAccelerationEntry {
            mph: 60,
            seconds: 7.5,
        },
    ];
    let complete_len = CarEncoder::compute_length()
        .fuel_figures_ragged(0, |_| Ok(()))?
        .performance_figures_ragged(1, |pf| {
            pf.add()?.acceleration(|acc| {
                acc.uniform(2)?;
                Ok(())
            })?;
            Ok(())
        })?
        .manufacturer(5)?
        .model(5)?
        .activation_code(3)?
        .encoded_length_with_header();
    const PAD: usize = 256;
    assert!(
        complete_len <= PAD,
        "bulk-add car length {complete_len} exceeds pad {PAD}"
    );
    let mut storage = [0u8; PAD];
    let buf = &mut storage[..complete_len];
    let len = CarEncoder::try_wrap_and_apply_header(buf, 0)?
        .fixed(&CarFixedFields {
            serial_number: 1234,
            model_year: 2013,
            available: true.into(),
            code: Model::A,
            some_numbers: [10, 20, 30, 40],
            vehicle_code: *b"ABCDEF",
            extras,
            engine: Engine::new(
                2000,
                4,
                *b"123",
                0i8,
                false.into(),
                Booster::new(BoostType::TURBO, 210),
            ),
        })
        .fuel_figures(0, |_| Ok(()))?
        .performance_figures(1, |g| {
            g.add(|mut e| {
                e.octane_rating(95);
                e.acceleration(2, |a| {
                    a.bulk_add(&rows)?;
                    Ok(())
                })
            })?;
            Ok(())
        })?
        .manufacturer(b"Honda")?
        .model(b"Civic")?
        .activation_code(b"abc")?
        .encoded_length_with_header();
    assert_eq!(len, complete_len);
    Ok(buf[..len].to_vec())
}
// ANCHOR_END: demo_bulk_add

// ─── 3. Decoder consuming stages ───────────────────────────────────────────

/// Walk Car in wire order: fixed random-access fields, then groups, then var-data.
// ANCHOR: demo_car_decode_stages
pub fn demo_car_decode_stages(wire: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: flyweight_access
    let car = CarDecoder::try_decode(wire, 0)?;
    assert_eq!(car.serial_number(), 1234);
    assert_eq!(car.model_year(), 2013);
    // Domain conversion: BooleanType → bool when configured.
    let available: bool = car.try_available()?;
    assert!(available);
    assert_eq!(car.code(), Model::A);
    assert_eq!(car.discounted_model(), Model::C); // constant field
    assert_eq!(car.engine().capacity(), 2000);
    // ANCHOR_END: flyweight_access

    // Consuming stages enforce fuelFigures → performanceFigures → strings.
    let mut fuel = car.into_fuel_figures()?;
    let mut speeds = Vec::new();
    for entry in &mut fuel {
        speeds.push(entry?.speed());
    }
    assert_eq!(speeds, vec![30, 60]);

    let decoder = fuel.finish()?;
    let mut decoder = decoder.into_performance_figures()?;
    let mut octanes = Vec::new();
    for entry in &mut decoder {
        let e = entry?;
        octanes.push(e.octane_rating());
        let mut acc = e.into_acceleration()?;
        let mut mphs = Vec::new();
        for a in &mut acc {
            mphs.push(a.mph());
        }
        assert_eq!(mphs, vec![30, 60]);
        let _ = acc.finish()?;
    }
    assert_eq!(octanes, vec![95]);

    let decoder = decoder.finish()?;
    let (mfr, decoder) = decoder.into_manufacturer_as_str()?;
    let (model, decoder) = decoder.into_model_as_str()?;
    let (code, _decoder) = decoder.into_activation_code_as_str()?;
    // All three &str coexist — each borrows 'a from the original wire buffer.
    assert_eq!((mfr, model, code), ("Honda", "Civic VTi", "abcdef"));
    Ok(())
}
// ANCHOR_END: demo_car_decode_stages

/// Walk Car through the ordered one-pass group path.
// ANCHOR: demo_car_visit_entries
pub fn demo_car_visit_entries(wire: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let car = CarDecoder::try_decode(wire, 0)?;
    let figures = car.into_fuel_figures()?;
    let count = figures.remaining_entries();
    assert!(count > 0);
    assert!(!figures.is_empty());

    let mut speeds = Vec::new();
    let mut octanes = Vec::new();
    let (mfr, car) = figures
        .visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
            speeds.push(entry.speed());
            let (_usage, complete) = entry.into_usage_description()?;
            Ok(complete)
        })?
        .into_performance_figures()?
        .visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
            octanes.push(entry.octane_rating());
            entry
                .into_acceleration()?
                .visit_entries(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })
        })?
        .into_manufacturer_as_str()?;
    let (model, car) = car.into_model_as_str()?;
    let (code, _) = car.into_activation_code_as_str()?;
    assert_eq!(speeds, vec![30, 60]);
    assert_eq!(octanes, vec![95]);
    assert_eq!((mfr, model, code), ("Honda", "Civic VTi", "abcdef"));
    Ok(())
}
// ANCHOR_END: demo_car_visit_entries

/// Random-access lane: dynamic getters may be called in any order.
// ANCHOR: demo_car_random_access
pub fn demo_car_random_access(wire: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let car = CarDecoder::try_decode(wire, 0)?;
    // Manufacturer is the first var-data field, after both groups — still legal
    // here because random access rescan preceding tails.
    assert_eq!(car.manufacturer()?, b"Honda");
    assert_eq!(car.serial_number(), 1234);
    let mut speeds = Vec::new();
    for entry in car.fuel_figures()? {
        speeds.push(entry?.speed());
    }
    assert_eq!(speeds, vec![30, 60]);
    assert_eq!(car.model()?, b"Civic VTi");
    Ok(())
}
// ANCHOR_END: demo_car_random_access

/// Mutable ordered lane: one cursor, schema-order tails, runtime OutOfOrder.
// ANCHOR: demo_car_mutable_ordered
pub fn demo_car_mutable_ordered(wire: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut car = CarDecoder::try_decode(wire, 0)?.ordered();
    assert_eq!(car.serial_number(), 1234);
    let figures = car.fuel_figures()?;
    assert!(figures.remaining_entries() > 0);
    let mut speeds = Vec::new();
    figures.visit_entries(|entry| -> Result<(), sbe_rt::DecodeError> {
        speeds.push(entry.speed());
        let _usage = entry.usage_description()?;
        Ok(())
    })?;
    car.performance_figures()?
        .visit_entries(|entry| -> Result<(), sbe_rt::DecodeError> {
            let _ = entry.octane_rating();
            entry
                .acceleration()?
                .visit_entries(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })?;
            Ok(())
        })?;
    let manufacturer = car.manufacturer_as_str()?;
    let model = car.model_as_str()?;
    let code = car.activation_code_as_str()?;
    let _complete = car.finish()?;
    assert_eq!(speeds, vec![30, 60]);
    assert_eq!(
        (manufacturer, model, code),
        ("Honda", "Civic VTi", "abcdef")
    );
    Ok(())
}
// ANCHOR_END: demo_car_mutable_ordered

// ─── 4. Domain DTO ─────────────────────────────────────────────────────────

/// Materialise owned `CarDomain`, re-encode, compare bytes.
// ANCHOR: demo_car_domain_dto
pub fn demo_car_domain_dto(wire: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let dec = CarDecoder::try_decode(wire, 0)?;
    // try_from_decoder (not TryFrom/From): two fallible sources — decoder vs
    // try_from_slice_with_header for framed bytes; materialisation can fail.
    let dto = CarDomain::try_from_decoder(dec)?;
    assert_eq!(dto.serial_number, 1234);
    assert!(dto.available); // bool domain field
    assert_eq!(dto.fuel_figures.len(), 2);
    assert_eq!(dto.fuel_figures[0].usage_description, "Urban");
    assert_eq!(dto.manufacturer, "Honda");

    const RE_PAD: usize = 512;
    assert!(wire.len() <= RE_PAD);
    let mut storage = [0u8; RE_PAD];
    let n = dto.encode(&mut storage[..wire.len()])?;
    assert_eq!(&storage[..n], wire, "DTO re-encode must be byte-identical");
    Ok(())
}
// ANCHOR_END: demo_car_domain_dto

// ─── 5. AnyMessage ─────────────────────────────────────────────────────────

/// Encode Heartbeat + Note into one buffer and dispatch by template id.
// ANCHOR: demo_any_message
pub fn demo_any_message() -> Result<(), Box<dyn std::error::Error>> {
    let mut hb = [0u8; HeartbeatEncoder::compute_length_with_header()];
    let hb_len = HeartbeatEncoder::compute_length_with_header();
    let nanos: u64 = 1_700_000_000_000_000_000;
    let _hb_len = HeartbeatEncoder::try_wrap_and_apply_header(&mut hb, 0)
        .unwrap()
        .fixed(&HeartbeatFixedFields {
            sequence: 1,
            timestamp: nanos,
        })
        .encoded_length_with_header();

    let note_body = b"hello AnyMessage";
    let note_len = NoteEncoder::compute_length_with_header(note_body.len());
    const NOTE_PAD: usize = 64;
    assert!(note_len <= NOTE_PAD);
    let mut note_storage = [0u8; NOTE_PAD];
    let note = &mut note_storage[..note_len];
    let note_written = NoteEncoder::try_wrap_and_apply_header(note, 0)?
        .fixed(&NoteFixedFields { note_id: 99 })
        .body(note_body)?
        .encoded_length_with_header();
    assert_eq!(note_written, note_len);

    // Concatenate framed messages (each includes its own SBE header).
    let mut stream = Vec::new();
    stream.extend_from_slice(&hb[..hb_len]);
    stream.extend_from_slice(&note[..note_written]);

    let mut offset = 0usize;
    let mut saw_heartbeat = false;
    let mut saw_note = false;
    while offset < stream.len() {
        match AnyMessage::try_decode(&stream, offset)? {
            AnyMessage::Heartbeat(d) => {
                assert_eq!(d.sequence(), 1);
                offset += d.encoded_length_with_header()?;
                saw_heartbeat = true;
            }
            AnyMessage::Note(d) => {
                assert_eq!(d.note_id(), 99);
                let (body, complete) = d.into_body()?;
                assert_eq!(body, note_body);
                offset += complete.encoded_length() + NoteDecoder::HEADER_LENGTH;
                saw_note = true;
            }
            AnyMessage::Car(_) => return Err("unexpected Car in this demo stream".into()),
            AnyMessage::Quote(_) => return Err("unexpected Quote in this demo stream".into()),
            AnyMessage::Unknown { .. } => {
                return Err("unexpected Unknown template".into());
            }
        }
    }
    assert!(saw_heartbeat && saw_note);
    Ok(())
}
// ANCHOR_END: demo_any_message

// ─── 6. Checked constructors + verify ───────────────────────────────────────

/// Trust-boundary constructors reject short / wrong-schema buffers.
// ANCHOR: demo_try_vs_trusted
pub fn demo_try_vs_trusted(valid_car: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // `decode` validates template_id, schema_id, version, and the
    // version-aware fixed body extent at message start (offset 0).
    let _dec = CarDecoder::try_decode(valid_car, 0)?;

    // `verify` walks the complete dynamic tail (groups + var-data), not a
    // header-only peek — use it when you need full structural acceptance
    // without materialising a long-lived decoder stage chain.
    CarDecoder::verify(valid_car)?;

    // Truncated buffers fail checked entry points with Result errors.
    assert!(
        CarDecoder::try_decode(&valid_car[..8.min(valid_car.len())], 0).is_err(),
        "truncated buffer should fail try_decode"
    );
    if valid_car.len() > 16 {
        assert!(
            CarDecoder::verify(&valid_car[..16]).is_err(),
            "truncated buffer should fail verify (incomplete tail)"
        );
    }

    // `wrap` still returns Result and validates the body extent given acting
    // block_length + version (message start, not sbe-tool body offset).
    let mut hdr_bytes = [0u8; 8];
    hdr_bytes.copy_from_slice(&valid_car[..8]);
    let hdr = MessageHeader(hdr_bytes);
    let dec = CarDecoder::try_wrap(valid_car, 0, hdr.block_length() as usize, hdr.version())?;
    assert_eq!(dec.serial_number(), 1234);
    Ok(())
}
// ANCHOR_END: demo_try_vs_trusted

// ─── 7. Display / Debug ────────────────────────────────────────────────────

/// Diagnostic formatting — not a stable serialization format.
// ANCHOR: demo_display_debug
pub fn demo_display_debug(valid_car: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let car = CarDecoder::try_from(valid_car)?;
    let display = format!("{car}");
    let debug = format!("{car:#?}");

    // Display is a compact one-liner; field names use the schema's camelCase.
    assert!(display.contains("serialNumber: 1234"));
    assert!(display.contains("modelYear: 2013"));
    assert!(display.contains("available: true"));
    assert!(display.contains(r#"code: A"#));
    assert!(display.contains("manufacturer: \"Honda\""));
    assert!(display.contains("model: \"Civic VTi\""));
    assert!(display.contains("fuelFigures: ["));
    assert!(display.contains("performanceFigures: ["));
    assert!(display.contains(r#"activationCode: "abcdef""#));

    // Pretty Debug ({:#?}) shows each field on its own line with indentation.
    for expected in &[
        "serialNumber: 1234",
        "modelYear: 2013",
        "available: true",
        r#"manufacturer: "Honda""#,
        r#"model: "Civic VTi""#,
        r#"activationCode: "abcdef""#,
        r#"usageDescription: Urban"#,
        "speed: 30",
        "mpg: 35.9",
        "octaneRating: 95",
    ] {
        assert!(
            debug.contains(expected),
            "Debug missing: {expected}\n--- full debug ---\n{debug}"
        );
    }

    // ── CarDomain DTO ───────────────────────────────────────────────────
    // ANCHOR: car_domain_dto_struct
    // Owned, heap-allocated, serialisable snapshot of the full message tree.
    // Generated by `with_domain_objects(DomainVarData::Strings)`.
    let dto = CarDomain::try_from_decoder(car)?;
    let dto_dbg = format!("{dto:#?}");
    // DTO field names use Rust snake_case (different from wire decoder camelCase).
    assert!(dto_dbg.contains("serial_number: 1234"));
    assert!(dto_dbg.contains("model_year: 2013"));
    assert!(dto_dbg.contains("available: true"));
    assert!(dto_dbg.contains("fuel_figures: ["));
    assert!(dto_dbg.contains(r#"usage_description: "Urban""#));
    assert!(dto_dbg.contains(r#"manufacturer: "Honda""#));
    assert!(dto_dbg.contains(r#"activation_code: "abcdef""#));
    // ANCHOR_END: car_domain_dto_struct

    Ok(())
}
// ANCHOR_END: demo_display_debug

// ─── 8. with_conversion only: generic price_as / price_from ────────────────

// App adapter — generator does not depend on rust_decimal.
impl TryFromSbe<Decimal> for Rd {
    type Error = &'static str;

    fn try_from_sbe(wire: Decimal) -> Result<Self, Self::Error> {
        let mantissa = i128::from(wire.mantissa());
        let exponent = wire.exponent();
        let (mantissa, scale) = if exponent < 0 {
            (mantissa, (-exponent) as u32)
        } else {
            (
                mantissa.saturating_mul(10i128.saturating_pow(exponent as u32)),
                0,
            )
        };
        Ok(Rd::from_i128_with_scale(mantissa, scale))
    }
}

impl TryToSbe<Decimal> for Rd {
    type Error = &'static str;

    fn try_to_sbe(&self) -> Result<Decimal, Self::Error> {
        let mantissa: i64 = self
            .mantissa()
            .try_into()
            .map_err(|_| "Decimal mantissa overflow i64")?;
        Ok(Decimal::new(mantissa, -(self.scale() as i8)))
    }
}

/// Second adapter for the same wire type (shows conversion is pluggable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedPrice {
    pub mantissa: i64,
    pub exponent: i8,
}

impl TryFromSbe<Decimal> for FixedPrice {
    type Error = &'static str;

    fn try_from_sbe(wire: Decimal) -> Result<Self, Self::Error> {
        Ok(Self {
            mantissa: wire.mantissa(),
            exponent: wire.exponent(),
        })
    }
}

impl TryToSbe<Decimal> for FixedPrice {
    type Error = &'static str;

    fn try_to_sbe(&self) -> Result<Decimal, Self::Error> {
        Ok(Decimal::new(self.mantissa, self.exponent))
    }
}

/// `with_conversion(Decimal)` only (see `build.rs`).
///
/// | API on Quote | Present? |
/// |--------------|----------|
/// | `price_from` / `price_as::<T>` | yes |
/// | `price() -> rust_decimal::Decimal` | **no** (that needs `with_domain_type`) |
/// | `price_value()` wire composite | yes |
// ANCHOR: demo_conversion_only
pub fn demo_conversion_only() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buf = [0u8; QuoteEncoder::compute_length_with_header()];

    let price = Rd::new(12345, 2); // 123.45
    let size = Rd::new(10, 0);
    let mut enc = QuoteEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
    enc.price_from(&price)?;
    enc.size_from(&size)?;
    let len = QuoteEncoder::compute_length_with_header();

    let dec = QuoteDecoder::try_from(&buf[..len])?;
    let wire = dec.price_value();
    assert_eq!(wire.mantissa(), 12345);
    assert_eq!(wire.exponent(), -2);

    let price2: Rd = dec.price_as()?;
    let size2: Rd = dec.size_as()?;
    assert_eq!(price2, price);
    assert_eq!(size2, size);

    // Same buffer, different app type — only possible with with_conversion.
    let fixed: FixedPrice = dec.price_as()?;
    assert_eq!(
        fixed,
        FixedPrice {
            mantissa: 12345,
            exponent: -2
        }
    );

    let dto = QuoteDomain::try_from_decoder(dec)?;
    assert_eq!(dto.price.mantissa(), 12345);
    let mut re = [0u8; QuoteEncoder::compute_length_with_header()];
    let n = dto.encode(&mut re)?;
    assert_eq!(&re[..n], &buf[..len]);
    Ok(buf[..len].to_vec())
}
// ANCHOR_END: demo_conversion_only

// ─── 9. with_manual_domain_type(..): concrete signatures, own impl ──

// App-supplied conversion for `ManualDecimal` (see build.rs). This is a
// straight copy-paste of the doc comment on `try_manual_price` in the
// generated module — DomainImpl::Manual gives you that snippet as a
// starting point precisely so you don't have to write this from scratch.
impl TryFromSbe<ManualDecimal> for Rd {
    type Error = &'static str;

    fn try_from_sbe(wire: ManualDecimal) -> Result<Self, Self::Error> {
        let mantissa = wire.mantissa() as i128;
        let exponent = wire.exponent() as i32;
        let (mantissa, scale) = if exponent < 0 {
            let scale = exponent.unsigned_abs();
            (mantissa, scale)
        } else {
            let pow = 10i128
                .checked_pow(exponent as u32)
                .ok_or("Decimal exponent overflow")?;
            let scaled = mantissa
                .checked_mul(pow)
                .ok_or("Decimal mantissa overflow")?;
            (scaled, 0)
        };
        Rd::from_i128_with_scale(mantissa, scale)
            .try_into()
            .map_err(|_| "Decimal overflow")
    }
}

impl TryToSbe<ManualDecimal> for Rd {
    type Error = &'static str;

    fn try_to_sbe(&self) -> Result<ManualDecimal, Self::Error> {
        let mantissa: i64 = self
            .mantissa()
            .try_into()
            .map_err(|_| "Decimal mantissa overflow i64")?;
        Ok(ManualDecimal::new(mantissa, -(self.scale() as i8)))
    }
}

/// `with_manual_domain_type(ManualDecimal, "rust_decimal::Decimal")`
/// (see `build.rs`). Same concrete `try_manual_price(...)?` / `try_manual_price()?`
/// signatures as `DomainImpl::Generated` gives `Decimal64`-style fields
/// elsewhere — the difference is entirely in who writes the two `impl`
/// blocks above.
///
/// | API on Quote | Present? |
/// |--------------|----------|
/// | `try_manual_price(rust_decimal::Decimal) -> Result<..>` | yes |
/// | `try_manual_price() -> Result<rust_decimal::Decimal, ..>` | yes |
/// | Auto-generated `impl TryFromSbe<ManualDecimal>` | **no** — that's the `impl` block above |
// ANCHOR: demo_domain_type_manual_impl
pub fn demo_domain_type_manual_impl() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buf = [0u8; QuoteEncoder::compute_length_with_header()];

    let price = Rd::new(12345, 2); // 123.45
    let mut enc = QuoteEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
    enc.price_from(&Rd::new(1, 0))?;
    enc.size_from(&Rd::new(1, 0))?;
    enc.try_manual_price(price)?;
    let len = QuoteEncoder::compute_length_with_header();

    let dec = QuoteDecoder::try_from(&buf[..len])?;
    let decoded = dec.try_manual_price()?;
    assert_eq!(decoded, price);
    Ok(buf[..len].to_vec())
}
// ANCHOR_END: demo_domain_type_manual_impl

// ─── Orchestrator ──────────────────────────────────────────────────────────

/// Run every feature demo. Returns Ok when all assertions pass.
pub fn run_all() -> Result<(), Box<dyn std::error::Error>> {
    println!("1) Fixed Heartbeat + compute_length_with_header()");
    let _hb = demo_fixed_heartbeat()?;
    println!("   ok\n");

    println!("2) Car EncodedLength + encode");
    let car = demo_car_size_and_encode()?;
    println!("   ok ({} bytes)\n", car.len());

    println!("3) Car decode consuming stages");
    demo_car_decode_stages(&car)?;
    println!("   ok\n");

    println!("3b) Car ordered visit_entries");
    demo_car_visit_entries(&car)?;
    println!("   ok\n");

    println!("3c) Car random-access lane");
    demo_car_random_access(&car)?;
    println!("   ok\n");

    println!("3d) Car mutable ordered lane");
    demo_car_mutable_ordered(&car)?;
    println!("   ok\n");

    println!("4) CarDomain DTO round-trip");
    demo_car_domain_dto(&car)?;
    println!("   ok\n");

    println!("5) AnyMessage multi-template dispatch");
    demo_any_message()?;
    println!("   ok\n");

    println!("6) checked decode / wrap / verify (trust boundary)");
    demo_try_vs_trusted(&car)?;
    println!("   ok\n");

    println!("7) Display / Debug diagnostics");
    demo_display_debug(&car)?;
    println!("   ok\n");

    println!("8) with_conversion only (generic price_as / price_from)");
    let _q = demo_conversion_only()?;
    println!("   ok\n");

    println!("9) with_manual_domain_type(..): concrete signatures, own impl");
    let _mq = demo_domain_type_manual_impl()?;
    println!("   ok\n");

    println!("10) bulk_add on fixed-stride acceleration rows");
    let _bulk = demo_bulk_add()?;
    println!("   ok\n");

    println!("All feature-tour demos passed.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_feature_demos() -> Result<(), Box<dyn std::error::Error>> {
        run_all()
    }

    #[test]
    fn heartbeat_encoded_length_is_constant() {
        assert!(HeartbeatEncoder::compute_length_with_header() >= 8 + 16);
    }

    #[test]
    fn conversion_only_roundtrip_rust_decimal_and_fixed() -> Result<(), Box<dyn std::error::Error>>
    {
        let wire = demo_conversion_only()?;
        let dec = QuoteDecoder::try_from(wire.as_slice())?;
        let rd: Rd = dec.price_as()?;
        assert_eq!(rd, Rd::new(12345, 2));
        let fixed: FixedPrice = dec.price_as()?;
        assert_eq!(fixed.mantissa, 12345);
        assert_eq!(fixed.exponent, -2);
        // Wire setter path still works without any adapter:
        let mut buf = [0u8; QuoteEncoder::compute_length_with_header()];
        QuoteEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .price_wire(Decimal::new(1, -2))
            .size_wire(Decimal::new(2, 0));
        let d2 = QuoteDecoder::try_from(buf.as_slice())?;
        assert_eq!(d2.price_value().mantissa(), 1);
        Ok(())
    }

    #[test]
    fn domain_type_manual_impl_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let wire = demo_domain_type_manual_impl()?;
        let dec = QuoteDecoder::try_from(wire.as_slice())?;
        assert_eq!(dec.try_manual_price()?, Rd::new(12345, 2));
        // The raw wire composite is a plain ManualDecimal — no auto-impl,
        // no hidden generated conversion beyond the app's own impl above.
        assert_eq!(dec.manual_price_value().mantissa(), 12345);
        assert_eq!(dec.manual_price_value().exponent(), -2);
        Ok(())
    }
}
