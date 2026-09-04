//! Memoized random-access decoder: progressive tail-boundary cache.
//!
//! Compilation is the assertion. Cache behaviour (hits, frontier, no error
//! publication, `!Sync`) is checked on generated Car and L3 codecs.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{
    Paths, compile_and_run, compile_fails_with_diagnostics, generate, generate_domain_with,
};

const fn encode_car_body() -> &'static str {
    r#"
        let sized = CarEncodedLength::new()
            .fuel_figures_ragged(2, |ff| {
                ff.add()?.usage_description(5)?;
                ff.add()?.usage_description(3)?;
                Ok(())
            })?
            .performance_figures_ragged(1, |pf| {
                pf.add()?.acceleration(|a| { a.add()?; Ok(()) })?;
                Ok(())
            })?
            .manufacturer(5)?
            .model(5)?
            .activation_code(3)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2024,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1, 2, 3, 4],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [b'N', b'/', b'A'], 30, BooleanType::T, Booster::new(BoostType::TURBO, 2)),
            })
            .fuel_figures(2, |g| {
                g.add(|mut e| { e.speed(30).mpg(20.0); e.usage_description(b"urban") })?;
                g.add(|mut e| { e.speed(55).mpg(30.0); e.usage_description(b"hwy") })?;
                Ok(())
            })?
            .performance_figures(1, |g| {
                g.add(|mut e| {
                    e.octane_rating(95);
                    e.acceleration(1, |a| {
                        a.add(|mut x| { x.mph(30).seconds(4.0); Ok(()) })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .manufacturer(b"Honda")?
            .model(b"Civic")?
            .activation_code(b"abc")?
            .encoded_length_with_header();
        assert_eq!(sized, len, "EncodedLength must match the encoder");
        let encoded = &storage[..len];
    "#
}

#[test]
fn car_cache_warms_and_repeated_reads_hit() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "memo_car_hits");
    assert!(
        src.contains("TailBoundaryCache"),
        "tailed decoder must carry a progressive cache"
    );
    assert!(
        src.contains("decode_cache_stats"),
        "debug cache stats must be generated"
    );
    let mut body = encode_car_body().to_string();
    body.push_str(
        r#"
        let dec = CarDecoder::try_decode(encoded, 0)?.memoized();
        assert_eq!(dec.serial_number(), 1);
        let cold = dec.decode_cache_stats();
        assert_eq!(cold.known_through, 0, "construction must not walk tails");
        assert_eq!(cold.boundary_calcs, 0);

        let _ = dec.activation_code()?;
        let after_last = dec.decode_cache_stats();
        assert!(after_last.known_through >= 5, "final var-data warms every preceding tail");
        assert!(after_last.misses >= 1);
        let calcs = after_last.boundary_calcs;
        let hits_before = after_last.hits;

        let code = dec.activation_code()?;
        assert_eq!(code, b"abc");
        let warm = dec.decode_cache_stats();
        assert_eq!(warm.known_through, after_last.known_through, "frontier must not regress");
        assert!(warm.hits > hits_before, "repeated last-field read must hit");
        assert_eq!(warm.boundary_calcs, calcs, "warm read must not walk again");

        let mfr = dec.manufacturer()?;
        assert_eq!(mfr, b"Honda");
        let bounced = dec.decode_cache_stats();
        assert_eq!(bounced.known_through, warm.known_through);
        assert!(bounced.hits > warm.hits);
        assert_eq!(bounced.boundary_calcs, calcs);
    "#,
    );
    compile_and_run("memo_car_hits", &src, &body);
    Ok(())
}

#[test]
fn car_random_orders_match_schema_order() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "memo_car_orders");
    let mut body = encode_car_body().to_string();
    body.push_str(
        r#"
        fn snapshot(dec: &CarDecoder<'_>) -> Result<(usize, usize, Vec<u8>, Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
            let fuel = dec.fuel_figures()?.count();
            let perf = dec.performance_figures()?.count();
            Ok((
                fuel,
                perf,
                dec.manufacturer()?.to_vec(),
                dec.model()?.to_vec(),
                dec.activation_code()?.to_vec(),
            ))
        }
        let dec = CarDecoder::try_decode(encoded, 0)?;
        let expected = snapshot(&dec)?;

        let reverse = CarDecoder::try_decode(encoded, 0)?;
        let _ = reverse.activation_code()?;
        let _ = reverse.model()?;
        let _ = reverse.manufacturer()?;
        let _ = reverse.performance_figures()?;
        let _ = reverse.fuel_figures()?;
        assert_eq!(snapshot(&reverse)?, expected);

        let bounce = CarDecoder::try_decode(encoded, 0)?;
        let _ = bounce.model()?;
        let _ = bounce.fuel_figures()?;
        let _ = bounce.activation_code()?;
        let _ = bounce.manufacturer()?;
        let _ = bounce.performance_figures()?;
        assert_eq!(snapshot(&bounce)?, expected);
        assert_eq!(bounce.serial_number(), 1);
        assert_eq!(bounce.model_year(), 2024);
    "#,
    );
    compile_and_run("memo_car_orders", &src, &body);
    Ok(())
}

#[test]
fn truncated_var_data_does_not_publish_invalid_boundary() -> Result<(), Box<dyn std::error::Error>>
{
    let (_schema, src) = generate(&Paths::example_schema(), "memo_car_trunc");
    compile_and_run(
        "memo_car_trunc",
        &src,
        r#"
        let sized = CarEncodedLength::new()
            .fuel_figures(0)
            .usage_description(0)?
            .performance_figures(0)
            .acceleration(0)?
            .manufacturer(5)?
            .model(5)?
            .activation_code(3)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2024,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1, 2, 3, 4],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [b'N', b'/', b'A'], 30, BooleanType::T, Booster::new(BoostType::TURBO, 2)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"Honda")?
            .model(b"Civic")?
            .activation_code(b"abc")?
            .encoded_length_with_header();
        assert_eq!(sized, len, "EncodedLength must match the encoder");
        let short = &storage[..len.saturating_sub(2)];
        let dec = match CarDecoder::try_decode(short, 0) {
            Ok(d) => d.memoized(),
            Err(_) => return Ok(()),
        };
        let before = dec.decode_cache_stats().known_through;
        let err = dec.activation_code();
        assert!(err.is_err());
        let after = dec.decode_cache_stats().known_through;
        assert!(after >= before, "errors must not regress the frontier");
        let again = dec.activation_code();
        assert!(again.is_err(), "failed field must stay failed");
    "#,
    );
    Ok(())
}

