//! The send helpers: `{Msg}Encoder::send` / `send_fixed` and
//! `sbe_rt::send_frame` claim, encode, check and commit one frame through the
//! generated `sbe_rt::Claim` trait, for a try-claim transport (a claimed
//! region it publishes in place) and an offer transport (`sbe_rt::OfferClaim`
//! over a scratch buffer). A frame that does not fill its claim exactly, or
//! does not start with the message's own header, is a typed error and the
//! claim is aborted — also when encoding fails or panics.
//!
//! Every test compiles the generated module into a scratch crate: the claim
//! below records commits and aborts, so the assertions are about what a
//! transport would actually publish.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{
    compile_and_run_modules, compile_fails_with_diagnostics,
    compile_fails_with_diagnostics_excluding,
};
use ergo_sbe::{GenerationConfig, Generator, Schema, parse, parse_file};

fn schema_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/domain-mapping-schema.xml"
    ))
}

fn generate(module: &str) -> Result<String, Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse_file(schema_path())?);
    let set = Generator::new(GenerationConfig::new(module)).generate(&schema)?;
    Ok(set
        .modules()
        .next()
        .ok_or("no module generated")?
        .source
        .clone())
}

/// A transport claim that records what it published and how often it was
/// given back, so a test can see exactly what reached the wire.
const TEST_CLAIM: &str = r#"
pub mod app {
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Debug, Default)]
    pub struct Log {
        pub committed: Vec<Vec<u8>>,
        pub aborted: usize,
    }

    pub struct TestClaim {
        pub buf: Vec<u8>,
        pub log: Rc<RefCell<Log>>,
        pub refuse_commit: bool,
    }

    impl TestClaim {
        pub fn new(len: usize, log: &Rc<RefCell<Log>>) -> Self {
            // Dirty bytes: a frame that leaves any of them untouched shows up.
            Self { buf: vec![0xAA; len], log: Rc::clone(log), refuse_commit: false }
        }
    }

    impl super::sbe_rt::Claim for TestClaim {
        type Error = &'static str;
        fn frame_mut(&mut self) -> &mut [u8] {
            &mut self.buf
        }
        fn commit(self) -> Result<(), Self::Error> {
            if self.refuse_commit {
                return Err("back pressured");
            }
            let Self { buf, log, .. } = self;
            log.borrow_mut().committed.push(buf);
            Ok(())
        }
        fn abort(self) {
            self.log.borrow_mut().aborted += 1;
        }
    }
}
"#;

