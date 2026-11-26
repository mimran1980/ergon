//! A group-writing closure keeps the group it is handed.
//!
//! A group-writing method (message or entry level) hands its closure a group
//! built over the frame's own buffer. If the closure could swap that group out
//! and keep it, safe code would hold two live `&mut` to one buffer; if it could
//! assign another group in, the method would carry on with that group's cursor
//! and count and give a wrong frame. So the closure takes the group for a
//! lifetime of its own (`for<'b>`), and the group is *attached*: only generated
//! code builds one, and the public `wrap` returns a detached group that cannot
//! be assigned in its place. Decoys are over leaked `'static` buffers, so the
//! group's type is the only thing that can fail to compile.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
use std::error::Error;

mod common;
use common::{Paths, compile_fails_with_diagnostics_excluding, generate};

const FIXED: &str = r#"
        let extras = OptionalExtras::default();
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras,
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_g| Ok(())).unwrap();
"#;

/// Swap a decoy in through `method`, keep the group that came out, and write
/// through it after the entry's completion exists.
fn escape_body(method: &str, count_arg: &str) -> String {
    format!(
        r#"
        let mut buf = [0u8; 512];
        {FIXED}
        let _ = car.performance_figures(1, |g| {{
            g.add(|mut e| {{
                e.octane_rating(95);
                let mut stolen = None;
                let complete = e.{method}({count_arg}|a| {{
                    let other: &'static mut [u8] = Box::leak(Box::new([0u8; 64]));
                    let mut decoy = PerformanceFiguresAccelerationEncoder::wrap(other, 0, 1);
                    decoy.add(|x| {{ x.mph(1).seconds(1.0); Ok(()) }})?;
                    stolen = Some(core::mem::replace(a, decoy));
                    Ok(())
                }})?;
                let mut s = stolen.take().unwrap();
                s.add(|x| {{ x.mph(0xBEEF).seconds(9.0); Ok(()) }})?;
                Ok(complete)
            }})?;
            Ok(())
        }});
        "#
    )
}

#[test]
fn nested_group_cannot_be_swapped_out_and_kept() -> Result<(), Box<dyn Error>> {
    let (_, src) = generate(&Paths::example_schema(), "nge_escape");
    compile_fails_with_diagnostics_excluding(
        "nge_escape",
        &src,
        &escape_body("acceleration", "1, "),
        &["mismatched types", "Detached"],
        &["E0597"],
    );
    Ok(())
}

#[test]
fn nested_unknown_size_group_cannot_be_swapped_out_and_kept() -> Result<(), Box<dyn Error>> {
    let (_, src) = generate(&Paths::example_schema(), "nge_escape_unknown");
    compile_fails_with_diagnostics_excluding(
        "nge_escape_unknown",
        &src,
        &escape_body("acceleration_unknown_size", ""),
        &["mismatched types", "Detached"],
        &["E0597"],
    );
    Ok(())
}

/// The same holds at an intermediate stage: the second nested group of an
/// entry, reached only after the first one.
#[test]
fn nested_group_at_an_intermediate_stage_cannot_be_swapped_out_and_kept()
-> Result<(), Box<dyn Error>> {
    let (_, src) = generate(&Paths::multi_nested_group_schema(), "nge_escape_mid");
    compile_fails_with_diagnostics_excluding(
        "nge_escape_mid",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let mut g = ParentEncoder::wrap(&mut buf, 0, 1);
        let _ = g.add(|mut e| {
            e.parent_field(1);
            let mut stolen = None;
            let after = e.child_a(0, |_| Ok(()))?.child_b(1, |b| {
                let other: &'static mut [u8] = Box::leak(Box::new([0u8; 64]));
                let mut decoy = ParentChildBEncoder::wrap(other, 0, 1);
                decoy.add(|x| { x.value_b(1); Ok(()) })?;
                stolen = Some(core::mem::replace(b, decoy));
                Ok(())
            })?;
            let mut s = stolen.take().unwrap();
            s.add(|x| { x.value_b(9); Ok(()) })?;
            after.note1(b"")?.note2(b"")
        });
        "#,
        &["mismatched types", "Detached"],
        &["E0597"],
    );
    Ok(())
}

/// Assign a group from the public `wrap` (over a leaked buffer) into the group
/// a nested-group method hands its closure. Its cursor and count would then
/// stand in for the entry's.
fn nested_swap_in_body(method: &str, count_arg: &str) -> String {
    format!(
        r#"
        let mut buf = [0u8; 512];
        {FIXED}
        let _ = car.performance_figures(1, |g| {{
            g.add(|mut e| {{
                e.octane_rating(95);
                e.{method}({count_arg}|a| {{
                    let other: &'static mut [u8] = Box::leak(Box::new([0u8; 64]));
                    *a = PerformanceFiguresAccelerationEncoder::wrap(other, 30, 1);
                    Ok(())
                }})
            }})?;
            Ok(())
        }});
        "#
    )
}

