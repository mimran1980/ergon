//! Performance parity: ergon vs sbe-tool (Java reference) head-to-head.
//!
//! Both codecs generated from the same Car schema, decoding the same
//! Java-produced binary fixture. Every maintained ratio must remain at or below
//! 1.00; a repeatable sbe-tool win is investigated as either a benchmark defect
//! or an ergon regression.
//!
//! Performance benchmarking generated codecs is notoriously difficult and
//! easy to get wrong. Treat surprising ratios as suspected benchmark bugs
//! until exact work, optimizer opacity, batching, LTO/no-LTO behaviour, and
//! optimized instructions have all been reviewed. Please report any mistake.
//!
//! Note: sbe-tool uses a different API pattern (mutable self, parent
//! references, advance()-based group iteration). These benchmarks compare
//! semantically equivalent operations — same fields, same buffer, same count.
//! Fair comparison uses `wrap` (sbe-tool's `wrap` does no bounds check).
//!
//! Results are profile-specific. The fairness audit found that sbe-tool
//! performs well with and without LTO because its generated accessors carry
//! explicit `#[inline]` annotations; pre-fix ergon did not and became slower
//! than sbe-tool without LTO. Ergon's hot accessors are now annotated, but both
//! profiles remain mandatory to catch a recurrence.
//! Header-inclusive, header-only, and body-only encode cases are reported
//! separately so fused header stores do not masquerade as scalar-field speed.
//! Within each case both arms must match sbe-tool:
//! - header+body / header-only: ergon `wrap_and_apply_header` ↔ sbe-tool
//!   `wrap(8)` then `header(0).parent()` (official order, before body setters)
//! - body-only: ergon `wrap(0)` ↔ sbe-tool `wrap(8)` with **no** `.header(0)`
//! Never invent length as `8 + encoded_length()` without a real header write.

#![allow(
    unsafe_code,
    missing_docs,
    unused_variables,
    dead_code,
    unused_mut,
    unused_must_use,
    unused_assignments,
    unused_comparisons,
    unused_attributes
)]
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

// ergon generated code

use ergo_sbe_benchmarks::{
    assert_encode_extent, assert_stream_wrap_extent, ergo_car::*, sbe_tool_car_body_decoder,
};

// sbe-tool Rust SBE generated code (patched for module inclusion)

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

#[path = "_common.rs"]
mod common;
use common::BASELINE;

// Sub-nanosecond codec operations are shorter than the timing harness and are
// highly sensitive to code placement. Repeat identical work inside each
// Criterion iteration so maintained ratios measure codec work.
const MICRO_BATCH_SIZE: usize = 1_024;
const BATCH_SIZE: usize = 10_000;

// Header bytes for sbe-tool decoder construction
fn sbe_tool_block_length() -> u16 {
    u16::from_le_bytes(BASELINE[0..2].try_into().unwrap())
}
fn sbe_tool_version() -> u16 {
    u16::from_le_bytes(BASELINE[6..8].try_into().unwrap())
}

// Pre-computed ergon header fields (validate once per stream, like sbe-tool).
// In a real feed handler these are read once at session setup, not per message.
fn ergo_sbe_header_fields() -> (usize, u16) {
    let header = MessageHeader(read_bytes::<8>(BASELINE, 0));
    (header.block_length() as usize, header.version())
}