/// `send` and `send_fixed` encode into the claim and commit it when the
/// frame fills it exactly; a wrong length, a failed encode, a panic, or a
/// refused commit each come back as their own `SendError` variant, and
/// nothing is published.
#[test]
fn send_commits_exact_frames_and_aborts_everything_else() -> Result<(), Box<dyn std::error::Error>>
{
    let source = generate("dm")? + TEST_CLAIM;
    compile_and_run_modules(
        "claim_send_runtime",
        &[("dm", &source)],
        r#"
        use dm::*;
        use dm::app::{Log, TestClaim};
        use std::cell::RefCell;
        use std::rc::Rc;

        fn encode_order<'b>(
            enc: OrderEncoder<'b>,
            fixed: &OrderFixedFields,
            note: &[u8],
        ) -> Result<OrderComplete<'b>, sbe_rt::EncodeError> {
            enc.fixed(fixed)
                .fills(1, |g| {
                    g.add(|e| {
                        let mut e = e;
                        e.px(101).qty(2).side(Side::Sell);
                        e.allocs(1, |a| {
                            a.add(|x| {
                                x.account(3);
                                Ok(())
                            })
                        })?
                        .venue_ref(b"V1")
                    })
                })?
                .note(note)
        }
        fn explode<'b>(_: OrderEncoder<'b>) -> Result<OrderComplete<'b>, sbe_rt::EncodeError> {
            panic!("encoder panicked mid-frame")
        }

        let log = Rc::new(RefCell::new(Log::default()));

        // ── send_fixed: fixed-only message ─────────────────────────────
        let hb = HeartbeatFixedFields { seq: 42, side: Side::Buy };
        let mut direct = [0u8; HeartbeatEncoder::ENCODED_LENGTH];
        let _ = HeartbeatEncoder::wrap_into_claim(&mut direct)?.fixed(&hb);
        let l = HeartbeatEncoder::ENCODED_LENGTH;

        HeartbeatEncoder::send_fixed(TestClaim::new(l, &log), &hb).map_err(|e| e.to_string())?;
        assert_eq!(log.borrow().committed, vec![direct.to_vec()]);
        assert_eq!(log.borrow().aborted, 0);

        for claimed in [l + 1, l - 1] {
            let r = HeartbeatEncoder::send_fixed(TestClaim::new(claimed, &log), &hb);
            assert_eq!(r, Err(sbe_rt::SendError::LengthMismatch { claimed, frame: l }));
        }
        assert_eq!(log.borrow().committed.len(), 1, "a mis-sized claim is never published");
        assert_eq!(log.borrow().aborted, 2, "every mis-sized claim is aborted");

        // `send` on a fixed-only message ends at the fixed stage.
        HeartbeatEncoder::send(TestClaim::new(l, &log), |enc| Ok(enc.fixed(&hb)))
            .map_err(|e| e.to_string())?;
        assert_eq!(log.borrow().committed[1], direct.to_vec());

        // ── send: message with groups and var-data ─────────────────────
        let fixed = OrderFixedFields {
            order_id: 7,
            side: Side::Buy,
            leg: Leg::new(Price::new(12_345, -2), Side::Sell, 10, 4, 0.5, Flags(1), *b"XLON"),
            limit: Some(99),
        };
        let mut scratch = [0u8; 256];
        let len = encode_order(OrderEncoder::try_wrap_and_apply_header(&mut scratch, 0)?, &fixed, b"hello")?
            .encoded_length_with_header();
        let expected = scratch[..len].to_vec();

        OrderEncoder::send(TestClaim::new(len, &log), |enc| encode_order(enc, &fixed, b"hello"))
            .map_err(|e| e.to_string())?;
        assert_eq!(log.borrow().committed[2], expected);
        let sent = log.borrow().committed[2].clone();
        assert_eq!(OrderDecoder::decode(&sent, 0)?.order_id(), 7);

        // A claim sized for another frame: typed error, claim aborted.
        let r = OrderEncoder::send(TestClaim::new(len + 3, &log), |enc| encode_order(enc, &fixed, b"hello"));
        assert_eq!(r, Err(sbe_rt::SendError::LengthMismatch { claimed: len + 3, frame: len }));
        assert_eq!(log.borrow().aborted, 3);

        // The encoder's own error comes back unchanged and the claim is aborted.
        let too_long = [b'x'; 65];
        let r = OrderEncoder::send(TestClaim::new(len, &log), |enc| encode_order(enc, &fixed, &too_long));
        assert!(
            matches!(r, Err(sbe_rt::SendError::Encode(sbe_rt::EncodeError::VarDataTooLong { field: "note", .. }))),
            "{r:?}"
        );
        assert_eq!(log.borrow().aborted, 4);

        // A panicking encoder unwinds through the guard, which aborts.
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            OrderEncoder::send(TestClaim::new(len, &log), explode)
        }));
        assert!(unwound.is_err());
        assert_eq!(log.borrow().aborted, 5, "unwinding must abort the claim");

        // The transport refusing the commit is its own variant; the claim
        // went to `commit`, so it is not aborted as well.
        let mut refused = TestClaim::new(len, &log);
        refused.refuse_commit = true;
        let r = OrderEncoder::send(refused, |enc| encode_order(enc, &fixed, b"hello"));
        assert_eq!(r, Err(sbe_rt::SendError::Commit("back pressured")));
        assert_eq!(log.borrow().aborted, 5);
        assert_eq!(log.borrow().committed.len(), 3);

        // ── send_frame: a raw encoder that writes another message ──────
        let r = sbe_rt::send_frame(TestClaim::new(l, &log), &OrderEncoder::HEADER_TEMPLATE, |buf| {
            let _ = HeartbeatEncoder::wrap_into_claim(buf)?.fixed(&hb);
            Ok(HeartbeatEncoder::ENCODED_LENGTH)
        });
        assert_eq!(r, Err(sbe_rt::SendError::HeaderMismatch));
        assert_eq!(log.borrow().aborted, 6);
        sbe_rt::send_frame(TestClaim::new(l, &log), &HeartbeatEncoder::HEADER_TEMPLATE, |buf| {
            let _ = HeartbeatEncoder::wrap_into_claim(buf)?.fixed(&hb);
            Ok(HeartbeatEncoder::ENCODED_LENGTH)
        })
        .map_err(|e| e.to_string())?;
        assert_eq!(log.borrow().committed[3], direct.to_vec());

        // ── offer style: OfferClaim over a scratch buffer ───────────────
        let mut offered: Vec<Vec<u8>> = Vec::new();
        let mut buf = [0u8; HeartbeatEncoder::ENCODED_LENGTH];
        let claim = sbe_rt::OfferClaim::new(&mut buf, |frame: &[u8]| -> Result<(), &'static str> {
            offered.push(frame.to_vec());
            Ok(())
        });
        HeartbeatEncoder::send_fixed(claim, &hb).map_err(|e| e.to_string())?;
        assert_eq!(offered, vec![direct.to_vec()]);

        let mut short = [0u8; HeartbeatEncoder::ENCODED_LENGTH - 1];
        let claim = sbe_rt::OfferClaim::new(&mut short, |frame: &[u8]| -> Result<(), &'static str> {
            offered.push(frame.to_vec());
            Ok(())
        });
        let r = HeartbeatEncoder::send_fixed(claim, &hb);
        assert_eq!(r, Err(sbe_rt::SendError::LengthMismatch { claimed: l - 1, frame: l }));
        assert_eq!(offered.len(), 1, "an aborted offer claim never offers");

        // Display names the mismatch.
        let shown = sbe_rt::SendError::<&str>::LengthMismatch { claimed: 9, frame: 8 }.to_string();
        assert!(shown.contains('9') && shown.contains('8'), "{shown}");
        "#,
    );
    Ok(())
}

