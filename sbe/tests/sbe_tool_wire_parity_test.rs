//! Live **byte-identical wire parity**: ergo-sbe vs official sbe-tool Rust
//! codecs for the Car example schema.
//!
//! ## What this proves
//!
//! Encode the **same logical payload** with both generators and require
//! `ergo_bytes == sbe_tool_bytes` for the full frame (header + body + tails).
//! Also cross-decode both directions and match the Java-captured full-car
//! fixture when applicable.
//!
//! ## What already existed
//!
//! - Java `.sbe` fixture decode + partial byte checks (`baseline_test`)
//! - Cluster goldens captured from sbe-tool, re-proven by ergo only
//! - Head-to-head **speed** benches (not byte asserts)
//!
//! This file is the missing **dual Rust encode** gate for Car.
//!
//! ## Reference codec
//!
//! Checked-in sbe-tool output (patched for in-tree inclusion):
//! `sbe/benchmarks/src/sbe_tool_car_patched.rs`
//!
//! Ergo side uses the committed golden module
//! `sbe/tests/golden/car_example.rs` (same schema as example-schema.xml).
//!
//! ## Coverage matrix (Car)
//!
//! | Area | Scenarios |
//! |------|-----------|
//! | Full baseline | Java fixture identity |
//! | Empty groups + empty var-data | scalars including extremes |
//! | Enums | Model A/B/C, Boolean T/F, BoostType all 4 |
//! | Bit set | all 8 OptionalExtras combinations |
//! | Fuel group | 0..N entries, empty/long usageDescription |
//! | Nested performance | 0..N outer × 0..M acceleration |
//! | Var-data | empty, 1-byte, multi-byte, longer ASCII/UTF-8 |
//! | Floats | 0.0, −0.0, normals, large, subnormal-ish |
//! | Cross-decode | ergo←tool, tool←ergo on full + sparse shapes |
//! | Header constants | blockLength/template/schema/version |

#![allow(
    unsafe_code,
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    dead_code,
    unused,
    non_camel_case_types,
    non_snake_case
)]

// ── ergo-sbe Car (golden codegen snapshot) ────────────────────────────────

// Keep the generated snapshot in generator formatting; stability_test compares
// its canonical syntax rather than rustfmt layout.
#[rustfmt::skip]
#[path = "golden/car_example.rs"]
mod ergo;

// ── sbe-tool Car (official generator, patched for module inclusion) ───────

#[path = "../benchmarks/src/sbe_tool_car_patched.rs"]
mod sbe_tool_car;

use ergo::{
    BooleanType as ErgoBool, BoostType as ErgoBoost, Booster as ErgoBooster, CarDecoder as ErgoDec,
    CarEncoder as ErgoEnc, Engine as ErgoEngine, Model as ErgoModel, OptionalExtras as ErgoExtras,
};
use sbe_tool_car::sbe_tool::{
    Encoder, ReadBuf, SBE_SCHEMA_ID, SBE_SCHEMA_VERSION, WriteBuf,
    boolean_type::BooleanType as ToolBool,
    boost_type::BoostType as ToolBoost,
    car_codec::{
        SBE_BLOCK_LENGTH, SBE_TEMPLATE_ID,
        decoder::CarDecoder as ToolDec,
        encoder::{
            AccelerationEncoder, CarEncoder as ToolEnc, FuelFiguresEncoder,
            PerformanceFiguresEncoder,
        },
    },
    message_header_codec::{self, decoder::MessageHeaderDecoder},
    model::Model as ToolModel,
    optional_extras::OptionalExtras as ToolExtras,
};

// ── Logical payload (shared by both encoders) ─────────────────────────────

#[derive(Clone, Debug)]
struct FuelEntry {
    speed: u16,
    mpg: f32,
    usage: &'static [u8],
}

#[derive(Clone, Debug)]
struct AccelEntry {
    mph: u16,
    seconds: f32,
}

#[derive(Clone, Debug)]
struct PerfEntry {
    octane: u8,
    accel: Vec<AccelEntry>,
}

#[derive(Clone, Debug)]
struct CarPayload {
    serial: u64,
    year: u16,
    available: bool,
    /// 0=A, 1=B, 2=C
    model: u8,
    some_numbers: [u32; 4],
    vehicle_code: [u8; 6],
    /// bit0 sun_roof, bit1 sports_pack, bit2 cruise_control
    extras_bits: u8,
    engine_capacity: u16,
    engine_cylinders: u8,
    manufacturer_code: [u8; 3],
    efficiency: i8,
    booster_enabled: bool,
    /// 0=TURBO, 1=SUPERCHARGER, 2=NITROUS, 3=KERS
    boost: u8,
    horse_power: u8,
    fuel: Vec<FuelEntry>,
    perf: Vec<PerfEntry>,
    manufacturer: &'static [u8],
    model_name: &'static [u8],
    activation_code: &'static [u8],
}

