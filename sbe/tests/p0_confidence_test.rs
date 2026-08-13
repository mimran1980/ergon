//! Executed confidence tests for the P0 wire-safety matrix.

mod common;

use std::path::PathBuf;

use common::{Paths, compile_and_run, dual_encode_run, generate};
use ergo_sbe::{GenerationConfig, Generator, Schema, parse};

fn generate_xml(xml: &str, module_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let schema = Schema::from_ir(parse(xml)?);
    Ok(Generator::new(GenerationConfig::new(module_name))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("no generated module")?
        .source
        .clone())
}

fn standard_types(dimension: &str) -> String {
    format!(
        r#"
        <types>
          <composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
          </composite>
          {dimension}
          <composite name="varDataEncoding">
            <type name="length" primitiveType="uint32" maxValue="4096"/>
            <type name="varData" primitiveType="uint8" length="0"/>
          </composite>
        </types>
        "#
    )
}

#[test]
fn float_wire_bits_match_sbe_tool_for_all_ieee_classes() {
    dual_encode_run(
        "float_wire_bits",
        &Paths::sbe_tool_test_resource("issue895.xml"),
        "issue895",
        r#"
        use tool::{
            Encoder, ReadBuf, WriteBuf,
            issue_895_codec::{
                Issue895Decoder as ToolDecoder,
                Issue895Encoder as ToolEncoder,
            },
            message_header_codec,
        };

        let f32_bits = [
            0x0000_0000u32,
            0x8000_0000,
            0x7f80_0000,
            0xff80_0000,
            0x0000_0001,
            0x007f_ffff,
            0x7fc0_0001,
            0x7f80_0001,
            0x1234_5678,
        ];
        let f64_bits = [
            0x0000_0000_0000_0000u64,
            0x8000_0000_0000_0000,
            0x7ff0_0000_0000_0000,
            0xfff0_0000_0000_0000,
            0x0000_0000_0000_0001,
            0x000f_ffff_ffff_ffff,
            0x7ff8_0000_0000_0001,
            0x7ff0_0000_0000_0001,
            0x1234_5678_9abc_def0,
        ];

        for (&single_bits, &double_bits) in f32_bits.iter().zip(&f64_bits) {
            let single = f32::from_bits(single_bits);
            let double = f64::from_bits(double_bits);

            let mut ergo_buf = [0u8; Issue895Encoder::ENCODED_LENGTH];
            let ergo_len = Issue895Encoder::try_wrap_and_apply_header(&mut ergo_buf, 0).unwrap()
                .fixed(&Issue895FixedFields {
                    optional_float: Some(single),
                    optional_double: Some(double),
                })
                .encoded_length_with_header();

            let mut tool_buf = [0u8; Issue895Encoder::ENCODED_LENGTH];
            let mut tool_encoder = ToolEncoder::default().wrap(
                WriteBuf::new(&mut tool_buf),
                message_header_codec::ENCODED_LENGTH,
            );
            tool_encoder = tool_encoder.header(0).parent()?;
            tool_encoder.optional_float(single);
            tool_encoder.optional_double(double);
            let tool_len = tool_encoder.get_limit();

            assert_frames_eq(
                &format!("f32={single_bits:08x} f64={double_bits:016x}"),
                &ergo_buf[..ergo_len],
                &tool_buf[..tool_len],
            );

            let ergo_decoder = Issue895Decoder::try_from(&tool_buf[..tool_len])?;
            if !single.is_nan() {
                assert_eq!(
                    ergo_decoder.optional_float().map(f32::to_bits),
                    Some(single_bits)
                );
            }
            if !double.is_nan() {
                assert_eq!(
                    ergo_decoder.optional_double().map(f64::to_bits),
                    Some(double_bits)
                );
            }

            let tool_header = tool::message_header_codec::MessageHeaderDecoder::default()
                .wrap(ReadBuf::new(&ergo_buf[..ergo_len]), 0);
            let tool_decoder = ToolDecoder::default().header(tool_header, 0);
            if !single.is_nan() {
                assert_eq!(
                    tool_decoder.optional_float().map(f32::to_bits),
                    Some(single_bits)
                );
            }
            if !double.is_nan() {
                assert_eq!(
                    tool_decoder.optional_double().map(f64::to_bits),
                    Some(double_bits)
                );
            }
        }
        "#,
    );
}

