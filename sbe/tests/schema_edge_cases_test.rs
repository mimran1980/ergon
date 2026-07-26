//! Structural tests for upstream schema edge cases (Items 1 & 2 of).
//!
//! For each upstream test schema we:
//! 1. Parse the XML via `generate()`
//! 2. Verify the generated Rust is syntactically valid (`assert_source_ok`)
//! 3. Check that expected types (decoders, encoders, enums, composites) exist
//!
//! Schemas that require Java-generated binary fixtures document the requirement.
//! Full round-trip tests are deferred until codegen bugs (tracked in `patch_source`)
//! are fixed.  For now, structural verification ensures the parser + codegen
//! produce valid Rust from each edge-case input.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, assert_source_ok, compile_and_run, generate};

fn assert_tool_schema(name: &str, filename: &str, expected: &[&str]) {
    let (_schema, src) = generate(&Paths::sbe_tool_test_resource(filename), name);
    assert_source_ok(&src, expected);
}

/// Schema extension (version 2): Car with `uuid`, `cupHolderCount`, `mpg (sinceVersion=2)`.
#[test]
fn extension_schema_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "ext",
        "example-extension-schema.xml",
        &[
            "CarDecoder",
            "CarEncoder",
            "Engine",
            "Booster",
            "OptionalExtras",
            "Model",
            "BooleanType",
            "FuelFiguresDecoder",
            "FuelFiguresEntryDecoder",
            "PerformanceFiguresDecoder",
            "PerformanceFiguresEntryDecoder",
            "AccelerationDecoder",
            "AccelerationEntryDecoder",
            "GroupSizeEncoding",
            "VarStringEncoding",
            "VarAsciiEncoding",
            "VarDataEncoding",
        ],
    );

    Ok(())
}

/// Null semantics: optional enum fields, optional enum encoding, optional composite.
#[test]
fn optional_enum_nullify_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "oen",
        "optional_enum_nullify.xml",
        &[
            "OptionalEnumNullifyDecoder",
            "OptionalEnumNullifyEncoder",
            "EnumType",
            "OptionalEncodingEnumType",
            "OptionalComposite",
        ],
    );

    Ok(())
}

/// Since-version filtering: types, fields, groups, composites, enums, sets with sinceVersion.
/// Note: plain <type name="TypeSince0" .../> definitions don't generate
/// standalone structs (they are inlined).  Only composites, enums, and sets
/// produce named types.
#[test]
fn since_version_filter_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "since_dep",
        "since-version-filter-schema.xml",
        &[
            "MessageSince0Decoder",
            "MessageSince0Encoder",
            "MessageSince4Decoder",
            "MessageSince4Encoder",
            "MessageSince5Decoder",
            "MessageSince5Encoder",
            "MessageWithSinceDecoder",
            "MessageWithSinceEncoder",
            "CompositeSince0",
            "CompositeSince4",
            "CompositeSince5",
            "CompositeWithSinceFields",
            "EnumSince0",
            "EnumSince4",
            "EnumSince5",
            "EnumWithSinceValues",
            "SetSince0",
            "SetSince4",
            "SetSince5",
            "SetWithSinceChoices",
            "GroupSizeEncoding",
            "VarStringEncoding",
        ],
    );

    Ok(())
}

/// Issue 895: optional float/double with NaN-as-null semantics.
#[test]
fn issue_895_optional_float_double_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "issue895",
        "issue895.xml",
        &["Issue895Decoder", "Issue895Encoder"],
    );

    Ok(())
}

/// Issue 972: optional composite with versioned fields.
#[test]
fn issue_972_optional_composite_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "issue972",
        "issue972.xml",
        &["Issue972Decoder", "Issue972Encoder", "NewComposite"],
    );

    Ok(())
}

/// Composite elements: enum, set, and nested composite inside a composite.
/// Note: inline enum/set/composite types (`EnumOne`, `SetOne`, Inner) are
/// embedded in the Outer composite and don't generate standalone structs.
#[test]
fn composite_elements_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "comp_el",
        "composite-elements-schema.xml",
        &["Outer(pub", "MsgDecoder", "MsgEncoder"],
    );

    Ok(())
}

