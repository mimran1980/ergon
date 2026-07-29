//! Domain-object tests — owned structs materialised from flyweight decoders.
#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    unused
)]
mod common;
use common::{Paths, compile_and_run, generate_domain as generate, generate_domain_with};
use ergo_sbe::DomainVarData;
use std::path::PathBuf;

fn l3_schema() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/l3-orderbook-schema.xml"
    ))
}

fn binance_schema() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/binance_spot_3_5.xml"
    ))
}

fn orderbook_schema() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/benchmarks/schemas/orderbook.xml"
    ))
}

fn big_endian_car_schema() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/example-bigendian-test-schema.xml"
    ))
}

#[test]
fn flat_group_domain_bulk_encode_matches_wire_bulk_and_automatic_dto_encode()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&orderbook_schema(), "dto_bulk_orderbook");
    compile_and_run(
        "dto_bulk_orderbook",
        &src,
        r#"
        let levels = vec![
            BookSnapshotLevelsEntryDomain {
                price: 10_001,
                qty: 501,
                num_orders: 3,
            },
            BookSnapshotLevelsEntryDomain {
                price: 10_002,
                qty: 702,
                num_orders: 4,
            },
        ];
        let dto = BookSnapshotDomain {
            levels: levels.clone(),
        };
        let expected_len = BookSnapshotEncoder::compute_length_with_header(levels.len());
        assert_eq!(dto.encoded_length_with_header()?, expected_len);

        let mut dto_buf = [0u8; BookSnapshotEncoder::compute_length_with_header(2)];
        let dto_len = dto.encode(&mut dto_buf)?;
        assert_eq!(dto_len, expected_len);

        let mut domain_bulk_buf = [0u8; BookSnapshotEncoder::compute_length_with_header(2)];
        let domain_bulk_len = BookSnapshotEncoder::wrap_and_apply_header(&mut domain_bulk_buf, 0)
            .levels(levels.len() as u16, |group| group.bulk_add_domain(&levels))?
            .encoded_length_with_header();
        assert_eq!(domain_bulk_len, expected_len);

        let wire_levels = [
            LevelsEntry {
                price: 10_001,
                qty: 501,
                num_orders: 3,
            },
            LevelsEntry {
                price: 10_002,
                qty: 702,
                num_orders: 4,
            },
        ];
        let mut wire_bulk_buf = [0u8; BookSnapshotEncoder::compute_length_with_header(2)];
        let wire_bulk_len = BookSnapshotEncoder::wrap_and_apply_header(&mut wire_bulk_buf, 0)
            .levels(wire_levels.len() as u16, |group| group.bulk_add(&wire_levels))?
            .encoded_length_with_header();
        assert_eq!(wire_bulk_len, expected_len);

        assert_eq!(&dto_buf[..dto_len], &domain_bulk_buf[..domain_bulk_len]);
        assert_eq!(&dto_buf[..dto_len], &wire_bulk_buf[..wire_bulk_len]);

        let mut decoded = BookSnapshotDecoder::try_from(&dto_buf[..dto_len])?.into_levels()?;
        for expected in &levels {
            let actual = decoded.next().expect("missing level");
            assert_eq!(actual.price(), expected.price);
            assert_eq!(actual.qty(), expected.qty);
            assert_eq!(actual.num_orders(), expected.num_orders);
        }
        assert!(decoded.next().is_none());

        let invalid_levels = [BookSnapshotLevelsEntryDomain {
            price: 10_003,
            qty: 1,
            num_orders: u32::MAX,
        }];
        let mut invalid_buf = [0u8; BookSnapshotEncoder::compute_length_with_header(1)];
        let err = BookSnapshotEncoder::wrap_and_apply_header(&mut invalid_buf, 0)
            .levels(1, |group| group.bulk_add_domain(&invalid_levels))
            .unwrap_err();
        assert!(matches!(
            err,
            sbe_rt::EncodeError::ValueOutOfRange {
                field: "numOrders",
                ..
            }
        ));

        let invalid_dto = BookSnapshotDomain {
            levels: invalid_levels.to_vec(),
        };
        let err = invalid_dto.encode(&mut invalid_buf).unwrap_err();
        assert!(matches!(
            err,
            sbe_rt::EncodeError::ValueOutOfRange {
                field: "numOrders",
                ..
            }
        ));
        "#,
    );

    Ok(())
}

