//! Multi-schema **byte-identical wire parity**: ergo-sbe vs official sbe-tool
//! Rust codecs.
//!
//! ## How this works
//!
//! 1. Checked-in sbe-tool reference crates live under
//!    `sbe/tests/sbe_tool_reference/<key>/` (package `parity_<key>`), regenerated
//!    by `scripts/regenerate-sbe-tool-reference.sh` from the vendored
//!    `simple-binary-encoding` submodule.
//! 2. Each test generates ergo-sbe for the same XML, path-depends the matching
//!    reference crate, dual-encodes the same logical payload, and asserts
//!    `ergo_bytes == tool_bytes`.
//!
//! ## Covered schemas
//!
//! | key | schema | notes |
//! |-----|--------|-------|
//! | basic_schema | basic-schema.xml | fixed scalar + declared blockLength padding |
//! | basic_group | basic-group-schema.xml | group 0/N + fixed char[20] |
//! | nested_group | nested-group-schema.xml | 3-level nested groups |
//! | composite_elements | composite-elements-schema.xml | nested composite/enum/set |
//! | issue984 | issue984.xml | versioned fixed strings in group |
//! | baseline | example-schema.xml | Car empty + full-ish |
//! | bigendian | example-bigendian-test-schema.xml | Car BE empty |
//! | nested_composite | nested-composite-name.xml | composite ref field names |
//!
//! Deep Car matrices (enums, extras, stress, Java fixture) live in
//! `sbe_tool_wire_parity_test.rs`.
//!
//! ## Known gaps (schemas vendored, dual-encode deferred)
//!
//! - **group_with_data / basic_var_length**: var-data length field is `uint8`;
//!   ergo currently has a prefix-size mismatch in generated codecs.
//! - **fixed_array**: fixed `i8`/`i16` array DTO codegen issues.
//! - **basic_types**: sbe-tool output for enum-as-field has a `value` bug in
//!   this generator snapshot.
//! - **issue895 / optional_enum_nullify / new_order_single**: API surface still
//!   being aligned; crates are vendored for follow-up.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, unused)]

mod common;
use common::{Paths, dual_encode_run};

fn schema(name: &str) -> std::path::PathBuf {
    Paths::sbe_tool_test_resource(name)
}

// ── basic_schema ──────────────────────────────────────────────────────────

#[test]
fn basic_schema_fixed_scalar_matrix() {
    dual_encode_run(
        "basic_schema_fixed",
        &schema("basic-schema.xml"),
        "basic_schema",
        r###"
        use tool::{
            Encoder, WriteBuf,
            message_header_codec,
            test_message_50001_codec::TestMessage50001Encoder as ToolEnc,
        };

        for tag in [0u32, 1, 42, 0xFFFF_FFFE] {
            let mut ebuf = [0u8; 64];
            let mut e = TestMessage50001Encoder::wrap_and_apply_header(&mut ebuf, 0);
            e.tag40001(tag);
            let el = TestMessage50001Encoder::ENCODED_LENGTH;

            let mut tbuf = [0u8; 64];
            let mut t = ToolEnc::default()
                .wrap(WriteBuf::new(&mut tbuf), message_header_codec::ENCODED_LENGTH);
            t = t.header(0).parent().unwrap();
            t.tag_40001(tag);
            let tl = t.get_limit();
            assert_frames_eq(&format!("basic_schema tag={tag}"), &ebuf[..el], &tbuf[..tl]);
        }
        println!("PASS: basic_schema_fixed_scalar_matrix");
        "###,
    );
}

// ── basic_group ───────────────────────────────────────────────────────────