#[test]
fn dimension_composites_are_byte_exact_for_u8_u16_u32_and_both_endians()
-> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (
            "u8",
            "uint8",
            3usize,
            0usize,
            vec![1, 0, 0, 4, 4, 3, 2, 1],
            vec![1, 0, 0, 4, 1, 2, 3, 4],
        ),
        (
            "u16",
            "uint16",
            6,
            0,
            vec![1, 0, 0, 0, 0, 0, 4, 0, 4, 3, 2, 1],
            vec![0, 1, 0, 0, 0, 0, 0, 4, 1, 2, 3, 4],
        ),
        (
            "u32",
            "uint32",
            8,
            0,
            vec![1, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 4, 3, 2, 1],
            vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 4, 1, 2, 3, 4],
        ),
    ];

    for (width, primitive, block_offset, count_offset, little, big) in cases {
        for (order, expected) in [("littleEndian", little), ("bigEndian", big)] {
            let xml = format!(
                r#"<?xml version="1.0"?>
                <messageSchema package="dims" id="901" version="0" byteOrder="{order}">
                  <types>
                    <composite name="messageHeader">
                      <type name="blockLength" primitiveType="uint16"/>
                      <type name="templateId" primitiveType="uint16"/>
                      <type name="schemaId" primitiveType="uint16"/>
                      <type name="version" primitiveType="uint16"/>
                    </composite>
                    <composite name="groupSizeEncoding">
                      <type name="numInGroup" primitiveType="{primitive}" offset="{count_offset}"/>
                      <type name="blockLength" primitiveType="{primitive}" offset="{block_offset}"/>
                    </composite>
                  </types>
                  <message name="Dims" id="1">
                    <group name="rows" id="2" dimensionType="groupSizeEncoding">
                      <field name="value" id="3" type="uint32"/>
                    </group>
                  </message>
                </messageSchema>"#
            );
            let module = format!("dims_{width}_{order}");
            let source = generate_xml(&xml, &module)?;
            let expected_literal = expected
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            compile_and_run(
                &module,
                &source,
                &format!(
                    r"
                    let expected = [{expected_literal}u8];
                    let frame_len =
                        DimsEncoder::try_compute_encoded_length_with_header(1)?;
                    let mut storage = vec![0u8; frame_len];
                    let len = DimsEncoder::try_wrap_and_apply_header(&mut storage, 0)?
                        .fixed(&DimsFixedFields {{}})
                        .rows(1, |rows| {{
                            rows.add(|row| {{
                                row.value(0x0102_0304);
                                Ok(())
                            }})?;
                            Ok(())
                        }})?
                        .encoded_length_with_header();
                    assert_eq!(len, frame_len);
                    assert_eq!(&storage[DimsEncoder::HEADER_LENGTH..len], &expected);
                    DimsDecoder::verify(&storage[..len])?;
                    "
                ),
            );
        }
    }
    Ok(())
}

fn offset_schema(byte_order: &str, custom_header: bool) -> String {
    let header = if custom_header {
        r#"
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint8"/>
          <type name="templateId" primitiveType="uint16" offset="3"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
          <type name="numGroups" primitiveType="uint16" offset="10"/>
          <type name="numVarDataFields" primitiveType="uint16"/>
        </composite>
        "#
    } else {
        r#"
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
        "#
    };
    format!(
        r#"<?xml version="1.0"?>
        <messageSchema package="offsets" id="902" version="0" byteOrder="{byte_order}">
          <types>
            {header}
            <composite name="groupSizeEncoding">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="numInGroup" primitiveType="uint16"/>
            </composite>
            <composite name="varDataEncoding">
              <type name="length" primitiveType="uint32" maxValue="4096"/>
              <type name="varData" primitiveType="uint8" length="0"/>
            </composite>
          </types>
          <message name="Probe" id="1">
            <field name="value" id="1" type="uint32"/>
            <group name="rows" id="2" dimensionType="groupSizeEncoding">
              <field name="quantity" id="3" type="uint64"/>
            </group>
            <data name="payload" id="4" type="varDataEncoding"/>
          </message>
        </messageSchema>"#
    )
}

