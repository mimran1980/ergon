//! Encode-side `<name>_as_str` var-data setters — the write counterpart of
//! the decode-side `<name>_as_str` / `<name>_as_str_unchecked` readers.
//!
//! Emitted at the same two locations as the readers (message-level in
//! `message_encoder.rs`, group-entry in `group_encoder.rs`), gated on the
//! same `characterEncoding`, and both forward into the existing checked byte
//! setter so length/buffer validation lives in one place. Compilation is the
//! assertion for shape; runtime assertions cover the one behavioural
//! difference from the byte setter — ASCII validation.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::compile_and_run;

const XML: &str = r#"<messageSchema package="vdstr" id="1" version="0" byteOrder="littleEndian">
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
    <composite name="varAscii">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="ASCII"/>
    </composite>
    <composite name="varUtf8">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
    </composite>
    <composite name="varBinary">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="Msg" id="1" blockLength="4">
    <field name="seq" id="1" type="uint32" offset="0"/>
    <group name="legs" id="5" dimensionType="groupSizeEncoding" blockLength="4">
      <field name="qty" id="6" type="uint32" offset="0"/>
      <data name="legTag" id="7" type="varAscii"/>
      <data name="legNote" id="8" type="varUtf8"/>
    </group>
    <data name="tag" id="2" type="varAscii"/>
    <data name="note" id="3" type="varUtf8"/>
    <data name="blob" id="4" type="varBinary"/>
  </message>
</messageSchema>"#;

fn generated_source(module_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    generated_source_from(module_name, XML)
}

fn generated_source_from(
    module_name: &str,
    xml: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
    let schema = Schema::from_ir(parse(xml)?);
    let src = Generator::new(GenerationConfig::new(module_name))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();
    Ok(src)
}

// A fixed field named `<vd>AsStr` already owns the name the var-data setter
// would claim, at both message and entry level. The setter must stand down —
// same guard as the decode-side reader in `var_data_as_str_methods`.
const COLLIDING_XML: &str = r#"<messageSchema package="vdstrcollide" id="1" version="0" byteOrder="littleEndian">
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
    <composite name="varAscii">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="ASCII"/>
    </composite>
  </types>
  <message name="Msg" id="1" blockLength="8">
    <field name="seq" id="1" type="uint32" offset="0"/>
    <field name="tagAsStr" id="2" type="uint32" offset="4"/>
    <group name="legs" id="5" dimensionType="groupSizeEncoding" blockLength="8">
      <field name="qty" id="6" type="uint32" offset="0"/>
      <field name="legTagAsStr" id="7" type="uint32" offset="4"/>
      <data name="legTag" id="8" type="varAscii"/>
    </group>
    <data name="tag" id="3" type="varAscii"/>
  </message>
</messageSchema>"#;

#[test]
fn as_str_setters_exist_only_where_text_is_declared() -> Result<(), Box<dyn std::error::Error>> {
    let src = generated_source("vdstr_shape")?;
    // Message level: ASCII and UTF-8 fields get the setter, binary does not.
    assert!(
        src.contains("fn tag_as_str("),
        "ASCII message field missing setter"
    );
    assert!(
        src.contains("fn note_as_str("),
        "UTF-8 message field missing setter"
    );
    assert!(
        !src.contains("fn blob_as_str("),
        "binary var-data must not get a text setter"
    );
    // Group entry: same rule.
    assert!(
        src.contains("fn leg_tag_as_str("),
        "ASCII entry field missing setter"
    );
    assert!(
        src.contains("fn leg_note_as_str("),
        "UTF-8 entry field missing setter"
    );
    Ok(())
}