fn assert_decode_parity() {
    let bl = sbe_tool_block_length();
    let ver = sbe_tool_version();
    let ergon = CarDecoder::try_from(BASELINE).unwrap();
    let sbe_tool = sbe_tool_car_body_decoder(BASELINE, 0, bl, ver);

    assert_eq!(ergon.serial_number(), sbe_tool.serial_number());
    assert_eq!(ergon.model_year(), sbe_tool.model_year());
    assert_eq!(ergon.available() as u8, sbe_tool.available() as u8);
    assert_eq!(ergon.code() as u8, sbe_tool.code() as u8);
    assert_eq!(ergon.some_numbers(), sbe_tool.some_numbers());
    assert_eq!(ergon.vehicle_code(), sbe_tool.vehicle_code());
    assert_eq!(ergon.extras().0, sbe_tool.extras().0);

    let ergon_engine = ergon.engine();
    let sbe_tool_engine = sbe_tool.engine_decoder();
    assert_eq!(ergon_engine.capacity(), sbe_tool_engine.capacity());
    assert_eq!(
        ergon_engine.num_cylinders(),
        sbe_tool_engine.num_cylinders()
    );
    assert_eq!(
        ergon_engine.manufacturer_code(),
        sbe_tool_engine.manufacturer_code()
    );
    assert_eq!(ergon_engine.efficiency(), sbe_tool_engine.efficiency());
    assert_eq!(
        ergon_engine.booster_enabled() as u8,
        sbe_tool_engine.booster_enabled() as u8
    );
    let ergon_booster = ergon_engine.booster();
    let sbe_tool_booster = sbe_tool_engine.booster_decoder();
    assert_eq!(
        ergon_booster.boost_type() as u8,
        sbe_tool_booster.boost_type() as u8
    );
    assert_eq!(ergon_booster.horse_power(), sbe_tool_booster.horse_power());

    let mut ergon_fuel = CarDecoder::try_from(BASELINE)
        .unwrap()
        .into_fuel_figures()
        .unwrap();
    let mut sbe_tool_fuel = sbe_tool_car_body_decoder(BASELINE, 0, bl, ver).fuel_figures_decoder();
    while let Some(ergon_entry) = ergon_fuel.next() {
        let ergon_entry = ergon_entry.unwrap();
        assert!(sbe_tool_fuel.advance().unwrap().is_some());
        assert_eq!(ergon_entry.speed(), sbe_tool_fuel.speed());
        assert_eq!(ergon_entry.mpg().to_bits(), sbe_tool_fuel.mpg().to_bits());
        let usage_description = sbe_tool_fuel.usage_description_decoder();
        assert_eq!(
            ergon_entry.usage_description().unwrap(),
            sbe_tool_fuel.usage_description_slice(usage_description)
        );
    }
    assert!(sbe_tool_fuel.advance().unwrap().is_none());

    let ergon_after_fuel = ergon_fuel.finish().unwrap();
    let mut sbe_tool_car = sbe_tool_fuel.parent().unwrap();
    let mut ergon_performance = ergon_after_fuel.into_performance_figures().unwrap();
    let mut sbe_tool_performance = sbe_tool_car.performance_figures_decoder();
    while let Some(ergon_entry) = ergon_performance.next() {
        let ergon_entry = ergon_entry.unwrap();
        assert!(sbe_tool_performance.advance().unwrap().is_some());
        assert_eq!(
            ergon_entry.octane_rating(),
            sbe_tool_performance.octane_rating()
        );
        let mut sbe_tool_acceleration = sbe_tool_performance.acceleration_decoder();
        for ergon_acceleration in ergon_entry.acceleration().unwrap() {
            assert!(sbe_tool_acceleration.advance().unwrap().is_some());
            assert_eq!(ergon_acceleration.mph(), sbe_tool_acceleration.mph());
            assert_eq!(
                ergon_acceleration.seconds().to_bits(),
                sbe_tool_acceleration.seconds().to_bits()
            );
        }
        assert!(sbe_tool_acceleration.advance().unwrap().is_none());
        sbe_tool_performance = sbe_tool_acceleration.parent().unwrap();
    }
    assert!(sbe_tool_performance.advance().unwrap().is_none());

    let ergon_after_performance = ergon_performance.finish().unwrap();
    sbe_tool_car = sbe_tool_performance.parent().unwrap();
    let (ergon_manufacturer, ergon_after_manufacturer) =
        ergon_after_performance.into_manufacturer().unwrap();
    let sbe_tool_manufacturer = sbe_tool_car.manufacturer_decoder();
    assert_eq!(
        ergon_manufacturer,
        sbe_tool_car.manufacturer_slice(sbe_tool_manufacturer)
    );
    let (ergon_model, ergon_after_model) = ergon_after_manufacturer.into_model().unwrap();
    let sbe_tool_model = sbe_tool_car.model_decoder();
    assert_eq!(ergon_model, sbe_tool_car.model_slice(sbe_tool_model));
    let (ergon_activation_code, _) = ergon_after_model.into_activation_code().unwrap();
    let sbe_tool_activation_code = sbe_tool_car.activation_code_decoder();
    assert_eq!(
        ergon_activation_code,
        sbe_tool_car.activation_code_slice(sbe_tool_activation_code)
    );
}

