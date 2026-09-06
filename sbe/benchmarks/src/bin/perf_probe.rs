//! `perf-probe` — stable-symbol wrappers for mechanism-level measurement.
//!
//! Criterion answers "how long did it take on this machine, today". It cannot
//! answer "did the call disappear" or "how many instructions did each arm
//! actually retire". This binary exists for the second question: each probe is
//! a named, `#[inline(never)]`, unmangled function that performs exactly
//! [`OPERATIONS`] logical operations and returns an observed checksum.
//!
//! `scripts/run-sbe-instruction-probes.sh` drives it under raw Callgrind with
//! `--toggle-collect=<symbol>`, so setup and validation — which happen in
//! `main`, before any probe is entered — are never counted.
//!
//! Every probe is registered in [`PROBES`]. The registry is the manifest:
//! `--list` prints it, the driver script compares that against the checked-in
//! `probes.tsv`, and an unknown, duplicate, or unregistered probe name fails
//! closed rather than silently measuring nothing.
//!
//! Adding a probe: append a [`Probe`] entry under the relevant topic,
//! add the matching wrapper, and regenerate `probes.tsv` with `--list`.

#![allow(unsafe_code)]
#![allow(
    missing_docs,
    unused_variables,
    unused_imports,
    dead_code,
    unused_mut,
    unused_must_use
)]
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

use std::hint::black_box;

use ergo_sbe_benchmarks::{ergo_car::*, sbe_tool_car_body_decoder};

/// Java-produced Car message (schema v0, template 1) — the same fixture the
/// Criterion parity benchmarks decode.
const BASELINE: &[u8] = include_bytes!("../../benches/fixtures/car_example_baseline_data.sbe");

/// Logical operations performed inside every probe. Fixed across probes so
/// normalised instruction counts are directly comparable between arms.
const OPERATIONS: usize = 10_000;

/// Which codec a probe measures. Both arms of a pair must exist, or the
/// normalised comparison is meaningless.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Arm {
    Ergon,
    SbeTool,
}

impl Arm {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ergon => "ergon",
            Self::SbeTool => "sbe-tool",
        }
    }
}

struct Probe {
    symbol: &'static str,
    arm: Arm,
    /// Maintained benchmark pair this probe corresponds to.
    pair: &'static str,
    /// Coarse grouping for selecting a related set. `--topic` selects on this.
    topic: &'static str,
    run: fn() -> u64,
}

