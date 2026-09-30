//! Compile and execute cross-schema dispatch with real generated codecs.
mod common;

use ergo_sbe::{GenerateError, GenerationConfig, Generator, Schema, parse};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn schema(id: u16, order: &str, custom: bool) -> Result<Schema, Box<dyn std::error::Error>> {
    schema_with_header(id, order, custom, "messageHeader")
}

fn schema_with_header(
    id: u16,
    order: &str,
    custom: bool,
    name: &str,
) -> Result<Schema, Box<dyn std::error::Error>> {
    let header = if custom {
        r#"<type name="version" primitiveType="uint16"/>
           <type name="schemaId" primitiveType="uint16"/>
           <type name="templateId" primitiveType="uint16"/>
           <type name="blockLength" primitiveType="uint16"/>"#
    } else {
        r#"<type name="blockLength" primitiveType="uint16"/>
           <type name="templateId" primitiveType="uint16"/>
           <type name="schemaId" primitiveType="uint16"/>
           <type name="version" primitiveType="uint16"/>"#
    };
    Ok(Schema::from_ir(parse(&format!(
        r#"<messageSchema package="dispatch" id="{id}" version="0" byteOrder="{order}" headerType="{name}">
        <types><composite name="{name}">{header}</composite></types>
        <message name="Ping" id="1"><field name="value" id="1" type="uint32"/></message>
        </messageSchema>"#
    ))?))
}