fn bench_decode_entry_point(c: &mut Criterion) {
    assert_decode_parity();
    let bl = sbe_tool_block_length();
    let ver = sbe_tool_version();
    let (bl_e, ver_e) = ergo_sbe_header_fields();

    let mut group = c.benchmark_group("parity/decode/entry_point");
    group.throughput(Throughput::Elements(MICRO_BATCH_SIZE as u64));

    // Fast path: pre-computed header, lean wrap (4 field assigns).
    group.bench_function("ergo-sbe_wrap", |b| {
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                let car = CarDecoder::wrap(black_box(BASELINE), 0, bl_e, ver_e);
                black_box(car);
            }
        });
    });

    // Informational: full validation (header read + schema_id check every call).
    group.bench_function("ergo-sbe_try_from", |b| {
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
                black_box(car);
            }
        });
    });

    group.bench_function("sbe-tool_wrap", |b| {
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                let car = sbe_tool_car_body_decoder(black_box(BASELINE), 0, bl, ver);
                black_box(car);
            }
        });
    });

    group.finish();
}

fn bench_decode_scalar(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();
    let bl = sbe_tool_block_length();
    let ver = sbe_tool_version();
    let sbe_tool_car = sbe_tool_car_body_decoder(BASELINE, 0, bl, ver);

    let mut group = c.benchmark_group("parity/decode/scalar");
    group.throughput(Throughput::Elements(MICRO_BATCH_SIZE as u64));

    group.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                let car = black_box(&car);
                let sn = car.serial_number();
                let my = car.model_year();
                black_box((sn, my));
            }
        });
    });

    group.bench_function("sbe-tool", |b| {
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                let sbe_tool_car = black_box(&sbe_tool_car);
                let sn = sbe_tool_car.serial_number();
                let my = sbe_tool_car.model_year();
                black_box((sn, my));
            }
        });
    });

    group.finish();
}

fn bench_decode_array(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();
    let bl = sbe_tool_block_length();
    let ver = sbe_tool_version();
    let sbe_tool_car = sbe_tool_car_body_decoder(BASELINE, 0, bl, ver);

    let mut group = c.benchmark_group("parity/decode/array");
    group.throughput(Throughput::Elements(MICRO_BATCH_SIZE as u64));

    group.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                let car = black_box(&car);
                let sn = car.some_numbers();
                black_box(sn);
            }
        });
    });

    group.bench_function("sbe-tool", |b| {
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                let sbe_tool_car = black_box(&sbe_tool_car);
                let sn = sbe_tool_car.some_numbers();
                black_box(sn);
            }
        });
    });

    group.finish();
}

fn bench_decode_composite(c: &mut Criterion) {
    let buf = replicate_baseline(MICRO_BATCH_SIZE);
    let msg_len = BASELINE.len();
    let bl = sbe_tool_block_length();
    let ver = sbe_tool_version();
    let (bl_e, ver_e) = ergo_sbe_header_fields();

    // Untimed: prove every message start the timed loop will wrap unchecked.
    assert_stream_wrap_extent(&buf, msg_len, MICRO_BATCH_SIZE, bl_e, ver_e);

    let ergo_car = CarDecoder::wrap(&buf, 0, bl_e, ver_e);
    let ergo_engine = ergo_car.engine();
    let tool_engine = sbe_tool_car_body_decoder(&buf, 0, bl, ver).engine_decoder();
    assert_eq!(ergo_engine.capacity(), tool_engine.capacity());
    assert_eq!(ergo_engine.num_cylinders(), tool_engine.num_cylinders());

    let mut group = c.benchmark_group("parity/decode/composite");
    group.throughput(Throughput::Elements(MICRO_BATCH_SIZE as u64));

    // Traverse a contiguous stream so the gate measures composite decoding,
    // not the code address of a loop repeatedly loading the same three bytes.
    //
    // Equal work: sbe-tool's `wrap` only stores buffer, offset, block length,
    // and version — it performs no extent check. Ergon's bare `wrap` proves the
    // version-aware fixed extent on every message, so timing it here would
    // charge ergon for a bounds proof its reference never performs. The extent
    // is proven once above (both arms decoded the same stream and agreed on
    // every field), and the timed region uses the matching unchecked
    // constructor — the same validation class the other maintained pairs use.
    group.bench_function("ergo-sbe_engine", |b| {
        b.iter(|| {
            let buf = black_box(buf.as_slice());
            let mut total_capacity = 0_u64;
            let mut total_cylinders = 0_u64;
            let mut off = 0;
            for _ in 0..MICRO_BATCH_SIZE {
                // SAFETY: `buf` is a prebuilt concat of whole baseline frames,
                // so every `off` is a message start with a proven extent.
                let car = unsafe { CarDecoder::wrap_unchecked(buf, off, bl_e, ver_e) };
                let engine = car.engine();
                total_capacity += u64::from(engine.capacity());
                total_cylinders += u64::from(engine.num_cylinders());
                off += msg_len;
            }
            black_box((total_capacity, total_cylinders));
        });
    });

    group.bench_function("sbe-tool_engine", |b| {
        b.iter(|| {
            let buf = black_box(buf.as_slice());
            let mut total_capacity = 0_u64;
            let mut total_cylinders = 0_u64;
            let mut off = 0;
            for _ in 0..MICRO_BATCH_SIZE {
                let engine = sbe_tool_car_body_decoder(buf, off, bl, ver).engine_decoder();
                total_capacity += u64::from(engine.capacity());
                total_cylinders += u64::from(engine.num_cylinders());
                off += msg_len;
            }
            black_box((total_capacity, total_cylinders));
        });
    });

    group.finish();
}