/// Registered probes: ergon and sbe-tool for each maintained pair.
/// New probes are appended here under an existing or new topic.
const PROBES: &[Probe] = &[
    Probe {
        symbol: "ergo_probe_decode_entry_point",
        arm: Arm::Ergon,
        pair: "decode_entry_point",
        topic: "decode",
        run: run_ergo_decode_entry_point,
    },
    Probe {
        symbol: "tool_probe_decode_entry_point",
        arm: Arm::SbeTool,
        pair: "decode_entry_point",
        topic: "decode",
        run: run_tool_decode_entry_point,
    },
    Probe {
        symbol: "ergo_probe_decode_composite",
        arm: Arm::Ergon,
        pair: "decode_composite",
        topic: "decode",
        run: run_ergo_decode_composite,
    },
    Probe {
        symbol: "tool_probe_decode_composite",
        arm: Arm::SbeTool,
        pair: "decode_composite",
        topic: "decode",
        run: run_tool_decode_composite,
    },
    Probe {
        symbol: "ergo_probe_decode_full_message",
        arm: Arm::Ergon,
        pair: "decode_full_message",
        topic: "decode",
        run: run_ergo_decode_full_message,
    },
    Probe {
        symbol: "tool_probe_decode_full_message",
        arm: Arm::SbeTool,
        pair: "decode_full_message",
        topic: "decode",
        run: run_tool_decode_full_message,
    },
    Probe {
        symbol: "ergo_probe_decode_full_message_ordered",
        arm: Arm::Ergon,
        pair: "decode_full_message_ordered",
        topic: "decode",
        run: run_ergo_decode_full_message_ordered,
    },
    Probe {
        symbol: "tool_probe_decode_full_message_ordered",
        arm: Arm::SbeTool,
        pair: "decode_full_message_ordered",
        topic: "decode",
        run: run_tool_decode_full_message_ordered,
    },
    Probe {
        symbol: "ergo_probe_decode_full_message_mutable_ordered",
        arm: Arm::Ergon,
        pair: "decode_full_message_mutable_ordered",
        topic: "decode",
        run: run_ergo_decode_full_message_mutable_ordered,
    },
    Probe {
        symbol: "tool_probe_decode_full_message_mutable_ordered",
        arm: Arm::SbeTool,
        pair: "decode_full_message_mutable_ordered",
        topic: "decode",
        run: run_tool_decode_full_message_mutable_ordered,
    },
    Probe {
        symbol: "ergo_probe_optional_enum_nullify",
        arm: Arm::Ergon,
        pair: "extended_optional_enum_nullify",
        topic: "extended",
        run: run_ergo_optional_enum_nullify,
    },
    Probe {
        symbol: "tool_probe_optional_enum_nullify",
        arm: Arm::SbeTool,
        pair: "extended_optional_enum_nullify",
        topic: "extended",
        run: run_tool_optional_enum_nullify,
    },
    Probe {
        symbol: "ergo_probe_group_with_data",
        arm: Arm::Ergon,
        pair: "extended_group_with_data",
        topic: "extended",
        run: run_ergo_group_with_data,
    },
    Probe {
        symbol: "tool_probe_group_with_data",
        arm: Arm::SbeTool,
        pair: "extended_group_with_data",
        topic: "extended",
        run: run_tool_group_with_data,
    },
    Probe {
        symbol: "ergo_probe_encode_scalar_header_and_body",
        arm: Arm::Ergon,
        pair: "encode_scalar_header_and_body",
        topic: "encode",
        run: run_ergo_encode_scalar_header_and_body,
    },
    Probe {
        symbol: "tool_probe_encode_scalar_header_and_body",
        arm: Arm::SbeTool,
        pair: "encode_scalar_header_and_body",
        topic: "encode",
        run: run_tool_encode_scalar_header_and_body,
    },
    Probe {
        symbol: "ergo_probe_encode_throughput",
        arm: Arm::Ergon,
        pair: "encode_throughput_10k",
        topic: "encode",
        run: run_ergo_encode_throughput,
    },
    Probe {
        symbol: "tool_probe_encode_throughput",
        arm: Arm::SbeTool,
        pair: "encode_throughput_10k",
        topic: "encode",
        run: run_tool_encode_throughput,
    },
    Probe {
        symbol: "ergo_probe_wire_parity_encode_full",
        arm: Arm::Ergon,
        pair: "wire_parity_encode_full",
        topic: "encode",
        run: run_ergo_wire_parity_encode_full,
    },
    Probe {
        symbol: "tool_probe_wire_parity_encode_full",
        arm: Arm::SbeTool,
        pair: "wire_parity_encode_full",
        topic: "encode",
        run: run_tool_wire_parity_encode_full,
    },
    // ── Candidate-5 symmetry probes ───────────────────────────────────────
    Probe {
        symbol: "ergo_probe_encode_composite",
        arm: Arm::Ergon,
        pair: "encode_composite",
        topic: "symmetry",
        run: run_ergo_encode_composite,
    },
    Probe {
        symbol: "ergo_probe_encode_group_entry",
        arm: Arm::Ergon,
        pair: "encode_group_entry",
        topic: "symmetry",
        run: run_ergo_encode_group_entry,
    },
    Probe {
        symbol: "ergo_probe_decode_vardata",
        arm: Arm::Ergon,
        pair: "decode_vardata",
        topic: "symmetry",
        run: run_ergo_decode_vardata,
    },
];

// ─── Setup, performed once in main, never inside a probe ───────────────────

fn sbe_tool_header_fields() -> (u16, u16) {
    (
        u16::from_le_bytes(BASELINE[0..2].try_into().expect("baseline header")),
        u16::from_le_bytes(BASELINE[6..8].try_into().expect("baseline header")),
    )
}

fn ergo_header_fields() -> (usize, u16) {
    let header = MessageHeader(read_bytes::<8>(BASELINE, 0));
    (header.block_length() as usize, header.version())
}

/// Extent proof for the trusted constructors used inside the probes. This is
/// the untimed preflight both arms share; sbe-tool's `wrap` performs no
/// equivalent check, so neither arm pays for it on the measured path.
fn assert_baseline_extent() {
    let (bl_e, ver_e) = ergo_header_fields();
    assert!(
        BASELINE.len() >= CarDecoder::HEADER_LENGTH + bl_e,
        "baseline fixture is shorter than its own declared block length"
    );
    // A checked wrap must succeed on this buffer, which is exactly the
    // precondition the unchecked constructor requires.
    let checked = CarDecoder::try_from(BASELINE).expect("baseline must decode");
    assert_eq!(checked.serial_number(), 1234);
}

// ─── Probes: decode entry point ────────────────────────────────────────────

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_decode_entry_point(buf: &[u8], block_length: usize, version: u16) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        // SAFETY: extent proven once by `assert_baseline_extent`.
        let car = unsafe { CarDecoder::wrap_unchecked(black_box(buf), 0, block_length, version) };
        checksum = checksum.wrapping_add(car.serial_number());
    }
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_decode_entry_point(buf: &[u8], block_length: u16, version: u16) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        let car = sbe_tool_car_body_decoder(black_box(buf), 0, block_length, version);
        checksum = checksum.wrapping_add(car.serial_number());
    }
    black_box(checksum)
}

fn run_ergo_decode_entry_point() -> u64 {
    let (bl, ver) = ergo_header_fields();
    ergo_probe_decode_entry_point(BASELINE, bl, ver)
}