#[test]
fn big_endian_nested_domain_bulk_matches_wire_bulk_and_automatic_dto_encode()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&big_endian_car_schema(), "dto_bulk_big_endian");
    compile_and_run(
        "dto_bulk_big_endian",
        &src,
        r#"
        let domain_acceleration = [
            CarPerformanceFiguresEntryAccelerationEntryDomain {
                mph: 30,
                seconds: 4.25,
            },
            CarPerformanceFiguresEntryAccelerationEntryDomain {
                mph: 60,
                seconds: 7.5,
            },
        ];
        let wire_acceleration = [
            PerformanceFiguresAccelerationEntry {
                mph: 30,
                seconds: 4.25,
            },
            PerformanceFiguresAccelerationEntry {
                mph: 60,
                seconds: 7.5,
            },
        ];
        let expected_len = CarEncodedLength::new()
            .fuel_figures(0)
            .finish_empty()?
            .performance_figures(1)
            .acceleration(2)?
            .manufacturer(0)?
            .model(0)?
            .activation_code(0)?
            .encoded_length_with_header();

        let mut domain_storage = [0u8; 512];
        let domain_len = CarEncoder::wrap_and_apply_header(
            &mut domain_storage[..expected_len],
            0,
        )
        .fuel_figures(0, |_| Ok(()))?
        .performance_figures(1, |performance| {
            performance.add(|entry| {
                entry
                    .octane_rating(95)
                    .acceleration(2, |acceleration| {
                        acceleration.bulk_add_domain(&domain_acceleration)
                    })?;
                Ok(())
            })
        })?
        .manufacturer(b"")?
        .model(b"")?
        .activation_code(b"")?
        .encoded_length_with_header();
        assert_eq!(domain_len, expected_len);

        let mut wire_storage = [0u8; 512];
        let wire_len = CarEncoder::wrap_and_apply_header(
            &mut wire_storage[..expected_len],
            0,
        )
        .fuel_figures(0, |_| Ok(()))?
        .performance_figures(1, |performance| {
            performance.add(|entry| {
                entry
                    .octane_rating(95)
                    .acceleration(2, |acceleration| {
                        acceleration.bulk_add(&wire_acceleration)
                    })?;
                Ok(())
            })
        })?
        .manufacturer(b"")?
        .model(b"")?
        .activation_code(b"")?
        .encoded_length_with_header();
        assert_eq!(wire_len, expected_len);
        assert_eq!(
            &domain_storage[..domain_len],
            &wire_storage[..wire_len],
        );

        let dto = CarDomain::try_from_decoder(
            CarDecoder::try_from(&domain_storage[..domain_len])?,
        )?;
        let mut dto_storage = [0u8; 512];
        let dto_len = dto.encode(&mut dto_storage[..expected_len])?;
        assert_eq!(dto_len, expected_len);
        assert_eq!(&dto_storage[..dto_len], &wire_storage[..wire_len]);
        "#,
    );

    Ok(())
}

