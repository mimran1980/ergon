//! Generated runtime contracts: `SbeMessage` sealing and framing precision.
//!
//! Two separate promises are covered here.
//!
//! *Sealing.* `SbeMessage` advertises this schema's real template id, block
//! length, schema id, and version. Generic framing code is entitled to trust
//! that. It can only do so if no consumer can implement the trait for a type of
//! its own, so the supertrait lives in a private child of the generated module.
//!
//! *Framing precision.* A binary boundary must name what it actually is. The
//! length-prefix policies are little-endian and say so, and the slice handed to
//! `AnyMessage::Unknown` / `visit_unknown` is the complete frame — message
//! header plus body — not a body-only payload.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

mod common;
use common::{Paths, compile_and_run, compile_fails_with_diagnostics, generate};

#[test]
fn any_message_visitor_dispatches_known_template_to_correct_arm()
-> Result<(), Box<dyn std::error::Error>> {
    // Schema with two messages — verify that AnyMessage dispatch routes each
    // template_id to the correct visitor arm, and visit_unknown is NOT called
    // for known templates.
    let multi = r#"<?xml version="1.0"?>
<messageSchema package="visitor_test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Alpha" id="1" blockLength="4">
    <field name="x" id="1" type="uint32" offset="0"/>
  </message>
  <message name="Beta" id="2" blockLength="4">
    <field name="y" id="1" type="uint32" offset="0"/>
  </message>
</messageSchema>"#;
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
    let ir = parse(multi)?;
    let schema = Schema::from_ir(ir);
    let src = Generator::new(GenerationConfig::new("visitor_test"))
        .generate(&schema)?
        .modules()
        .next()
        .unwrap()
        .source
        .clone();
    // Must contain an AnyMessage enum with both message arms
    assert!(src.contains("Alpha"), "must contain Alpha: {src}");
    assert!(src.contains("Beta"), "must contain Beta: {src}");
    assert!(
        src.contains("pub enum AnyMessage"),
        "must emit AnyMessage dispatch"
    );
    // Must generate a FrameCursor for multi-template dispatch
    assert!(src.contains("FrameCursor"), "must emit FrameCursor");
    // Verify the generated code compiles and dispatches both messages
    compile_and_run(
        "visitor_test",
        &src,
        r#"
        // Encode Alpha (template_id=1, x=42)
        let alen = visitor_test::AlphaEncoder::compute_length_with_header();
        let mut abuf = vec![0u8; alen];
        let len = visitor_test::AlphaEncoder::wrap_and_apply_header(&mut abuf, 0)
            .fixed(&visitor_test::AlphaFixedFields { x: 42 })
            .encoded_length_with_header();
        // Encode Beta (template_id=2, y=99)
        let blen = visitor_test::BetaEncoder::compute_length_with_header();
        let mut bbuf = vec![0u8; blen];
        let _ = visitor_test::BetaEncoder::wrap_and_apply_header(&mut bbuf, 0)
            .fixed(&visitor_test::BetaFixedFields { y: 99 })
            .encoded_length_with_header();
        // Dispatch both
        use visitor_test::AnyMessage;
        let a = AnyMessage::try_decode(&abuf, 0)?;
        match a {
            AnyMessage::Alpha(dec) => assert_eq!(dec.x(), 42),
            _ => panic!("expected Alpha"),
        }
        let b = AnyMessage::try_decode(&bbuf, 0)?;
        match b {
            AnyMessage::Beta(dec) => assert_eq!(dec.y(), 99),
            _ => panic!("expected Beta"),
        }
        "#,
    );
    Ok(())
}