fn run_tool_decode_entry_point() -> u64 {
    let (bl, ver) = sbe_tool_header_fields();
    tool_probe_decode_entry_point(BASELINE, bl, ver)
}

// ─── Probes: composite decode ──────────────────────────────────────────────

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_decode_composite(buf: &[u8], block_length: usize, version: u16) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        // SAFETY: extent proven once by `assert_baseline_extent`.
        let car = unsafe { CarDecoder::wrap_unchecked(black_box(buf), 0, block_length, version) };
        let engine = car.engine();
        checksum = checksum
            .wrapping_add(u64::from(engine.capacity()))
            .wrapping_add(u64::from(engine.num_cylinders()));
    }
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_decode_composite(buf: &[u8], block_length: u16, version: u16) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        let engine =
            sbe_tool_car_body_decoder(black_box(buf), 0, block_length, version).engine_decoder();
        checksum = checksum
            .wrapping_add(u64::from(engine.capacity()))
            .wrapping_add(u64::from(engine.num_cylinders()));
    }
    black_box(checksum)
}

fn run_ergo_decode_composite() -> u64 {
    let (bl, ver) = ergo_header_fields();
    ergo_probe_decode_composite(BASELINE, bl, ver)
}

fn run_tool_decode_composite() -> u64 {
    let (bl, ver) = sbe_tool_header_fields();
    tool_probe_decode_composite(BASELINE, bl, ver)
}

// ─── Probes: full-message decode ───────────────────────────────────────────

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_decode_full_message(buf: &[u8], block_length: usize, version: u16) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        // SAFETY: extent proven once by `assert_baseline_extent`.
        let car = unsafe { CarDecoder::wrap_unchecked(black_box(buf), 0, block_length, version) };
        checksum = checksum
            .wrapping_add(car.serial_number())
            .wrapping_add(u64::from(car.model_year()));
        let engine = car.engine();
        checksum = checksum.wrapping_add(u64::from(engine.capacity()));
        let mut fuel = car.into_fuel_figures().expect("fuel figures");
        while let Some(Ok(entry)) = fuel.next() {
            checksum = checksum.wrapping_add(u64::from(entry.speed()));
            checksum =
                checksum.wrapping_add(entry.usage_description().expect("usage").len() as u64);
        }
        let after_fuel = fuel.finish().expect("fuel finish");
        let mut perf = after_fuel
            .into_performance_figures()
            .expect("performance figures");
        while let Some(Ok(entry)) = perf.next() {
            checksum = checksum.wrapping_add(u64::from(entry.octane_rating()));
            for acceleration in entry.acceleration().expect("acceleration") {
                checksum = checksum.wrapping_add(u64::from(acceleration.mph()));
            }
        }
        let after_perf = perf.finish().expect("performance finish");
        let (manufacturer, next) = after_perf.into_manufacturer().expect("manufacturer");
        let (model, next) = next.into_model().expect("model");
        let (code, _) = next.into_activation_code().expect("activation code");
        checksum = checksum
            .wrapping_add(manufacturer.len() as u64)
            .wrapping_add(model.len() as u64)
            .wrapping_add(code.len() as u64);
    }
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_decode_full_message(buf: &[u8], block_length: u16, version: u16) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        let car = sbe_tool_car_body_decoder(black_box(buf), 0, block_length, version);
        checksum = checksum
            .wrapping_add(car.serial_number())
            .wrapping_add(u64::from(car.model_year()));
        let mut engine = car.engine_decoder();
        checksum = checksum.wrapping_add(u64::from(engine.capacity()));
        let mut car = engine.parent().expect("engine parent");
        let mut fuel = car.fuel_figures_decoder();
        while fuel.advance().expect("fuel advance").is_some() {
            checksum = checksum.wrapping_add(u64::from(fuel.speed()));
            let coordinate = fuel.usage_description_decoder();
            checksum = checksum.wrapping_add(fuel.usage_description_slice(coordinate).len() as u64);
        }
        car = fuel.parent().expect("fuel parent");
        let mut perf = car.performance_figures_decoder();
        while perf.advance().expect("perf advance").is_some() {
            checksum = checksum.wrapping_add(u64::from(perf.octane_rating()));
            let mut acceleration = perf.acceleration_decoder();
            while acceleration
                .advance()
                .expect("acceleration advance")
                .is_some()
            {
                checksum = checksum.wrapping_add(u64::from(acceleration.mph()));
            }
            perf = acceleration.parent().expect("acceleration parent");
        }
        car = perf.parent().expect("perf parent");
        let manufacturer = car.manufacturer_decoder();
        let manufacturer_len = car.manufacturer_slice(manufacturer).len() as u64;
        let model = car.model_decoder();
        let model_len = car.model_slice(model).len() as u64;
        let code = car.activation_code_decoder();
        let code_len = car.activation_code_slice(code).len() as u64;
        checksum = checksum
            .wrapping_add(manufacturer_len)
            .wrapping_add(model_len)
            .wrapping_add(code_len);
    }
    black_box(checksum)
}

