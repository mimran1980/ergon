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
//! | [`demo_car_decode_stages`] | Consuming decoder stages (groups → var-data) |
//! | [`demo_car_domain_dto`] | Owned `CarDomain` DTO + re-encode round-trip |
//! | [`demo_any_message`] | Multi-template `AnyMessage` dispatch |
//! | [`demo_try_vs_trusted`] | `try_wrap` / `try_from` vs trusted wrap; `verify` |
//! | [`demo_display_debug`] | Diagnostic `Display` / `Debug` (not a wire format) |
//! | [`demo_conversion_only`] | **`with_conversion` only** — generic `price_as` / `price_from` (no domain type on field) |
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
    // Buffer is exact size from const compute_length_with_header — no bounds check needed.
    let written = HeartbeatEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&HeartbeatFixedFields {
            sequence: 7,
            timestamp: nanos as u64,
        })
        .encoded_length_with_header();

    let dec = HeartbeatDecoder::try_from(&buf[..written])?;
    assert_eq!(dec.sequence(), 7);
    let decoded_ts: DateTime<Utc> = dec.timestamp();
    assert_eq!(decoded_ts.timestamp_nanos_opt(), Some(nanos));
    Ok(buf[..written].to_vec())
}
// ANCHOR_END: demo_fixed_heartbeat

// ─── 2. EncodedLength + encode ─────────────────────────────────────────────