#[test]
fn ascii_as_str_setter_accepts_ascii_and_rejects_non_ascii()
-> Result<(), Box<dyn std::error::Error>> {
    let src = generated_source("vdstr_ascii")?;
    compile_and_run(
        "vdstr_ascii",
        &src,
        r#"
        // Accepts plain ASCII, message level and group entry both.
        let sized = MsgEncodedLength::new()
            .legs_ragged(1, |g| { g.add()?.leg_tag(3)?.leg_note("wörld".len())?; Ok(()) })?
            .tag(3)?
            .note("héllo".len())?
            .blob(1)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = MsgEncoder::wrap_and_apply_header(&mut storage, 0)
            .fixed(&MsgFixedFields { seq: 1 })
            .legs(1, |g| {
                g.add(|mut e| {
                    e.qty(9u32);
                    e.leg_tag_as_str("xyz")?.leg_note_as_str("wörld")
                })?;
                Ok(())
            })?
            .tag_as_str("abc")?
            .note_as_str("héllo")?
            .blob(&[0xFFu8])?
            .encoded_length_with_header();
        assert_eq!(sized, len);
        let buf = &storage[..len];

        let dec = MsgDecoder::try_decode(&buf[..len], 0)?;
        assert_eq!(dec.tag_as_str()?, "abc");
        assert_eq!(dec.note_as_str()?, "héllo");
        assert_eq!(dec.blob()?, &[0xFFu8]);
        let leg = dec.legs()?.next().unwrap()?;
        assert_eq!(leg.leg_tag_as_str()?, "xyz");
        assert_eq!(leg.leg_note_as_str()?, "wörld");

        // Rejects non-ASCII text on the ASCII field — message level. Sized
        // for the call to succeed on bytes; the ASCII check must still
        // reject it before any byte is written.
        let sized2 = MsgEncodedLength::new()
            .legs_ragged(0, |_| Ok(()))?
            .tag("café".len())?
            .note(0)?
            .blob(0)?
            .encoded_length_with_header();
        let mut buf2 = vec![0u8; sized2];
        let err = MsgEncoder::wrap_and_apply_header(&mut buf2, 0)
            .fixed(&MsgFixedFields { seq: 1 })
            .legs(0, |_| Ok(()))?
            .tag_as_str("café")
            .unwrap_err();
        assert!(
            matches!(err, sbe_rt::EncodeError::InvalidAscii { field: "tag" }),
            "expected InvalidAscii for non-ASCII text on an ASCII field, got {err:?}"
        );

        // Rejects non-ASCII text on the ASCII field — group entry.
        let sized3 = MsgEncodedLength::new()
            .legs_ragged(1, |g| { g.add()?.leg_tag("café".len())?.leg_note(0)?; Ok(()) })?
            .tag(0)?
            .note(0)?
            .blob(0)?
            .encoded_length_with_header();
        let mut buf3 = vec![0u8; sized3];
        let err = MsgEncoder::wrap_and_apply_header(&mut buf3, 0)
            .fixed(&MsgFixedFields { seq: 1 })
            .legs(1, |g| {
                g.add(|mut e| {
                    e.qty(9u32);
                    e.leg_tag_as_str("café")?.leg_note(b"")
                })?;
                Ok(())
            })
            .unwrap_err();
        assert!(
            matches!(err, sbe_rt::EncodeError::InvalidAscii { field: "legTag" }),
            "expected InvalidAscii for non-ASCII text on an ASCII entry field, got {err:?}"
        );
        "#,
    );
    Ok(())
}

#[test]
fn as_str_setter_matches_the_byte_setter_bit_for_bit() -> Result<(), Box<dyn std::error::Error>> {
    // The `_as_str` setter is a thin wrapper over the byte setter: same
    // length/buffer validation, same wire bytes, for any text that would
    // pass both. Encoding through each path from the same source text must
    // produce an identical wire image.
    let src = generated_source("vdstr_match")?;
    compile_and_run(
        "vdstr_match",
        &src,
        r#"
        let text = "identical";
        let sized = MsgEncodedLength::new()
            .legs_ragged(0, |_| Ok(()))?
            .tag(text.len())?
            .note(0)?
            .blob(0)?
            .encoded_length_with_header();

        let mut via_bytes = vec![0u8; sized];
        let len_a = MsgEncoder::wrap_and_apply_header(&mut via_bytes, 0)
            .fixed(&MsgFixedFields { seq: 1 })
            .legs(0, |_| Ok(()))?
            .tag(text.as_bytes())?
            .note(b"")?
            .blob(&[])?
            .encoded_length_with_header();

        let mut via_str = vec![0u8; sized];
        let len_b = MsgEncoder::wrap_and_apply_header(&mut via_str, 0)
            .fixed(&MsgFixedFields { seq: 1 })
            .legs(0, |_| Ok(()))?
            .tag_as_str(text)?
            .note(b"")?
            .blob(&[])?
            .encoded_length_with_header();

        assert_eq!(sized, len_a);
        assert_eq!(len_a, len_b);
        assert_eq!(&via_bytes[..len_a], &via_str[..len_b]);
        "#,
    );
    Ok(())
}

// `characterEncoding` is free text in the SBE spec, not a closed enum: ASCII
// legally spells as `ASCII` or `US-ASCII`, UTF-8 as `UTF-8` or `UTF8`, both
// case-insensitively. Message decode, group-entry decode, and message/entry
// encode must all recognise every spelling the same way — a schema using
// `US-ASCII` (as `samples/cluster-rfq`'s real schema does) or `UTF8` must not
// silently lose the `_as_str` accessor at some locations while keeping it at
// others.
const SPELLING_VARIANTS_XML: &str = r#"<messageSchema package="vdstrspell" id="1" version="0" byteOrder="littleEndian">
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
    <composite name="varUsAscii">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="US-ASCII"/>
    </composite>
    <composite name="varUtf8NoHyphen">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF8"/>
    </composite>
  </types>
  <message name="Msg" id="1" blockLength="4">
    <field name="seq" id="1" type="uint32" offset="0"/>
    <group name="legs" id="5" dimensionType="groupSizeEncoding" blockLength="4">
      <field name="qty" id="6" type="uint32" offset="0"/>
      <data name="legTag" id="7" type="varUsAscii"/>
    </group>
    <data name="note" id="2" type="varUtf8NoHyphen"/>
  </message>