fn replicate_baseline(count: usize) -> Vec<u8> {
    let msg_len = BASELINE.len();
    let mut buf = vec![0; count * msg_len];
    for chunk in buf.chunks_mut(msg_len) {
        chunk.copy_from_slice(BASELINE);
    }
    buf
}

fn bench_throughput_batch(c: &mut Criterion) {
    let buf = replicate_baseline(BATCH_SIZE);
    let msg_len = BASELINE.len();
    let bl = sbe_tool_block_length();
    let ver = sbe_tool_version();
    // Validate the stream header once (real feed-handler pattern), then decode fast.
    let (bl_e, ver_e) = ergo_sbe_header_fields();
    // Untimed: prove every message start the timed loop will wrap unchecked.
    assert_stream_wrap_extent(&buf, msg_len, BATCH_SIZE, bl_e, ver_e);

    let mut group = c.benchmark_group("parity/throughput/batch_10k");
    group.throughput(Throughput::Elements(BATCH_SIZE as u64));

    // Equal work: both arms stride absolute offsets into one prebuilt buffer
    // (no per-message re-slice). Body starts at off+8 after the LE header.
    // sbe-tool `wrap` does no extent check — use ergon `wrap_unchecked` so the
    // gated pair measures equal logical work (field loads), not product bare
    // wrap's intentional version-aware bounds proof on every message.
    group.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            let mut total: u64 = 0;
            let mut total_year: u64 = 0;
            let mut off = 0;
            for _ in 0..BATCH_SIZE {
                // SAFETY: `buf` is a prebuilt concat of full baseline frames;
                // each `off` is a message start with proven header+body extent.
                let car = unsafe {
                    CarDecoder::wrap_unchecked(black_box(buf.as_slice()), off, bl_e, ver_e)
                };
                total += car.serial_number();
                total_year += car.model_year() as u64;
                off += msg_len;
            }
            black_box((total, total_year));
        });
    });

    group.bench_function("sbe-tool", |b| {
        b.iter(|| {
            let mut total: u64 = 0;
            let mut total_year: u64 = 0;
            let mut off = 0;
            for _ in 0..BATCH_SIZE {
                let car = sbe_tool_car_body_decoder(black_box(buf.as_slice()), off, bl, ver);
                total += car.serial_number() as u64;
                total_year += car.model_year() as u64;
                off += msg_len;
            }
            black_box((total, total_year));
        });
    });

    group.finish();
}