#[test]
fn message_offsets_0_through_63_preserve_prefix_and_suffix_canaries()
-> Result<(), Box<dyn std::error::Error>> {
    for (label, order, custom) in [
        ("little", "littleEndian", false),
        ("big", "bigEndian", false),
        ("custom", "littleEndian", true),
    ] {
        let source = generate_xml(&offset_schema(order, custom), &format!("offset_{label}"))?;
        compile_and_run(
            &format!("offset_{label}"),
            &source,
            r#"
            const CANARY: u8 = 0xa5;
            let frame_len =
                ProbeEncoder::try_compute_encoded_length_with_header(2u16, 3)?;
            for offset in 0usize..=63 {
                let total = offset + frame_len + 64;
                let mut storage = vec![CANARY; total];
                let len = ProbeEncoder::try_wrap_and_apply_header(&mut storage, offset)?
                    .fixed(&ProbeFixedFields { value: 0x1020_3040 })
                    .rows(2, |rows| {
                        rows.add(|row| {
                            row.quantity(7);
                            Ok(())
                        })?;
                        rows.add(|row| {
                            row.quantity(9);
                            Ok(())
                        })?;
                        Ok(())
                    })?
                    .payload(b"xyz")?
                    .encoded_length_with_header();
                assert_eq!(len, frame_len);
                assert!(storage[..offset].iter().all(|&byte| byte == CANARY));
                assert!(
                    storage[offset + len..]
                        .iter()
                        .all(|&byte| byte == CANARY)
                );
                let decoder = ProbeDecoder::try_decode(&storage, offset)?;
                assert_eq!(decoder.value(), 0x1020_3040);
                ProbeDecoder::verify(&storage[offset..offset + len])?;
            }
            "#,
        );
    }
    Ok(())
}

fn hostile_schema() -> String {
    let dimension = r#"
      <composite name="groupSizeEncoding">
        <type name="blockLength" primitiveType="uint16"/>
        <type name="numInGroup" primitiveType="uint16"/>
      </composite>
    "#;
    format!(
        r#"<?xml version="1.0"?>
        <messageSchema package="hostile" id="903" version="0" byteOrder="littleEndian">
          {}
          <message name="Fixed" id="1">
            <field name="value" id="1" type="uint64"/>
          </message>
          <message name="Grouped" id="2">
            <field name="seq" id="1" type="uint32"/>
            <group name="rows" id="2" dimensionType="groupSizeEncoding">
              <field name="value" id="3" type="uint64"/>
            </group>
          </message>
          <message name="Nested" id="3">
            <field name="seq" id="1" type="uint32"/>
            <group name="outer" id="2" dimensionType="groupSizeEncoding">
              <field name="value" id="3" type="uint32"/>
              <group name="inner" id="4" dimensionType="groupSizeEncoding">
                <field name="value" id="5" type="uint64"/>
              </group>
            </group>
          </message>
          <message name="WithData" id="4">
            <field name="seq" id="1" type="uint32"/>
            <data name="payload" id="2" type="varDataEncoding"/>
          </message>
        </messageSchema>"#,
        standard_types(dimension)
    )
}