fn run_ergo_decode_full_message() -> u64 {
    let (bl, ver) = ergo_header_fields();
    ergo_probe_decode_full_message(BASELINE, bl, ver)
}

fn run_tool_decode_full_message() -> u64 {
    let (bl, ver) = sbe_tool_header_fields();
    tool_probe_decode_full_message(BASELINE, bl, ver)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_decode_full_message_ordered(
    buf: &[u8],
    block_length: usize,
    version: u16,
) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        // SAFETY: extent proven once by `assert_baseline_extent`.
        let car = unsafe { CarDecoder::wrap_unchecked(black_box(buf), 0, block_length, version) };
        checksum = checksum
            .wrapping_add(car.serial_number())
            .wrapping_add(u64::from(car.model_year()));
        let engine = car.engine();
        checksum = checksum.wrapping_add(u64::from(engine.capacity()));
        let after_fuel = car
            .into_fuel_figures()
            .expect("fuel figures")
            .visit_entries(
                |entry| -> Result<_, ergo_sbe_benchmarks::ergo_car::sbe_rt::DecodeError> {
                    checksum = checksum.wrapping_add(u64::from(entry.speed()));
                    let (usage, complete) = entry.into_usage_description()?;
                    checksum = checksum.wrapping_add(usage.len() as u64);
                    Ok(complete)
                },
            )
            .expect("fuel visit");
        let after_perf = after_fuel
            .into_performance_figures()
            .expect("performance figures")
            .visit_entries(|entry| -> Result<_, ergo_sbe_benchmarks::ergo_car::sbe_rt::DecodeError> {
                checksum = checksum.wrapping_add(u64::from(entry.octane_rating()));
                entry.into_acceleration()?.visit_entries(
                    |acceleration| -> Result<(), ergo_sbe_benchmarks::ergo_car::sbe_rt::DecodeError> {
                        checksum = checksum.wrapping_add(u64::from(acceleration.mph()));
                        Ok(())
                    },
                )
            })
            .expect("perf visit");
        let (manufacturer, next) = after_perf.into_manufacturer().expect("manufacturer");
        let (model, next) = next.into_model().expect("model");
        let (code, _) = next.into_activation_code().expect("activation code");
        checksum = checksum
            .wrapping_add(manufacturer.len() as u64)
            .wrapping_add(model.len() as u64)
            .wrapping_add(code.len() as u64);
    }
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_decode_full_message_ordered(buf: &[u8], block_length: u16, version: u16) -> u64 {
    tool_probe_decode_full_message(buf, block_length, version)
}

fn run_ergo_decode_full_message_ordered() -> u64 {
    let (bl, ver) = ergo_header_fields();
    ergo_probe_decode_full_message_ordered(BASELINE, bl, ver)
}

fn run_tool_decode_full_message_ordered() -> u64 {
    let (bl, ver) = sbe_tool_header_fields();
    tool_probe_decode_full_message_ordered(BASELINE, bl, ver)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_decode_full_message_mutable_ordered(
    buf: &[u8],
    block_length: usize,
    version: u16,
) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        // SAFETY: extent proven once by `assert_baseline_extent`.
        let mut car =
            unsafe { CarDecoder::wrap_unchecked(black_box(buf), 0, block_length, version) }
                .ordered();
        checksum = checksum
            .wrapping_add(car.serial_number())
            .wrapping_add(u64::from(car.model_year()));
        let engine = car.engine();
        checksum = checksum.wrapping_add(u64::from(engine.capacity()));
        car.fuel_figures()
            .expect("fuel figures")
            .visit_entries(
                |entry| -> Result<(), ergo_sbe_benchmarks::ergo_car::sbe_rt::DecodeError> {
                    checksum = checksum.wrapping_add(u64::from(entry.speed()));
                    checksum = checksum.wrapping_add(entry.usage_description()?.len() as u64);
                    Ok(())
                },
            )
            .expect("fuel visit");
        car.performance_figures()
            .expect("performance figures")
            .visit_entries(
                |entry| -> Result<(), ergo_sbe_benchmarks::ergo_car::sbe_rt::DecodeError> {
                    checksum = checksum.wrapping_add(u64::from(entry.octane_rating()));
                    entry.acceleration()?.visit_entries(
                        |acceleration| -> Result<(), ergo_sbe_benchmarks::ergo_car::sbe_rt::DecodeError> {
                            checksum = checksum.wrapping_add(u64::from(acceleration.mph()));
                            Ok(())
                        },
                    )
                },
            )
            .expect("perf visit");
        let manufacturer = car.manufacturer().expect("manufacturer");
        let model = car.model().expect("model");
        let code = car.activation_code().expect("activation code");
        checksum = checksum
            .wrapping_add(manufacturer.len() as u64)
            .wrapping_add(model.len() as u64)
            .wrapping_add(code.len() as u64);
    }
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_decode_full_message_mutable_ordered(
    buf: &[u8],
    block_length: u16,
    version: u16,
) -> u64 {
    tool_probe_decode_full_message(buf, block_length, version)
}