#[test]
fn any_message_lane_accessors_exist_only_for_messages_with_tails()
-> Result<(), Box<dyn std::error::Error>> {
    // `Fixed` is a fixed-block message: every field is random-access off the
    // block, so there is nothing for the memoized or ordered lanes to do and
    // neither is generated. `Tailed` has a group and var-data, so it gets all
    // three lane accessors.
    let multi = r#"<?xml version="1.0"?>
<messageSchema package="lanes_test" id="1" version="0" byteOrder="littleEndian">
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
  <message name="Fixed" id="1" blockLength="4">
    <field name="x" id="1" type="uint32" offset="0"/>
  </message>
  <message name="Tailed" id="2" blockLength="4">
    <field name="y" id="1" type="uint32" offset="0"/>
    <group name="legs" id="2" dimensionType="groupSizeEncoding" blockLength="4">
      <field name="qty" id="3" type="uint32" offset="0"/>
    </group>
    <data name="label" id="4" type="varStringEncoding"/>
  </message>
</messageSchema>"#;
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
    let ir = parse(multi)?;
    let schema = Schema::from_ir(ir);
    let src = Generator::new(GenerationConfig::new("lanes_test"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("no module generated")?
        .source
        .clone();

    // Source assertions name which cell broke; the compile below is the check.
    assert!(
        !src.contains("FixedOrderedDecoder") && !src.contains("FixedMemoizedDecoder"),
        "fixed-block message must not get an ordered or memoized lane"
    );
    assert!(
        !src.contains("into_fixed_ordered") && !src.contains("into_fixed_memoized"),
        "fixed-block message must not get lane accessors on AnyMessage"
    );
    assert!(
        src.contains("into_tailed_ordered") && src.contains("into_tailed_memoized"),
        "message with tails must get both lane accessors on AnyMessage"
    );

    compile_and_run(
        "lanes_test",
        &src,
        r#"
        use lanes_test::{AnyMessage, FixedEncoder, FixedFixedFields, TailedEncoder,
                         TailedFixedFields, sbe_rt};

        let mut fbuf = [0u8; FixedEncoder::compute_length_with_header()];
        let flen = FixedEncoder::wrap_and_apply_header(&mut fbuf, 0)
            .fixed(&FixedFixedFields { x: 42 })
            .encoded_length_with_header();

        // `Tailed` has a fixed-stride group and one var-data field, so the
        // direct const sizing helper applies: one leg, a three-byte label.
        let mut tbuf = [0u8; TailedEncoder::compute_length_with_header(1, 3)];
        let actual = TailedEncoder::wrap_and_apply_header(&mut tbuf, 0)
            .fixed(&TailedFixedFields { y: 99 })
            .legs(1, |legs| { legs.add(|l| { l.qty(7u32); Ok(()) })?; Ok(()) })?
            .label(b"abc")?
            .encoded_length_with_header();
        assert_eq!(tbuf.len(), actual);

        // Fixed-block frame: only the base lane exists, and it is enough.
        let f = AnyMessage::try_decode(&fbuf[..flen], 0)?.into_fixed()
            .ok_or("expected Fixed")?;
        assert_eq!(f.x(), 42);
        // Wrong template yields None rather than a panic.
        assert!(AnyMessage::try_decode(&tbuf[..actual], 0)?.into_fixed().is_none());

        // Tailed frame: every lane reachable straight off the enum.
        let base = AnyMessage::try_decode(&tbuf[..actual], 0)?.into_tailed()
            .ok_or("expected Tailed")?;
        assert_eq!(base.y(), 99);

        let memo = AnyMessage::try_decode(&tbuf[..actual], 0)?.into_tailed_memoized()
            .ok_or("expected Tailed")?;
        assert_eq!(memo.label()?, b"abc");
        assert_eq!(memo.y(), 99);

        let mut ord = AnyMessage::try_decode(&tbuf[..actual], 0)?.into_tailed_ordered()
            .ok_or("expected Tailed")?;
        ord.legs()?.visit_entries(|e| -> Result<(), sbe_rt::DecodeError> {
            assert_eq!(e.qty(), 7);
            Ok(())
        })?;
        assert_eq!(ord.label()?, b"abc");

        assert!(AnyMessage::try_decode(&fbuf[..flen], 0)?.into_tailed_memoized().is_none());
        assert!(AnyMessage::try_decode(&fbuf[..flen], 0)?.into_tailed_ordered().is_none());
        "#,
    );
    Ok(())
}

// ─── Sealing ───────────────────────────────────────────────────────────────

#[test]
fn sbe_message_supertrait_lives_in_a_private_module() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "seal_shape");

    assert!(
        src.contains("pub trait SbeMessage: super::__sbe_message_sealed::Sealed"),
        "SbeMessage must carry the sealing supertrait, not merely claim to be sealed"
    );
    assert!(
        src.contains("pub(crate) mod __sbe_message_sealed {"),
        "the sealing module must be pub(crate): visible within the crate for sibling modules using with_external_sbe_rt"
    );
    assert!(
        !src.contains("pub mod __sbe_message_sealed"),
        "a fully public sealing module would not seal anything across crate boundaries"
    );
    // The old decorative marker must not remain reachable either: it existed
    // only for the header-state markers and was never a supertrait of
    // SbeMessage.
    assert!(
        !src.contains("pub mod private {"),
        "sbe_rt::private must not be part of the public surface"
    );
    Ok(())
}