impl CarPayload {
    fn baseline() -> Self {
        Self {
            serial: 1234,
            year: 2013,
            available: true,
            model: 0,
            some_numbers: [1, 2, 3, 4],
            vehicle_code: [97, 98, 99, 100, 101, 102],
            extras_bits: 0b110, // sports_pack + cruise_control
            engine_capacity: 2000,
            engine_cylinders: 4,
            manufacturer_code: [b'1', b'2', b'3'],
            efficiency: 35,
            booster_enabled: true,
            boost: 2, // NITROUS
            horse_power: 200,
            fuel: vec![
                FuelEntry {
                    speed: 30,
                    mpg: 35.9,
                    usage: b"Urban Cycle",
                },
                FuelEntry {
                    speed: 55,
                    mpg: 49.0,
                    usage: b"Combined Cycle",
                },
                FuelEntry {
                    speed: 75,
                    mpg: 40.0,
                    usage: b"Highway Cycle",
                },
            ],
            perf: vec![
                PerfEntry {
                    octane: 95,
                    accel: vec![
                        AccelEntry {
                            mph: 30,
                            seconds: 4.0,
                        },
                        AccelEntry {
                            mph: 60,
                            seconds: 7.5,
                        },
                        AccelEntry {
                            mph: 100,
                            seconds: 12.2,
                        },
                    ],
                },
                PerfEntry {
                    octane: 99,
                    accel: vec![
                        AccelEntry {
                            mph: 30,
                            seconds: 3.8,
                        },
                        AccelEntry {
                            mph: 60,
                            seconds: 7.1,
                        },
                        AccelEntry {
                            mph: 100,
                            seconds: 11.8,
                        },
                    ],
                },
            ],
            manufacturer: b"Honda",
            model_name: b"Civic VTi",
            activation_code: b"abcdef",
        }
    }

    fn empty_tails(serial: u64, year: u16) -> Self {
        Self {
            serial,
            year,
            available: false,
            model: 1, // B
            some_numbers: [0; 4],
            vehicle_code: [0; 6],
            extras_bits: 0,
            engine_capacity: 0,
            engine_cylinders: 0,
            manufacturer_code: [0; 3],
            efficiency: 0,
            booster_enabled: false,
            boost: 0, // TURBO
            horse_power: 0,
            fuel: vec![],
            perf: vec![],
            manufacturer: b"",
            model_name: b"",
            activation_code: b"",
        }
    }

    fn ergo_bool(v: bool) -> ErgoBool {
        if v { ErgoBool::T } else { ErgoBool::F }
    }

    fn tool_bool(v: bool) -> ToolBool {
        if v { ToolBool::T } else { ToolBool::F }
    }

    fn ergo_model(&self) -> ErgoModel {
        match self.model {
            0 => ErgoModel::A,
            1 => ErgoModel::B,
            _ => ErgoModel::C,
        }
    }

    fn tool_model(&self) -> ToolModel {
        match self.model {
            0 => ToolModel::A,
            1 => ToolModel::B,
            _ => ToolModel::C,
        }
    }

    fn ergo_boost(&self) -> ErgoBoost {
        match self.boost {
            0 => ErgoBoost::TURBO,
            1 => ErgoBoost::SUPERCHARGER,
            2 => ErgoBoost::NITROUS,
            _ => ErgoBoost::KERS,
        }
    }

    fn tool_boost(&self) -> ToolBoost {
        match self.boost {
            0 => ToolBoost::TURBO,
            1 => ToolBoost::SUPERCHARGER,
            2 => ToolBoost::NITROUS,
            _ => ToolBoost::KERS,
        }
    }

    fn encode_ergo(&self, buf: &mut [u8]) -> usize {
        let mut car = ErgoEnc::wrap_and_apply_header(buf, 0);
        car.serial_number(self.serial);
        car.model_year(self.year);
        car.available(Self::ergo_bool(self.available));
        car.code(self.ergo_model());
        car.some_numbers(self.some_numbers);
        car.vehicle_code(self.vehicle_code);
        let mut extras = ErgoExtras::default();
        extras.sun_roof(self.extras_bits & 0b001 != 0);
        extras.sports_pack(self.extras_bits & 0b010 != 0);
        extras.cruise_control(self.extras_bits & 0b100 != 0);
        car.extras(extras);
        car.engine(ErgoEngine::new(
            self.engine_capacity,
            self.engine_cylinders,
            self.manufacturer_code,
            self.efficiency,
            Self::ergo_bool(self.booster_enabled),
            ErgoBooster::new(self.ergo_boost(), self.horse_power),
        ));

        let fuel = self.fuel.clone();
        let car = car
            .fuel_figures(fuel.len() as u16, |g| {
                for f in &fuel {
                    g.add(|e| {
                        e.speed(f.speed).mpg(f.mpg);
                        e.usage_description(f.usage)?;
                        Ok(())
                    })?;
                }
                Ok(())
            })
            .unwrap();

        let perf = self.perf.clone();
        let car = car
            .performance_figures(perf.len() as u16, |g| {
                for p in &perf {
                    g.add(|e| {
                        e.octane_rating(p.octane);
                        e.acceleration(p.accel.len() as u16, |a| {
                            for x in &p.accel {
                                a.add(|y| {
                                    y.mph(x.mph).seconds(x.seconds);
                                    Ok(())
                                })?;
                            }
                            Ok(())
                        })?;
                        Ok(())
                    })?;
                }
                Ok(())
            })
            .unwrap();

        let car = car.manufacturer(self.manufacturer).unwrap();
        let car = car.model(self.model_name).unwrap();
        let done = car.activation_code(self.activation_code).unwrap();
        done.encoded_length_with_header()
    }