/// Explicit field offsets in composites and messages.
#[test]
fn composite_offsets_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "comp_off",
        "composite-offsets-schema.xml",
        &[
            "TestMessage1Decoder",
            "TestMessage1Encoder",
            "TestMessage2Decoder",
            "TestMessage2Encoder",
            "TestComposite",
        ],
    );

    Ok(())
}

/// Basic repeating group with custom dimensionType.
#[test]
fn basic_group_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "basic_grp",
        "basic-group-schema.xml",
        &[
            "TestMessage1Decoder",
            "TestMessage1Encoder",
            "GroupSizeEncoding",
        ],
    );

    Ok(())
}

/// Triply-nested repeating groups.
#[test]
fn nested_group_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "nested_grp",
        "nested-group-schema.xml",
        &["TopDecoder", "TopEncoder", "GroupSizeEncoding"],
    );

    Ok(())
}

/// Groups that contain var-data fields (single, multiple, nested).
#[test]
fn group_with_data_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "grp_data",
        "group-with-data-schema.xml",
        &[
            "TestMessage1Decoder",
            "TestMessage1Encoder",
            "TestMessage2Decoder",
            "TestMessage2Encoder",
            "TestMessage3Decoder",
            "TestMessage3Encoder",
            "TestMessage4Decoder",
            "TestMessage4Encoder",
            "VarDataEncoding",
            "GroupSizeEncoding",
        ],
    );

    Ok(())
}

/// Message with embedded length/count (group dimension inside composite) and var data.
#[test]
fn embedded_length_and_count_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "emb_len",
        "embedded-length-and-count-schema.xml",
        &[
            "Message1Decoder",
            "Message1Encoder",
            "Message2Decoder",
            "Message2Encoder",
            "GroupSizeEncoding",
            "VarDataEncoding",
        ],
    );

    Ok(())
}

/// Group with constant fields inside and outside messages.
/// Note: `PrimitiveConst` and `StrConst` are simple type aliases;
/// they don't generate standalone structs.
#[test]
fn group_with_constant_fields_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "const_flds",
        "group-with-constant-fields.xml",
        &[
            "ConstantsGaloreDecoder",
            "ConstantsGaloreEncoder",
            "CompositeWithConst",
            "GroupSizeEncoding",
        ],
    );

    Ok(())
}

/// Value-ref schemas: constant enum valueRef and constant type valueRef.
#[test]
fn value_ref_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "val_ref",
        "value-ref-schema.xml",
        &[
            "MsgOneDecoder",
            "MsgOneEncoder",
            "MsgTwoDecoder",
            "MsgTwoEncoder",
            "MsgThreeDecoder",
            "MsgThreeEncoder",
            "MsgFourDecoder",
            "MsgFourEncoder",
            "MsgFiveDecoder",
            "MsgFiveEncoder",
            "TimeUnit",
            "UTCTimestampNanos",
        ],
    );

    Ok(())
}

/// Basic minimal schema (single uint32 field, no groups/var-data).
#[test]
fn basic_schema_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "basic",
        "basic-schema.xml",
        &["TestMessage50001Decoder", "TestMessage50001Encoder"],
    );

    Ok(())
}

/// Types schema with various primitive types.
#[test]
fn basic_types_schema_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "basic_types",
        "basic-types-schema.xml",
        &[
            "Message1Decoder",
            "Message1Encoder",
            "Message1WithOffsetsDecoder",
            "Message1WithOffsetsEncoder",
        ],
    );

    Ok(())
}

/// Block-length test schema with explicit blockLength on messages.
#[test]
fn block_length_schema_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "bl_len",
        "block-length-schema.xml",
        &[
            "Message1Decoder",
            "Message1Encoder",
            "Message2Decoder",
            "Message2Encoder",
            "Message3Decoder",
            "Message3Encoder",
            "Message4Decoder",
            "Message4Encoder",
        ],
    );

    Ok(())
}

