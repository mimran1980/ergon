//! Every truncation and single-byte corruption of the car frame produces the
//! same checked-decode outcome (`Ok` value or exact `DecodeError` variant,
//! field and numbers) on the staged, ordered and skip lanes, followed by
//! `AnyMessage::try_decode` on every truncation and every lane at offsets
//! reaching and passing the end of the buffer.
//!
//! The snapshot pins behaviour while hot-path checks are restructured: a
//! cheaper check that changes which error a hostile frame reports, or that
//! lets one through, changes a line here.
//!
//! Regenerate deliberately with `UPDATE_DECODE_SNAPSHOTS=1` and review the diff.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
use std::error::Error;
use std::fs;
use std::path::PathBuf;

mod common;
use common::{Paths, compile_and_run_capture, generate};

const PROGRAM: &str = r#"
    fn staged(buf: &[u8], offset: usize) -> Result<u64, sbe_rt::DecodeError> {
        let mut sum = 0u64;
        let car = CarDecoder::try_decode(buf, offset)?;
        let (manufacturer, next) = car
            .into_fuel_figures(|entry| -> Result<_, sbe_rt::DecodeError> {
                sum += u64::from(entry.speed());
                let (usage, done) = entry.into_usage_description()?;
                sum += usage.len() as u64;
                Ok(done)
            })?
            .into_performance_figures(|entry| -> Result<_, sbe_rt::DecodeError> {
                sum += u64::from(entry.octane_rating());
                let mut acceleration = entry.into_acceleration()?;
                for a in &mut acceleration {
                    sum += u64::from(a.mph());
                }
                acceleration.finish()
            })?
            .into_manufacturer()?;
        let (model, next) = next.into_model()?;
        let (code, _) = next.into_activation_code()?;
        Ok(sum + (manufacturer.len() + model.len() + code.len()) as u64)
    }

    fn ordered(buf: &[u8], offset: usize) -> Result<u64, sbe_rt::DecodeError> {
        let mut sum = 0u64;
        CarDecoder::try_decode(buf, offset)?
            .ordered()
            .fuel_figures(|entry, _| -> Result<_, sbe_rt::DecodeError> {
                sum += u64::from(entry.speed());
                entry.ordered().usage_description(|usage| {
                    sum += usage.len() as u64;
                    Ok(())
                })
            })?
            .performance_figures(|entry, _| -> Result<_, sbe_rt::DecodeError> {
                sum += u64::from(entry.octane_rating());
                entry.ordered().acceleration(|a, _| {
                    sum += u64::from(a.mph());
                    Ok(())
                })
            })?
            .manufacturer(|v| -> Result<(), sbe_rt::DecodeError> {
                sum += v.len() as u64;
                Ok(())
            })?
            .model(|v| -> Result<(), sbe_rt::DecodeError> {
                sum += v.len() as u64;
                Ok(())
            })?
            .activation_code(|v| -> Result<(), sbe_rt::DecodeError> {
                sum += v.len() as u64;
                Ok(())
            })?;
        Ok(sum)
    }

    fn skipped(buf: &[u8], offset: usize) -> Result<u64, sbe_rt::DecodeError> {
        let next = CarDecoder::try_decode(buf, offset)?
            .skip_fuel_figures()?
            .skip_performance_figures()?;
        let (manufacturer, next) = next.into_manufacturer()?;
        let (model, next) = next.into_model()?;
        let (code, _) = next.into_activation_code()?;
        Ok((manufacturer.len() + model.len() + code.len()) as u64)
    }

    // Random access: each getter re-walks every preceding group entry and
    // var-data field from the message start.
    fn random(buf: &[u8], offset: usize) -> Result<u64, sbe_rt::DecodeError> {
        let car = CarDecoder::try_decode(buf, offset)?;
        Ok((car.manufacturer()?.len() + car.model()?.len() + car.activation_code()?.len()) as u64)
    }

    let full: Vec<u8> = FRAME;
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for cut in 0..=full.len() {
        cases.push((format!("cut {cut}"), full[..cut].to_vec()));
    }
    for at in 0..full.len() {
        for value in [0x00u8, 0x01, 0x7f, 0xff] {
            if full[at] != value {
                let mut bytes = full.clone();
                bytes[at] = value;
                cases.push((format!("byte {at}={value:#04x}"), bytes));
            }
        }
    }
    fn any(buf: &[u8], offset: usize) -> Result<u64, sbe_rt::DecodeError> {
        Ok(match AnyMessage::try_decode(buf, offset)? {
            AnyMessage::Car(car) => car.serial_number(),
            _ => u64::MAX,
        })
    }

    type Lane = fn(&[u8], usize) -> Result<u64, sbe_rt::DecodeError>;
    fn report(label: &str, lanes: &[(&str, Lane)], bytes: &[u8], offset: usize) {
        for (lane, run) in lanes {
            match std::panic::catch_unwind(|| run(bytes, offset)) {
                Ok(result) => println!("{label} {lane}: {result:?}"),
                Err(_) => println!("{label} {lane}: PANIC"),
            }
        }
    }
    let decoders: [(&str, Lane); 4] =
        [("staged", staged), ("ordered", ordered), ("skipped", skipped), ("random", random)];
    for (label, bytes) in &cases {
        report(label, &decoders, bytes, 0);
    }

    // Header guard edges: AnyMessage::try_decode on every truncation, then
    // every lane on a frame placed after a prefix and at offsets reaching
    // and passing the end of the buffer.
    for (label, bytes) in &cases[..=full.len()] {
        report(label, &[("any", any)], bytes, 0);
    }
    let all: [(&str, Lane); 5] = [decoders[0], decoders[1], decoders[2], decoders[3], ("any", any)];
    let mut prefixed = vec![0xaau8; 3];
    prefixed.extend_from_slice(&full);
    report("prefix 3 offset 3", &all, &prefixed, 3);
    for offset in [full.len() - 1, full.len(), full.len() + 1, full.len() + 8, usize::MAX] {
        report(&format!("offset {offset}"), &all, &full, offset);
    }