#[test]
fn basic_group_counts_and_symbols() {
    dual_encode_run(
        "basic_group_matrix",
        &schema("basic-group-schema.xml"),
        "basic_group",
        r###"
        use tool::{
            Encoder, WriteBuf,
            message_header_codec,
            test_message_1_codec::{
                TestMessage1Encoder as ToolEnc,
                encoder::EntriesEncoder as ToolEntries,
            },
        };

        let cases: &[(u32, &[(&str, i64)])] = &[
            (0, &[]),
            (1, &[("ABC", 10)]),
            (7, &[("SYM1", 1), ("SYM2", -2), ("", 0)]),
            (99, &[("ABCDEFGHIJ1234567890", i64::MAX - 1)]),
            (12345, &[("x", 0), ("y", 1), ("z", -1), ("w", 42), ("v", -99)]),
        ];

        for (tag1, entries) in cases {
            let mut ebuf = [0u8; 512];
            let mut e = TestMessage1Encoder::wrap_and_apply_header(&mut ebuf, 0);
            e.tag1(*tag1);
            let e = e.entries(entries.len() as u8, |g| {
                for (sym, v) in *entries {
                    g.add(|ent| {
                        let mut arr = [0u8; 20];
                        let b = sym.as_bytes();
                        let n = b.len().min(20);
                        arr[..n].copy_from_slice(&b[..n]);
                        ent.tag_group1(arr);
                        ent.tag_group2(*v);
                        Ok(())
                    })?;
                }
                Ok(())
            })?;
            let el = e.encoded_length_with_header();

            let mut tbuf = [0u8; 512];
            let mut t = ToolEnc::default()
                .wrap(WriteBuf::new(&mut tbuf), message_header_codec::ENCODED_LENGTH);
            t = t.header(0).parent().unwrap();
            t.tag_1(*tag1);
            let mut entries_enc = ToolEntries::default();
            entries_enc = t.entries_encoder(entries.len() as u8, entries_enc);
            for (i, (sym, v)) in entries.iter().enumerate() {
                assert_eq!(Some(i), entries_enc.advance().unwrap());
                let mut arr = [0u8; 20];
                let b = sym.as_bytes();
                let n = b.len().min(20);
                arr[..n].copy_from_slice(&b[..n]);
                entries_enc.tag_group_1(&arr).tag_group_2(*v);
            }
            t = entries_enc.parent().unwrap();
            let tl = t.get_limit();
            assert_frames_eq(
                &format!("basic_group tag1={tag1} n={}", entries.len()),
                &ebuf[..el],
                &tbuf[..tl],
            );
        }
        println!("PASS: basic_group_counts_and_symbols");
        "###,
    );
}

// ── nested_group ──────────────────────────────────────────────────────────

#[test]
fn nested_group_depth_matrix() {
    dual_encode_run(
        "nested_group_matrix",
        &schema("nested-group-schema.xml"),
        "nested_group",
        r###"
        use tool::{
            Encoder, WriteBuf,
            message_header_codec,
            top_codec::{
                TopEncoder as ToolEnc,
                encoder::{XEncoder as ToolX, YEncoder as ToolY, ZEncoder as ToolZ},
            },
        };

        let shapes = [
            (0u8, 0usize, 0usize, 0usize),
            (1, 1, 0, 0),
            (2, 1, 1, 0),
            (3, 1, 1, 1),
            (4, 2, 2, 2),
            (5, 3, 1, 2),
            (255, 1, 3, 0),
        ];

        for (a, nx, ny, nz) in shapes {
            let mut ebuf = [0u8; 1024];
            let mut e = TopEncoder::wrap_and_apply_header(&mut ebuf, 0);
            e.a(a);
            let e = e.x(nx as u8, |xg| {
                for xi in 0..nx {
                    xg.add(|x| {
                        x.b(xi as u8);
                        x.y(ny as u8, |yg| {
                            for yi in 0..ny {
                                yg.add(|y| {
                                    y.c(yi as u8);
                                    y.z(nz as u8, |zg| {
                                        for zi in 0..nz {
                                            zg.add(|z| {
                                                z.d(zi as u8);
                                                Ok(())
                                            })?;
                                        }
                                        Ok(())
                                    })?;
                                    Ok(())
                                })?;
                            }
                            Ok(())
                        })?;
                        Ok(())
                    })?;
                }
                Ok(())
            })?;
            let el = e.encoded_length_with_header();

            let mut tbuf = [0u8; 1024];
            let mut t = ToolEnc::default()
                .wrap(WriteBuf::new(&mut tbuf), message_header_codec::ENCODED_LENGTH);
            t = t.header(0).parent().unwrap();
            t.a(a);
            let mut x_enc = ToolX::default();
            // sbe-tool mangles single-letter group names: x_encoder → xe_ncoder
            x_enc = t.xe_ncoder(nx as u8, x_enc);
            for xi in 0..nx {
                assert_eq!(Some(xi), x_enc.advance().unwrap());
                x_enc.b(xi as u8);
                let mut y_enc = ToolY::default();
                y_enc = x_enc.ye_ncoder(ny as u8, y_enc);
                for yi in 0..ny {
                    assert_eq!(Some(yi), y_enc.advance().unwrap());
                    y_enc.c(yi as u8);
                    let mut z_enc = ToolZ::default();
                    z_enc = y_enc.ze_ncoder(nz as u8, z_enc);
                    for zi in 0..nz {
                        assert_eq!(Some(zi), z_enc.advance().unwrap());
                        z_enc.d(zi as u8);
                    }
                    y_enc = z_enc.parent().unwrap();
                }
                x_enc = y_enc.parent().unwrap();
            }
            t = x_enc.parent().unwrap();
            let tl = t.get_limit();
            assert_frames_eq(
                &format!("nested a={a} x={nx} y={ny} z={nz}"),
                &ebuf[..el],
                &tbuf[..tl],
            );
        }
        println!("PASS: nested_group_depth_matrix");
        "###,
    );
}