    fn encode_tool(&self, buf: &mut [u8]) -> usize {
        let mut car = ToolEnc::default();
        let mut fuel_figures = FuelFiguresEncoder::default();
        let mut performance_figures = PerformanceFiguresEncoder::default();
        let mut acceleration = AccelerationEncoder::default();

        car = car.wrap(WriteBuf::new(buf), message_header_codec::ENCODED_LENGTH);
        car = car.header(0).parent().expect("header parent");

        car.serial_number(self.serial)
            .model_year(self.year)
            .available(Self::tool_bool(self.available))
            .code(self.tool_model())
            .some_numbers(&self.some_numbers)
            .vehicle_code(&self.vehicle_code);

        let mut extras = ToolExtras::default();
        extras
            .sun_roof(self.extras_bits & 0b001 != 0)
            .sports_pack(self.extras_bits & 0b010 != 0)
            .cruise_control(self.extras_bits & 0b100 != 0);
        car.extras(extras);

        let mut engine = car.engine_encoder();
        engine
            .capacity(self.engine_capacity)
            .num_cylinders(self.engine_cylinders)
            .manufacturer_code(&self.manufacturer_code)
            .efficiency(self.efficiency)
            .booster_enabled(Self::tool_bool(self.booster_enabled));
        let mut booster = engine.booster_encoder();
        booster.boost_type(self.tool_boost());
        booster.horse_power(self.horse_power);
        engine = booster.parent().expect("booster parent");
        car = engine.parent().expect("engine parent");

        fuel_figures = car.fuel_figures_encoder(self.fuel.len() as u16, fuel_figures);
        for (i, f) in self.fuel.iter().enumerate() {
            assert_eq!(Some(i), fuel_figures.advance().unwrap());
            fuel_figures
                .speed(f.speed)
                .mpg(f.mpg)
                .usage_description(f.usage);
        }
        car = fuel_figures.parent().expect("fuel parent");

        performance_figures =
            car.performance_figures_encoder(self.perf.len() as u16, performance_figures);
        for (i, p) in self.perf.iter().enumerate() {
            assert_eq!(Some(i), performance_figures.advance().unwrap());
            performance_figures.octane_rating(p.octane);
            acceleration =
                performance_figures.acceleration_encoder(p.accel.len() as u16, acceleration);
            for (j, x) in p.accel.iter().enumerate() {
                assert_eq!(Some(j), acceleration.advance().unwrap());
                acceleration.mph(x.mph).seconds(x.seconds);
            }
            performance_figures = acceleration.parent().expect("acc parent");
        }
        car = performance_figures.parent().expect("perf parent");

        car.manufacturer(std::str::from_utf8(self.manufacturer).unwrap_or(""))
            .model(std::str::from_utf8(self.model_name).unwrap_or(""))
            .activation_code(self.activation_code);

        car.get_limit()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn assert_frames_eq(label: &str, ergo: &[u8], tool: &[u8]) {
    assert_eq!(
        ergo.len(),
        tool.len(),
        "{label}: encoded length mismatch — ergon={}, sbe_tool={}",
        ergo.len(),
        tool.len()
    );
    if ergo != tool {
        let n = ergo.len().min(tool.len());
        let mut first = None;
        for i in 0..n {
            if ergo[i] != tool[i] {
                first = Some(i);
                break;
            }
        }
        let window = |b: &[u8], at: Option<usize>| -> String {
            match at {
                Some(i) => {
                    let start = i.saturating_sub(8);
                    let end = (i + 16).min(b.len());
                    format!("[{start}..{end}]={:02x?}", &b[start..end])
                }
                None => format!("[:64]={:02x?}", &b[..b.len().min(64)]),
            }
        };
        panic!(
            "{label}: frames differ\n  ergo len={} tool len={}\n  first mismatch at {:?}\n  ergo {}\n  tool {}",
            ergo.len(),
            tool.len(),
            first,
            window(ergo, first),
            window(tool, first),
        );
    }
}

fn assert_header_constants(frame: &[u8]) {
    assert!(frame.len() >= 8, "frame shorter than header");
    let bl = u16::from_le_bytes([frame[0], frame[1]]);
    let tid = u16::from_le_bytes([frame[2], frame[3]]);
    let sid = u16::from_le_bytes([frame[4], frame[5]]);
    let ver = u16::from_le_bytes([frame[6], frame[7]]);
    assert_eq!(bl, SBE_BLOCK_LENGTH, "blockLength");
    assert_eq!(tid, SBE_TEMPLATE_ID, "templateId");
    assert_eq!(sid, SBE_SCHEMA_ID, "schemaId");
    assert_eq!(ver, SBE_SCHEMA_VERSION, "version");
}

fn dual_encode(label: &str, p: &CarPayload) -> Vec<u8> {
    let mut ergo_buf = vec![0u8; 4096];
    let mut tool_buf = vec![0u8; 4096];
    let el = p.encode_ergo(&mut ergo_buf);
    let tl = p.encode_tool(&mut tool_buf);
    let ergo = &ergo_buf[..el];
    let tool = &tool_buf[..tl];
    assert_eq!(
        el, tl,
        "{label}: encoded length mismatch — ergon={el}, sbe_tool={tl}"
    );
    assert_header_constants(ergo);
    assert_header_constants(tool);
    assert_frames_eq(label, ergo, tool);
    ergo.to_vec()
}

/// Decode fixed fields from an ergo frame and check against the payload.
fn assert_ergo_decodes_payload(frame: &[u8], p: &CarPayload) {
    let car = ErgoDec::try_wrap_and_apply_header(frame, 0).unwrap();
    assert_eq!(car.serial_number(), p.serial);
    assert_eq!(car.model_year(), p.year);
    assert_eq!(car.available(), CarPayload::ergo_bool(p.available));
    assert_eq!(car.code(), p.ergo_model());
    assert_eq!(car.some_numbers(), p.some_numbers);
    assert_eq!(car.vehicle_code(), p.vehicle_code);
    let extras = car.extras();
    assert_eq!(extras.is_sun_roof(), p.extras_bits & 0b001 != 0);
    assert_eq!(extras.is_sports_pack(), p.extras_bits & 0b010 != 0);
    assert_eq!(extras.is_cruise_control(), p.extras_bits & 0b100 != 0);
    let eng = car.engine();
    assert_eq!(eng.capacity(), p.engine_capacity);
    assert_eq!(eng.num_cylinders(), p.engine_cylinders);
    assert_eq!(eng.manufacturer_code(), p.manufacturer_code);
    assert_eq!(eng.efficiency(), p.efficiency);
    assert_eq!(
        eng.booster_enabled(),
        CarPayload::ergo_bool(p.booster_enabled)
    );
    assert_eq!(eng.booster().boost_type(), p.ergo_boost());
    assert_eq!(eng.booster().horse_power(), p.horse_power);

    let mut fuel = car.into_fuel_figures().unwrap();
    let mut fuel_i = 0usize;
    while let Some(Ok(e)) = fuel.next() {
        let expected = &p.fuel[fuel_i];
        assert_eq!(e.speed(), expected.speed);
        assert_eq!(e.mpg().to_bits(), expected.mpg.to_bits());
        assert_eq!(e.usage_description().unwrap(), expected.usage);
        fuel_i += 1;
    }
    assert_eq!(fuel_i, p.fuel.len());
    let after = fuel.finish().unwrap();

    let mut perf = after.into_performance_figures().unwrap();
    let mut perf_i = 0usize;
    while let Some(Ok(e)) = perf.next() {
        let expected = &p.perf[perf_i];
        assert_eq!(e.octane_rating(), expected.octane);
        let acc = e.acceleration().unwrap();
        let acc_v: Vec<_> = acc.collect();
        assert_eq!(acc_v.len(), expected.accel.len());
        for (a, exp) in acc_v.iter().zip(expected.accel.iter()) {
            assert_eq!(a.mph(), exp.mph);
            assert_eq!(a.seconds().to_bits(), exp.seconds.to_bits());
        }
        perf_i += 1;
    }
    assert_eq!(perf_i, p.perf.len());
    let after_p = perf.finish().unwrap();
    let (mfr, a1) = after_p.into_manufacturer().unwrap();
    assert_eq!(mfr, p.manufacturer);
    let (model, a2) = a1.into_model().unwrap();
    assert_eq!(model, p.model_name);
    let (code, _) = a2.into_activation_code().unwrap();
    assert_eq!(code, p.activation_code);
}

/// Decode fixed fields from a tool frame (any source) via sbe-tool decoder.
fn assert_tool_decodes_payload(frame: &[u8], p: &CarPayload) {
    let header = MessageHeaderDecoder::default().wrap(ReadBuf::new(frame), 0);
    let mut car = ToolDec::default().header(header, 0);
    assert_eq!(car.serial_number(), p.serial);
    assert_eq!(car.model_year(), p.year);
    assert_eq!(car.available(), CarPayload::tool_bool(p.available));
    assert_eq!(car.code(), p.tool_model());
    assert_eq!(car.some_numbers(), p.some_numbers);
    assert_eq!(car.vehicle_code(), p.vehicle_code);
    let extras = car.extras();
    assert_eq!(extras.is_sun_roof(), p.extras_bits & 0b001 != 0);
    assert_eq!(extras.is_sports_pack(), p.extras_bits & 0b010 != 0);
    assert_eq!(extras.is_cruise_control(), p.extras_bits & 0b100 != 0);

    let engine = car.engine_decoder();
    assert_eq!(engine.capacity(), p.engine_capacity);
    assert_eq!(engine.num_cylinders(), p.engine_cylinders);
    assert_eq!(engine.manufacturer_code(), p.manufacturer_code);
    assert_eq!(engine.efficiency(), p.efficiency);
    assert_eq!(
        engine.booster_enabled(),
        CarPayload::tool_bool(p.booster_enabled)
    );
    let mut booster = engine.booster_decoder();
    assert_eq!(booster.boost_type(), p.tool_boost());
    assert_eq!(booster.horse_power(), p.horse_power);
    let mut engine = booster.parent().unwrap();
    car = engine.parent().unwrap();

    let mut ff = car.fuel_figures_decoder();
    let mut fuel_i = 0usize;
    while ff.advance().unwrap().is_some() {
        let expected = &p.fuel[fuel_i];
        assert_eq!(ff.speed(), expected.speed);
        assert_eq!(ff.mpg().to_bits(), expected.mpg.to_bits());
        let c = ff.usage_description_decoder();
        assert_eq!(ff.usage_description_slice(c), expected.usage);
        fuel_i += 1;
    }
    assert_eq!(fuel_i, p.fuel.len());
    car = ff.parent().unwrap();

    let mut pf = car.performance_figures_decoder();
    let mut perf_i = 0usize;
    while pf.advance().unwrap().is_some() {
        let expected = &p.perf[perf_i];
        assert_eq!(pf.octane_rating(), expected.octane);
        let mut acc = pf.acceleration_decoder();
        let mut acc_i = 0usize;
        while acc.advance().unwrap().is_some() {
            let exp = &expected.accel[acc_i];
            assert_eq!(acc.mph(), exp.mph);
            assert_eq!(acc.seconds().to_bits(), exp.seconds.to_bits());
            acc_i += 1;
        }
        assert_eq!(acc_i, expected.accel.len());
        pf = acc.parent().unwrap();
        perf_i += 1;
    }
    assert_eq!(perf_i, p.perf.len());
    car = pf.parent().unwrap();

    let mfr = car.manufacturer_decoder();
    assert_eq!(car.manufacturer_slice(mfr), p.manufacturer);
    let model = car.model_decoder();
    assert_eq!(car.model_slice(model), p.model_name);
    let code = car.activation_code_decoder();
    assert_eq!(car.activation_code_slice(code), p.activation_code);
}

// ── Core dual-encode tests ────────────────────────────────────────────────

#[test]
fn full_car_ergo_matches_sbe_tool_bytes() {
    let p = CarPayload::baseline();
    let frame = dual_encode("full_car", &p);
    let java = include_bytes!("fixtures/car_example_baseline_data.sbe");
    assert_eq!(
        &frame[..],
        &java[..],
        "dual-encoded full car must match Java baseline fixture"
    );
}

#[test]
fn empty_tails_ergo_matches_sbe_tool_bytes() {
    dual_encode("empty_tails", &CarPayload::empty_tails(1, 2000));
}

#[test]
fn empty_tails_varied_scalars() {
    for (serial, year) in [(0u64, 0u16), (u64::MAX - 1, 65534), (42, 1999), (1, 65534)] {
        dual_encode(
            &format!("empty_tails serial={serial} year={year}"),
            &CarPayload::empty_tails(serial, year),
        );
    }
}

#[test]
fn single_fuel_ergo_matches_sbe_tool_bytes() {
    let mut p = CarPayload::empty_tails(99, 2020);
    p.available = true;
    p.model = 2;
    p.some_numbers = [9, 8, 7, 6];
    p.vehicle_code = *b"XYZXYZ";
    p.engine_capacity = 1600;
    p.engine_cylinders = 4;
    p.manufacturer_code = *b"ABC";
    p.efficiency = 10;
    p.boost = 1; // SUPERCHARGER
    p.horse_power = 50;
    p.fuel = vec![FuelEntry {
        speed: 40,
        mpg: 33.3,
        usage: b"city",
    }];
    p.manufacturer = b"Toyota";
    p.model_name = b"Yaris";
    p.activation_code = b"zz";
    dual_encode("single_fuel", &p);
}

#[test]
fn single_fuel_empty_usage_and_ascii_edge() {
    let cases: &[(u16, f32, &[u8], &[u8], &[u8], &[u8])] = &[
        (0, 0.0, b"", b"", b"", b""),
        (120, 99.9, b"x", b"A", b"B", b"C"),
        (
            55,
            1.0 / 3.0,
            b"Combined Cycle Extra Long Description!!",
            b"ManufacturerName",
            b"ModelNameHere",
            b"actcode",
        ),
        (1, f32::from_bits(1), b"subnormal", b"mfr", b"mdl", b"act"),
        (u16::MAX - 1, -0.0, b"negzero", b"Neg", b"Zero", b"nz"),
    ];
    for (i, (speed, mpg, usage, mfr, model, code)) in cases.iter().enumerate() {
        let mut p = CarPayload::empty_tails(i as u64, 2010 + i as u16);
        p.fuel = vec![FuelEntry {
            speed: *speed,
            mpg: *mpg,
            usage,
        }];
        p.manufacturer = mfr;
        p.model_name = model;
        p.activation_code = code;
        dual_encode(&format!("single_fuel_case_{i}"), &p);
    }
}

// ── Enum / bitset / boost matrices ────────────────────────────────────────

#[test]
fn model_enum_all_variants_dual_encode() {
    for (model, label) in [(0u8, "A"), (1, "B"), (2, "C")] {
        let mut p = CarPayload::empty_tails(10 + model as u64, 2015);
        p.model = model;
        dual_encode(&format!("model_{label}"), &p);
    }
}

#[test]
fn available_boolean_both_values() {
    for available in [false, true] {
        let mut p = CarPayload::empty_tails(20, 2016);
        p.available = available;
        dual_encode(&format!("available_{available}"), &p);
    }
}

#[test]
fn boost_type_all_variants_dual_encode() {
    for (boost, label) in [
        (0u8, "TURBO"),
        (1, "SUPERCHARGER"),
        (2, "NITROUS"),
        (3, "KERS"),
    ] {
        let mut p = CarPayload::empty_tails(30 + boost as u64, 2017);
        p.boost = boost;
        p.booster_enabled = true;
        p.horse_power = 10 + boost;
        dual_encode(&format!("boost_{label}"), &p);
    }
}

#[test]
fn optional_extras_all_8_bit_combinations() {
    for bits in 0u8..8 {
        let mut p = CarPayload::empty_tails(40 + bits as u64, 2018);
        p.extras_bits = bits;
        dual_encode(&format!("extras_bits_{bits:03b}"), &p);
    }
}

// ── Nested group shape matrix ─────────────────────────────────────────────

#[test]
fn fuel_count_matrix_dual_encode() {
    for n in [0u16, 1, 2, 3, 5, 8] {
        let mut p = CarPayload::empty_tails(100 + n as u64, 2021);
        p.fuel = (0..n)
            .map(|i| FuelEntry {
                speed: 20 + i * 10,
                mpg: 30.0 + i as f32,
                usage: match i % 3 {
                    0 => b"Urban",
                    1 => b"",
                    _ => b"Highway Cycle",
                },
            })
            .collect();
        dual_encode(&format!("fuel_count_{n}"), &p);
    }
}

#[test]
fn performance_nesting_matrix_dual_encode() {
    // (perf_count, accel_per_entry) — covers empty nest, single, multi
    let shapes: &[(usize, usize)] = &[
        (0, 0),
        (1, 0),
        (1, 1),
        (1, 3),
        (2, 0),
        (2, 1),
        (2, 3),
        (3, 2),
        (4, 1),
    ];
    for &(n_perf, n_acc) in shapes {
        let mut p = CarPayload::empty_tails(200 + n_perf as u64 * 10 + n_acc as u64, 2022);
        p.perf = (0..n_perf)
            .map(|i| PerfEntry {
                octane: 90 + i as u8,
                accel: (0..n_acc)
                    .map(|j| AccelEntry {
                        mph: 30 + j as u16 * 30,
                        seconds: 3.0 + j as f32 * 0.5 + i as f32 * 0.01,
                    })
                    .collect(),
            })
            .collect();
        dual_encode(&format!("perf_{n_perf}_acc_{n_acc}"), &p);
    }
}

#[test]
fn fuel_and_performance_combined_shapes() {
    let shapes: &[(usize, usize, usize)] = &[
        (0, 0, 0),
        (1, 0, 0),
        (0, 1, 0),
        (0, 1, 2),
        (2, 2, 3),
        (3, 1, 1),
        (5, 2, 0),
    ];
    for &(nf, np, na) in shapes {
        let mut p =
            CarPayload::empty_tails(300 + nf as u64 * 100 + np as u64 * 10 + na as u64, 2023);
        p.fuel = (0..nf)
            .map(|i| FuelEntry {
                speed: 10 + i as u16,
                mpg: 20.5 + i as f32,
                usage: b"u",
            })
            .collect();
        p.perf = (0..np)
            .map(|i| PerfEntry {
                octane: 91 + i as u8,
                accel: (0..na)
                    .map(|j| AccelEntry {
                        mph: 40 + j as u16,
                        seconds: 5.0 + j as f32,
                    })
                    .collect(),
            })
            .collect();
        p.manufacturer = b"Mfr";
        p.model_name = b"Mdl";
        p.activation_code = b"AC";
        dual_encode(&format!("combo_f{nf}_p{np}_a{na}"), &p);
    }
}

// ── Engine / fixed-field edges ────────────────────────────────────────────

#[test]
fn engine_field_extremes_dual_encode() {
    let cases = [
        (0u16, 0u8, [0u8; 3], 0i8, false, 0u8),
        (65534, 255, *b"ZZZ", -128, true, 255),
        (1, 1, *b"abc", 127, true, 1),
        (2000, 4, [49, 50, 51], 35, true, 200),
    ];
    for (i, (cap, cyl, mcode, eff, en, hp)) in cases.iter().enumerate() {
        let mut p = CarPayload::empty_tails(400 + i as u64, 2024);
        p.engine_capacity = *cap;
        p.engine_cylinders = *cyl;
        p.manufacturer_code = *mcode;
        p.efficiency = *eff;
        p.booster_enabled = *en;
        p.horse_power = *hp;
        dual_encode(&format!("engine_extreme_{i}"), &p);
    }
}

#[test]
fn some_numbers_and_vehicle_code_patterns() {
    let number_cases = [
        [0u32; 4],
        [1, 2, 3, 4],
        [u32::MAX - 1; 4],
        [0, u32::MAX - 1, 1, 2],
    ];
    let vehicle_cases = [[0u8; 6], *b"abcdef", *b"XYZXYZ", *b"~~~~~~", *b"      "];
    for (i, nums) in number_cases.iter().enumerate() {
        for (j, vc) in vehicle_cases.iter().enumerate() {
            let mut p = CarPayload::empty_tails(500 + i as u64 * 10 + j as u64, 2025);
            p.some_numbers = *nums;
            p.vehicle_code = *vc;
            dual_encode(&format!("nums_{i}_vc_{j}"), &p);
        }
    }
}

// ── Var-data length edges ─────────────────────────────────────────────────

#[test]
fn var_data_length_matrix_dual_encode() {
    // Keep payloads modest so the test stays fast, but cover empty/short/medium
    // and independent length of the three var-data fields.
    let samples: &[&[u8]] = &[
        b"",
        b"a",
        b"ab",
        b"Honda",
        b"Civic VTi",
        b"abcdef",
        b"0123456789ABCDEF",
        b"long-ish manufacturer or model or code payload!!!",
    ];
    for (i, mfr) in samples.iter().enumerate() {
        for (j, model) in samples.iter().enumerate().take(4) {
            for (k, code) in samples.iter().enumerate().take(4) {
                let mut p =
                    CarPayload::empty_tails(600 + i as u64 * 100 + j as u64 * 10 + k as u64, 2026);
                p.manufacturer = mfr;
                p.model_name = model;
                p.activation_code = code;
                dual_encode(&format!("vardata_m{i}_d{j}_c{k}"), &p);
            }
        }
    }
}

// ── Cross-decode both directions ──────────────────────────────────────────

#[test]
fn ergo_decodes_sbe_tool_full_car() {
    let p = CarPayload::baseline();
    let mut tool_buf = vec![0u8; 1024];
    let tl = p.encode_tool(&mut tool_buf);
    assert_ergo_decodes_payload(&tool_buf[..tl], &p);
}

#[test]
fn sbe_tool_decodes_ergo_full_car() {
    let p = CarPayload::baseline();
    let mut ergo_buf = vec![0u8; 1024];
    let el = p.encode_ergo(&mut ergo_buf);
    assert_tool_decodes_payload(&ergo_buf[..el], &p);
}

#[test]
fn cross_decode_sparse_shapes_both_directions() {
    let shapes = [
        CarPayload::empty_tails(1, 2000),
        {
            let mut p = CarPayload::empty_tails(2, 2001);
            p.fuel = vec![FuelEntry {
                speed: 10,
                mpg: 11.0,
                usage: b"u",
            }];
            p
        },
        {
            let mut p = CarPayload::empty_tails(3, 2002);
            p.perf = vec![PerfEntry {
                octane: 95,
                accel: vec![
                    AccelEntry {
                        mph: 30,
                        seconds: 4.0,
                    },
                    AccelEntry {
                        mph: 60,
                        seconds: 8.0,
                    },
                ],
            }];
            p.manufacturer = b"X";
            p.model_name = b"Y";
            p.activation_code = b"Z";
            p
        },
        {
            let mut p = CarPayload::baseline();
            p.serial = 9999;
            p
        },
    ];
    for (i, p) in shapes.iter().enumerate() {
        let mut ergo_buf = vec![0u8; 2048];
        let mut tool_buf = vec![0u8; 2048];
        let el = p.encode_ergo(&mut ergo_buf);
        let tl = p.encode_tool(&mut tool_buf);
        assert_frames_eq(
            &format!("cross_shape_{i}_bytes"),
            &ergo_buf[..el],
            &tool_buf[..tl],
        );
        assert_ergo_decodes_payload(&tool_buf[..tl], p);
        assert_tool_decodes_payload(&ergo_buf[..el], p);
    }
}

#[test]
fn sbe_tool_bytes_roundtrip_via_ergo_reencode() {
    let p = CarPayload::baseline();
    let mut tool_buf = vec![0u8; 1024];
    let tl = p.encode_tool(&mut tool_buf);
    let tool = tool_buf[..tl].to_vec();
    let mut ergo_buf = vec![0u8; 1024];
    let el = p.encode_ergo(&mut ergo_buf);
    assert_frames_eq("reencode_full", &ergo_buf[..el], &tool);
}

// ── Java fixture + constants ──────────────────────────────────────────────

#[test]
fn fixed_body_prefix_matches_java_fixture() {
    let java = include_bytes!("fixtures/car_example_baseline_data.sbe");
    let p = CarPayload::baseline();
    let mut ergo_buf = vec![0u8; 1024];
    let el = p.encode_ergo(&mut ergo_buf);
    let mut tool_buf = vec![0u8; 1024];
    let tl = p.encode_tool(&mut tool_buf);
    let n = 8 + 45;
    assert_eq!(&ergo_buf[..n], &java[..n], "ergo fixed prefix vs java");
    assert_eq!(&tool_buf[..n], &java[..n], "tool fixed prefix vs java");
    assert_eq!(&ergo_buf[..el], &tool_buf[..tl], "full frame ergo vs tool");
    assert_eq!(&ergo_buf[..el], &java[..], "full frame vs java fixture");
}

#[test]
fn schema_ids_align() {
    assert_eq!(SBE_SCHEMA_ID, 1);
    assert_eq!(SBE_TEMPLATE_ID, 1);
    assert_eq!(SBE_BLOCK_LENGTH, 45);
    assert_eq!(ErgoEnc::SCHEMA_ID, SBE_SCHEMA_ID);
    assert_eq!(ErgoEnc::TEMPLATE_ID, SBE_TEMPLATE_ID);
    assert_eq!(ErgoEnc::BLOCK_LENGTH as u16, SBE_BLOCK_LENGTH);
    assert_eq!(ErgoEnc::SCHEMA_VERSION, SBE_SCHEMA_VERSION);
}

#[test]
fn dual_encode_matches_java_and_cross_decodes() {
    // End-to-end: both encoders == Java fixture, and each decoder accepts both.
    let p = CarPayload::baseline();
    let java = include_bytes!("fixtures/car_example_baseline_data.sbe");
    let frame = dual_encode("e2e_full", &p);
    assert_eq!(frame.as_slice(), &java[..]);
    assert_ergo_decodes_payload(java, &p);
    assert_tool_decodes_payload(java, &p);
    assert_ergo_decodes_payload(&frame, &p);
    assert_tool_decodes_payload(&frame, &p);
}

// ── Stress: many small dual-encodes in one test ───────────────────────────

#[test]
fn stress_many_randomish_payloads_dual_encode() {
    // Deterministic pseudo-random mix of shapes — catches accidental padding /
    // limit bugs without a proptest dependency.
    let mut n_ok = 0u32;
    for seed in 0u64..64 {
        let mut p = CarPayload::empty_tails(
            seed.wrapping_mul(0x9E37_79B9_7F4A_7C15),
            (2000 + (seed % 50)) as u16,
        );
        p.available = seed % 2 == 0;
        p.model = (seed % 3) as u8;
        p.extras_bits = (seed % 8) as u8;
        p.boost = (seed % 4) as u8;
        p.booster_enabled = seed % 3 != 0;
        p.horse_power = (seed % 250) as u8;
        p.engine_capacity = (seed % 6000) as u16;
        p.engine_cylinders = (seed % 12) as u8;
        p.efficiency = ((seed % 200) as i16 - 100) as i8;
        p.manufacturer_code = [
            b'A' + (seed % 26) as u8,
            b'A' + ((seed / 3) % 26) as u8,
            b'A' + ((seed / 7) % 26) as u8,
        ];
        p.some_numbers = [
            seed as u32,
            (seed >> 8) as u32,
            (seed.wrapping_mul(3)) as u32,
            (seed.wrapping_mul(7)) as u32,
        ];
        for k in 0..6 {
            p.vehicle_code[k] = b'a' + ((seed as usize + k) % 26) as u8;
        }
        let nf = (seed % 4) as usize;
        p.fuel = (0..nf)
            .map(|i| FuelEntry {
                speed: (seed as u16).wrapping_add(i as u16 * 11),
                mpg: (seed as f32) * 0.01 + i as f32,
                usage: if i % 2 == 0 { b"u" } else { b"" },
            })
            .collect();
        let np = (seed % 3) as usize;
        let na = (seed % 3) as usize;
        p.perf = (0..np)
            .map(|i| PerfEntry {
                octane: 90 + (i as u8),
                accel: (0..na)
                    .map(|j| AccelEntry {
                        mph: 20 + j as u16 * 20,
                        seconds: 2.0 + j as f32 + i as f32 * 0.1,
                    })
                    .collect(),
            })
            .collect();
        p.manufacturer = match seed % 5 {
            0 => b"",
            1 => b"H",
            2 => b"Honda",
            3 => b"VeryLongManufacturerName",
            _ => b"M",
        };
        p.model_name = match seed % 4 {
            0 => b"",
            1 => b"X",
            2 => b"Civic VTi",
            _ => b"Model",
        };
        p.activation_code = match seed % 3 {
            0 => b"",
            1 => b"ab",
            _ => b"abcdef",
        };
        dual_encode(&format!("stress_seed_{seed}"), &p);
        n_ok += 1;
    }
    assert_eq!(n_ok, 64);
}