fn bench_encode_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("parity/encode/scalar");
    group.throughput(Throughput::Elements(MICRO_BATCH_SIZE as u64));

    // ── length parity: both codecs must produce identical encoded lengths ──
    {
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
        // Encode a complete Car message with both codecs, verify lengths match.
        let mut ebuf = [0u8; 512];
        let ergo_len = CarEncoder::wrap_and_apply_header(&mut ebuf, 0)
            .fixed(&CarFixedFields {
                serial_number: 42,
                model_year: 2020,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0; 4],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(
                    0,
                    0,
                    *b"ABC",
                    0,
                    BooleanType::F,
                    Booster::new(BoostType::TURBO, 0),
                ),
            })
            .fuel_figures(0, |_| Ok(()))
            .unwrap()
            .performance_figures(0, |_| Ok(()))
            .unwrap()
            .manufacturer(b"X")
            .unwrap()
            .model(b"Y")
            .unwrap()
            .activation_code(b"Z")
            .unwrap()
            .encoded_length_with_header();

        let mut tbuf = [0u8; 512];
        // Official order: wrap body @ 8, header @ 0, then body fields.
        let t = ToolCarEnc::default().wrap(WriteBuf::new(&mut tbuf), 8);
        let mut h = t.header(0);
        let mut t = h.parent().unwrap();
        t.serial_number(42)
            .model_year(2020)
            .available(ToolBool::T)
            .code(ToolModel::A)
            .some_numbers(&[0; 4])
            .vehicle_code(b"ABCDEF")
            .extras(ToolExtras::default());
        let mut eng = t.engine_encoder();
        eng.capacity(0)
            .num_cylinders(0)
            .manufacturer_code(b"ABC")
            .efficiency(0)
            .booster_enabled(ToolBool::F);
        let mut boost = eng.booster_encoder();
        boost.boost_type(ToolBoost::TURBO).horse_power(0);
        eng = boost.parent().unwrap();
        t = eng.parent().unwrap();
        let mut fuel = ToolFuel::default();
        fuel = t.fuel_figures_encoder(0, fuel);
        t = fuel.parent().unwrap();
        let mut perf = ToolPerf::default();
        perf = t.performance_figures_encoder(0, perf);
        t = perf.parent().unwrap();
        t.manufacturer("X").model("Y").activation_code(b"Z");
        // Absolute frame end after wrap@8 + header write (not invented `8 + body`).
        let tool_len = t.get_limit();
        assert_eq!(
            ergo_len, tool_len,
            "encode/scalar length mismatch: ergon={ergo_len}, sbe-tool={tool_len}"
        );
    }

    {
        use ergo_sbe_benchmarks::sbe_tool_car::sbe_tool::{
            WriteBuf, car_codec::encoder::CarEncoder as ToolCarEncoder,
        };

        // Constructor proves HEADER+BLOCK_LENGTH (8+45); compare only the
        // touched prefix (header + serial_number + model_year = 18 bytes).
        const NEED: usize = 8 + 45;
        let mut ergon = [0u8; NEED];
        black_box(
            CarEncoder::wrap_and_apply_header(&mut ergon, 0)
                .serial_number(1234)
                .model_year(2013),
        );
        let mut sbe_tool = [0u8; NEED];
        black_box(
            ToolCarEncoder::default()
                .wrap(WriteBuf::new(&mut sbe_tool), 8)
                .header(0)
                .parent()
                .unwrap()
                .serial_number(1234)
                .model_year(2013),
        );
        assert_eq!(&ergon[..18], &sbe_tool[..18], "scalar header+body bytes");
    }

    // Header-inclusive API comparison.
    group.bench_function("ergo-sbe_header_and_body", |b| {
        let mut buf = [0u8; 512];
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                    .serial_number(black_box(1234))
                    .model_year(black_box(2013));
            }
            black_box(&buf[..18]);
        });
    });

    group.bench_function("sbe-tool_header_and_body", |b| {
        let mut buf = [0u8; 512];
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                ergo_sbe_benchmarks::sbe_tool_car::sbe_tool::car_codec::encoder::CarEncoder::default()
                    .wrap(
                        ergo_sbe_benchmarks::sbe_tool_car::sbe_tool::WriteBuf::new(black_box(&mut buf)),
                        8,
                    )
                    .header(0)
                    .parent()
                    .unwrap()
                    .serial_number(black_box(1234))
                    .model_year(black_box(2013));
            }
            black_box(&buf[..18]);
        });
    });

    // Header-only isolates fixed API setup and header-store costs.
    group.bench_function("ergo-sbe_header_only", |b| {
        let mut buf = [0u8; 512];
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0);
            }
            black_box(&buf[..8]);
        });
    });

    group.bench_function("sbe-tool_header_only", |b| {
        let mut buf = [0u8; 512];
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                ergo_sbe_benchmarks::sbe_tool_car::sbe_tool::car_codec::encoder::CarEncoder::default()
                    .wrap(
                        ergo_sbe_benchmarks::sbe_tool_car::sbe_tool::WriteBuf::new(black_box(&mut buf)),
                        8,
                    )
                    .header(0);
            }
            black_box(&buf[..8]);
        });
    });

    // Body-only isolates the two scalar setters. ergon wrap takes the message
    // start; sbe-tool wrap takes the absolute body offset. sbe-tool wrap does
    // no extent check — use wrap_unchecked so the gated pair is equal work
    // (same unfairness class as batch decode; product bare wrap still proves).
    group.bench_function("ergo-sbe_body_only", |b| {
        let mut buf = [0u8; 512];
        // Untimed: prove the buffer holds a complete frame before the timed
        // region wraps it unchecked.
        // This arm writes only the fixed block, so header + block length is the
        // exact extent the timed region touches.
        assert_encode_extent(&buf, CarEncoder::HEADER_LENGTH + CarEncoder::BLOCK_LENGTH);
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                // SAFETY: extent asserted directly above.
                unsafe { CarEncoder::wrap_unchecked(black_box(&mut buf), 0) }
                    .serial_number(black_box(1234))
                    .model_year(black_box(2013));
            }
            black_box(&buf[8..18]);
        });
    });

    group.bench_function("sbe-tool_body_only", |b| {
        let mut buf = [0u8; 512];
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                ergo_sbe_benchmarks::sbe_tool_car::sbe_tool::car_codec::encoder::CarEncoder::default()
                    .wrap(
                        ergo_sbe_benchmarks::sbe_tool_car::sbe_tool::WriteBuf::new(black_box(&mut buf)),
                        8,
                    )
                    .serial_number(black_box(1234))
                    .model_year(black_box(2013));
            }
            black_box(&buf[8..18]);
        });
    });

    group.finish();
}