// ── composite_elements ────────────────────────────────────────────────────

#[test]
fn composite_elements_structure_variants() {
    dual_encode_run(
        "composite_elements",
        &schema("composite-elements-schema.xml"),
        "composite_elements",
        r###"
        use tool::{
            Encoder, WriteBuf,
            enum_one::EnumOne as ToolEnum,
            message_header_codec,
            msg_codec::MsgEncoder as ToolEnc,
            set_one::SetOne as ToolSet,
        };

        for (enum_v, zeroth, bit0, bit16, bit26, first, second) in [
            (1u8, 0u8, false, false, false, 0i64, 0i64),
            (1, 1, true, false, false, 1, -1),
            (10, 255, true, true, true, i64::MAX - 1, i64::MIN + 1),
            (1, 7, false, true, false, 100, 200),
        ] {
            let mut ebuf = [0u8; 128];
            let mut e = MsgEncoder::wrap_and_apply_header(&mut ebuf, 0);
            let enum_one = match enum_v {
                10 => EnumOne::Value10,
                _ => EnumOne::Value1,
            };
            let mut set = SetOne::default();
            set.set_bit0(bit0);
            set.set_bit16(bit16);
            set.set_bit26(bit26);
            let structure = Outer::new(enum_one, zeroth, set, Inner::new(first, second));
            e.structure(structure);
            let el = MsgEncoder::ENCODED_LENGTH;

            let mut tbuf = [0u8; 128];
            let mut t = ToolEnc::default()
                .wrap(WriteBuf::new(&mut tbuf), message_header_codec::ENCODED_LENGTH);
            t = t.header(0).parent().unwrap();
            let mut outer = t.structure_encoder();
            outer.enum_one(match enum_v {
                10 => ToolEnum::Value10,
                _ => ToolEnum::Value1,
            });
            outer.zeroth(zeroth);
            let mut set = ToolSet::default();
            set.set_bit_0(bit0).set_bit_16(bit16).set_bit_26(bit26);
            outer.set_one(set);
            let mut inner = outer.inner_encoder();
            inner.first(first).second(second);
            outer = inner.parent().unwrap();
            t = outer.parent().unwrap();
            let tl = t.get_limit();
            assert_frames_eq(
                &format!("composite enum={enum_v} z={zeroth}"),
                &ebuf[..el],
                &tbuf[..tl],
            );
        }
        println!("PASS: composite_elements_structure_variants");
        "###,
    );
}

// ── nested_composite ──────────────────────────────────────────────────────

