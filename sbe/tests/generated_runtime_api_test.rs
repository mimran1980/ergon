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

// ─── Sealing ───────────────────────────────────────────────────────────────

#[test]
fn sbe_message_supertrait_lives_in_a_private_module() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "seal_shape");

    assert!(
        src.contains("pub trait SbeMessage: super::__sbe_message_sealed::Sealed"),
        "SbeMessage must carry the sealing supertrait, not merely claim to be sealed"
    );
    assert!(
        src.contains("\nmod __sbe_message_sealed {"),
        "the sealing module must be a private child of the generated module"
    );
    assert!(
        !src.contains("pub mod __sbe_message_sealed"),
        "a public sealing module would not seal anything"
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
fn consumer_cannot_name_the_sealing_trait() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "seal_name");

    compile_fails_with_diagnostics(
        "seal_name",
        &src,
        r#"
        struct ForgedMessage;
        impl seal_name::__sbe_message_sealed::Sealed for ForgedMessage {}
        "#,
        &["__sbe_message_sealed"],
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