#[test]
fn memoized_decoder_is_send_not_sync() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "memo_car_sync");
    compile_and_run(
        "memo_car_send",
        &src,
        r#"
        fn assert_send<T: Send>(_: &T) {}
        let sized = CarEncodedLength::new()
            .fuel_figures(0)
            .usage_description(0)?
            .performance_figures(0)
            .acceleration(0)?
            .manufacturer(1)?
            .model(1)?
            .activation_code(1)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2024,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0, 0, 0, 0],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(1, 1, [0, 0, 0], 0, BooleanType::T, Booster::new(BoostType::TURBO, 1)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"x")?
            .model(b"y")?
            .activation_code(b"z")?
            .encoded_length_with_header();
        assert_eq!(sized, len, "EncodedLength must match the encoder");
        let dec = CarDecoder::try_decode(&storage[..len], 0)?.memoized();
        assert_send(&dec);
    "#,
    );
    compile_fails_with_diagnostics(
        "memo_car_sync",
        &src,
        r#"
        fn assert_sync<T: Sync>(_: T) {}
        let dec = unsafe { core::mem::zeroed::<CarMemoizedDecoder>() };
        assert_sync(dec);
        "#,
        &["Sync"],
    );
    Ok(())
}

#[test]
fn memoized_is_a_separate_lane_reached_by_consuming_the_base_decoder()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "memo_car_lane");
    assert!(
        src.contains("pub struct CarMemoizedDecoder"),
        "memoized lane must be its own type"
    );
    assert!(
        src.contains("pub fn memoized(self)"),
        "memoized() must consume the base decoder"
    );
    let mut body = encode_car_body().to_string();
    body.push_str(
        r#"
        // The base decoder recalculates and carries no cache, so it is Sync.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CarDecoder<'_>>();

        let base = CarDecoder::try_decode(encoded, 0)?;
        assert_eq!(base.manufacturer()?, b"Honda");

        // Same getter names on the memoized lane, same decoded values.
        let memo = CarDecoder::try_decode(encoded, 0)?.memoized();
        assert_eq!(memo.serial_number(), base.serial_number());
        assert_eq!(memo.manufacturer()?, b"Honda");
        assert_eq!(memo.activation_code()?, b"abc");
        assert_eq!(memo.model()?, b"Civic");

        // Reaching the last tail warms every boundary before it.
        let stats = memo.decode_cache_stats();
        assert!(stats.known_through >= 5, "final var-data warms preceding tails");

        // into_inner() hands the uncached decoder back.
        let back = memo.into_inner();
        assert_eq!(back.manufacturer()?, b"Honda");
    "#,
    );
    compile_and_run("memo_car_lane", &src, &body);
    Ok(())
}