</messageSchema>"#;

#[test]
fn as_str_setter_recognises_every_character_encoding_spelling()
-> Result<(), Box<dyn std::error::Error>> {
    let src = generated_source_from("vdstr_spell", SPELLING_VARIANTS_XML)?;
    // Decode side: message-level `note` (UTF8, no hyphen) and group-entry
    // `legTag` (US-ASCII) both get the reader.
    assert!(
        src.contains("fn note_as_str("),
        "UTF8 (no hyphen) message field missing decode _as_str, got source:\n{src}"
    );
    assert!(
        src.contains("fn leg_tag_as_str("),
        "US-ASCII entry field missing decode _as_str, got source:\n{src}"
    );
    // Encode side: same two fields get the `&str`-taking setter too — the
    // `fn note_as_str(`/`fn leg_tag_as_str(` count above already includes it
    // (one per decode lane plus the encoder); the compile-and-run round-trip
    // below is the precise proof, calling both as `_as_str(&str) -> Result<_,
    // EncodeError>`.

    compile_and_run(
        "vdstr_spell",
        &src,
        r#"
        let sized = MsgEncodedLength::new()
            .legs_ragged(1, |g| { g.add()?.leg_tag(3)?; Ok(()) })?
            .note("héllo".len())?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = MsgEncoder::wrap_and_apply_header(&mut storage, 0)
            .fixed(&MsgFixedFields { seq: 1 })
            .legs(1, |g| {
                g.add(|mut e| {
                    e.qty(9u32);
                    e.leg_tag_as_str("xyz")
                })?;
                Ok(())
            })?
            .note_as_str("héllo")?
            .encoded_length_with_header();
        assert_eq!(sized, len);
        let buf = &storage[..len];

        // Base decoder, both lanes.
        let dec = MsgDecoder::try_decode(buf, 0)?;
        assert_eq!(dec.note_as_str()?, "héllo");
        let leg = dec.legs()?.next().unwrap()?;
        assert_eq!(leg.leg_tag_as_str()?, "xyz");

        // Memoized lane.
        let mem = MsgDecoder::try_decode(buf, 0)?.memoized();
        assert_eq!(mem.note_as_str()?, "héllo");

        // Mutable ordered lane — wire order is legs, then note.
        let mut ord = MsgDecoder::try_decode(buf, 0)?.ordered();
        ord.legs()?.visit_entries(|e| -> Result<(), sbe_rt::DecodeError> {
            assert_eq!(e.leg_tag_as_str()?, "xyz");
            Ok(())
        })?;
        assert_eq!(ord.note_as_str()?, "héllo");
        "#,
    );
    Ok(())
}

#[test]
fn as_str_setter_stands_down_when_a_sibling_field_already_claims_the_name()
-> Result<(), Box<dyn std::error::Error>> {
    // A group entry has its own fixed field `legTagAsStr` alongside var-data
    // `legTag` (ASCII) — same collision shape the decode-side guard in
    // `var_data_as_str_methods` handles. If the encoder emitted the var-data
    // `_as_str` setter unconditionally, the generated module would carry two
    // `fn leg_tag_as_str` methods on the same entry type and fail to compile.
    // Compilation succeeding is the proof: without the guard, the entry
    // encoder would carry both `fn leg_tag_as_str(&mut self, val: u32)` (the
    // fixed field) and `fn leg_tag_as_str(self, src: &str)` (the var-data
    // setter) on the same type, which rustc rejects as a duplicate
    // definition. `leg_tag_as_str` itself legitimately appears elsewhere too
    // (decoder getters, per-lane accessors), so only the var-data setter's
    // distinctive `&str` signature is checked directly.
    let src = generated_source_from("vdstr_collide", COLLIDING_XML)?;
    assert!(
        !src.contains("fn leg_tag_as_str(self, src: &str)"),
        "the var-data `_as_str` setter must stand down when a fixed field \
         already claims the name, got source:\n{src}"
    );
    compile_and_run(
        "vdstr_collide",
        &src,
        r#"
        let sized = MsgEncodedLength::new()
            .legs_ragged(1, |g| { g.add()?.leg_tag(3)?; Ok(()) })?
            .tag(3)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; sized];
        let len = MsgEncoder::wrap_and_apply_header(&mut storage, 0)
            .fixed(&MsgFixedFields { seq: 1, tag_as_str: 9 })
            .legs(1, |g| {
                g.add(|mut e| {
                    e.qty(9u32);
                    e.leg_tag_as_str(7u32);
                    e.leg_tag(b"xyz")
                })?;
                Ok(())
            })?
            .tag(b"abc")?
            .encoded_length_with_header();
        assert_eq!(sized, len);
        "#,
    );
    Ok(())
}