#[test]
fn nested_composite_ref_field() {
    dual_encode_run(
        "nested_composite",
        &schema("nested-composite-name.xml"),
        "nested_composite",
        r###"
        use tool::{
            Encoder, WriteBuf,
            message_header_codec,
            my_message_codec::MyMessageEncoder as ToolEnc,
        };

        for val in [0u16, 1, 42, 65534] {
            let mut ebuf = [0u8; 64];
            let mut e = MyMessageEncoder::wrap_and_apply_header(&mut ebuf, 0);
            // Field 1 is a messageHeader composite *inside* the body (unusual).
            e.irrelevant_header(MessageHeader::new(
                MyMessageEncoder::BLOCK_LENGTH as u16,
                MyMessageEncoder::TEMPLATE_ID,
                MyMessageEncoder::SCHEMA_ID,
                MyMessageEncoder::SCHEMA_VERSION,
            ));
            e.irrelevant_field(MyComposite::new(MyNestedComposite::new(val)));
            let el = MyMessageEncoder::ENCODED_LENGTH;

            let mut tbuf = [0u8; 64];
            let mut t = ToolEnc::default()
                .wrap(WriteBuf::new(&mut tbuf), message_header_codec::ENCODED_LENGTH);
            t = t.header(0).parent().unwrap();
            {
                let mut h = t.irrelevant_header_encoder();
                h.block_length(tool::my_message_codec::SBE_BLOCK_LENGTH)
                    .template_id(tool::my_message_codec::SBE_TEMPLATE_ID)
                    .schema_id(tool::SBE_SCHEMA_ID)
                    .version(tool::SBE_SCHEMA_VERSION);
                t = h.parent().unwrap();
            }
            {
                let mut c = t.irrelevant_field_encoder();
                let mut n = c.my_field_name_encoder();
                n.irrelevant_field(val);
                c = n.parent().unwrap();
                t = c.parent().unwrap();
            }
            let tl = t.get_limit();
            assert_frames_eq(&format!("nested_composite val={val}"), &ebuf[..el], &tbuf[..tl]);
        }
        println!("PASS: nested_composite_ref_field");
        "###,
    );
}

// ── issue984 ──────────────────────────────────────────────────────────────

#[test]
fn issue984_group_fixed_strings() {
    dual_encode_run(
        "issue984",
        &schema("issue984.xml"),
        "issue984",
        r###"
        use tool::{
            Encoder, WriteBuf,
            message_header_codec,
            simple_message_codec::{
                SimpleMessageEncoder as ToolEnc,
                encoder::MyGroupEncoder as ToolG,
            },
        };

        let cases: &[(u16, &[(&str, &str, &str)])] = &[
            (0, &[]),
            (1, &[("abcd", "efghi", "jklmno")]),
            (2, &[("AAAA", "BBBBB", "CCCCCC"), ("1234", "12345", "123456")]),
            (99, &[("    ", "     ", "      ")]),
        ];

        for (id, rows) in cases {
            let mut ebuf = [0u8; 512];
            let mut e = SimpleMessageEncoder::wrap_and_apply_header(&mut ebuf, 0);
            e.id(*id);
            let e = e.my_group(rows.len() as u8, |g| {
                for (f1, f2, f3) in *rows {
                    g.add(|ent| {
                        let mut a = [b' '; 4];
                        let mut b = [b' '; 5];
                        let mut c = [b' '; 6];
                        a[..f1.len().min(4)].copy_from_slice(&f1.as_bytes()[..f1.len().min(4)]);
                        b[..f2.len().min(5)].copy_from_slice(&f2.as_bytes()[..f2.len().min(5)]);
                        c[..f3.len().min(6)].copy_from_slice(&f3.as_bytes()[..f3.len().min(6)]);
                        ent.f1(a).f2(b).f3(c);
                        Ok(())
                    })?;
                }
                Ok(())
            })?;
            let el = e.encoded_length_with_header();

            let mut tbuf = [0u8; 512];
            let mut t = ToolEnc::default()
                .wrap(WriteBuf::new(&mut tbuf), message_header_codec::ENCODED_LENGTH);
            t = t.header(0).parent().unwrap();
            t.id(*id);
            let mut ge = ToolG::default();
            ge = t.my_group_encoder(rows.len() as u8, ge);
            for (i, (f1, f2, f3)) in rows.iter().enumerate() {
                assert_eq!(Some(i), ge.advance().unwrap());
                let mut a = [b' '; 4];
                let mut b = [b' '; 5];
                let mut c = [b' '; 6];
                a[..f1.len().min(4)].copy_from_slice(&f1.as_bytes()[..f1.len().min(4)]);
                b[..f2.len().min(5)].copy_from_slice(&f2.as_bytes()[..f2.len().min(5)]);
                c[..f3.len().min(6)].copy_from_slice(&f3.as_bytes()[..f3.len().min(6)]);
                ge.f1(&a).f2(&b).f3(&c);
            }
            t = ge.parent().unwrap();
            let tl = t.get_limit();
            assert_frames_eq(&format!("issue984 id={id} n={}", rows.len()), &ebuf[..el], &tbuf[..tl]);
        }
        println!("PASS: issue984_group_fixed_strings");
        "###,
    );
}

// ── baseline Car ──────────────────────────────────────────────────────────

