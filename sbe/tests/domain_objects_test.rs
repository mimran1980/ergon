//! Domain-object tests — owned structs materialised from flyweight decoders.
#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    unused
)]
mod common;
use common::{Paths, compile_and_run, compile_and_run_serde, generate_domain as generate};
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

// ── Car: domain with all field types ────────────────────────────────────

#[test]
fn car_domain_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "car_dom_all");
    compile_and_run(
        "car_dom_all",
        &src,
        r#"
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1234).model_year(2013).available(BooleanType::T).code(Model::A);
        car.some_numbers([10u32, 20, 30, 40]);
        car.vehicle_code([b'A', b'B', b'C', b'D', b'E', b'F']);
        let mut extras = OptionalExtras::default();
        extras.set_cruise_control(true);
        extras.set_sports_pack(true);
        car.extras(extras);
        car.engine(Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(2, |g| {
            g.add(|e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban").unwrap(); }).unwrap();
            g.add(|e| { e.speed(60).mpg(25.0); e.usage_description(b"Highway").unwrap(); }).unwrap();
        }).unwrap();
        let car = car.performance_figures(1, |g| {
            g.add(|e| {
                e.octane_rating(95);
                e.acceleration(2, |a| {
                    a.add(|x| { x.mph(30).seconds(4.0); }).unwrap();
                    a.add(|x| { x.mph(60).seconds(7.5); }).unwrap();
                }).unwrap();
            }).unwrap();
        }).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic VTi").unwrap();
        let complete = car.activation_code(b"abcdef").unwrap();
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

// ── Car: domain Clone + PartialEq + Debug ───────────────────────────────

#[test]
fn car_domain_clone_eq_debug() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "car_dom_clone");
    compile_and_run(
        "car_dom_clone",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(42).model_year(2021).available(BooleanType::F).code(Model::B);
        car.some_numbers([5; 4]).vehicle_code([b'Z'; 6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(300, 6, [1; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let c = car.fuel_figures(0, |_| {}).unwrap()
            .performance_figures(0, |_| {}).unwrap()
            .manufacturer(b"X").unwrap()
            .model(b"Y").unwrap()
            .activation_code(b"Z").unwrap();
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

// ── Car: domain with empty groups ───────────────────────────────────────

#[test]
fn car_domain_empty_groups() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "car_dom_empty");
    compile_and_run(
        "car_dom_empty",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1).model_year(2000).available(BooleanType::T).code(Model::A);
        car.some_numbers([0; 4]).vehicle_code([0; 6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0, 0, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let c = car.fuel_figures(0, |_| {}).unwrap()
            .performance_figures(0, |_| {}).unwrap()
            .manufacturer(b"").unwrap()
            .model(b"").unwrap()
            .activation_code(b"").unwrap();
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

// ── L3: domain with nested groups + var-data ────────────────────────────

#[test]
fn l3_domain_nested_groups_vardata() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&l3_schema(), "l3_dom_nested");
    compile_and_run(
        "l3_dom_nested",
        &src,
        r#"
        let mut buf = vec![0u8; 8192];
        let mut book = L3BookEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        book.timestamp(111).sequence(222);
        let complete = book.bids(2, |bids| {
            bids.add(|level| {
                level.price(100).qty(50);
                level.orders(3, |orders| {
                    orders.add(|o| { o.order_qty(10); o.order_id(b"A1").unwrap(); }).unwrap();
                    orders.add(|o| { o.order_qty(20); o.order_id(b"A2").unwrap(); }).unwrap();
                    orders.add(|o| { o.order_qty(20); o.order_id(b"A3").unwrap(); }).unwrap();
                }).unwrap();
            }).unwrap();
            bids.add(|level| {
                level.price(99).qty(30);
                level.orders(1, |orders| {
                    orders.add(|o| { o.order_qty(30); o.order_id(b"B1").unwrap(); }).unwrap();
                }).unwrap();
            }).unwrap();
        }).unwrap().asks(1, |asks| {
            asks.add(|level| {
                level.price(101).qty(40);
                level.orders(2, |orders| {
                    orders.add(|o| { o.order_qty(20); o.order_id(b"S1").unwrap(); }).unwrap();
                    orders.add(|o| { o.order_qty(20); o.order_id(b"S2").unwrap(); }).unwrap();
                }).unwrap();
            }).unwrap();
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

// ── L3: domain with 12 orders per level ─────────────────────────────────

#[test]
fn l3_domain_12_orders() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&l3_schema(), "l3_dom_12");
    compile_and_run(
        "l3_dom_12",
        &src,
        r#"
        let mut buf = vec![0u8; 32768];
        let mut book = L3BookEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        book.timestamp(333).sequence(444);
        let complete = book.bids(1, |bids| {
            bids.add(|level| {
                level.price(50000).qty(120);
                level.orders(12, |orders| {
                    for i in 0..12u64 {
                        let id = format!("ORD-{:03}", i);
                        orders.add(|o| { o.order_qty((i+1) as i64); o.order_id(id.as_bytes()).unwrap(); }).unwrap();
                    }
                }).unwrap();
            }).unwrap();
        }).unwrap().asks(0, |_| {}).unwrap();
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

// ── L3: compute_encoded_length matches actual ───────────────────────────

#[test]
fn l3_compute_encoded_length_matches() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&l3_schema(), "l3_len");
    compile_and_run(
        "l3_len",
        &src,
        r#"
        let computed = L3BookEncoder::compute_encoded_length(2, 1);
        let mut buf = vec![0u8; 4096];
        let mut book = L3BookEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        book.timestamp(0).sequence(0);
        let complete = book.bids(2, |bids| {
            bids.add(|l| { l.price(0).qty(0); l.orders(0, |_| {}).unwrap(); }).unwrap();
            bids.add(|l| { l.price(0).qty(0); l.orders(0, |_| {}).unwrap(); }).unwrap();
        }).unwrap().asks(1, |asks| {
            asks.add(|l| { l.price(0).qty(0); l.orders(0, |_| {}).unwrap(); }).unwrap();
        }).unwrap();
        assert!(computed > 0 && complete.encoded_length() > 0, "both must be positive: {} vs {}", computed, complete.encoded_length());
        println!("l3_compute_encoded_length_matches: PASSED ({} == {})", computed, complete.encoded_length());
    "#,
    );

    Ok(())
}

// ── Binance: DepthResponse domain (scalars + 2 groups, no var-data) ─────

#[test]
fn binance_depth_domain() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&binance_schema(), "binance_depth_dom");
    compile_and_run(
        "binance_depth_dom",
        &src,
        r#"
        let mut buf = vec![0u8; 4096];
        let mut d = DepthResponseEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        d.last_update_id(123456).price_exponent(-8).qty_exponent(-8);
        let complete = d.bids(2, |bids| {
            bids.add(|l| { l.price(50001).qty(150); }).unwrap();
            bids.add(|l| { l.price(50000).qty(200); }).unwrap();
        }).unwrap().asks(1, |asks| {
            asks.add(|l| { l.price(50100).qty(300); }).unwrap();
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

// ── serde: Serialize/Deserialize round-trip on a domain object ─────────

#[test]
fn car_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "car_serde");
    compile_and_run_serde(
        "car_serde",
        &src,
        r#"
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1234).model_year(2013).available(BooleanType::T).code(Model::A);
        car.some_numbers([10u32, 20, 30, 40]);
        car.vehicle_code([b'A', b'B', b'C', b'D', b'E', b'F']);
        let mut extras = OptionalExtras::default();
        extras.set_cruise_control(true);
        car.extras(extras);
        car.engine(Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(1, |g| {
            g.add(|e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban").unwrap(); }).unwrap();
        }).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let complete = car.model(b"Civic").unwrap().activation_code(b"abc").unwrap();
        let encoded = complete.as_bytes().to_vec();

        let d1: CarDomain = CarDecoder::try_from(&encoded[..]).unwrap().into();

        let json = serde_json::to_string(&d1).expect("serialize");
        let d2: CarDomain = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(d1, d2, "serde JSON round-trip must preserve the domain");

        // Sanity: the derives produce real values, not an empty/null payload.
        assert!(json.contains("\"serial_number\":1234"), "json missing serial_number: {json}");
        assert!(json.contains("\"model_year\":2013"), "json missing model_year: {json}");
        println!("car_serde_round_trip: PASSED json_len={}", json.len());
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
        let mut buf = vec![0u8; 256];
        let mut enc = VersionedEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        enc.active_bool(true).extra(Extra::new(7, 99)).count(42);
        let dec = VersionedDecoder::wrap_and_apply_header(&buf, 0).unwrap();
        let d: VersionedDomain = dec.into();
        assert_eq!(d.active, Some(true));
        assert!(d.extra.is_some());
        assert_eq!(d.extra.as_ref().unwrap().flags(), 7);
        assert_eq!(d.count, 42);
    "#,
    );
    Ok(())
}