/// Compute exact wire length for a Car with known group shapes, allocate once,
/// encode with `try_wrap_and_apply_header` + fixed phase + consuming tails.
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

    // Buffer is pre-sized from compute_length — no bounds check needed.
    let len = CarEncoder::wrap_and_apply_header(buf, 0)
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
            g.add(|e| {
                e.speed(30).mpg(35.9).usage_description(b"Urban")?;
                Ok(())
            })?;
            g.add(|e| {
                e.speed(60).mpg(25.0).usage_description(b"Highway")?;
                Ok(())
            })?;
            Ok(())
        })?
        .performance_figures(1, |g| {
            g.add(|e| {
                e.octane_rating(95).acceleration(2, |a| {
                    a.add(|x| {
                        x.mph(30).seconds(4.0);
                        Ok(())
                    })?;
                    a.add(|x| {
                        x.mph(60).seconds(7.5);
                        Ok(())
                    })?;
                    Ok(())
                })?;
                Ok(())
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

// ─── 3. Decoder consuming stages ───────────────────────────────────────────

/// Walk Car in wire order: fixed random-access fields, then groups, then var-data.
// ANCHOR: demo_car_decode_stages
pub fn demo_car_decode_stages(wire: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: flyweight_access
    let car = CarDecoder::try_from(wire)?;
    assert_eq!(car.serial_number(), 1234);
    assert_eq!(car.model_year(), 2013);
    // Domain conversion: BooleanType → bool when configured.
    let available: bool = car.available();
    assert!(available);
    assert_eq!(car.code(), Model::A);
    assert_eq!(car.discounted_model(), Model::C); // constant field
    assert_eq!(car.engine().capacity(), 2000);
    // ANCHOR_END: flyweight_access

    // Consuming stages enforce fuelFigures → performanceFigures → strings.
    let mut fuel = car.into_fuel_figures()?;
    let mut speeds = Vec::new();
    while let Some(entry) = fuel.next() {
        let e = entry?;
        speeds.push(e.speed());
        let _usage = e.usage_description()?; // ASCII var-data on entry
    }
    assert_eq!(speeds, vec![30, 60]);

    let after_fuel = fuel.finish()?;
    let mut perf = after_fuel.into_performance_figures()?;
    let mut octanes = Vec::new();
    while let Some(entry) = perf.next() {
        let e = entry?;
        octanes.push(e.octane_rating());
        let mut acc = e.into_acceleration()?;
        let mut mphs = Vec::new();
        while let Some(a) = acc.next() {
            mphs.push(a.mph());
        }
        assert_eq!(mphs, vec![30, 60]);
        let _ = acc.finish()?;
    }
    assert_eq!(octanes, vec![95]);

    let after_perf = perf.finish()?;
    let (mfr, after_mfr) = after_perf.into_manufacturer_as_str()?;
    assert_eq!(mfr, "Honda");
    let (model, after_model) = after_mfr.into_model_as_str()?;
    assert_eq!(model, "Civic VTi");
    let (code, _done) = after_model.into_activation_code_as_str()?;
    assert_eq!(code, "abcdef");
    Ok(())
}
// ANCHOR_END: demo_car_decode_stages

// ─── 4. Domain DTO ─────────────────────────────────────────────────────────

/// Materialise owned `CarDomain`, re-encode, compare bytes.
// ANCHOR: demo_car_domain_dto
pub fn demo_car_domain_dto(wire: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let dec = CarDecoder::try_from(wire)?;
    // Prefer try_from_decoder when you need fallible conversion; From panics on
    // malformed tails (see generated docs).
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
    HeartbeatEncoder::wrap_and_apply_header(&mut hb, 0)
        .fixed(&HeartbeatFixedFields {
            sequence: 1,
            timestamp: nanos,
        });

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
        match AnyMessage::decode(&stream, offset)? {
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

// ─── 6. try_* vs trusted wrap ──────────────────────────────────────────────

/// Trust-boundary constructors reject short / wrong-schema buffers.
// ANCHOR: demo_try_vs_trusted
pub fn demo_try_vs_trusted(valid_car: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // `try_wrap_and_apply_header` validates template_id, schema_id, version,
    // block_length, and buffer bounds at the given offset. The `_dec` binding
    // holds the decoder alive until dropped (showing both paths).
    let _dec = CarDecoder::try_wrap_and_apply_header(valid_car, 0)?;

    // `verify` is a cheaper header-only check — no decoder constructed.
    CarDecoder::verify(valid_car)?;

    // Truncated buffer must fail try_from / verify.
    assert!(
        CarDecoder::try_from(&valid_car[..8.min(valid_car.len())]).is_err(),
        "truncated buffer should fail try_from"
    );
    if valid_car.len() > 16 {
        assert!(
            CarDecoder::verify(&valid_car[..16]).is_err(),
            "truncated buffer should fail verify"
        );
    }

    // Trusted wrap is only for already-validated buffers (header already checked
    // by try_*). Signature: wrap(buf, body_pos, acting_block_length, version).
    let mut hdr_bytes = [0u8; 8];
    hdr_bytes.copy_from_slice(&valid_car[..8]);
    let hdr = MessageHeader(hdr_bytes);
    let trusted = CarDecoder::wrap(valid_car, 0, hdr.block_length() as usize, hdr.version());
    assert_eq!(trusted.serial_number(), 1234);
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
    // Generated by `enable_domain_objects(DomainVarData::LossyStrings)`.
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
    let len = QuoteEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .price_from(&price)?
        .size_from(&size)?
        .encoded_length_with_header();

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

    let dto = QuoteDomain::from(dec);
    assert_eq!(dto.price.mantissa(), 12345);
    let mut re = [0u8; QuoteEncoder::compute_length_with_header()];
    let n = dto.encode(&mut re)?;
    assert_eq!(&re[..n], &buf[..len]);
    Ok(buf[..len].to_vec())
}
// ANCHOR_END: demo_conversion_only

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

    println!("4) CarDomain DTO round-trip");
    demo_car_domain_dto(&car)?;
    println!("   ok\n");

    println!("5) AnyMessage multi-template dispatch");
    demo_any_message()?;
    println!("   ok\n");

    println!("6) try_* trust boundary vs trusted wrap");
    demo_try_vs_trusted(&car)?;
    println!("   ok\n");

    println!("7) Display / Debug diagnostics");
    demo_display_debug(&car)?;
    println!("   ok\n");

    println!("8) with_conversion only (generic price_as / price_from)");
    let _q = demo_conversion_only()?;
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
}