fn run_ergo_decode_full_message_mutable_ordered() -> u64 {
    let (bl, ver) = ergo_header_fields();
    ergo_probe_decode_full_message_mutable_ordered(BASELINE, bl, ver)
}

fn run_tool_decode_full_message_mutable_ordered() -> u64 {
    let (bl, ver) = sbe_tool_header_fields();
    tool_probe_decode_full_message_mutable_ordered(BASELINE, bl, ver)
}

// ─── Probes: scalar encode (header + body) ─────────────────────────────────

const SCALAR_FRAME: usize = 8 + 45;

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_encode_scalar_header_and_body(buf: &mut [u8]) -> u64 {
    let mut checksum = 0_u64;
    for i in 0..OPERATIONS {
        // SAFETY: caller supplies a buffer of at least SCALAR_FRAME bytes,
        // asserted in main before this probe is entered.
        unsafe { CarEncoder::wrap_and_apply_header_unchecked(black_box(&mut *buf), 0) }
            .serial_number(black_box(i as u64))
            .model_year(black_box(2013));
        checksum = checksum.wrapping_add(u64::from(buf[8]));
    }
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_encode_scalar_header_and_body(buf: &mut [u8]) -> u64 {
    use ergo_sbe_benchmarks::sbe_tool_car::sbe_tool::{
        WriteBuf, car_codec::encoder::CarEncoder as ToolCarEncoder,
    };
    let mut checksum = 0_u64;
    for i in 0..OPERATIONS {
        ToolCarEncoder::default()
            .wrap(WriteBuf::new(black_box(&mut *buf)), 8)
            .header(0)
            .parent()
            .expect("tool header parent")
            .serial_number(black_box(i as u64))
            .model_year(black_box(2013));
        checksum = checksum.wrapping_add(u64::from(buf[8]));
    }
    black_box(checksum)
}

fn run_ergo_encode_scalar_header_and_body() -> u64 {
    let mut buf = [0_u8; SCALAR_FRAME];
    ergo_probe_encode_scalar_header_and_body(&mut buf)
}

fn run_tool_encode_scalar_header_and_body() -> u64 {
    let mut buf = [0_u8; SCALAR_FRAME];
    tool_probe_encode_scalar_header_and_body(&mut buf)
}

// ─── Probes: encode throughput ─────────────────────────────────────────────

const SLOT: usize = 64;

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_encode_throughput(buf: &mut [u8]) -> u64 {
    let mut checksum = 0_u64;
    for i in 0..OPERATIONS {
        let off = i * SLOT;
        // SAFETY: `buf` is OPERATIONS * SLOT bytes, asserted in main.
        unsafe {
            CarEncoder::wrap_and_apply_header_unchecked(black_box(&mut buf[off..off + SLOT]), 0)
        }
        .serial_number(black_box(i as u64))
        .model_year(black_box(2013));
        checksum = checksum.wrapping_add(u64::from(buf[off + 8]));
    }
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_encode_throughput(buf: &mut [u8]) -> u64 {
    use ergo_sbe_benchmarks::sbe_tool_car::sbe_tool::{
        WriteBuf, car_codec::encoder::CarEncoder as ToolCarEncoder,
    };
    let mut checksum = 0_u64;
    for i in 0..OPERATIONS {
        let off = i * SLOT;
        ToolCarEncoder::default()
            .wrap(WriteBuf::new(black_box(&mut buf[off..off + SLOT])), 8)
            .header(0)
            .parent()
            .expect("tool header parent")
            .serial_number(black_box(i as u64))
            .model_year(black_box(2013));
        checksum = checksum.wrapping_add(u64::from(buf[off + 8]));
    }
    black_box(checksum)
}

fn run_ergo_encode_throughput() -> u64 {
    let mut buf = vec![0_u8; OPERATIONS * SLOT];
    ergo_probe_encode_throughput(&mut buf)
}

fn run_tool_encode_throughput() -> u64 {
    let mut buf = vec![0_u8; OPERATIONS * SLOT];
    tool_probe_encode_throughput(&mut buf)
}

// ─── Probes: full-message encode (wire parity shape) ───────────────────────

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_wire_parity_encode_full(buf: &mut [u8]) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        // SAFETY: `buf` is 512 bytes, asserted in main.
        let len = unsafe { CarEncoder::wrap_and_apply_header_unchecked(black_box(&mut *buf), 0) }
            .fixed(&CarFixedFields {
                serial_number: 99,
                model_year: 2020,
                available: BooleanType::T,
                code: Model::C,
                some_numbers: [9, 8, 7, 6],
                vehicle_code: *b"XYZXYZ",
                extras: OptionalExtras::default(),
                engine: Engine::new(
                    1600,
                    4,
                    *b"ABC",
                    10,
                    BooleanType::F,
                    Booster::new(BoostType::SUPERCHARGER, 50),
                ),
            })
            .fuel_figures(1, |g| {
                g.add(|mut entry| {
                    entry.speed(40).mpg(33.3);
                    entry.usage_description(b"city")
                })
            })
            .expect("fuel figures")
            .performance_figures(0, |_| Ok(()))
            .expect("performance figures")
            .manufacturer(b"Toyota")
            .expect("manufacturer")
            .model(b"Yaris")
            .expect("model")
            .activation_code(b"zz")
            .expect("activation code")
            .encoded_length_with_header();
        checksum = checksum.wrapping_add(len as u64);
    }
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_wire_parity_encode_full(buf: &mut [u8]) -> u64 {
    use ergo_sbe_benchmarks::sbe_tool_car::sbe_tool::{
        Encoder, WriteBuf,
        boolean_type::BooleanType as ToolBool,
        boost_type::BoostType as ToolBoost,
        car_codec::encoder::{
            CarEncoder as ToolCarEnc, FuelFiguresEncoder as ToolFuel,
            PerformanceFiguresEncoder as ToolPerf,
        },
        model::Model as ToolModel,
        optional_extras::OptionalExtras as ToolExtras,
    };
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        let t = ToolCarEnc::default().wrap(WriteBuf::new(black_box(&mut *buf)), 8);
        let mut h = t.header(0);
        let mut t = h.parent().expect("tool header parent");
        t.serial_number(99)
            .model_year(2020)
            .available(ToolBool::T)
            .code(ToolModel::C)
            .some_numbers(&[9, 8, 7, 6])
            .vehicle_code(b"XYZXYZ")
            .extras(ToolExtras::default());
        let mut eng = t.engine_encoder();
        eng.capacity(1600)
            .num_cylinders(4)
            .manufacturer_code(b"ABC")
            .efficiency(10)
            .booster_enabled(ToolBool::F);
        let mut boost = eng.booster_encoder();
        boost.boost_type(ToolBoost::SUPERCHARGER).horse_power(50);
        eng = boost.parent().expect("booster parent");
        t = eng.parent().expect("engine parent");
        let mut fuel = ToolFuel::default();
        fuel = t.fuel_figures_encoder(1, fuel);
        fuel.advance().expect("fuel advance");
        fuel.speed(40).mpg(33.3);
        fuel.usage_description(b"city");
        t = fuel.parent().expect("fuel parent");
        let mut perf = ToolPerf::default();
        perf = t.performance_figures_encoder(0, perf);
        t = perf.parent().expect("perf parent");
        t.manufacturer("Toyota")
            .model("Yaris")
            .activation_code(b"zz");
        checksum = checksum.wrapping_add(t.get_limit() as u64);
    }
    black_box(checksum)
}