#[test]
fn dispatch_compiles_with_standalone_shared_and_custom_headers() -> TestResult {
    for (case, order, custom, shared, name) in [
        ("renamed", "littleEndian", false, false, "foo"),
        ("standalone", "littleEndian", false, false, "messageHeader"),
        ("shared", "littleEndian", false, true, "messageHeader"),
        ("custom_le", "littleEndian", true, false, "messageHeader"),
        ("custom_be", "bigEndian", true, false, "messageHeader"),
    ] {
        let a = schema_with_header(11, order, custom, name)?;
        let b = schema_with_header(22, order, custom, name)?;
        let config = if shared {
            GenerationConfig::new("a").with_shared_module("a")
        } else {
            GenerationConfig::new("a")
        };
        let generator = Generator::new(config);
        let schemas = [(&a, "a"), (&b, "b")];
        let set = generator.generate_multi(&schemas)?;
        let dispatch = generator.generate_schema_dispatch(&schemas, "all")?;
        let mut modules: Vec<_> = set
            .modules()
            .map(|m| (m.path.trim_end_matches(".rs"), m.source.as_str()))
            .collect();
        modules.push(("all", &dispatch.source));
        common::compile_and_run_modules(
            &format!("schema_dispatch_{case}"),
            &modules,
            &(format!(
                "let wire = u16::to_{}_bytes; let (ti, si, bl) = ({}, {}, {});\n",
                if order == "bigEndian" { "be" } else { "le" },
                if custom { 4 } else { 2 },
                if custom { 2 } else { 4 },
                if custom { 6 } else { 0 }
            ) + r#"
            use all::{AnySchemaMessage, SchemaDecodeError};
            const LEN: usize = a::PingEncoder::compute_length_with_header();
            const HEADER: usize = LEN - 4;
            let mut first = [0u8; LEN];
            a::PingEncoder::wrap_and_apply_header(&mut first, 0)
                .fixed(&a::PingFixedFields { value: 10 }).encoded_length_with_header();
            let mut second = [0u8; LEN];
            b::PingEncoder::wrap_and_apply_header(&mut second, 0)
                .fixed(&b::PingFixedFields { value: 20 }).encoded_length_with_header();
            let decoded = AnySchemaMessage::decode(&first, 0)?;
            assert_eq!(decoded.schema_id(), 11);
            assert_eq!(decoded.as_bytes(), first);
            match decoded {
                AnySchemaMessage::A(a::AnyMessage::Ping(ping), _) => assert_eq!(ping.value(), 10),
                _ => return Err("wrong first schema".into()),
            }
            match AnySchemaMessage::decode(&second, 0)? {
                AnySchemaMessage::B(b::AnyMessage::Ping(ping), _) => assert_eq!(ping.value(), 20),
                _ => return Err("wrong second schema".into()),
            }
            let mut offset_frame = [0u8; LEN + 3];
            offset_frame[3..].copy_from_slice(&second);
            assert_eq!(AnySchemaMessage::decode(&offset_frame, 3)?.as_bytes(), second);
            for offset in [LEN, usize::MAX] {
                assert!(matches!(AnySchemaMessage::decode(&first, offset),
                    Err(SchemaDecodeError::BufferTooShort { available: 0, .. })));
            }
            for end in 0..HEADER {
                assert!(matches!(AnySchemaMessage::decode(&first[..end], 0),
                    Err(SchemaDecodeError::BufferTooShort { .. })));
            }
            assert!(matches!(AnySchemaMessage::decode(&first[..LEN-1], 0),
                Err(SchemaDecodeError::A(_))));
            let mut unknown = [0u8; HEADER + 2];
            unknown[..HEADER].copy_from_slice(&first[..HEADER]);
            unknown[bl..bl+2].copy_from_slice(&wire(0));
            unknown[ti..ti+2].copy_from_slice(&wire(77));
            unknown[si..si+2].copy_from_slice(&wire(99));
            let decoded = AnySchemaMessage::decode(&unknown, 0)?;
            assert_eq!(decoded.schema_id(), 99);
            assert_eq!(decoded.as_bytes(), unknown);
            assert!(matches!(decoded, AnySchemaMessage::Other { template_id: 77, .. }));
            unknown[si..si+2].copy_from_slice(&wire(11));
            let decoded = AnySchemaMessage::decode(&unknown, 0)?;
            assert!(matches!(decoded, AnySchemaMessage::A(a::AnyMessage::Unknown { .. }, _)));
            assert_eq!(decoded.as_bytes(), unknown);
            "#),
        );
    }
    Ok(())
}

#[test]
fn ambiguous_dispatch_sets_are_rejected() -> TestResult {
    let a = schema(11, "littleEndian", false)?;
    let same_id = schema(11, "littleEndian", false)?;
    let b = schema(22, "littleEndian", false)?;
    let be = schema(22, "bigEndian", false)?;
    let custom = schema(22, "littleEndian", true)?;
    let generator = Generator::new(GenerationConfig::new("a"));
    for schemas in [
        vec![],
        vec![(&a, "a"), (&same_id, "b")],
        vec![(&a, "a"), (&b, "a")],
        vec![(&a, "foo_bar"), (&b, "fooBar")],
        vec![(&a, "other")],
        vec![(&a, "bad/name")],
        vec![(&a, "a"), (&be, "b")],
        vec![(&a, "a"), (&custom, "b")],
    ] {
        assert!(matches!(
            generator.generate_schema_dispatch(&schemas, "all"),
            Err(GenerateError::InvalidConfiguration { .. })
        ));
    }
    for name in ["a", "../all", "mod", ""] {
        assert!(
            generator
                .generate_schema_dispatch(&[(&a, "a")], name)
                .is_err()
        );
    }
    assert!(
        Generator::new(GenerationConfig::new("a").with_dispatch(false))
            .generate_schema_dispatch(&[(&a, "a")], "all")
            .is_err()
    );
    Ok(())
}

#[test]
fn forwarding_retains_a_new_versions_trailing_fields() -> TestResult {
    let old = schema(11, "littleEndian", false)?;
    let future = Schema::from_ir(parse(
        r#"
        <messageSchema package="dispatch" id="11" version="1" byteOrder="littleEndian">
        <types>
          <composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
          </composite>
          <composite name="varString">
            <type name="length" primitiveType="uint16"/>
            <type name="varData" primitiveType="uint8" length="0"/>
          </composite>
        </types>
        <message name="Ping" id="1">
          <field name="value" id="1" type="uint32"/>
          <data name="extra" id="2" type="varString" sinceVersion="1"/>
        </message>
        </messageSchema>"#,
    )?);
    let generator = Generator::new(GenerationConfig::new("a"));
    let set = generator.generate_multi(&[(&old, "a"), (&future, "future")])?;
    let dispatcher = generator.generate_schema_dispatch(&[(&old, "a")], "all")?;
    let mut modules: Vec<_> = set
        .modules()
        .map(|m| (m.path.trim_end_matches(".rs"), m.source.as_str()))
        .collect();
    modules.push(("all", &dispatcher.source));
    common::compile_and_run_modules(
        "schema_dispatch_future_tail",
        &modules,
        r#"
        let mut frame = [0u8; future::PingEncoder::compute_length_with_header(3)];
        let len = future::PingEncoder::wrap_and_apply_header(&mut frame, 0)
            .fixed(&future::PingFixedFields { value: 10 })
            .extra(b"new")?.encoded_length_with_header();
        assert_eq!(len, frame.len());
        let message = all::AnySchemaMessage::decode(&frame, 0)?;
        assert_eq!(message.as_bytes(), frame);
        match message {
            all::AnySchemaMessage::A(a::AnyMessage::Ping(ping), _) => {
                assert_eq!(ping.value(), 10);
                assert!(ping.as_bytes_with_header()?.len() < frame.len());
            }
            _ => return Err("wrong old schema variant".into()),
        }
    "#,
    );
    Ok(())
}

#[test]
fn constant_schema_id_with_a_decoy_member_is_rejected() -> TestResult {
    for decoy in ["", r#"<type name="extraSchemaId" primitiveType="uint16"/>"#] {
        let schema = Schema::from_ir(parse(&format!(
            r#"
            <messageSchema package="dispatch" id="11" version="0" byteOrder="littleEndian">
            <types><composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16" presence="constant">11</type>
              {decoy}
              <type name="version" primitiveType="uint16"/>
            </composite></types>
            <message name="Ping" id="1"><field name="value" id="1" type="uint32"/></message>
            </messageSchema>"#
        ))?);
        let generator = Generator::new(GenerationConfig::new("a"));
        assert!(matches!(
            generator.generate_schema_dispatch(&[(&schema, "a")], "all"),
            Err(GenerateError::InvalidConfiguration { .. })
        ));
    }
    Ok(())
}

#[test]
fn encoded_identity_members_with_decoys_are_rejected() -> TestResult {
    for decoy in [
        "extraSchemaId",
        "extraTemplateId",
        "extraBlockLength",
        "extraVersion",
    ] {
        let schema = Schema::from_ir(parse(&format!(
            r#"
            <messageSchema package="dispatch" id="11" version="0" byteOrder="littleEndian">
            <types><composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
              <type name="{decoy}" primitiveType="uint16"/>
            </composite></types>
            <message name="Ping" id="1"><field name="value" id="1" type="uint32"/></message>
            </messageSchema>"#
        ))?);
        let generator = Generator::new(GenerationConfig::new("a"));
        assert!(matches!(
            generator.generate_schema_dispatch(&[(&schema, "a")], "all"),
            Err(GenerateError::InvalidConfiguration { reason, .. })
                if reason.contains("unambiguous")
        ));
    }
    Ok(())
}

#[test]
fn wide_header_identity_is_checked_without_truncation() -> TestResult {
    let schema = Schema::from_ir(parse(
        r#"
        <messageSchema package="dispatch" id="11" version="0" byteOrder="littleEndian">
        <types><composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint32"/>
          <type name="schemaId" primitiveType="uint32"/>
          <type name="version" primitiveType="uint16"/>
        </composite></types>
        <message name="Ping" id="1"><field name="value" id="1" type="uint32"/></message>
        </messageSchema>"#,
    )?);
    let generator = Generator::new(GenerationConfig::new("a"));
    let set = generator.generate(&schema)?;
    let module = set.modules().next().ok_or("missing schema module")?;
    let dispatcher = generator.generate_schema_dispatch(&[(&schema, "a")], "all")?;
    common::compile_and_run_modules(
        "schema_dispatch_wide_header",
        &[("a", &module.source), ("all", &dispatcher.source)],
        r"
        let mut frame = [0u8; a::PingEncoder::compute_length_with_header()];
        a::PingEncoder::wrap_and_apply_header(&mut frame, 0)
            .fixed(&a::PingFixedFields { value: 10 }).encoded_length_with_header();
        assert_eq!(all::AnySchemaMessage::decode(&frame, 0)?.schema_id(), 11);
        frame[6..10].copy_from_slice(&65536u32.to_le_bytes());
        assert!(matches!(all::AnySchemaMessage::decode(&frame, 0),
            Err(all::SchemaDecodeError::InvalidHeader)));
        frame[6..10].copy_from_slice(&11u32.to_le_bytes());
        frame[2..6].copy_from_slice(&65536u32.to_le_bytes());
        assert!(matches!(all::AnySchemaMessage::decode(&frame, 0),
            Err(all::SchemaDecodeError::InvalidHeader)));
    ",
    );
    Ok(())
}