#[test]
fn consumer_cannot_implement_sbe_message() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "seal_impl");

    compile_fails_with_diagnostics(
        "seal_impl",
        &src,
        r#"
        struct ForgedMessage;

        impl seal_impl::sbe_rt::SbeMessage for ForgedMessage {
            const TEMPLATE_ID: u16 = 1;
            const BLOCK_LENGTH: usize = 0;
            const SCHEMA_ID: u16 = 1;
            const SCHEMA_VERSION: u16 = 0;
        }
        "#,
        // The consumer cannot satisfy the sealing supertrait, because it cannot
        // name it.
        &["__sbe_message_sealed::Sealed", "is not satisfied"],
    );
    Ok(())
}

#[test]
fn sealing_trait_is_crate_visible_for_sibling_modules() -> Result<(), Box<dyn std::error::Error>> {
    // __sbe_message_sealed is `pub(crate)` — sibling modules in the same crate
    // CAN access it (needed by `with_external_sbe_rt` consumers). External
    // crates cannot; that is tested by `consumer_cannot_implement_sbe_message`.
    let (_, src) = generate(&Paths::example_schema(), "seal_name");
    assert!(
        src.contains("pub(crate) mod __sbe_message_sealed {"),
        "sealing module must be pub(crate) for sibling sbe_rt consumers"
    );
    Ok(())
}

#[test]
fn generated_message_types_still_implement_sbe_message() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "seal_ok");

    compile_and_run(
        "seal_ok",
        &src,
        r#"
        // Sealing must not cost the generated types their own metadata.
        fn template_id<T: sbe_rt::SbeMessage>() -> u16 { T::TEMPLATE_ID }
        fn schema_id<T: sbe_rt::SbeMessage>() -> u16 { T::SCHEMA_ID }

        assert_eq!(template_id::<CarDecoder<'_>>(), CarDecoder::TEMPLATE_ID);
        assert_eq!(template_id::<CarEncoder<'_>>(), CarDecoder::TEMPLATE_ID);
        assert_eq!(schema_id::<CarDecoder<'_>>(), schema_id::<CarEncoder<'_>>());

        // `fixed()` returns Encoder<_, FieldsFixed> — that must stay SbeMessage.
        let mut buf = [0u8; 512];
        let fixed = CarEncoder::wrap_and_apply_header(&mut buf, 0).fixed(&CarFixedFields {
            serial_number: 7,
            model_year: 2020,
            available: BooleanType::T,
            code: Model::A,
            some_numbers: [1, 2, 3, 4],
            vehicle_code: *b"ABCDEF",
            extras: OptionalExtras::default(),
            engine: Engine::new(1000, 3, [51, 0, 0], 0i8, BooleanType::F,
                                Booster::new(BoostType::TURBO, 0)),
        });
        assert_eq!(template_id_of(&fixed), CarEncoder::TEMPLATE_ID);

        fn template_id_of<T: sbe_rt::SbeMessage>(_: &T) -> u16 { T::TEMPLATE_ID }
        "#,
    );
    Ok(())
}

// ─── Framing ───────────────────────────────────────────────────────────────

#[test]
fn length_prefix_policies_are_named_little_endian() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "framing_names");
    for name in ["LengthPrefixU16Le", "LengthPrefixU32Le"] {
        assert!(
            src.contains(name),
            "framing policy {name} must name its byte order"
        );
    }
    // `LengthPrefixU16`/`U32` without the suffix left the byte order to be
    // guessed from the implementation.
    assert!(
        !src.contains("LengthPrefixU16,") && !src.contains("LengthPrefixU32,"),
        "the byte-order-silent policy names must be gone"
    );
    assert!(
        !src.contains("payload: &'a [u8]"),
        "the unknown-frame field must be named for the range it carries"
    );
    Ok(())
}