/// `Ping` has fields named `send` and `sendFixed`, `Echo` one named `send`
/// (a domain-typed one in `Route`), `Pong` neither.
const SEND_FIELDS_XML: &str = r#"<messageSchema package="ping" id="9" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <enum name="Venue" encodingType="uint8">
      <validValue name="Lit">1</validValue>
      <validValue name="Dark">2</validValue>
    </enum>
  </types>
  <message name="Ping" id="1">
    <field name="send" id="1" type="uint32"/>
    <field name="sendFixed" id="2" type="uint32"/>
  </message>
  <message name="Echo" id="2">
    <field name="send" id="1" type="uint32"/>
  </message>
  <message name="Pong" id="3">
    <field name="seq" id="1" type="uint32"/>
  </message>
  <message name="Route" id="4">
    <field name="send" id="1" type="Venue"/>
  </message>
</messageSchema>"#;

fn send_fields_module() -> Result<String, Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(SEND_FIELDS_XML)?);
    let source = Generator::new(
        GenerationConfig::new("ping")
            .with_enum_mapping(ergo_sbe::EnumMapping::new("Venue", "venue::Venue")),
    )
    .generate(&schema)?
    .modules()
    .next()
    .ok_or("no module")?
    .source
    .clone();
    Ok(source
        + TEST_CLAIM
        + r#"
pub mod venue {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Venue { Lit, Dark }
}
"#)
}