fn run_ergo_wire_parity_encode_full() -> u64 {
    let mut buf = [0_u8; 512];
    ergo_probe_wire_parity_encode_full(&mut buf)
}

fn run_tool_wire_parity_encode_full() -> u64 {
    let mut buf = [0_u8; 512];
    tool_probe_wire_parity_encode_full(&mut buf)
}

// ─── Candidate-5 probes: encoder/decoder symmetry ──────────────────────────
// These measure the encode direction of operations whose decode direction is
// already covered. Grouped under `topic: "symmetry"` so a future unification
// of mirrored codegen can be checked for instruction-level asymmetry.

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_encode_composite(buf: &mut [u8]) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        // SAFETY: SCALAR_FRAME bytes of stack, extent proven in main.
        unsafe { CarEncoder::wrap_unchecked(black_box(&mut *buf), 0) }
            .engine(Engine([0u8; 10]))
            .serial_number(7);
        // msg_offset=0 → engine writes at buf[43], serial_number at buf[8].
        checksum = checksum
            .wrapping_add(u64::from(buf[8]))
            .wrapping_add(u64::from(buf[43]));
    }
    black_box(checksum)
}

fn run_ergo_encode_composite() -> u64 {
    let mut buf = [0_u8; SCALAR_FRAME];
    ergo_probe_encode_composite(&mut buf)
}

// Group entry write: encode one fuel_figures entry, mirroring what
// decode_full_message already decodes.

const FUEL_ENTRY_BUF: usize = 64;

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_encode_group_entry(buf: &mut [u8]) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        FuelFiguresEncoder::wrap(black_box(&mut buf[..]), 0, 1)
            .add(|mut e| {
                e.speed(30_u16).mpg(35.9_f32);
                e.usage_description(b"urban")
            })
            .expect("flat entry");
        // Group dimension header: blockLength at buf[0..2], numInGroup at buf[2..4],
        // then entry bytes. Checksum the dimension + first entry byte.
        checksum = checksum
            .wrapping_add(u64::from(buf[0]))
            .wrapping_add(u64::from(buf[4]));
    }
    black_box(checksum)
}

fn run_ergo_encode_group_entry() -> u64 {
    let mut buf = [0_u8; FUEL_ENTRY_BUF];
    ergo_probe_encode_group_entry(&mut buf)
}