#[test]
fn baseline_car_empty_and_minimal() {
    dual_encode_run(
        "baseline_car_minimal",
        &schema("example-schema.xml"),
        "baseline",
        r###"
        use tool::{
            Encoder, WriteBuf,
            boolean_type::BooleanType as ToolBool,
            boost_type::BoostType as ToolBoost,
            car_codec::encoder::{
                CarEncoder as ToolEnc, FuelFiguresEncoder, PerformanceFiguresEncoder,
            },
            message_header_codec,
            model::Model as ToolModel,
            optional_extras::OptionalExtras as ToolExtras,
        };

        // Empty tails
        {
            let mut ebuf = [0u8; 256];
            let mut e = CarEncoder::wrap_and_apply_header(&mut ebuf, 0);
            e.serial_number(1).model_year(2000).available(BooleanType::F).code(Model::B);
            e.some_numbers([0; 4]).vehicle_code([0; 6]).extras(OptionalExtras::default());
            e.engine(Engine::new(0, 0, [0; 3], 0, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
            let e = e.fuel_figures(0, |_| Ok(()))?;
            let e = e.performance_figures(0, |_| Ok(()))?;
            let e = e.manufacturer(b"")?;
            let e = e.model(b"")?;
            let e = e.activation_code(b"")?;
            let el = e.encoded_length_with_header();

            let mut tbuf = [0u8; 256];
            let mut t = ToolEnc::default()
                .wrap(WriteBuf::new(&mut tbuf), message_header_codec::ENCODED_LENGTH);
            t = t.header(0).parent().unwrap();
            t.serial_number(1)
                .model_year(2000)
                .available(ToolBool::F)
                .code(ToolModel::B)
                .some_numbers(&[0, 0, 0, 0])
                .vehicle_code(&[0; 6])
                .extras(ToolExtras::default());
            let mut eng = t.engine_encoder();
            eng.capacity(0)
                .num_cylinders(0)
                .manufacturer_code(&[0; 3])
                .efficiency(0)
                .booster_enabled(ToolBool::F);
            let mut boost = eng.booster_encoder();
            boost.boost_type(ToolBoost::TURBO).horse_power(0);
            eng = boost.parent().unwrap();
            t = eng.parent().unwrap();
            let mut fuel = FuelFiguresEncoder::default();
            fuel = t.fuel_figures_encoder(0, fuel);
            t = fuel.parent().unwrap();
            let mut perf = PerformanceFiguresEncoder::default();
            perf = t.performance_figures_encoder(0, perf);
            t = perf.parent().unwrap();
            t.manufacturer("").model("").activation_code(b"");
            let tl = t.get_limit();
            assert_frames_eq("baseline empty", &ebuf[..el], &tbuf[..tl]);
        }

        // One fuel entry, empty perf, short var-data
        {
            let mut ebuf = [0u8; 512];
            let mut e = CarEncoder::wrap_and_apply_header(&mut ebuf, 0);
            e.serial_number(99).model_year(2020).available(BooleanType::T).code(Model::C);
            e.some_numbers([9, 8, 7, 6]).vehicle_code(*b"XYZXYZ").extras(OptionalExtras::default());
            e.engine(Engine::new(1600, 4, *b"ABC", 10, BooleanType::F, Booster::new(BoostType::SUPERCHARGER, 50)));
            let e = e.fuel_figures(1, |g| {
                g.add(|ent| {
                    ent.speed(40).mpg(33.3);
                    ent.usage_description(b"city")?;
                    Ok(())
                })?;
                Ok(())
            })?;
            let e = e.performance_figures(0, |_| Ok(()))?;
            let e = e.manufacturer(b"Toyota")?;
            let e = e.model(b"Yaris")?;
            let e = e.activation_code(b"zz")?;
            let el = e.encoded_length_with_header();

            let mut tbuf = [0u8; 512];
            let mut t = ToolEnc::default()
                .wrap(WriteBuf::new(&mut tbuf), message_header_codec::ENCODED_LENGTH);
            t = t.header(0).parent().unwrap();
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
            let mut fuel = FuelFiguresEncoder::default();
            fuel = t.fuel_figures_encoder(1, fuel);
            assert_eq!(Some(0), fuel.advance().unwrap());
            fuel.speed(40).mpg(33.3).usage_description(b"city");
            t = fuel.parent().unwrap();
            let mut perf = PerformanceFiguresEncoder::default();
            perf = t.performance_figures_encoder(0, perf);
            t = perf.parent().unwrap();
            t.manufacturer("Toyota").model("Yaris").activation_code(b"zz");
            let tl = t.get_limit();
            assert_frames_eq("baseline single fuel", &ebuf[..el], &tbuf[..tl]);
        }
        println!("PASS: baseline_car_empty_and_minimal");
        "###,
    );
}

// ── bigendian Car empty ───────────────────────────────────────────────────

#[test]
fn bigendian_car_empty() {
    dual_encode_run(
        "bigendian_car_empty",
        &schema("example-bigendian-test-schema.xml"),
        "bigendian",
        r###"
        use tool::{
            Encoder, WriteBuf,
            boolean_type::BooleanType as ToolBool,
            boost_type::BoostType as ToolBoost,
            car_codec::encoder::{
                CarEncoder as ToolEnc, FuelFiguresEncoder, PerformanceFiguresEncoder,
            },
            message_header_codec,
            model::Model as ToolModel,
            optional_extras::OptionalExtras as ToolExtras,
        };

        // BE schema someNumbers length may be 5 — detect from encoder constants / API.
        let mut ebuf = [0u8; 256];
        let mut e = CarEncoder::wrap_and_apply_header(&mut ebuf, 0);
        e.serial_number(1).model_year(2000).available(BooleanType::F).code(Model::B);
        // Probe array size via meta / try both? Use length from type.
        // Read generated: some_numbers takes [u32; N]
        e.some_numbers([0; 5]);
        e.vehicle_code([0; 6]).extras(OptionalExtras::default());
        e.engine(Engine::new(0, 0, [0; 3], 0, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let e = e.fuel_figures(0, |_| Ok(()))?;
        let e = e.performance_figures(0, |_| Ok(()))?;
        let e = e.manufacturer(b"")?;
        let e = e.model(b"")?;
        let e = e.activation_code(b"")?;
        let el = e.encoded_length_with_header();

        let mut tbuf = [0u8; 256];
        let mut t = ToolEnc::default()
            .wrap(WriteBuf::new(&mut tbuf), message_header_codec::ENCODED_LENGTH);
        t = t.header(0).parent().unwrap();
        t.serial_number(1)
            .model_year(2000)
            .available(ToolBool::F)
            .code(ToolModel::B)
            .some_numbers(&[0, 0, 0, 0, 0])
            .vehicle_code(&[0; 6])
            .extras(ToolExtras::default());
        let mut eng = t.engine_encoder();
        eng.capacity(0)
            .num_cylinders(0)
            .manufacturer_code(&[0; 3])
            .efficiency(0)
            .booster_enabled(ToolBool::F);
        let mut boost = eng.booster_encoder();
        boost.boost_type(ToolBoost::TURBO).horse_power(0);
        eng = boost.parent().unwrap();
        t = eng.parent().unwrap();
        let mut fuel = FuelFiguresEncoder::default();
        fuel = t.fuel_figures_encoder(0, fuel);
        t = fuel.parent().unwrap();
        let mut perf = PerformanceFiguresEncoder::default();
        perf = t.performance_figures_encoder(0, perf);
        t = perf.parent().unwrap();
        // BE schema activation_code may be &str
        t.manufacturer("").model("").activation_code("");
        let tl = t.get_limit();
        assert_frames_eq("bigendian empty", &ebuf[..el], &tbuf[..tl]);
        println!("PASS: bigendian_car_empty");
        "###,
    );
}

// ── reference crates exist ────────────────────────────────────────────────

#[test]
fn all_vendored_reference_crates_have_manifests() {
    let keys = [
        "baseline",
        "basic_schema",
        "basic_group",
        "nested_group",
        "composite_elements",
        "issue984",
        "bigendian",
        "nested_composite",
        "group_with_data",
        "basic_var_length",
        "fixed_array",
        "basic_types",
        "issue435",
        "issue895",
        "issue972",
        "issue987",
        "issue1028",
        "issue1057",
        "issue1066",
        "optional_enum_nullify",
        "new_order_single",
        "code_generation",
        "dto_test",
        "extension",
        "bench_car",
        "fix_messages",
        "value_ref",
        "block_length",
        "embedded_length",
        "encoding_types",
    ];
    for k in keys {
        let p = Paths::sbe_tool_reference(k).join("Cargo.toml");
        assert!(p.is_file(), "missing vendored sbe-tool crate: {p:?}");
    }
}