/// A field named `send` or `send_fixed` keeps the accessor it had before
/// the helpers existed (no `_field` rename), as `tail_accessor_ident`'s
/// convenience methods yield to a field: the clashing helper is left out of
/// that message, which still sends through `sbe_rt::send_frame`. A
/// domain-typed field's setter owns its plain name the same way, and a
/// message without the clash keeps both helpers.
#[test]
fn fields_named_send_keep_their_names_and_the_helper_yields()
-> Result<(), Box<dyn std::error::Error>> {
    let ping = send_fields_module()?;
    compile_and_run_modules(
        "claim_send_field_wins",
        &[("ping", &ping)],
        r#"
        use ping::*;
        use ping::app::{Log, TestClaim};
        use std::cell::RefCell;
        use std::rc::Rc;

        let log = Rc::new(RefCell::new(Log::default()));

        // The field setters keep the schema's names, exactly as before.
        let mut buf = [0u8; PingEncoder::ENCODED_LENGTH];
        let mut enc = PingEncoder::wrap_into_claim(&mut buf)?;
        let _: &mut PingEncoder<'_> = enc.send(1).send_fixed(2);
        let mut via_fixed = [0u8; PingEncoder::ENCODED_LENGTH];
        let _ = PingEncoder::wrap_into_claim(&mut via_fixed)?
            .fixed(&PingFixedFields { send: 1, send_fixed: 2 });
        assert_eq!(buf, via_fixed);
        let dec = PingDecoder::decode(&buf, 0)?;
        assert_eq!((dec.send(), dec.send_fixed()), (1, 2));

        // Its frames go out through send_frame, with the same checks.
        sbe_rt::send_frame(TestClaim::new(PingEncoder::ENCODED_LENGTH, &log), &PingEncoder::HEADER_TEMPLATE, |b| {
            let mut enc = PingEncoder::wrap_into_claim(b)?;
            enc.send(1).send_fixed(2);
            Ok(PingEncoder::ENCODED_LENGTH)
        })
        .map_err(|e| e.to_string())?;
        assert_eq!(log.borrow().committed, vec![buf.to_vec()]);

        // Only the clashing helper yields: Echo keeps `send_fixed`.
        let mut echo = [0u8; EchoEncoder::ENCODED_LENGTH];
        EchoEncoder::wrap_into_claim(&mut echo)?.send(3);
        EchoEncoder::send_fixed(TestClaim::new(EchoEncoder::ENCODED_LENGTH, &log), &EchoFixedFields { send: 3 })
            .map_err(|e| e.to_string())?;
        assert_eq!(log.borrow().committed[1], echo.to_vec());

        // A domain-typed field's setter is `send` too, so Route has no
        // `send` helper either; it keeps `send_fixed`.
        let mut route = [0u8; RouteEncoder::ENCODED_LENGTH];
        RouteEncoder::wrap_into_claim(&mut route)?.send(venue::Venue::Dark)?;
        assert_eq!(RouteDecoder::decode(&route, 0)?.send(), Ok(venue::Venue::Dark));
        RouteEncoder::send_fixed(TestClaim::new(RouteEncoder::ENCODED_LENGTH, &log), &RouteFixedFields { send: Venue::Dark })
            .map_err(|e| e.to_string())?;
        assert_eq!(log.borrow().committed[2], route.to_vec());

        // No clash: both helpers.
        let pong = PongFixedFields { seq: 4 };
        PongEncoder::send_fixed(TestClaim::new(PongEncoder::ENCODED_LENGTH, &log), &pong)
            .map_err(|e| e.to_string())?;
        PongEncoder::send(TestClaim::new(PongEncoder::ENCODED_LENGTH, &log), |enc| Ok(enc.fixed(&pong)))
            .map_err(|e| e.to_string())?;
        assert_eq!(log.borrow().committed[3], log.borrow().committed[4]);
        "#,
    );
    Ok(())
}

/// The yielded helpers are really gone: `send` on `Ping` and `Route` is the
/// field setter, and `Ping` has no `send_fixed` helper. The setters sit on
/// two stage impls, so a call by path is ambiguous between those two, and
/// only those: a helper would be a third candidate.
#[test]
fn a_message_whose_field_owns_send_has_no_send_helper() -> Result<(), Box<dyn std::error::Error>> {
    let ping = send_fields_module()?;
    compile_fails_with_diagnostics_excluding(
        "claim_send_field_owns_name",
        &ping,
        r#"
        let log = std::rc::Rc::new(std::cell::RefCell::new(app::Log::default()));
        let _ = PingEncoder::send_fixed(
            app::TestClaim::new(PingEncoder::ENCODED_LENGTH, &log),
            &PingFixedFields { send: 1, send_fixed: 2 },
        );
        let _ = PingEncoder::send(app::TestClaim::new(PingEncoder::ENCODED_LENGTH, &log), |enc| {
            Ok(enc.fixed(&PingFixedFields { send: 1, send_fixed: 2 }))
        });
        let _ = RouteEncoder::send(app::TestClaim::new(RouteEncoder::ENCODED_LENGTH, &log), |enc| {
            Ok(enc.fixed(&RouteFixedFields { send: Venue::Lit }))
        });
        "#,
        &[
            "multiple `send_fixed` found",
            "pub fn send_fixed(&mut self, val: u32) -> &mut Self",
            "multiple `send` found",
            "pub fn send(&mut self, val: u32) -> &mut Self",
            "value: venue::Venue,",
        ],
        &["note: candidate #3", "duplicate definitions"],
    );
    Ok(())
}

