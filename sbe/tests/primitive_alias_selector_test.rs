//! `ConversionSelector::named_type` on a primitive alias
//! (`<type name="Timestamp" primitiveType="uint64"/>`) selects every field
//! declared with that alias, in messages and groups, and no field declared
//! with the bare primitive. The mapping never changes a wire byte: the
//! runtime test compares the mapped codec's frame with the plain codec's.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

mod common;
use common::compile_and_run_modules;
use ergo_sbe::{
    ConversionSelector, GenerateError, GenerationConfig, Generator, Schema, SchemaSource, parse,
};

const SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="alias" id="73" version="0" byteOrder="littleEndian">
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
    <type name="Timestamp" primitiveType="uint64"/>
  </types>
  <sbe:message name="NewOrder" id="1">
    <field name="ts" id="1" type="Timestamp"/>
    <field name="tickTs" id="2" type="Timestamp"/>
    <field name="seq" id="3" type="uint64"/>
    <group name="fills" id="10" dimensionType="groupSizeEncoding">
      <field name="at" id="11" type="Timestamp"/>
    </group>
  </sbe:message>
  <sbe:message name="AggBook" id="2">
    <field name="ts" id="1" type="Timestamp"/>
  </sbe:message>
</sbe:messageSchema>"#;

fn generate(config: GenerationConfig) -> Result<String, GenerateError> {
    let schema = parse(SchemaSource::xml(SCHEMA))
        .map(Schema::from_ir)
        .map_err(|e| GenerateError::InvalidConversion {
            selector: "fixture".into(),
            reason: e.to_string(),
        })?;
    let set = Generator::new(config).generate(&schema)?;
    Ok(set
        .modules()
        .next()
        .map(|m| m.source.clone())
        .unwrap_or_default())
}

fn mapped(module: &str) -> GenerationConfig {
    GenerationConfig::new(module)
        .with_domain_type(ConversionSelector::named_type("Timestamp"), "app::EpochNs")
}

#[test]
fn named_type_on_a_primitive_alias_is_accepted() -> Result<(), GenerateError> {
    generate(mapped("alias_accept")).map(|_| ())
}

#[test]
fn named_type_on_a_builtin_primitive_name_is_rejected() {
    for name in ["uint64", "u64"] {
        match generate(
            GenerationConfig::new("alias_builtin")
                .with_domain_type(ConversionSelector::named_type(name), "app::EpochNs"),
        )
        .map(|_| ())
        {
            Err(GenerateError::InvalidConversion { selector, reason }) => {
                assert_eq!(selector, format!("NamedType({name:?})"));
                assert_eq!(reason, "no matching type found in schema");
            }
            other => panic!("named_type({name:?}) must be rejected, got {other:?}"),
        }
    }
}

const APP: &str = r#"
pub mod app {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EpochNs(pub u64);

    impl super::TryFromSbe<u64> for EpochNs {
        type Error = &'static str;
        fn try_from_sbe(wire: u64) -> Result<Self, Self::Error> {
            Ok(Self(wire))
        }
    }

    impl super::TryToSbe<u64> for EpochNs {
        type Error = &'static str;
        fn try_to_sbe(&self) -> Result<u64, Self::Error> {
            Ok(self.0)
        }
    }
}
"#;

/// Every `Timestamp` field — two message fields, a group field and a second
/// message's field — takes the domain type; `seq` (a bare `uint64`) keeps
/// `u64`. The mapped frames equal the plain codec's byte for byte.
#[test]
fn alias_fields_take_the_domain_type_without_changing_wire_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let mapped_src = generate(mapped("am"))? + APP;
    let plain_src = generate(GenerationConfig::new("ap"))?;
    compile_and_run_modules(
        "primitive_alias_runtime",
        &[("am", &mapped_src), ("ap", &plain_src)],
        r#"
        // ── plain codec: the reference frame ─────────────────────────
        let mut plain_buf = [0u8; 128];
        let plain_len = ap::NewOrderEncoder::try_wrap_and_apply_header(&mut plain_buf, 0)?
            .fixed(&ap::NewOrderFixedFields { ts: 11, tick_ts: 12, seq: 13 })
            .fills(1, |g| {
                g.add(|mut e| { e.at(14); Ok(()) })?;
                Ok(())
            })?
            .encoded_length_with_header();

        // ── mapped codec: alias fields through the domain setters ────
        let mut buf = [0u8; 128];
        let mut enc = am::NewOrderEncoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&am::NewOrderFixedFields { ts: 0, tick_ts: 0, seq: 13 });
        enc.ts(am::app::EpochNs(11))?.tick_ts(am::app::EpochNs(12))?;
        let len = enc
            .fills(1, |g| {
                g.add(|mut e| {
                    e.at(am::app::EpochNs(14)).map_err(|_| am::sbe_rt::EncodeError::DomainConversionFailed {
                        field: "at",
                        reason: "conversion",
                    })?;
                    Ok(())
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        assert_eq!(&buf[..len], &plain_buf[..plain_len], "alias mapping must not change the wire");

        let dec = am::NewOrderDecoder::try_from(&buf[..len])?;
        assert_eq!(dec.ts()?, am::app::EpochNs(11));
        assert_eq!(dec.tick_ts()?, am::app::EpochNs(12));
        assert_eq!((dec.ts_wire(), dec.tick_ts_wire()), (11, 12));
        // A bare uint64 is not an alias: its accessor stays u64.
        let seq: u64 = dec.seq();
        assert_eq!(seq, 13);
        let mut fills = 0;
        for fill in dec.fills()? {
            assert_eq!(fill.at()?, am::app::EpochNs(14));
            assert_eq!(fill.at_wire(), 14);
            fills += 1;
        }
        assert_eq!(fills, 1);

        // A second message's alias field is selected too.
        let mut book = [0u8; 64];
        let mut enc = am::AggBookEncoder::try_wrap_and_apply_header(&mut book, 0)?
            .fixed(&am::AggBookFixedFields { ts: 0 });
        enc.ts(am::app::EpochNs(21))?;
        let n = enc.encoded_length_with_header();
        let mut plain_book = [0u8; 64];
        let plain_n = ap::AggBookEncoder::try_wrap_and_apply_header(&mut plain_book, 0)?
            .fixed(&ap::AggBookFixedFields { ts: 21 })
            .encoded_length_with_header();
        assert_eq!(&book[..n], &plain_book[..plain_n]);
        assert_eq!(am::AggBookDecoder::try_from(&book[..n])?.ts()?, am::app::EpochNs(21));
        "#,
    );
    Ok(())
}