#[test]
fn every_truncation_boundary_is_rejected_for_fixed_group_nested_and_var_data()
-> Result<(), Box<dyn std::error::Error>> {
    let source = generate_xml(&hostile_schema(), "truncation_matrix")?;
    compile_and_run(
        "truncation_matrix",
        &source,
        r#"
        fn assert_all_cuts(
            label: &str,
            frame: &[u8],
            verify: fn(&[u8]) -> Result<(), sbe_rt::VerifyError>,
        ) {
            for cut in 0..frame.len() {
                assert!(
                    verify(&frame[..cut]).is_err(),
                    "{label} cut {cut} unexpectedly verified"
                );
            }
            verify(frame).expect("complete frame must verify");
        }

        let mut fixed = [0u8; FixedEncoder::ENCODED_LENGTH];
        let fixed_len = FixedEncoder::try_wrap_and_apply_header(&mut fixed, 0)?
            .fixed(&FixedFixedFields { value: 7 })
            .encoded_length_with_header();
        assert_all_cuts("fixed", &fixed[..fixed_len], FixedDecoder::verify);

        let grouped_expected =
            GroupedEncoder::try_compute_encoded_length_with_header(2u16)?;
        let mut grouped = vec![0u8; grouped_expected];
        let grouped_len = GroupedEncoder::try_wrap_and_apply_header(&mut grouped, 0)?
            .fixed(&GroupedFixedFields { seq: 1 })
            .rows(2, |rows| {
                rows.add(|row| {
                    row.value(10);
                    Ok(())
                })?;
                rows.add(|row| {
                    row.value(20);
                    Ok(())
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        assert_all_cuts("grouped", &grouped[..grouped_len], GroupedDecoder::verify);
        let mut rows = GroupedDecoder::try_from(&grouped[..grouped_len])?.into_rows()?;
        assert!(rows.skip_n(usize::MAX).is_err());

        let dimension_offset = GroupedEncoder::HEADER_LENGTH + GroupedEncoder::BLOCK_LENGTH;
        let mut zero_stride = grouped.clone();
        zero_stride[dimension_offset..dimension_offset + 2]
            .copy_from_slice(&0u16.to_le_bytes());
        zero_stride[dimension_offset + 2..dimension_offset + 4]
            .copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(GroupedDecoder::verify(&zero_stride).is_err());

        let nested_expected = NestedEncodedLength::new()
            .outer(1u16)
            .inner(1u16)?
            .encoded_length_with_header();
        let mut nested = vec![0u8; nested_expected];
        let nested_len = NestedEncoder::try_wrap_and_apply_header(&mut nested, 0)?
            .fixed(&NestedFixedFields { seq: 2 })
            .outer(1, |outer| {
                outer.add(|mut entry| {
                    entry.value(30);
                    // The entry completes with its nested group, so return that
                    // proof rather than `Ok(())`.
                    entry.inner(1, |inner| {
                        inner.add(|row| {
                            row.value(40);
                            Ok(())
                        })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        assert_all_cuts("nested", &nested[..nested_len], NestedDecoder::verify);

        let data_expected =
            WithDataEncoder::try_compute_encoded_length_with_header(5)?;
        let mut data = vec![0u8; data_expected];
        let data_len = WithDataEncoder::try_wrap_and_apply_header(&mut data, 0)?
            .fixed(&WithDataFixedFields { seq: 3 })
            .payload(b"hello")?
            .encoded_length_with_header();
        assert_all_cuts("var-data", &data[..data_len], WithDataDecoder::verify);
        assert!(WithDataEncoder::try_compute_encoded_length_with_header(4097).is_err());
        let max_len = WithDataEncoder::try_compute_encoded_length_with_header(4096)?;
        let mut oversized_storage = vec![0u8; max_len];
        let oversized = WithDataEncoder::try_wrap_and_apply_header(&mut oversized_storage, 0)?
            .fixed(&WithDataFixedFields { seq: 4 })
            .payload(&[0u8; 4097]);
        assert!(matches!(
            oversized,
            Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "payload",
                max_length: 4096,
                actual: 4097,
            })
        ));
        "#,
    );
    Ok(())
}

#[test]
fn name_collisions_compile_execute_and_preserve_wire_bytes() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/encoded-length-method-collision.xml");
    let (_schema, source) = generate(&path, "collision_runtime");
    compile_and_run(
        "collision_runtime",
        &source,
        r#"
        let expected = CollisionMsgEncodedLength::new()
            .outer(0u16)
            .finish_empty()?
            .items(1u16)?
            .payload(3)?
            .encoded_length_with_header();
        let mut storage = vec![0u8; expected];
        let len = CollisionMsgEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CollisionMsgFixedFields { id: 9 })
            .outer(0, |_| Ok(()))?
            .items(1, |items| {
                items.add(|item| {
                    item.key(17);
                    Ok(())
                })?;
                Ok(())
            })?
            .payload(b"abc")?
            .encoded_length_with_header();
        assert_eq!(len, expected);
        CollisionMsgDecoder::verify(&storage[..len])?;
        assert_eq!(
            &storage[CollisionMsgEncoder::HEADER_LENGTH
                ..CollisionMsgEncoder::HEADER_LENGTH + 8],
            &9u64.to_le_bytes()
        );
        "#,
    );
}

#[test]
fn explicit_implicit_offsets_and_derived_block_lengths_are_byte_exact() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/composite-offsets-schema.xml");
    let (_schema, source) = generate(&path, "offset_runtime");
    compile_and_run(
        "offset_runtime",
        &source,
        r"
        assert_eq!(TestMessage2Encoder::HEADER_LENGTH, 12);
        assert_eq!(TestMessage2Encoder::BLOCK_LENGTH, 32);
        let mut storage = [0u8; TestMessage2Encoder::ENCODED_LENGTH];
        let len = TestMessage2Encoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&TestMessage2FixedFields {
                field_one: 0x0102_0304,
                field_two: TestComposite::new(0x7f, 0x0102_0304_0506_0708),
                field_three: 0x1112_1314_1516_1718,
            })
            .encoded_length_with_header();
        assert_eq!(len, TestMessage2Encoder::ENCODED_LENGTH);
        let body = &storage[TestMessage2Encoder::HEADER_LENGTH..len];
        assert_eq!(&body[0..4], &0x0102_0304i32.to_le_bytes());
        assert_eq!(&body[4..8], &[0; 4]);
        assert_eq!(body[8], 0x7f);
        assert_eq!(&body[9..16], &[0; 7]);
        assert_eq!(
            &body[16..24],
            &0x0102_0304_0506_0708i64.to_le_bytes()
        );
        assert_eq!(
            &body[24..32],
            &0x1112_1314_1516_1718i64.to_le_bytes()
        );
        let decoder = TestMessage2Decoder::try_from(&storage[..len])?;
        assert_eq!(decoder.field_one(), 0x0102_0304);
        assert_eq!(decoder.field_three(), 0x1112_1314_1516_1718);
        ",
    );
}