// Var-data decode: read the manufacturer string through the consuming stage
// chain that matches what encode throughput already writes.

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_decode_vardata(buf: &[u8], block_length: usize, version: u16) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        // SAFETY: extent proven once in assert_baseline_extent.
        let car = unsafe { CarDecoder::wrap_unchecked(black_box(buf), 0, block_length, version) };
        let (mfr, _after) = car
            .into_fuel_figures()
            .expect("fuel figures")
            .finish()
            .expect("fuel finish")
            .into_performance_figures()
            .expect("perf figures")
            .finish()
            .expect("perf finish")
            .into_manufacturer()
            .expect("manufacturer");
        checksum = checksum.wrapping_add(mfr[0] as u64);
    }
    black_box(checksum)
}

fn run_ergo_decode_vardata() -> u64 {
    let (bl, ver) = ergo_header_fields();
    ergo_probe_decode_vardata(BASELINE, bl, ver)
}

// ─── Driver ────────────────────────────────────────────────────────────────

fn print_manifest() {
    println!("symbol\tarm\tpair\ttopic\toperations");
    for probe in PROBES {
        println!(
            "{}\t{}\t{}\t{}\t{OPERATIONS}",
            probe.symbol,
            probe.arm.as_str(),
            probe.pair,
            probe.topic
        );
    }
}

fn encode_optional_enum_fixture() -> ([u8; ergo_sbe_benchmarks::parity_optional_enum_nullify::OptionalEnumNullifyEncoder::ENCODED_LENGTH], usize)
{
    use ergo_sbe_benchmarks::parity_optional_enum_nullify::{
        EnumType, OptionalComposite, OptionalEncodingEnumType, OptionalEnumNullifyEncoder,
        OptionalEnumNullifyFixedFields,
    };
    let mut buf = [0u8; OptionalEnumNullifyEncoder::ENCODED_LENGTH];
    let len = OptionalEnumNullifyEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&OptionalEnumNullifyFixedFields {
            optional_enum: Some(EnumType::One),
            required_enum_from_optional_type: OptionalEncodingEnumType::Alpha,
            optional_composite: OptionalComposite::new(42u16),
        })
        .encoded_length_with_header();
    (buf, len)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_optional_enum_nullify(buf: &[u8], block_length: usize, version: u16) -> u64 {
    use ergo_sbe_benchmarks::parity_optional_enum_nullify::OptionalEnumNullifyDecoder;
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        let dec = unsafe {
            OptionalEnumNullifyDecoder::wrap_unchecked(black_box(buf), 0, block_length, version)
        };
        checksum = checksum.wrapping_add(dec.optional_enum() as u64);
        checksum = checksum.wrapping_add(dec.required_enum_from_optional_type() as u64);
    }
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_optional_enum_nullify(buf: &[u8], block_length: u16, version: u16) -> u64 {
    use sbe_tool_optional_enum_nullify::{
        ReadBuf, optional_enum_nullify_codec::decoder::OptionalEnumNullifyDecoder as StDecoder,
    };
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        let dec = StDecoder::default().wrap(ReadBuf::new(black_box(buf)), 8, block_length, version);
        checksum = checksum.wrapping_add(dec.optional_enum() as u64);
        checksum = checksum.wrapping_add(dec.required_enum_from_optional_type() as u64);
    }
    black_box(checksum)
}

fn run_ergo_optional_enum_nullify() -> u64 {
    use ergo_sbe_benchmarks::parity_optional_enum_nullify::MessageHeader;
    let (buf, len) = encode_optional_enum_fixture();
    let encoded = &buf[..len];
    let header = MessageHeader(read_bytes::<8>(encoded, 0));
    ergo_probe_optional_enum_nullify(encoded, header.block_length() as usize, header.version())
}

fn run_tool_optional_enum_nullify() -> u64 {
    use ergo_sbe_benchmarks::parity_optional_enum_nullify::{
        MessageHeader, OptionalEnumNullifyDecoder,
    };
    let (buf, len) = encode_optional_enum_fixture();
    let encoded = &buf[..len];
    let header = MessageHeader(read_bytes::<8>(encoded, 0));
    tool_probe_optional_enum_nullify(
        encoded,
        OptionalEnumNullifyDecoder::BLOCK_LENGTH as u16,
        header.version(),
    )
}