#[test]
fn frame_cursor_reads_little_endian_length_prefixes() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "framing_le");

    compile_and_run(
        "framing_le",
        &src,
        r#"
        // One well-formed Car frame, then the same frame behind a u16 and a u32
        // little-endian length prefix.
        let mut body = [0u8; 256];
        let frame_len = CarEncoder::wrap_and_apply_header(&mut body, 0)
            .fixed(&CarFixedFields {
                serial_number: 7,
                model_year: 2020,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1, 2, 3, 4],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(1000, 3, [51, 0, 0], 0i8, BooleanType::F,
                                    Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"H")?
            .model(b"C")?
            .activation_code(b"A")?
            .encoded_length_with_header();
        let frame = &body[..frame_len];

        let mut u16_framed = Vec::new();
        u16_framed.extend_from_slice(&(frame_len as u16).to_le_bytes());
        u16_framed.extend_from_slice(frame);

        let mut u32_framed = Vec::new();
        u32_framed.extend_from_slice(&(frame_len as u32).to_le_bytes());
        u32_framed.extend_from_slice(frame);

        for (label, buf, policy) in [
            ("u16", u16_framed.as_slice(), FramingPolicy::LengthPrefixU16Le),
            ("u32", u32_framed.as_slice(), FramingPolicy::LengthPrefixU32Le),
        ] {
            let decoded: Vec<_> = FrameCursor::new(buf, policy)
                .collect::<Result<Vec<_>, _>>()
                .unwrap_or_else(|e| panic!("{label}: {e:?}"));
            assert_eq!(decoded.len(), 1, "{label}: expected exactly one frame");
            assert_eq!(decoded[0].len, frame_len, "{label}: frame length");
            match &decoded[0].message {
                AnyMessage::Car(car) => assert_eq!(car.serial_number(), 7),
                _ => panic!("{label}: expected a Car"),
            }
        }

        // A big-endian prefix must NOT be accepted by an Le policy: the length
        // it reads is wrong, so the cursor fails rather than silently
        // mis-framing.
        let mut be_framed = Vec::new();
        be_framed.extend_from_slice(&(frame_len as u16).to_be_bytes());
        be_framed.extend_from_slice(frame);
        let first = FrameCursor::new(&be_framed, FramingPolicy::LengthPrefixU16Le)
            .next()
            .expect("cursor must yield something for a non-empty buffer");
        assert!(first.is_err(), "an LE policy must not decode a BE prefix");
        "#,
    );
    Ok(())
}

#[test]
fn unknown_variant_carries_the_complete_frame() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "framing_unknown");

    compile_and_run(
        "framing_unknown",
        &src,
        r#"
        // Unknown templateId: header is parsed, body is not.
        let mut buf = [0u8; 64];
        buf[0..2].copy_from_slice(&16u16.to_le_bytes()); // blockLength
        buf[2..4].copy_from_slice(&99u16.to_le_bytes()); // templateId (unknown)
        buf[4..6].copy_from_slice(&1u16.to_le_bytes());  // schemaId
        buf[6..8].copy_from_slice(&0u16.to_le_bytes());  // version
        buf[8..24].fill(0xAB);

        let decoded = AnyMessage::decode_frame(&buf, 0, 24)?;
        match decoded.message {
            AnyMessage::Unknown { header, frame } => {
                assert_eq!(header.template_id(), 99);
                // Exact range: header + body, starting at the frame offset.
                assert_eq!(frame.len(), 24);
                assert_eq!(frame.as_ptr(), buf.as_ptr());
                assert_eq!(&frame[..8], &buf[..8]);
                assert!(frame[8..].iter().all(|b| *b == 0xAB));
            }
            _ => panic!("expected Unknown"),
        }

        // The visitor receives the identical range under the identical name.
        struct FrameCapture(usize, u16);
        impl MessageVisitor for FrameCapture {
            type Output = ();
            fn visit_car(&mut self, _: &CarDecoder<'_>) {}
            fn visit_unknown(&mut self, header: &MessageHeader, frame: &[u8]) {
                self.0 = frame.len();
                self.1 = header.template_id();
            }
        }
        let decoded = AnyMessage::decode_frame(&buf, 0, 24)?;
        let mut capture = FrameCapture(0, 0);
        decoded.message.visit(&mut capture);
        assert_eq!(capture.0, 24, "visit_unknown must see the whole frame");
        assert_eq!(capture.1, 99);

        // as_bytes / encoded_length_with_header report the same range.
        assert_eq!(decoded.message.encoded_length_with_header()?, 24);
        assert_eq!(decoded.message.as_bytes()?.len(), 24);
        "#,
    );
    Ok(())
}