fn bench_encode_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("parity/encode/throughput_10k");
    group.throughput(Throughput::Elements(BATCH_SIZE as u64));

    // Equal work: both arms encode header + 2 scalars per message.
    // ergon wrap_and_apply_header() writes header at 0 and body at 8.
    // sbe-tool wrap(buf,8) + header(0) writes header at 0 and body at 8.
    // Buffer allocated once and reused — no alloc on the timed path.
    group.bench_function("ergo-sbe", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * 64];
        b.iter(|| {
            for i in 0..BATCH_SIZE {
                let off = i * 64;
                black_box(
                    CarEncoder::wrap_and_apply_header(&mut buf[off..off + 64], 0)
                        .serial_number(i as u64)
                        .model_year(2013),
                );
            }
            black_box(&buf);
        });
    });

    group.bench_function("sbe-tool", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * 64];
        b.iter(|| {
            for i in 0..BATCH_SIZE {
                let off = i * 64;
                black_box(
                    ergo_sbe_benchmarks::sbe_tool_car::sbe_tool::car_codec::encoder::CarEncoder::default()
                        .wrap(
                            ergo_sbe_benchmarks::sbe_tool_car::sbe_tool::WriteBuf::new(
                                &mut buf[off..off + 64],
                            ),
                            8,
                        )
                        .header(0)
                        .parent()
                        .unwrap()
                        .serial_number(i as u64)
                        .model_year(2013),
                );
            }
            black_box(&buf);
        });
    });

    group.finish();
}