"#;

fn bytes_to_vec_literal(bytes: &[u8]) -> String {
    let inner = bytes
        .iter()
        .map(|b| format!("{b}u8"))
        .collect::<Vec<_>>()
        .join(",");
    format!("vec![{inner}]")
}

#[test]
fn checked_decode_outcomes_match_snapshot_for_every_truncation_and_byte_corruption()
-> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "checked_decode_snapshot");
    let frame = fs::read(Paths::baseline_binary())?;
    let program = PROGRAM.replace("FRAME", &bytes_to_vec_literal(&frame));
    let actual = compile_and_run_capture("checked_decode_snapshot", &src, &program);
    assert!(
        !actual.contains("PANIC"),
        "a checked decode lane panicked on hostile input"
    );

    let snapshot = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/checked-decode-errors-car.txt");
    if std::env::var_os("UPDATE_DECODE_SNAPSHOTS").is_some() {
        fs::write(&snapshot, &actual)?;
    }
    let expected = fs::read_to_string(&snapshot)?;
    let mismatches: Vec<_> = expected
        .lines()
        .zip(actual.lines())
        .filter(|(e, a)| e != a)
        .take(10)
        .collect();
    assert!(
        mismatches.is_empty() && expected.lines().count() == actual.lines().count(),
        "checked decode outcomes changed ({} expected lines, {} actual); first differences:\n{mismatches:#?}",
        expected.lines().count(),
        actual.lines().count(),
    );
    Ok(())
}

/// A schema whose header fields are all constant has a zero-length header,
/// and an empty message a zero-length body, so every length guard sees N = 0
/// and `N > len - offset` can never fire on its own. Each checked entry point
/// must still reject an offset past the buffer with an error, never a panic
/// (or, behind an unchecked read, undefined behaviour).
#[test]
fn zero_length_frame_checked_entry_points_reject_offset_past_buffer() -> Result<(), Box<dyn Error>>
{
    let (_schema, src) = generate(&Paths::zero_length_header_schema(), "zero_length_frame");
    let program = r#"
        fn show<T>(f: impl FnOnce() -> Result<T, String> + std::panic::UnwindSafe) -> String {
            match std::panic::catch_unwind(f) {
                Ok(Ok(_)) => "Ok".to_string(),
                Ok(Err(e)) => format!("Err({e})"),
                Err(_) => "PANIC".to_string(),
            }
        }
        assert_eq!(EmptyDecoder::HEADER_LENGTH, 0);
        let buf = [0u8; 4];
        for offset in [0usize, 4, 5, 100, usize::MAX] {
            let decode = show(|| EmptyDecoder::try_decode(&buf, offset).map_err(|e| format!("{e:?}")));
            let any = show(|| AnyMessage::try_decode(&buf, offset).map_err(|e| format!("{e:?}")));
            let frame = show(|| AnyMessage::decode_frame(&buf, offset, 0).map(|_| ()).map_err(|e| format!("{e:?}")));
            let mut out = [0u8; 4];
            let encode = show(move || {
                EmptyEncoder::try_wrap_and_apply_header(&mut out, offset)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            });
            let encode_body = show(move || {
                EmptyEncoder::try_wrap(&mut out, offset)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            });
            println!("offset {offset}: try_decode {decode}; AnyMessage::try_decode {any}; decode_frame {frame}; try_wrap_and_apply_header {encode}; encoder try_wrap {encode_body}");
        }
    "#;
    let actual = compile_and_run_capture("zero_length_frame", &src, program);
    let past = |offset: usize| {
        let header = "BufferTooShort { field: \"message header\", needed: 0, available: 0 }";
        let frame = "BufferTooShort { field: \"message header+body\", needed: 0, available: 0 }";
        format!(
            "offset {offset}: try_decode Err({header}); AnyMessage::try_decode Err({header}); \
             decode_frame Err({header}); \
             try_wrap_and_apply_header Err({frame}); encoder try_wrap Err({frame})"
        )
    };
    let fits = |offset: usize| {
        format!(
            "offset {offset}: try_decode Ok; AnyMessage::try_decode Ok; decode_frame Ok; \
             try_wrap_and_apply_header Ok; encoder try_wrap Ok"
        )
    };
    let expected = [fits(0), fits(4), past(5), past(100), past(usize::MAX)].join("\n");
    assert_eq!(actual.trim_end(), expected);
    Ok(())
}