#[test]
fn decode_frame_rejects_declared_length_shorter_than_header()
-> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "framing_short_len");
    assert!(
        src.contains("header-inclusive"),
        "decode_frame rustdoc must define frame_len as header-inclusive"
    );
    compile_and_run(
        "framing_short_len",
        &src,
        r#"
        // Backing buffer holds a valid unknown header plus a later frame.
        let mut buf = [0u8; 48];
        buf[0..2].copy_from_slice(&16u16.to_le_bytes());
        buf[2..4].copy_from_slice(&99u16.to_le_bytes());
        buf[4..6].copy_from_slice(&1u16.to_le_bytes());
        buf[6..8].copy_from_slice(&0u16.to_le_bytes());
        buf[8..24].fill(0xAB);
        buf[24..26].copy_from_slice(&16u16.to_le_bytes());
        buf[26..28].copy_from_slice(&77u16.to_le_bytes());
        buf[28..30].copy_from_slice(&1u16.to_le_bytes());
        buf[30..32].copy_from_slice(&0u16.to_le_bytes());
        buf[32..48].fill(0xCD);

        for frame_len in 0..CarDecoder::HEADER_LENGTH {
            match AnyMessage::decode_frame(&buf, 0, frame_len) {
                Err(sbe_rt::DecodeError::BufferTooShort { field, needed, available }) => {
                    assert_eq!(field, "message header");
                    assert_eq!(needed, CarDecoder::HEADER_LENGTH);
                    assert_eq!(available, frame_len);
                }
                Err(other) => panic!("unexpected error for frame_len={frame_len}: {other:?}"),
                Ok(_) => panic!("short declared length {frame_len} must not decode"),
            }
        }

        // A short first declared length must not consume the second header.
        match AnyMessage::decode_frame(&buf, 0, 4) {
            Err(sbe_rt::DecodeError::BufferTooShort { field, needed, available }) => {
                assert_eq!(field, "message header");
                assert_eq!(needed, 8);
                assert_eq!(available, 4);
            }
            Err(other) => panic!("unexpected error: {other:?}"),
            Ok(_) => panic!("must not read the following frame's header"),
        }

        let first = AnyMessage::decode_frame(&buf, 0, 24)?;
        match first.message {
            AnyMessage::Unknown { header, frame } => {
                assert_eq!(header.template_id(), 99);
                assert_eq!(frame.len(), 24);
            }
            _ => panic!("expected Unknown for template 99"),
        }
        let second = AnyMessage::decode_frame(&buf, 24, 24)?;
        match second.message {
            AnyMessage::Unknown { header, frame } => {
                assert_eq!(header.template_id(), 77);
                assert_eq!(frame.len(), 24);
            }
            _ => panic!("expected Unknown for template 77"),
        }

        let known_need = CarEncoder::compute_length()
            .fuel_figures(0)
            .finish_empty()?
            .performance_figures(0)
            .finish_empty()?
            .manufacturer(1)?
            .model(1)?
            .activation_code(1)?
            .encoded_length_with_header();
        let mut body = vec![0u8; known_need];
        let known_len = CarEncoder::wrap_and_apply_header(&mut body, 0)
            .fixed(&CarFixedFields {
                serial_number: 7,
                model_year: 2020,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0; 4],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(1, 1, [0; 3], 0i8, BooleanType::F,
                                    Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"H")?
            .model(b"C")?
            .activation_code(b"A")?
            .encoded_length_with_header();
        let known = AnyMessage::decode_frame(&body, 0, known_len)?;
        match known.message {
            AnyMessage::Car(car) => assert_eq!(car.serial_number(), 7),
            _ => panic!("expected known Car"),
        }
        "#,
    );
    Ok(())
}