fn bench_decode_consuming_full(c: &mut Criterion) {
    // Fair three-way full-message decode over the same BASELINE buffer. All
    // arms read all encoded fixed fields, both composite levels, every fuel entry
    // (speed, mpg, usage_description), every performance entry (octane_rating +
    // nested acceleration mph/seconds), and all message-level var-data. Constants
    // are excluded because they do not read the wire. sbe-tool's advance() does
    // not skip per-entry tails, so every codec must traverse the same dynamic
    // members to advance.
    let bl = sbe_tool_block_length();
    let ver = sbe_tool_version();
    let (bl_e, ver_e) = ergo_sbe_header_fields();

    let mut group = c.benchmark_group("parity/decode/full_message");
    group.throughput(Throughput::Elements(1));

    group.bench_function("ergo-sbe_consuming", |b| {
        b.iter(|| {
            let car = CarDecoder::wrap(black_box(BASELINE), 0, bl_e, ver_e);
            black_box((
                car.serial_number(),
                car.model_year(),
                car.available(),
                car.code(),
                car.some_numbers(),
                car.vehicle_code(),
                car.extras(),
            ));
            let engine = car.engine();
            let booster = engine.booster();
            black_box((
                engine.capacity(),
                engine.num_cylinders(),
                engine.manufacturer_code(),
                engine.efficiency(),
                engine.booster_enabled(),
                booster.boost_type(),
                booster.horse_power(),
            ));
            let mut fuel = car.into_fuel_figures().unwrap();
            while let Some(Ok(e)) = fuel.next() {
                black_box((e.speed(), e.mpg()));
                black_box(e.usage_description().unwrap());
            }
            let after_fuel = fuel.finish().unwrap();
            let mut perf = after_fuel.into_performance_figures().unwrap();
            while let Some(Ok(e)) = perf.next() {
                black_box(e.octane_rating());
                for a in e.acceleration().unwrap() {
                    black_box((a.mph(), a.seconds()));
                }
            }
            let after_perf = perf.finish().unwrap();
            let (mfr, a1) = after_perf.into_manufacturer().unwrap();
            let (model, a2) = a1.into_model().unwrap();
            let (code, _done) = a2.into_activation_code().unwrap();
            black_box((mfr, model, code));
        });
    });

    group.bench_function("sbe-tool", |b| {
        b.iter(|| {
            let mut car = sbe_tool_car_body_decoder(black_box(BASELINE), 0, bl, ver);
            black_box((
                car.serial_number(),
                car.model_year(),
                car.available(),
                car.code(),
                car.some_numbers(),
                car.vehicle_code(),
                car.extras(),
            ));
            let engine = car.engine_decoder();
            black_box((
                engine.capacity(),
                engine.num_cylinders(),
                engine.manufacturer_code(),
                engine.efficiency(),
                engine.booster_enabled(),
            ));
            let mut booster = engine.booster_decoder();
            black_box((booster.boost_type(), booster.horse_power()));
            let mut engine = booster.parent().unwrap();
            car = engine.parent().unwrap();
            let mut ff = car.fuel_figures_decoder();
            while let Some(_) = ff.advance().unwrap() {
                black_box((ff.speed(), ff.mpg()));
                let c = ff.usage_description_decoder();
                black_box(ff.usage_description_slice(c));
            }
            car = ff.parent().unwrap();
            let mut pf = car.performance_figures_decoder();
            while let Some(_) = pf.advance().unwrap() {
                black_box(pf.octane_rating());
                let mut acc = pf.acceleration_decoder();
                while let Some(_) = acc.advance().unwrap() {
                    black_box((acc.mph(), acc.seconds()));
                }
                pf = acc.parent().unwrap();
            }
            car = pf.parent().unwrap();
            let mfr = car.manufacturer_decoder();
            black_box(car.manufacturer_slice(mfr));
            let model = car.model_decoder();
            black_box(car.model_slice(model));
            let code = car.activation_code_decoder();
            black_box(car.activation_code_slice(code));
        });
    });

    group.finish();
}

fn bench_encode_full_stage_transition(c: &mut Criterion) {
    // ergon-only stage-transition diagnostic (no sbe-tool equivalent) — not a
    // parity scenario, so the group is not under parity/.
    let mut group = c.benchmark_group("encode/full_stage");
    group.throughput(Throughput::Elements(1));

    group.bench_function("ergo-sbe", |b| {
        let mut buf = [0u8; 512];
        b.iter(|| {
            let len = CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                .fixed(&CarFixedFields {
                    serial_number: 1234,
                    model_year: 2013,
                    available: BooleanType::T,
                    code: Model::A,
                    some_numbers: [1u32, 2, 3, 4],
                    vehicle_code: [97, 98, 99, 100, 101, 102],
                    extras: OptionalExtras::default(),
                    engine: Engine::new(
                        2000,
                        4,
                        [49, 0, 0],
                        0i8,
                        BooleanType::F,
                        Booster::new(BoostType::TURBO, 0),
                    ),
                })
                .fuel_figures(2, |g| {
                    g.add(|e| {
                        e.speed(30).mpg(35.9);
                        Ok(())
                    })?;
                    g.add(|e| {
                        e.speed(55).mpg(40.0);
                        Ok(())
                    })?;
                    Ok(())
                })
                .unwrap()
                .performance_figures(1, |g| {
                    g.add(|e| {
                        e.octane_rating(95);
                        Ok(())
                    })?;
                    Ok(())
                })
                .unwrap()
                .manufacturer(b"Honda")
                .unwrap()
                .model(b"Civic")
                .unwrap()
                .activation_code(b"abc")
                .unwrap()
                .encoded_length_with_header();
            black_box(&buf[..len]);
        });
    });

    group.finish();
}