/// UTF-16 var-data encoding: verify schema parses and generates valid Rust.
/// The generated code currently hardcodes `core::str::from_utf8` for var-data
/// accessors, so UTF-16 decoding will fail at runtime. This test asserts the
/// structural gap: valid schema, valid Rust output, no UTF-16-specific method.
#[test]
fn utf16_encoding_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "utf16",
        "utf16-test-schema.xml",
        &[
            "Utf16MessageDecoder",
            "Utf16MessageEncoder",
            "VarDataEncoding",
            "AsciiDataEncoding",
        ],
    );

    Ok(())
}

/// Unit attribute: verify schema with `unit` on types parses successfully.
/// The `unit` attribute is currently NOT stored in the IR or emitted into
/// generated `field_meta`. This test asserts the schema parses and produces
/// valid Rust despite the gap. The generated code should include
/// `FIELD_MIN` / `FIELD_MAX` constants for types with `minValue`/`maxValue`.
#[test]
fn unit_attribute_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "unit_attr",
        "unit-attribute-test-schema.xml",
        &[
            "TradeMessageDecoder",
            "TradeMessageEncoder",
            "PRICE_MIN",
            "PRICE_MAX",
            "QUANTITY_MIN",
            "QUANTITY_MAX",
            "PERCENTAGE_MIN",
            "PERCENTAGE_MAX",
        ],
    );

    Ok(())
}

/// Decode older group-entry layouts using the acting block length carried in
/// the wire dimension header, rather than the latest compiled entry size.
#[test]
fn group_extension_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(
        &Paths::sbe_tool_test_resource("group-extension-test-schema.xml"),
        "grp_ext",
    );
    assert_source_ok(
        &src,
        &[
            "GroupExtensionMessageDecoder",
            "GroupExtensionMessageEncoder",
            "EntriesDecoder",
            "EntriesEntryDecoder",
            "ENTRY_BLOCK_LENGTH",
        ],
    );
    compile_and_run(
        "grp_ext",
        &src,
        r#"
        fn append_u16(buf: &mut Vec<u8>, value: u16) {
            buf.extend_from_slice(&value.to_le_bytes());
        }
        fn append_u32(buf: &mut Vec<u8>, value: u32) {
            buf.extend_from_slice(&value.to_le_bytes());
        }
        fn append_u64(buf: &mut Vec<u8>, value: u64) {
            buf.extend_from_slice(&value.to_le_bytes());
        }

        // Version 0: group entries contain only price + volume (12 bytes).
        let mut v0 = Vec::new();
        append_u16(&mut v0, 4);   // root blockLength
        append_u16(&mut v0, 1);   // templateId
        append_u16(&mut v0, 303); // schemaId
        append_u16(&mut v0, 0);   // acting version
        append_u32(&mut v0, 77);  // seqNum
        append_u16(&mut v0, 12);  // acting entry blockLength
        append_u16(&mut v0, 2);   // count
        append_u64(&mut v0, 101);
        append_u32(&mut v0, 11);
        append_u64(&mut v0, 202);
        append_u32(&mut v0, 22);

        let decoded = GroupExtensionMessageDecoder::try_from(v0.as_slice())?;
        assert_eq!(decoded.seq_num(), 77);
        let rows: Vec<_> = decoded.into_entries()?.collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].price(), 101);
        assert_eq!(rows[0].volume(), 11);
        assert_eq!(rows[0].counterparty_id(), None);
        assert_eq!(rows[0].flags(), None);
        assert_eq!(rows[1].price(), 202);
        assert_eq!(rows[1].volume(), 22);

        // Version 1: counterpartyId is present; flags is not. Two entries
        // prove that iteration advances by wire blockLength=20, not latest=21.
        let mut v1 = Vec::new();
        append_u16(&mut v1, 4);
        append_u16(&mut v1, 1);
        append_u16(&mut v1, 303);
        append_u16(&mut v1, 1);
        append_u32(&mut v1, 88);
        append_u16(&mut v1, 20);
        append_u16(&mut v1, 2);
        append_u64(&mut v1, 303);
        append_u32(&mut v1, 33);
        append_u64(&mut v1, 3003);
        append_u64(&mut v1, 404);
        append_u32(&mut v1, 44);
        append_u64(&mut v1, 4004);

        let decoded = GroupExtensionMessageDecoder::try_from(v1.as_slice())?;
        let rows: Vec<_> = decoded.into_entries()?.collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].price(), 303);
        assert_eq!(rows[0].volume(), 33);
        assert_eq!(rows[0].counterparty_id(), Some(3003));
        assert_eq!(rows[0].flags(), None);
        assert_eq!(rows[1].price(), 404);
        assert_eq!(rows[1].volume(), 44);
        assert_eq!(rows[1].counterparty_id(), Some(4004));
        assert_eq!(rows[1].flags(), None);
        "#,
    );

    Ok(())
}