/// A message's first group or var-data is a method on the encoder itself,
/// so one named `send` would make every `XEncoder::send` path ambiguous,
/// the helper's and the tail's alike. The helper yields instead: the tail
/// keeps the path it had before the helpers existed, the frame still goes
/// out through `sbe_rt::send_frame`, and a `send` on a later tail, which
/// lives on another stage type, leaves the helper in place.
#[test]
fn a_first_tail_named_send_keeps_its_path() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<messageSchema package="tails" id="10" version="0" byteOrder="littleEndian">
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
    <composite name="varDataEncoding">
      <type name="length" primitiveType="uint16"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="Order" id="1">
    <field name="id" id="1" type="uint64"/>
    <group name="send" id="2" dimensionType="groupSizeEncoding">
      <field name="px" id="3" type="int64"/>
    </group>
  </message>
  <message name="Note" id="2">
    <field name="id" id="1" type="uint64"/>
    <data name="send" id="2" type="varDataEncoding"/>
  </message>
  <message name="Quote" id="3">
    <field name="id" id="1" type="uint64"/>
    <group name="legs" id="2" dimensionType="groupSizeEncoding">
      <field name="px" id="3" type="int64"/>
    </group>
    <data name="send" id="4" type="varDataEncoding"/>
  </message>
</messageSchema>"#;
    let schema = Schema::from_ir(parse(xml)?);
    let tails = Generator::new(GenerationConfig::new("tails"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("no module")?
        .source
        .clone();
    let tails = tails + TEST_CLAIM;
    compile_and_run_modules(
        "claim_send_first_tail",
        &[("tails", &tails)],
        r#"
        use tails::*;
        use tails::app::{Log, TestClaim};
        use std::cell::RefCell;
        use std::rc::Rc;

        // The tails keep their full paths.
        let mut order = [0u8; 64];
        let stage = OrderEncoder::try_wrap_and_apply_header(&mut order, 0)?
            .fixed(&OrderFixedFields { id: 1 });
        let order_len = OrderEncoder::send(stage, 1, |g| g.add(|e| { e.px(5); Ok(()) }))?
            .encoded_length_with_header();
        let mut note = [0u8; 64];
        let stage = NoteEncoder::try_wrap_and_apply_header(&mut note, 0)?
            .fixed(&NoteFixedFields { id: 2 });
        let note_len = NoteEncoder::send(stage, b"hi")?.encoded_length_with_header();

        // Their frames still go out with the same checks.
        let log = Rc::new(RefCell::new(Log::default()));
        sbe_rt::send_frame(TestClaim::new(order_len, &log), &OrderEncoder::HEADER_TEMPLATE, |buf| {
            let stage = OrderEncoder::try_wrap_and_apply_header(buf, 0)?.fixed(&OrderFixedFields { id: 1 });
            Ok(stage.send(1, |g| g.add(|e| { e.px(5); Ok(()) }))?.encoded_length_with_header())
        })
        .map_err(|e| e.to_string())?;
        sbe_rt::send_frame(TestClaim::new(note_len, &log), &NoteEncoder::HEADER_TEMPLATE, |buf| {
            let stage = NoteEncoder::try_wrap_and_apply_header(buf, 0)?.fixed(&NoteFixedFields { id: 2 });
            Ok(stage.send(b"hi")?.encoded_length_with_header())
        })
        .map_err(|e| e.to_string())?;
        assert_eq!(log.borrow().committed, vec![order[..order_len].to_vec(), note[..note_len].to_vec()]);

        // A later tail named `send` is on another stage: the helper stays.
        let mut quote = [0u8; 64];
        let quote_len = QuoteEncoder::try_wrap_and_apply_header(&mut quote, 0)?
            .fixed(&QuoteFixedFields { id: 3 })
            .legs(0, |_| Ok(()))?
            .send(b"q")?
            .encoded_length_with_header();
        QuoteEncoder::send(TestClaim::new(quote_len, &log), |enc| {
            enc.fixed(&QuoteFixedFields { id: 3 }).legs(0, |_| Ok(()))?.send(b"q")
        })
        .map_err(|e| e.to_string())?;
        assert_eq!(log.borrow().committed[2], quote[..quote_len].to_vec());
        "#,
    );
    Ok(())
}

/// `send` only commits a finished frame: its closure must return the
/// message's terminal stage, so a frame with groups and var-data still to
/// write is a compile error, not a short frame on the wire.
#[test]
fn send_requires_the_terminal_stage() -> Result<(), Box<dyn std::error::Error>> {
    let source = generate("dm")? + TEST_CLAIM;
    compile_fails_with_diagnostics(
        "claim_send_incomplete",
        &source,
        r#"
        let log = std::rc::Rc::new(std::cell::RefCell::new(app::Log::default()));
        let fixed = OrderFixedFields {
            order_id: 7,
            side: Side::Buy,
            leg: Leg::new(Price::new(1, 0), Side::Sell, 1, 1, 0.5, Flags(0), *b"XLON"),
            limit: None,
        };
        let _ = OrderEncoder::send(app::TestClaim::new(64, &log), |enc| Ok(enc.fixed(&fixed)));
        let _ = OrderEncoder::send_fixed(app::TestClaim::new(64, &log), &fixed);
        "#,
        &[
            "OrderComplete",
            "no associated function or constant named `send_fixed` found",
        ],
    );
    Ok(())
}

/// `Padded`: a 64-byte block holding 12 bytes of fields, then a group whose
/// 16-byte entries hold 8, then var-data. `Tick`: a fixed-only 32-byte block
/// holding 8. Every other byte of those blocks is padding.
const PADDED_XML: &str = r#"<messageSchema package="padded" id="11" version="0" byteOrder="littleEndian">
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
    <composite name="varDataEncoding">
      <type name="length" primitiveType="uint16"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="Padded" id="1" blockLength="64">
    <field name="id" id="1" type="uint64"/>
    <field name="qty" id="2" type="uint32"/>
    <group name="legs" id="3" dimensionType="groupSizeEncoding" blockLength="16">
      <field name="px" id="4" type="int64"/>
    </group>
    <data name="note" id="5" type="varDataEncoding"/>
  </message>
  <message name="Tick" id="2" blockLength="32">
    <field name="seq" id="1" type="uint64"/>
  </message>
</messageSchema>"#;

/// No encoder writes padding, so a frame's padding is whatever the claimed
/// region held. An Aeron `try_claim` region starts zeroed; the try-claim
/// claim here hands out 0xAA bytes instead, and every padding byte is still
/// 0xAA after `send`: the encoder spends nothing on padding, and that path
/// is unchanged. `OfferClaim` reuses one scratch buffer for every frame, so
/// it zeroes what it hands out: a frame sent through it after another one
/// carries zero padding, whichever path wrote it (`fixed()` in a `send`
/// closure, `send_fixed`, `encode_from`, the DTO through `send_frame`).
#[test]
fn offer_claims_zero_padding_and_try_claims_leave_it_untouched()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(PADDED_XML)?);
    let padded = Generator::new(
        GenerationConfig::new("padded")
            .with_source_traits(true)
            .with_domain_objects(ergo_sbe::DomainVarData::Strings),
    )
    .generate(&schema)?
    .modules()
    .next()
    .ok_or("no module")?
    .source
    .clone()
        + TEST_CLAIM;
    compile_and_run_modules(
        "claim_send_padding",
        &[("padded", &padded)],
        r#"
        use padded::*;
        use padded::app::{Log, TestClaim};
        use std::cell::RefCell;
        use std::rc::Rc;

        impl PaddedSource for PaddedDomain {
            fn id(&self) -> u64 { self.id }
            fn qty(&self) -> u32 { self.qty }
            fn legs(&self) -> impl ExactSizeIterator<Item = impl PaddedLegsEntrySource> { self.legs.iter() }
            fn note(&self) -> &[u8] { self.note.as_bytes() }
        }
        impl PaddedLegsEntrySource for PaddedLegsEntryDomain {
            fn px(&self) -> i64 { self.px }
        }

        // Header 8, block 64 (fields 0..12), group header 4, two 16-byte
        // entries (field 0..8): the padding of a two-leg frame.
        fn padding(frame: &[u8]) -> Vec<u8> {
            let mut out = frame[8 + 12..8 + 64].to_vec();
            for entry in 0..2 {
                let start = 8 + 64 + 4 + entry * 16;
                out.extend_from_slice(&frame[start + 8..start + 16]);
            }
            out
        }
        fn encode<'b>(enc: PaddedEncoder<'b>) -> Result<PaddedComplete<'b>, sbe_rt::EncodeError> {
            enc.fixed(&PaddedFixedFields { id: 7, qty: 3 })
                .legs(2, |g| {
                    g.add(|e| { e.px(-1); Ok(()) })?;
                    g.add(|e| { e.px(2); Ok(()) })
                })?
                .note(b"n")
        }
        let dto = PaddedDomain {
            id: 7,
            qty: 3,
            legs: vec![PaddedLegsEntryDomain { px: -1 }, PaddedLegsEntryDomain { px: 2 }],
            note: "n".into(),
        };
        let len = PaddedEncoder::encoded_length_with_header_from(&dto)?;
        let log = Rc::new(RefCell::new(Log::default()));

        // try_claim: the region's bytes stay as handed out.
        PaddedEncoder::send(TestClaim::new(len, &log), encode).map_err(|e| e.to_string())?;
        let claimed = log.borrow().committed[0].clone();
        assert!(padding(&claimed).iter().all(|b| *b == 0xAA), "the encoder wrote padding");

        // offer: one scratch buffer, dirty from the frame before each send.
        let mut scratch = vec![0u8; len];
        let mut offered: Vec<Vec<u8>> = Vec::new();
        let mut offer = |frame: &[u8]| -> Result<(), &'static str> {
            offered.push(frame.to_vec());
            Ok(())
        };
        scratch.fill(0x55);
        PaddedEncoder::send(sbe_rt::OfferClaim::new(&mut scratch, &mut offer), encode)
            .map_err(|e| e.to_string())?;
        scratch.fill(0x55);
        PaddedEncoder::send(sbe_rt::OfferClaim::new(&mut scratch, &mut offer), |enc| enc.encode_from(&dto))
            .map_err(|e| e.to_string())?;
        scratch.fill(0x55);
        sbe_rt::send_frame(
            sbe_rt::OfferClaim::new(&mut scratch, &mut offer),
            &PaddedEncoder::HEADER_TEMPLATE,
            |buf| dto.encode(buf),
        )
        .map_err(|e| e.to_string())?;
        let mut tick = [0x55u8; TickEncoder::ENCODED_LENGTH];
        TickEncoder::send_fixed(sbe_rt::OfferClaim::new(&mut tick, &mut offer), &TickFixedFields { seq: 9 })
            .map_err(|e| e.to_string())?;
        tick.fill(0x55);
        TickEncoder::send(sbe_rt::OfferClaim::new(&mut tick, &mut offer), |enc| {
            Ok(enc.fixed(&TickFixedFields { seq: 9 }))
        })
        .map_err(|e| e.to_string())?;

        assert_eq!(offered.len(), 5);
        for frame in &offered[..3] {
            assert!(padding(frame).iter().all(|b| *b == 0), "stale padding offered: {frame:?}");
            // The same frame as the try-claim one, padding aside.
            let mut expected = claimed.clone();
            for (i, b) in expected.iter_mut().enumerate() {
                if frame[i] == 0 && *b == 0xAA {
                    *b = 0;
                }
            }
            assert_eq!(frame, &expected);
            assert_eq!(PaddedDomain::try_from_slice_with_header(frame, 0)?, dto);
        }
        for frame in &offered[3..] {
            assert!(frame[8 + 8..].iter().all(|b| *b == 0), "stale padding offered: {frame:?}");
            assert_eq!(TickDecoder::decode(frame, 0)?.seq(), 9);
        }
        "#,
    );
    Ok(())
}