fn assert_full_message_encode_wire_parity() {
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

    let mut ebuf = [0u8; 512];
    let ergo_len = CarEncoder::wrap_and_apply_header(&mut ebuf, 0)
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
            g.add(|ent| {
                ent.speed(40).mpg(33.3).usage_description(b"city")?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap()
        .performance_figures(0, |_| Ok(()))
        .unwrap()
        .manufacturer(b"Toyota")
        .unwrap()
        .model(b"Yaris")
        .unwrap()
        .activation_code(b"zz")
        .unwrap()
        .encoded_length_with_header();

    let mut tbuf = [0u8; 512];
    let t = ToolCarEnc::default().wrap(WriteBuf::new(&mut tbuf), 8);
    let mut h = t.header(0);
    let mut t = h.parent().unwrap();
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
    eng = boost.parent().unwrap();
    t = eng.parent().unwrap();
    let mut fuel = ToolFuel::default();
    fuel = t.fuel_figures_encoder(1, fuel);
    fuel.advance().unwrap();
    fuel.speed(40).mpg(33.3).usage_description(b"city");
    t = fuel.parent().unwrap();
    let mut perf = ToolPerf::default();
    perf = t.performance_figures_encoder(0, perf);
    t = perf.parent().unwrap();
    t.manufacturer("Toyota")
        .model("Yaris")
        .activation_code(b"zz");
    let tool_len = t.get_limit();
    assert_eq!(ergo_len, tool_len, "wire parity length");
    assert_eq!(&ebuf[..ergo_len], &tbuf[..tool_len], "wire parity bytes");
}

fn bench_wire_parity_encode_full_message(c: &mut Criterion) {
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

    assert_full_message_encode_wire_parity();
    let mut group = c.benchmark_group("parity/wire_parity/encode_full");
    group.throughput(Throughput::Elements(MICRO_BATCH_SIZE as u64));

    group.bench_function("ergo-sbe", |b| {
        let mut buf = [0u8; 512];
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                let len = CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
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
                        g.add(|ent| {
                            ent.speed(40).mpg(33.3).usage_description(b"city")?;
                            Ok(())
                        })?;
                        Ok(())
                    })
                    .unwrap()
                    .performance_figures(0, |_| Ok(()))
                    .unwrap()
                    .manufacturer(b"Toyota")
                    .unwrap()
                    .model(b"Yaris")
                    .unwrap()
                    .activation_code(b"zz")
                    .unwrap()
                    .encoded_length_with_header();
                black_box(&buf[..len]);
                black_box(len);
            }
        });
    });

    group.bench_function("sbe-tool", |b| {
        let mut buf = [0u8; 512];
        b.iter(|| {
            for _ in 0..MICRO_BATCH_SIZE {
                let t = ToolCarEnc::default().wrap(WriteBuf::new(black_box(&mut buf)), 8);
                let mut h = t.header(0);
                let mut t = h.parent().unwrap();
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
                eng = boost.parent().unwrap();
                t = eng.parent().unwrap();
                let mut fuel = ToolFuel::default();
                fuel = t.fuel_figures_encoder(1, fuel);
                fuel.advance().unwrap();
                fuel.speed(40).mpg(33.3).usage_description(b"city");
                t = fuel.parent().unwrap();
                let mut perf = ToolPerf::default();
                perf = t.performance_figures_encoder(0, perf);
                t = perf.parent().unwrap();
                t.manufacturer("Toyota")
                    .model("Yaris")
                    .activation_code(b"zz");
                let len = t.get_limit();
                drop(t);
                black_box(&buf[..len]);
                black_box(len);
            }
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_decode_entry_point,
    bench_decode_scalar,
    bench_decode_array,
    bench_decode_composite,
    bench_throughput_batch,
    bench_encode_scalar,
    bench_encode_throughput,
    bench_decode_consuming_full,
    bench_encode_full_stage_transition,
    bench_wire_parity_encode_full_message,
);
criterion_main!(benches);