#[test]
fn versioned_group_non_scalar_fields_do_not_read_past_older_entry_blocks()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(
        &Paths::sbe_tool_test_resource("group-versioned-types-schema.xml"),
        "grp_versioned_types",
    );
    compile_and_run(
        "grp_versioned_types",
        &src,
        r#"
        // Version-0 frame decoded by the version-1 schema. The entry block
        // contains only `base`; all sinceVersion=1 members are absent.
        let wire = [
            0, 0,       // root blockLength
            1, 0,       // templateId
            48, 1,      // schemaId 304
            0, 0,       // acting version
            1, 0,       // entry blockLength
            2, 0,       // count
            7, 9,       // two version-0 entries
        ];
        let decoded = VersionedGroupMessageDecoder::try_from(wire.as_slice())?;
        let rows: Vec<_> = decoded.into_entries()?.collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].base(), 7);
        assert_eq!(rows[1].base(), 9);
        for row in rows {
            assert_eq!(row.later_array(), [0, 0]);
            assert!(row.later_composite().is_none());
            assert_eq!(row.later_composite_value(), None);
            assert_eq!(row.raw_later_composite(), None);
            assert_eq!(row.later_enum(), None);
            assert_eq!(row.raw_later_enum(), None);
            assert_eq!(row.later_set(), None);
            assert_eq!(row.raw_later_set(), None);
        }

        // Latest-version add_struct covers multi-byte primitive arrays plus
        // composite/enum/set fields in a flat group entry.
        let mut latest = [0u8; 64];
        let enc = VersionedGroupMessageEncoder::wrap_and_apply_header(&mut latest, 0);
        let enc = enc.entries(1, |group| {
            group.add_struct(&EntriesEntry {
                base: 5,
                later_array: [0x1122, 0x3344],
                later_composite: LaterComposite::new(0x5566),
                later_enum: LaterEnum::A,
                later_set: LaterSet(1),
            })
        })?;
        let decoded = VersionedGroupMessageDecoder::try_from(enc.as_bytes())?;
        let row = decoded.into_entries()?.next().unwrap();
        assert_eq!(row.base(), 5);
        assert_eq!(row.later_array(), [0x1122, 0x3344]);
        assert_eq!(row.later_composite_value(), Some(LaterComposite::new(0x5566)));
        assert_eq!(row.later_enum(), Some(LaterEnum::A));
        assert_eq!(row.later_set(), Some(LaterSet(1)));
        "#,
    );

    Ok(())
}

/// Constant enum valueRef fields: top-level and group entry constants.
#[test]
fn constant_enum_fields_types_exist() -> Result<(), Box<dyn std::error::Error>> {
    assert_tool_schema(
        "const_enum",
        "constant-enum-fields.xml",
        &[
            "ConstantEnumsDecoder",
            "ConstantEnumsEncoder",
            "Model",
            "GroupSizeEncoding",
        ],
    );

    Ok(())
}