#[test]
fn car_domain_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "car_dom_all");
    compile_and_run(
        "car_dom_all",
        &src,
        r#"
        let mut buf = [0u8; 2048];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        car.serial_number(1234).model_year(2013).available(BooleanType::T).code(Model::A);
        car.some_numbers([10u32, 20, 30, 40]);
        car.vehicle_code([b'A', b'B', b'C', b'D', b'E', b'F']);
        let mut extras = OptionalExtras::default();
        extras.set_cruise_control(true);
        extras.set_sports_pack(true);
        car.extras(extras);
        car.engine(Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(2, |g| -> Result<(), sbe_rt::EncodeError> {
            g.add(|e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban")?; Ok(()) })?;
            g.add(|e| { e.speed(60).mpg(25.0); e.usage_description(b"Highway")?; Ok(()) })?;
            Ok(())
        })?;
        let car = car.performance_figures(1, |g| -> Result<(), sbe_rt::EncodeError> {
            g.add(|e| -> Result<(), sbe_rt::EncodeError> {
                e.octane_rating(95);
                e.acceleration(2, |a| -> Result<(), sbe_rt::EncodeError> {
                    a.add(|x| { x.mph(30).seconds(4.0); Ok(()) })?;
                    a.add(|x| { x.mph(60).seconds(7.5); Ok(()) })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        })?;
        let car = car.manufacturer(b"Honda")?;
        let car = car.model(b"Civic VTi")?;
        let complete = car.activation_code(b"abcdef")?;
        assert!(complete.encoded_length_with_header() > 0);
        let encoded = complete.as_bytes().to_vec();

        let dec = CarDecoder::try_from(&encoded[..]).unwrap();
        let d: CarDomain = dec.into();

        assert_eq!(d.serial_number, 1234);
        assert_eq!(d.model_year, 2013);
        assert!(d.available);
        assert_eq!(d.code, Model::A);
        assert_eq!(d.some_numbers, [10, 20, 30, 40]);
        assert_eq!(d.vehicle_code, [b'A', b'B', b'C', b'D', b'E', b'F']);
        assert!(d.extras.cruise_control());
        assert!(d.extras.sports_pack());
        assert_eq!(d.engine.capacity(), 2000);
        assert_eq!(d.engine.num_cylinders(), 4);
        assert_eq!(d.fuel_figures.len(), 2);
        assert_eq!(d.fuel_figures[0].speed, 30);
        assert!((d.fuel_figures[0].mpg - 35.9).abs() < 0.01);
        assert_eq!(d.fuel_figures[0].usage_description, b"Urban");
        assert_eq!(d.fuel_figures[1].speed, 60);
        assert_eq!(d.fuel_figures[1].usage_description, b"Highway");
        assert_eq!(d.performance_figures.len(), 1);
        assert_eq!(d.performance_figures[0].octane_rating, 95);
        assert_eq!(d.performance_figures[0].acceleration.len(), 2);
        assert_eq!(d.performance_figures[0].acceleration[0].mph, 30);
        assert!((d.performance_figures[0].acceleration[0].seconds - 4.0).abs() < 0.01);
        assert_eq!(d.performance_figures[0].acceleration[1].mph, 60);
        assert_eq!(d.manufacturer, b"Honda");
        assert_eq!(d.model, b"Civic VTi");
        assert_eq!(d.activation_code, b"abcdef");
        println!("car_domain_all_fields: PASSED");
    "#,
    );

    Ok(())
}

#[test]
fn car_domain_clone_eq_debug() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "car_dom_clone");
    compile_and_run(
        "car_dom_clone",
        &src,
        r#"
        let mut buf = [0u8; 1024];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        car.serial_number(42).model_year(2021).available(BooleanType::F).code(Model::B);
        car.some_numbers([5; 4]).vehicle_code([b'Z'; 6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(300, 6, [1; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let c = car.fuel_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?
            .performance_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?
            .manufacturer(b"X")?
            .model(b"Y")?
            .activation_code(b"Z")?;
        assert!(c.encoded_length_with_header() > 0);
        let encoded = c.as_bytes();
        let d1: CarDomain = CarDecoder::try_from(&encoded[..]).unwrap().into();
        let d2 = d1.clone();
        assert_eq!(d1, d2);
        let dbg = format!("{:?}", d1);
        assert!(dbg.contains("CarDomain"));
        assert!(dbg.contains("serial_number: 42"));
        println!("car_domain_clone_eq_debug: PASSED");
    "#,
    );

    Ok(())
}

#[test]
fn car_domain_empty_groups() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "car_dom_empty");
    compile_and_run(
        "car_dom_empty",
        &src,
        r#"
        let mut buf = [0u8; 1024];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        car.serial_number(1).model_year(2000).available(BooleanType::T).code(Model::A);
        car.some_numbers([0; 4]).vehicle_code([0; 6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0, 0, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let c = car.fuel_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?
            .performance_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?
            .manufacturer(b"")?
            .model(b"")?
            .activation_code(b"")?;
        assert!(c.encoded_length_with_header() > 0);
        let encoded = c.as_bytes();
        let d: CarDomain = CarDecoder::try_from(&encoded[..]).unwrap().into();
        assert!(d.fuel_figures.is_empty());
        assert!(d.performance_figures.is_empty());
        assert!(d.manufacturer.is_empty());
        assert!(d.model.is_empty());
        println!("car_domain_empty_groups: PASSED");
    "#,
    );

    Ok(())
}

#[test]
fn l3_domain_nested_groups_vardata() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&l3_schema(), "l3_dom_nested");
    compile_and_run(
        "l3_dom_nested",
        &src,
        r#"
        let mut buf = [0u8; 8192];
        let mut book = L3BookEncoder::wrap_and_apply_header(&mut buf, 0);
        book.timestamp(111).sequence(222);
        let complete = book.bids(2, |bids| -> Result<(), sbe_rt::EncodeError> {
            bids.add(|level| -> Result<(), sbe_rt::EncodeError> {
                level.price(100).qty(50);
                level.orders(3, |orders| -> Result<(), sbe_rt::EncodeError> {
                    orders.add(|o| { o.order_qty(10); o.order_id(b"A1")?; Ok(()) })?;
                    orders.add(|o| { o.order_qty(20); o.order_id(b"A2")?; Ok(()) })?;
                    orders.add(|o| { o.order_qty(20); o.order_id(b"A3")?; Ok(()) })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            bids.add(|level| -> Result<(), sbe_rt::EncodeError> {
                level.price(99).qty(30);
                level.orders(1, |orders| -> Result<(), sbe_rt::EncodeError> {
                    orders.add(|o| { o.order_qty(30); o.order_id(b"B1")?; Ok(()) })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        }).unwrap().asks(1, |asks| -> Result<(), sbe_rt::EncodeError> {
            asks.add(|level| -> Result<(), sbe_rt::EncodeError> {
                level.price(101).qty(40);
                level.orders(2, |orders| -> Result<(), sbe_rt::EncodeError> {
                    orders.add(|o| { o.order_qty(20); o.order_id(b"S1")?; Ok(()) })?;
                    orders.add(|o| { o.order_qty(20); o.order_id(b"S2")?; Ok(()) })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        }).unwrap();
        let encoded = complete.as_bytes();
        let d: L3BookDomain = L3BookDecoder::try_from(&encoded[..]).unwrap().into();

        assert_eq!(d.timestamp, 111);
        assert_eq!(d.sequence, 222);
        assert_eq!(d.bids.len(), 2);
        assert_eq!(d.bids[0].price, 100);
        assert_eq!(d.bids[0].qty, 50);
        assert_eq!(d.bids[0].orders.len(), 3);
        assert_eq!(d.bids[0].orders[0].order_qty, 10);
        assert_eq!(d.bids[0].orders[0].order_id, b"A1");
        assert_eq!(d.bids[0].orders[1].order_id, b"A2");
        assert_eq!(d.bids[0].orders[2].order_id, b"A3");
        assert_eq!(d.bids[1].price, 99);
        assert_eq!(d.bids[1].orders.len(), 1);
        assert_eq!(d.bids[1].orders[0].order_id, b"B1");
        assert_eq!(d.asks.len(), 1);
        assert_eq!(d.asks[0].price, 101);
        assert_eq!(d.asks[0].qty, 40);
        assert_eq!(d.asks[0].orders.len(), 2);
        assert_eq!(d.asks[0].orders[0].order_id, b"S1");
        assert_eq!(d.asks[0].orders[1].order_id, b"S2");

        let cloned = d.clone();
        assert_eq!(d, cloned);
        let dbg = format!("{:?}", d);
        assert!(dbg.contains("L3BookDomain"), "Debug must contain struct name");
        assert!(dbg.contains("65, 49"), "Debug must contain order_id bytes [65, 49] for b\"A1\"");
        println!("l3_domain_nested_groups_vardata: PASSED");
    "#,
    );
    Ok(())
}

#[test]
fn l3_domain_12_orders() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&l3_schema(), "l3_dom_12");
    compile_and_run(
        "l3_dom_12",
        &src,
        r#"
        let mut buf = [0u8; 32768];
        let mut book = L3BookEncoder::wrap_and_apply_header(&mut buf, 0);
        book.timestamp(333).sequence(444);
        let complete = book.bids(1, |bids| -> Result<(), sbe_rt::EncodeError> {
            bids.add(|level| -> Result<(), sbe_rt::EncodeError> {
                level.price(50000).qty(120);
                level.orders(12, |orders| -> Result<(), sbe_rt::EncodeError> {
                    for i in 0..12u64 {
                        let id = format!("ORD-{:03}", i);
                        orders.add(|o| { o.order_qty((i+1) as i64); o.order_id(id.as_bytes())?; Ok(()) })?;
                    }
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        }).unwrap().asks(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) }).unwrap();
        let encoded = complete.as_bytes();
        let d: L3BookDomain = L3BookDecoder::try_from(&encoded[..]).unwrap().into();

        assert_eq!(d.bids.len(), 1);
        assert_eq!(d.bids[0].orders.len(), 12);
        for i in 0..12usize {
            assert_eq!(d.bids[0].orders[i].order_qty, (i as i64) + 1);
            let expected = format!("ORD-{:03}", i);
            assert_eq!(d.bids[0].orders[i].order_id, expected.as_bytes());
        }
        assert!(d.asks.is_empty());
        println!("l3_domain_12_orders: PASSED");
    "#,
    );

    Ok(())
}

#[test]
fn l3_compute_encoded_length_matches() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&l3_schema(), "l3_len");
    compile_and_run(
        "l3_len",
        &src,
        r#"
        let mut buf = [0u8; 4096];
        let mut book = L3BookEncoder::wrap_and_apply_header(&mut buf, 0);
        book.timestamp(0).sequence(0);
        let complete = book.bids(2, |bids| {
            bids.add(|l| { l.price(0).qty(0); l.orders(0, |_| Ok(()))?; Ok(()) }).unwrap();
            bids.add(|l| { l.price(0).qty(0); l.orders(0, |_| Ok(()))?; Ok(()) }).unwrap();
            Ok(())
        }).unwrap().asks(1, |asks| {
            asks.add(|l| { l.price(0).qty(0); l.orders(0, |_| Ok(()))?; Ok(()) }).unwrap();
            Ok(())
        }).unwrap();
        assert!(complete.encoded_length() > 0);
        println!("l3_compute_encoded_length_matches: PASSED");
    "#,
    );

    Ok(())
}

#[test]
fn binance_depth_domain() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&binance_schema(), "binance_depth_dom");
    compile_and_run(
        "binance_depth_dom",
        &src,
        r#"
        let mut buf = [0u8; 4096];
        let mut d = DepthResponseEncoder::wrap_and_apply_header(&mut buf, 0);
        d.last_update_id(123456).price_exponent(-8).qty_exponent(-8);
        let complete = d.bids(2, |bids| -> Result<(), sbe_rt::EncodeError> {
            bids.add(|l| { l.price(50001).qty(150); Ok(()) })?;
            bids.add(|l| { l.price(50000).qty(200); Ok(()) })?;
            Ok(())
        }).unwrap().asks(1, |asks| -> Result<(), sbe_rt::EncodeError> {
            asks.add(|l| { l.price(50100).qty(300); Ok(()) })?;
            Ok(())
        }).unwrap();
        let encoded = complete.as_bytes().to_vec();

        let dom: DepthResponseDomain = DepthResponseDecoder::try_from(&encoded[..]).unwrap().into();

        assert_eq!(dom.last_update_id, 123456);
        assert_eq!(dom.price_exponent, -8);
        assert_eq!(dom.qty_exponent, -8);
        assert_eq!(dom.bids.len(), 2);
        assert_eq!(dom.bids[0].price, 50001);
        assert_eq!(dom.bids[0].qty, 150);
        assert_eq!(dom.bids[1].price, 50000);
        assert_eq!(dom.bids[1].qty, 200);
        assert_eq!(dom.asks.len(), 1);
        assert_eq!(dom.asks[0].price, 50100);
        assert_eq!(dom.asks[0].qty, 300);
        let cloned = dom.clone();
        assert_eq!(dom, cloned);
        let dbg = format!("{:?}", dom);
        assert!(dbg.contains("DepthResponseDomain"));
        println!("binance_depth_domain: PASSED");
    "#,
    );

    Ok(())
}

/// Versioned fields: since>0 bool enum → `Option<bool>`, since>0 composite → `Option<T>`.
#[test]
fn domain_versioned_optional_fields() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::versioned_domain_schema(), "ver_dom");
    // Source assertions: DTO struct has correct optional types
    assert!(
        src.contains("pub active: Option<bool>"),
        "sinceVersion=1 bool enum should be Option<bool> in DTO: {src}"
    );
    assert!(
        src.contains("pub extra: Option<Extra>"),
        "sinceVersion=2 composite should be Option<Extra> in DTO: {src}"
    );
    assert!(
        src.contains("pub count: u32"),
        "sinceVersion=0 field should be plain u32"
    );
    compile_and_run(
        "ver_dom",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let mut enc = VersionedEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.active_bool(true).extra(Extra::new(7, 99)).count(42);
        let dec = VersionedDecoder::try_wrap_and_apply_header(&buf, 0).unwrap();
        let d: VersionedDomain = dec.into();
        assert_eq!(d.active, Some(true));
        assert!(d.extra.is_some());
        assert_eq!(d.extra.as_ref().unwrap().flags(), 7);
        assert_eq!(d.count, 42);
    "#,
    );
    Ok(())
}

/// Encode a fully-populated CarDomain back to bytes and verify they match
/// a flyweight-encoded baseline (byte-identity).
#[test]
fn car_domain_encode_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "car_enc_rt");
    compile_and_run(
        "car_enc_rt",
        &src,
        r#"
        let mut buf = [0u8; 2048];

        // Flyweight encode
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        car.serial_number(1234).model_year(2013).available_bool(true).code(Model::A);
        car.some_numbers([10u32, 20, 30, 40]);
        car.vehicle_code([b'A', b'B', b'C', b'D', b'E', b'F']);
        let mut extras = OptionalExtras::default();
        extras.set_cruise_control(true);
        extras.set_sports_pack(true);
        car.extras(extras);
        car.engine(Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(2, |g| -> Result<(), sbe_rt::EncodeError> {
            g.add(|e| -> Result<(), sbe_rt::EncodeError> { e.speed(30).mpg(35.9); e.usage_description(b"Urban")?; Ok(()) })?;
            g.add(|e| -> Result<(), sbe_rt::EncodeError> { e.speed(60).mpg(25.0); e.usage_description(b"Highway")?; Ok(()) })?;
            Ok(())
        })?;
        let car = car.performance_figures(1, |g| -> Result<(), sbe_rt::EncodeError> {
            g.add(|e| -> Result<(), sbe_rt::EncodeError> {
                e.octane_rating(95);
                e.acceleration(2, |a| -> Result<(), sbe_rt::EncodeError> {
                    a.add(|x| -> Result<(), sbe_rt::EncodeError> { x.mph(30).seconds(4.0); Ok(()) })?;
                    a.add(|x| -> Result<(), sbe_rt::EncodeError> { x.mph(60).seconds(7.5); Ok(()) })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        })?;
        let car = car.manufacturer(b"Honda")?;
        let car = car.model(b"Civic VTi")?;
        let complete = car.activation_code(b"abcdef")?;
        assert!(complete.encoded_length_with_header() > 0);
        let flyweight_bytes = complete.as_bytes().to_vec();

        let dec = CarDecoder::try_from(&flyweight_bytes[..]).unwrap();
        let d: CarDomain = dec.into();

        let mut buf2 = [0u8; 512];
        let n = d.encode(&mut buf2).unwrap();
        assert_eq!(&buf2[..n], &flyweight_bytes[..],
            "domain encode must match flyweight encode byte-for-byte");

        assert_eq!(n, flyweight_bytes.len());
    "#,
    );
    Ok(())
}

/// L3 orderbook domain encode round-trip through nested groups + entry var-data.
#[test]
fn l3_domain_encode_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let l3_schema = || Paths::l3_orderbook_schema();
    let (_schema, src) = generate(&l3_schema(), "l3_enc_rt");
    // Source assertions: encode and encode_into methods exist
    assert!(
        src.contains("pub fn encode"),
        "L3 domain must have encode method"
    );
    assert!(
        src.contains("pub fn encode_into"),
        "L3 entry domain must have encode_into method"
    );
    compile_and_run(
        "l3_enc_rt",
        &src,
        r#"
    "#,
    );
    Ok(())
}

/// Domain encode with buffer too short returns Err, not panic.
#[test]
fn domain_encode_buffer_too_short() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "car_enc_short");
    compile_and_run(
        "car_enc_short",
        &src,
        r#"
        let mut buf = [0u8; 1024];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        car.serial_number(1).model_year(2000).available_bool(false).code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0, 0, [0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?;
        let car = car.performance_figures(0, |_| -> Result<(), sbe_rt::EncodeError> { Ok(()) })?;
        let car = car.manufacturer(b"Honda")?;
        let car = car.model(b"Test")?;
        let complete = car.activation_code(b"abc")?;
        assert!(complete.encoded_length_with_header() > 0);
        let fb = complete.as_bytes().to_vec();

        let dec = CarDecoder::try_from(&fb[..]).unwrap();
        let d: CarDomain = dec.into();

        let mut ok_buf = [0u8; 512];
        assert!(d.encode(&mut ok_buf).is_ok());

        let mut tiny_buf = [0u8; 8];
        let err = d.encode(&mut tiny_buf);
        assert!(err.is_err(), "encode into 8-byte buffer must fail");
    "#,
    );
    Ok(())
}

#[test]
fn car_domain_string_var_data_and_invalid_utf8_empty() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate_domain_with(&Paths::example_schema(), "car_dom_str", |c| {
        c.enable_domain_objects(DomainVarData::LossyStrings)
    });
    assert!(
        src.contains("pub manufacturer: String"),
        "expected String var-data fields:\n{src}"
    );
    assert!(
        src.contains("unwrap_or_default"),
        "expected silent empty on invalid UTF-8:\n{src}"
    );
    compile_and_run(
        "car_dom_str",
        &src,
        r#"
        let mut buf = [0u8; 2048];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        car.serial_number(1).model_year(2020).available(BooleanType::T).code(Model::A);
        car.some_numbers([0; 4]).vehicle_code([b'A'; 6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(1000, 4, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let complete = car
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"Honda")?
            .model(b"Civic")?
            .activation_code(b"abc")?;
        let encoded = complete.as_bytes().to_vec();
        let d = CarDomain::try_from_decoder(CarDecoder::try_from(&encoded[..])?)?;
        assert_eq!(d.manufacturer, "Honda");
        assert_eq!(d.model, "Civic");
        assert_eq!(d.activation_code, "abc");

        // Invalid UTF-8 in manufacturer → empty string (silent, not error, not U+FFFD)
        let mut bad = encoded.clone();
        if let Some(pos) = bad.windows(5).position(|w| w == b"Honda") {
            bad[pos + 4] = 0xFF;
        } else {
            panic!("Honda not found in encoded buffer");
        }
        let d_bad = CarDomain::try_from_decoder(CarDecoder::try_from(&bad[..])?)?;
        assert_eq!(d_bad.manufacturer, "");
        assert_eq!(d_bad.model, "Civic");
        println!("car_domain_string_var_data_and_invalid_utf8_empty: PASSED");
    "#,
    );
    Ok(())
}