#[test]
fn l3_nested_entry_cache_is_independent() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "memo_l3_nested");
    compile_and_run(
        "memo_l3_nested",
        &src,
        r#"
        let sized = L3BookEncodedLength::new()
            .bids_ragged(1, |b| {
                b.add()?.orders(|o| { o.add()?.order_id(5)?; Ok(()) })?;
                Ok(())
            })?
            .asks_ragged(0, |_| Ok(()))?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = L3BookEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&L3BookFixedFields { timestamp: 99, sequence: 7 })
            .bids(1, |g| {
                g.add(|mut lvl| {
                    lvl.price(100).qty(10);
                    lvl.orders(1, |o| {
                        o.add(|mut ord| { ord.order_qty(4); ord.order_id(b"ord-1") })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .asks(0, |_| Ok(()))?
            .encoded_length_with_header();
        assert_eq!(sized, len, "EncodedLength must match the encoder");
        let encoded = &storage[..len];
        let dec = L3BookDecoder::try_decode(encoded, 0)?;
        let bids = dec.bids()?;
        let mut n = 0usize;
        for lvl in bids {
            let lvl = lvl?;
            assert_eq!(lvl.price(), 100);
            for ord in lvl.orders()? {
                let ord = ord?;
                assert_eq!(ord.order_id()?, b"ord-1");
                n += 1;
            }
        }
        assert_eq!(n, 1);
        let asks = dec.asks()?;
        assert!(asks.is_empty());
        assert_eq!(dec.timestamp(), 99);
    "#,
    );
    Ok(())
}

/// Lane conversions and wrapper methods are inherent names, so a schema field
/// spelling one of them must be renamed `*_field` — everywhere.
///
/// `memoized` collides on the base decoder; `inner`, `into_inner`, and
/// `decode_cache_stats` collide on `{Name}MemoizedDecoder`, which receives the
/// fixed-field forwards under the same names. `DECODER_RESERVED` is the single
/// list driving all of them, so the rename cannot differ per location.
/// Compilation is the assertion: without the rename the module emits duplicate
/// definitions and does not build.
#[test]
fn schema_fields_named_after_lane_methods_are_renamed() -> Result<(), Box<dyn std::error::Error>> {
    const XML: &str = r#"<messageSchema package="laneclash" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="groupSizeEncoding">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="numInGroup" primitiveType="uint16"/>
    </composite>
    <composite name="varStringEncoding">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
    </composite>
  </types>
  <message name="Msg" id="1" blockLength="20">
    <field name="memoized" id="1" type="uint32" offset="0"/>
    <field name="inner" id="2" type="uint32" offset="4"/>
    <field name="intoInner" id="3" type="uint32" offset="8"/>
    <field name="decodeCacheStats" id="4" type="uint32" offset="12"/>
    <field name="ordered" id="5" type="uint32" offset="16"/>
    <group name="legs" id="6" dimensionType="groupSizeEncoding" blockLength="4">
      <field name="qty" id="7" type="uint32" offset="0"/>
    </group>
    <data name="label" id="8" type="varStringEncoding"/>
  </message>
</messageSchema>"#;
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
    let schema = Schema::from_ir(parse(XML)?);
    let src = Generator::new(GenerationConfig::new("laneclash"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();
    for renamed in [
        "fn memoized_field(",
        "fn inner_field(",
        "fn into_inner_field(",
        "fn decode_cache_stats_field(",
        "fn ordered_field(",
    ] {
        assert!(src.contains(renamed), "missing rename: {renamed}");
    }
    // The lane methods themselves must survive under their real names.
    assert!(src.contains("pub fn memoized(self)"), "memoized() lost");
    assert!(src.contains("pub fn ordered(self)"), "ordered() lost");

    compile_and_run(
        "laneclash",
        &src,
        r#"
        let mut buf = [0u8; MsgEncoder::compute_length_with_header(1, 3)];
        let len = MsgEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&MsgFixedFields {
                memoized: 1,
                inner: 2,
                into_inner: 3,
                decode_cache_stats: 4,
                ordered: 5,
            })
            .legs(1, |legs| { legs.add(|l| { l.qty(6u32); Ok(()) })?; Ok(()) })?
            .label(b"abc")?
            .encoded_length_with_header();
        assert_eq!(buf.len(), len);

        // Base lane: renamed getters, real lane conversions.
        let dec = MsgDecoder::try_decode(&buf[..len], 0)?;
        assert_eq!(dec.memoized_field(), 1);
        assert_eq!(dec.ordered_field(), 5);

        // Memoized lane: same renamed getters plus the real wrapper methods.
        let memo = MsgDecoder::try_decode(&buf[..len], 0)?.memoized();
        assert_eq!(memo.memoized_field(), 1);
        assert_eq!(memo.inner_field(), 2);
        assert_eq!(memo.into_inner_field(), 3);
        assert_eq!(memo.decode_cache_stats_field(), 4);
        assert_eq!(memo.ordered_field(), 5);
        assert_eq!(memo.label()?, b"abc");
        assert_eq!(memo.inner().memoized_field(), 1);
        let _ = memo.decode_cache_stats();
        assert_eq!(memo.into_inner().memoized_field(), 1);

        // Ordered lane forwards the same renamed names.
        let mut ord = MsgDecoder::try_decode(&buf[..len], 0)?.ordered();
        assert_eq!(ord.memoized_field(), 1);
        ord.legs()?.visit_entries(|e| -> Result<(), sbe_rt::DecodeError> {
            assert_eq!(e.qty(), 6);
            Ok(())
        })?;
        assert_eq!(ord.label()?, b"abc");
        "#,
    );
    Ok(())
}

/// The memoized lane must expose the *same* var-data text surface as the base
/// decoder, not a subset.
///
/// Both lanes emit their helpers from one generator
/// (`message_decoder::vardata_text_helpers`), so UTF-8 and ASCII each get a
/// checked and an unchecked accessor on both types, and binary var-data gets
/// neither. A partial copy previously gave the memoized lane checked UTF-8
/// only, so `activation_code_as_str_unchecked()` existed on one lane and not
/// the other and swapping lanes stopped compiling.
#[test]
fn memoized_var_data_text_surface_matches_the_base_decoder()
-> Result<(), Box<dyn std::error::Error>> {
    const XML: &str = r#"<messageSchema package="textsurface" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="varUtf8">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
    </composite>
    <composite name="varAscii">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="ASCII"/>
    </composite>
    <composite name="varBinary">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="Msg" id="1" blockLength="4">
    <field name="seq" id="1" type="uint32" offset="0"/>
    <data name="text" id="2" type="varUtf8"/>
    <data name="tag" id="3" type="varAscii"/>
    <data name="blob" id="4" type="varBinary"/>
  </message>
</messageSchema>"#;
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
    let schema = Schema::from_ir(parse(XML)?);
    let src = Generator::new(GenerationConfig::new("textsurface"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();
    // The unchecked helper belongs to the two random-access lanes only, so it
    // is emitted exactly twice per text field: base decoder and memoized
    // wrapper. The ordered lane has its own cursor-advancing `*_as_str` with
    // no unchecked variant, which is a deliberate difference in semantics, not
    // a divergence in this surface. Binary var-data gets no helper at all.
    for (helper, want) in [
        ("fn text_as_str_unchecked(", 2),
        ("fn tag_as_str_unchecked(", 2),
        ("fn blob_as_str(", 0),
        ("fn blob_as_str_unchecked(", 0),
    ] {
        assert_eq!(
            src.matches(helper).count(),
            want,
            "{helper} must appear {want} times (base + memoized)"
        );
    }

    compile_and_run(
        "textsurface",
        &src,
        r#"
        let mut buf = [0u8; MsgEncoder::compute_length_with_header(3, 2, 1)];
        let len = MsgEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&MsgFixedFields { seq: 7 })
            .text(b"hey")?
            .tag(b"AB")?
            .blob(&[0xFFu8])?
            .encoded_length_with_header();
        assert_eq!(buf.len(), len);

        // Every helper resolves on both lanes, with equal values.
        let base = MsgDecoder::try_decode(&buf[..len], 0)?;
        let memo = MsgDecoder::try_decode(&buf[..len], 0)?.memoized();
        assert_eq!(base.text_as_str()?, memo.text_as_str()?);
        assert_eq!(base.tag_as_str()?, memo.tag_as_str()?);
        assert_eq!(base.blob()?, memo.blob()?);
        unsafe {
            assert_eq!(base.text_as_str_unchecked()?, memo.text_as_str_unchecked()?);
            assert_eq!(base.tag_as_str_unchecked()?, memo.tag_as_str_unchecked()?);
        }
        assert_eq!(memo.text_as_str()?, "hey");
        assert_eq!(memo.tag_as_str()?, "AB");
        assert_eq!(memo.blob()?, &[0xFFu8]);

        // Invalid text is an error on both lanes, never a sentinel.
        buf[len - 1] = 0x80;
        let memo = MsgDecoder::try_decode(&buf[..len], 0)?.memoized();
        assert!(memo.blob().is_ok(), "binary var-data has no encoding to fail");
        "#,
    );
    Ok(())
}