fn encode_group_with_data_fixture() -> (Vec<u8>, usize) {
    use ergo_sbe_benchmarks::parity_group_with_data::{
        TestMessage1Encoder, TestMessage1FixedFields,
    };
    let var = b"test";
    let len = TestMessage1Encoder::compute_length()
        .entries(1)
        .var_data_field(var.len())
        .unwrap()
        .encoded_length_with_header();
    let mut buf = vec![0u8; len];
    let actual = TestMessage1Encoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&TestMessage1FixedFields { tag1: 42u32 })
        .entries(1, |g| {
            g.add(|mut e| {
                e.tag_group1(*b"ABCDEFGHI").tag_group2(7);
                e.var_data_field(var)
            })?;
            Ok(())
        })
        .unwrap()
        .encoded_length_with_header();
    (buf, actual)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_group_with_data(buf: &[u8], block_length: usize, version: u16) -> u64 {
    use ergo_sbe_benchmarks::parity_group_with_data::TestMessage1Decoder;
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        let dec = unsafe {
            TestMessage1Decoder::wrap_unchecked(black_box(buf), 0, block_length, version)
        };
        checksum = checksum.wrapping_add(u64::from(dec.tag1()));
        let mut entries = dec.into_entries().expect("entries");
        let entry = entries.next().expect("one entry").expect("entry");
        checksum = checksum.wrapping_add(entry.tag_group2() as u64);
        checksum = checksum.wrapping_add(entry.var_data_field().expect("var").len() as u64);
    }
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_group_with_data(buf: &[u8], block_length: u16, version: u16) -> u64 {
    use sbe_tool_group_with_data::{
        ReadBuf, test_message_1_codec::decoder::TestMessage1Decoder as StDecoder,
    };
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        let dec = StDecoder::default().wrap(ReadBuf::new(black_box(buf)), 8, block_length, version);
        checksum = checksum.wrapping_add(u64::from(dec.tag_1()));
        let mut entries = dec.entries_decoder();
        assert!(entries.advance().expect("advance").is_some());
        checksum = checksum.wrapping_add(entries.tag_group_2() as u64);
        let coords = entries.var_data_field_decoder();
        checksum = checksum.wrapping_add(entries.var_data_field_slice(coords).len() as u64);
    }
    black_box(checksum)
}

fn run_ergo_group_with_data() -> u64 {
    use ergo_sbe_benchmarks::parity_group_with_data::MessageHeader;
    let (buf, len) = encode_group_with_data_fixture();
    let encoded = &buf[..len];
    let header = MessageHeader(read_bytes::<8>(encoded, 0));
    ergo_probe_group_with_data(encoded, header.block_length() as usize, header.version())
}

fn run_tool_group_with_data() -> u64 {
    use ergo_sbe_benchmarks::parity_group_with_data::MessageHeader;
    let (buf, len) = encode_group_with_data_fixture();
    let encoded = &buf[..len];
    let header = MessageHeader(read_bytes::<8>(encoded, 0));
    tool_probe_group_with_data(encoded, header.block_length(), header.version())
}

fn usage() -> ! {
    eprintln!(
        "usage: perf-probe --list | --probe <symbol> | --topic <name>\n\
         \n\
         --list          print the probe manifest (symbol, arm, pair, topic, operations)\n\
         --probe SYMBOL  run one registered probe and print its checksum\n\
         --topic NAME    run every probe in a topic"
    );
    std::process::exit(2)
}

/// Reject a registry that cannot support a fair comparison, before any
/// measurement is taken.
fn validate_registry() {
    let mut seen: Vec<&str> = Vec::new();
    for probe in PROBES {
        assert!(
            !seen.contains(&probe.symbol),
            "duplicate probe symbol {}: the driver could not tell the two apart",
            probe.symbol
        );
        seen.push(probe.symbol);
    }
    for probe in PROBES {
        // Symmetry probes measure encode vs decode within the same codec,
        // not ergon vs sbe-tool — they have no opposing arm.
        if probe.topic == "symmetry" {
            continue;
        }
        let counterpart = PROBES
            .iter()
            .filter(|other| other.pair == probe.pair)
            .filter(|other| other.arm != probe.arm)
            .count();
        assert!(
            counterpart == 1,
            "probe {} has no single opposing arm for pair {} — a one-sided \
             probe cannot support a comparison",
            probe.symbol,
            probe.pair
        );
    }
}

fn run(probe: &Probe) {
    let checksum = (probe.run)();
    println!(
        "probe={} arm={} pair={} topic={} operations={OPERATIONS} checksum={checksum}",
        probe.symbol,
        probe.arm.as_str(),
        probe.pair,
        probe.topic
    );
}

fn main() {
    validate_registry();

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--list" => {
            print_manifest();
        }
        [flag, symbol] if flag == "--probe" => {
            // Setup and validation happen here, outside every collected region.
            assert_baseline_extent();
            let Some(probe) = PROBES.iter().find(|p| p.symbol == symbol) else {
                eprintln!(
                    "unknown probe {symbol:?}; registered probes:\n{}",
                    PROBES
                        .iter()
                        .map(|p| format!("  {}", p.symbol))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                std::process::exit(2);
            };
            run(probe);
        }
        [flag, topic] if flag == "--topic" => {
            assert_baseline_extent();
            let selected: Vec<&Probe> = PROBES.iter().filter(|p| p.topic == topic).collect();
            if selected.is_empty() {
                eprintln!("no probes registered for topic {topic:?}");
                std::process::exit(2);
            }
            for probe in selected {
                run(probe);
            }
        }
        _ => usage(),
    }
}