#[test]
fn nested_group_cannot_be_replaced_by_a_wrapped_group() -> Result<(), Box<dyn Error>> {
    let (_, src) = generate(&Paths::example_schema(), "nge_swap_in");
    compile_fails_with_diagnostics_excluding(
        "nge_swap_in",
        &src,
        &nested_swap_in_body("acceleration", "1, "),
        &["mismatched types", "Detached"],
        &["E0597"],
    );
    Ok(())
}

#[test]
fn nested_unknown_size_group_cannot_be_replaced_by_a_wrapped_group() -> Result<(), Box<dyn Error>> {
    let (_, src) = generate(&Paths::example_schema(), "nge_swap_in_unknown");
    compile_fails_with_diagnostics_excluding(
        "nge_swap_in_unknown",
        &src,
        &nested_swap_in_body("acceleration_unknown_size", ""),
        &["mismatched types", "Detached"],
        &["E0597"],
    );
    Ok(())
}

/// The same at message level: the next stage would carry on in the other
/// buffer, and `send` would commit a frame whose tail was never written.
fn message_swap_in_body(method: &str, count_arg: &str) -> String {
    format!(
        r#"
        let mut buf = [0u8; 512];
        {FIXED}
        let _ = car.{method}({count_arg}|g| {{
            let other: &'static mut [u8] = Box::leak(Box::new([0u8; 64]));
            *g = PerformanceFiguresEncoder::wrap(other, 30, 0);
            Ok(())
        }});
        "#
    )
}

#[test]
fn message_group_cannot_be_replaced_by_a_wrapped_group() -> Result<(), Box<dyn Error>> {
    let (_, src) = generate(&Paths::example_schema(), "nge_msg_swap_in");
    compile_fails_with_diagnostics_excluding(
        "nge_msg_swap_in",
        &src,
        &message_swap_in_body("performance_figures", "0, "),
        &["mismatched types", "Detached"],
        &["E0597"],
    );
    Ok(())
}

#[test]
fn message_unknown_size_group_cannot_be_replaced_by_a_wrapped_group() -> Result<(), Box<dyn Error>>
{
    let (_, src) = generate(&Paths::example_schema(), "nge_msg_swap_in_unknown");
    compile_fails_with_diagnostics_excluding(
        "nge_msg_swap_in_unknown",
        &src,
        &message_swap_in_body("performance_figures_unknown_size", ""),
        &["mismatched types", "Detached"],
        &["E0597"],
    );
    Ok(())
}

/// Two encodes of one message in flight at once must not trade groups: each
/// message's tail would land in the other's buffer.
#[test]
fn message_groups_of_two_encodes_cannot_be_swapped() -> Result<(), Box<dyn Error>> {
    let (_, src) = generate(&Paths::example_schema(), "nge_msg_swap_two");
    let fixed = |var: &str, buf: &str| {
        FIXED
            .replace("let car =", &format!("let {var} ="))
            .replace("(&mut buf,", &format!("(&mut {buf},"))
            .replace("car.fuel_figures", &format!("{var}.fuel_figures"))
    };
    let body = format!(
        r#"
        let mut buf1 = [0u8; 512];
        let mut buf2 = [0u8; 512];
        {}
        {}
        let _ = car1.performance_figures(0, move |g1| {{
            let _ = car2.performance_figures(0, |g2| {{
                core::mem::swap(g1, g2);
                Ok(())
            }});
            Ok(())
        }});
        "#,
        fixed("car1", "buf1"),
        fixed("car2", "buf2"),
    );
    compile_fails_with_diagnostics_excluding(
        "nge_msg_swap_two",
        &src,
        &body,
        &["borrowed data escapes outside of closure"],
        &["E0597", "E0308"],
    );
    Ok(())
}

/// Nor the nested groups of two encodes in flight: an attached group of the
/// same type is the one replacement a closure could reach, and lifetimes of
/// their own keep the two apart.
#[test]
fn nested_groups_of_two_encodes_cannot_be_swapped() -> Result<(), Box<dyn Error>> {
    let (_, src) = generate(&Paths::example_schema(), "nge_nested_swap_two");
    let fixed = |var: &str, buf: &str| {
        FIXED
            .replace("let car =", &format!("let {var} ="))
            .replace("(&mut buf,", &format!("(&mut {buf},"))
            .replace("car.fuel_figures", &format!("{var}.fuel_figures"))
    };
    let body = format!(
        r#"
        let mut buf1 = [0u8; 512];
        let mut buf2 = [0u8; 512];
        {}
        {}
        let _ = car1.performance_figures(1, move |g1| {{
            g1.add(move |e1| {{
                e1.acceleration(0, move |a1| {{
                    let _ = car2.performance_figures(1, |g2| {{
                        g2.add(|e2| {{
                            e2.acceleration(0, |a2| {{
                                core::mem::swap(a1, a2);
                                Ok(())
                            }})
                        }})
                    }});
                    Ok(())
                }})
            }})
        }});
        "#,
        fixed("car1", "buf1"),
        fixed("car2", "buf2"),
    );
    compile_fails_with_diagnostics_excluding(
        "nge_nested_swap_two",
        &src,
        &body,
        &["borrowed data escapes outside of closure"],
        &["E0597", "E0308"],
    );
    Ok(())
}
