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

/// Flat group slices use one checked destination region and preserve entry order.
#[test]
fn basic_group_bulk_add_encodes_and_checks_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(
        &Paths::sbe_tool_test_resource("basic-group-schema.xml"),
        "basic_group_bulk",
    );
    compile_and_run(
        "basic_group_bulk",
        &src,
        r#"
        let first = EntriesEntry {
            tag_group1: {
                let mut symbol = [0u8; 20];
                symbol[..3].copy_from_slice(b"ABC");
                symbol
            },
            tag_group2: 101,
        };
        let second = EntriesEntry {
            tag_group1: {
                let mut symbol = [0u8; 20];
                symbol[..3].copy_from_slice(b"XYZ");
                symbol
            },
            tag_group2: -202,
        };
        let entries = [first.clone(), second.clone()];

        let mut buf = [0u8; 128];
        let len = TestMessage1Encoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
            .entries(2, |group| group.bulk_add(&entries))?
            .encoded_length_with_header();
        let mut decoded = TestMessage1Decoder::try_from(&buf[..len])?.into_entries()?;
        let row = decoded.next().unwrap();
        assert_eq!(row.tag_group1(), first.tag_group1);
        assert_eq!(row.tag_group2(), first.tag_group2);
        let row = decoded.next().unwrap();
        assert_eq!(row.tag_group1(), second.tag_group1);
        assert_eq!(row.tag_group2(), second.tag_group2);
        assert!(decoded.next().is_none());

        let mut full_buf = [0u8; 128];
        let len = TestMessage1Encoder::try_wrap_and_apply_header(&mut full_buf, 0).unwrap()
            .entries(1, |group| {
                let err = group.bulk_add(&entries).unwrap_err();
                assert!(matches!(
                    err,
                    sbe_rt::EncodeError::GroupFull {
                        declared: 1,
                        attempted: 2,
                    }
                ));
                group.bulk_add(&entries[..1])
            })?
            .encoded_length_with_header();
        assert_eq!(
            TestMessage1Decoder::try_from(&full_buf[..len])?
                .into_entries()?
                .count(),
            1
        );

        let mut short_buf = [0u8; 54];
        let err = TestMessage1Encoder::try_wrap_and_apply_header(&mut short_buf, 0).unwrap()
            .entries(1, |group| group.bulk_add(&entries[..1]))
            .unwrap_err();
        assert!(matches!(
            err,
            sbe_rt::EncodeError::BufferTooShort {
                needed: 28,
                available: 27,
                ..
            }
        ));

        let mut empty_buf = [0u8; 32];
        let len = TestMessage1Encoder::try_wrap_and_apply_header(&mut empty_buf, 0).unwrap()
            .entries(0, |group| group.bulk_add(&[]))?
            .encoded_length_with_header();
        assert_eq!(len, 27);
        "#,
    );

    Ok(())
}

/// Zero-width fixed entries carry count but no entry bytes; bulk encode must not panic.
#[test]
fn zero_block_group_bulk_add_records_count_without_chunks_panic()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(
        &Paths::sbe_tool_test_resource("zero-block-group-schema.xml"),
        "zero_block_group_bulk",
    );
    compile_and_run(
        "zero_block_group_bulk",
        &src,
        r#"
        let entries = [EntriesEntry {}, EntriesEntry {}, EntriesEntry {}];
        let mut buf = [0u8; 12];
        let len = ZeroBlockMessageEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap()
            .entries(3, |group| group.bulk_add(&entries))?
            .encoded_length_with_header();
        assert_eq!(len, 12);

        let decoded = ZeroBlockMessageDecoder::try_from(&buf[..len])?.into_entries()?;
        assert_eq!(decoded.count(), 3);
        "#,
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
            assert_eq!(row.later_bool(), None);
            assert_eq!(row.raw_later_bool(), None);
            assert_eq!(row.later_bool_bool(), None);
        }

        // Acting version 1 with a seven-byte entry: the array and composite
        // fit, but the enum at offset 7 does not. Keep a second entry so an
        // off-by-one extent check would read its base byte as the first
        // entry's enum instead of running off the supplied slice.
        let enum_short = [
            0, 0,       // root blockLength
            1, 0,       // templateId
            48, 1,      // schemaId 304
            1, 0,       // acting version
            7, 0,       // entry blockLength
            2, 0,       // count
            7,          // first base
            0x22, 0x11, 0x44, 0x33,
            0x66, 0x55,
            1,          // second base: valid LaterEnum::A if read incorrectly
            0, 0, 0, 0,
            0, 0,
        ];
        let decoded = VersionedGroupMessageDecoder::try_from(enum_short.as_slice())?;
        let first = decoded.into_entries()?.next().unwrap();
        assert_eq!(first.later_array(), [0x1122, 0x3344]);
        assert_eq!(first.later_composite_value(), Some(LaterComposite::new(0x5566)));
        assert_eq!(first.later_enum(), None);
        assert_eq!(first.raw_later_enum(), None);

        // One more byte makes the enum available, but not the set at offset
        // 8. Again, the next entry begins with a byte that would look like a
        // valid set if the complete-field extent check regressed.
        let set_short = [
            0, 0,       // root blockLength
            1, 0,       // templateId
            48, 1,      // schemaId 304
            1, 0,       // acting version
            8, 0,       // entry blockLength
            2, 0,       // count
            7,          // first base
            0x22, 0x11, 0x44, 0x33,
            0x66, 0x55,
            1,          // first enum
            1,          // second base: LaterSet bit 0 if read incorrectly
            0, 0, 0, 0,
            0, 0,
            1,
        ];
        let decoded = VersionedGroupMessageDecoder::try_from(set_short.as_slice())?;
        let first = decoded.into_entries()?.next().unwrap();
        assert_eq!(first.later_enum(), Some(LaterEnum::A));
        assert_eq!(first.later_set(), None);
        assert_eq!(first.raw_later_set(), None);

        // Nine bytes include the set but not the versioned BooleanType at
        // offset 9. Its convenience `_bool` accessor must preserve absence
        // instead of reading the next entry or collapsing `None` to false.
        let bool_short = [
            0, 0,       // root blockLength
            1, 0,       // templateId
            48, 1,      // schemaId 304
            1, 0,       // acting version
            9, 0,       // entry blockLength
            2, 0,       // count
            7,          // first base
            0x22, 0x11, 0x44, 0x33,
            0x66, 0x55,
            1,          // first enum
            1,          // first set
            1,          // second base: true if read incorrectly as laterBool
            0, 0, 0, 0,
            0, 0,
            1,
            1,
        ];
        let decoded = VersionedGroupMessageDecoder::try_from(bool_short.as_slice())?;
        let first = decoded.into_entries()?.next().unwrap();
        assert_eq!(first.later_set(), Some(LaterSet(1)));
        assert_eq!(first.later_bool(), None);
        assert_eq!(first.raw_later_bool(), None);
        assert_eq!(first.later_bool_bool(), None);

        // Ten bytes include the versioned bool but not the optional uint32 at
        // offset 10. Its extent check uses `offset + prim_size` — a `+` → `-`
        // mutation would compute `10 - 4 = 6 > 10` (false) and return a garbage
        // value read from the second entry.
        let opt_short = [
            0, 0,       // root blockLength
            1, 0,       // templateId
            48, 1,      // schemaId 304
            1, 0,       // acting version
            10, 0,      // entry blockLength
            2, 0,       // count
            7,          // first base
            0x22, 0x11, 0x44, 0x33,
            0x66, 0x55,
            1,          // first enum
            1,          // first set
            1,          // first bool (BooleanType::T)
            0xDD, 0xCC, 0xBB, 0xAA, // second entry data: would be read if check regresses
            9,          // second base
            0, 0, 0, 0,
            0, 0,
            0,
            0,
            0,
        ];
        let decoded = VersionedGroupMessageDecoder::try_from(opt_short.as_slice())?;
        let first = decoded.into_entries()?.next().unwrap();
        assert_eq!(first.later_bool(), Some(BooleanType::T));
        assert_eq!(first.later_bool_bool(), Some(true));
        assert!(first.later_value().is_none(), "laterValue at offset 10 does not fit in blockLength 10");

        // Latest-version add_struct covers multi-byte primitive arrays plus
        // composite/enum/set fields in a flat group entry.
        let mut latest = [0u8; 64];
        let len = VersionedGroupMessageEncoder::try_wrap_and_apply_header(&mut latest, 0).unwrap()
            .entries(1, |group| {
                group.add_struct(&EntriesEntry {
                    base: 5,
                    later_array: [0x1122, 0x3344],
                    later_composite: LaterComposite::new(0x5566),
                    later_enum: LaterEnum::A,
                    later_set: LaterSet(1),
                    later_bool: BooleanType::T,
                    later_value: 42u32,
                })
            })?
            .encoded_length_with_header();
        let decoded = VersionedGroupMessageDecoder::try_from(&latest[..len])?;
        let row = decoded.into_entries()?.next().unwrap();
        assert_eq!(row.base(), 5);
        assert_eq!(row.later_array(), [0x1122, 0x3344]);
        assert_eq!(row.later_composite_value(), Some(LaterComposite::new(0x5566)));
        assert_eq!(row.later_enum(), Some(LaterEnum::A));
        assert_eq!(row.later_set(), Some(LaterSet(1)));
        assert_eq!(row.later_bool(), Some(BooleanType::T));
        assert_eq!(row.later_bool_bool(), Some(true));
        assert_eq!(row.later_value(), Some(42u32));
        "#,
    );

    Ok(())
}

#[test]
fn group_primitive_array_respects_the_wire_entry_block_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(
        &Paths::sbe_tool_test_resource("group-array-boundary-schema.xml"),
        "group_array_boundary",
    );
    compile_and_run(
        "group_array_boundary",
        &src,
        r#"
        // The generated array starts at offset 1 and needs eight bytes. A
        // wire blockLength of 8 is therefore one byte short. Keep a second
        // entry present so an incorrect check could read across the boundary
        // without reaching the end of the supplied slice.
        let short = [
            0, 0,       // root blockLength
            1, 0,       // templateId
            49, 1,      // schemaId 305
            0, 0,       // acting version
            8, 0,       // entry blockLength (correct minimum is 9)
            2, 0,       // count
            7, 1, 2, 3, 4, 5, 6, 7,
            9, 8, 7, 6, 5, 4, 3, 2,
        ];
        let decoded = ArrayBoundaryMessageDecoder::try_from(short.as_slice())?;
        let first = decoded.into_entries()?.next().unwrap();
        assert_eq!(first.base(), 7);
        assert_eq!(
            first.values(),
            [0, 0],
            "an array that exceeds the acting entry block must be absent"
        );

        let complete = [
            0, 0,       // root blockLength
            1, 0,       // templateId
            49, 1,      // schemaId 305
            0, 0,       // acting version
            9, 0,       // complete entry blockLength
            1, 0,       // count
            7,
            0x44, 0x33, 0x22, 0x11,
            0x88, 0x77, 0x66, 0x55,
        ];
        let decoded = ArrayBoundaryMessageDecoder::try_from(complete.as_slice())?;
        let row = decoded.into_entries()?.next().unwrap();
        assert_eq!(row.base(), 7);
        assert_eq!(row.values(), [0x1122_3344, 0x5566_7788]);
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

/// A group with 2+ nested groups and 2+ var-data fields exercises the
/// `ng_idx` and `nvd_idx` counters in `generate_group_decoder`. Mutations
/// that break the counter (e.g. `+=` → `*=`) produce duplicate tail-offset
/// function names → compile error in the generated code.
#[test]
fn multi_nested_group_compiles_and_roundtrips() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::multi_nested_group_schema(), "multi_nested");
    compile_and_run(
        "multi_nested",
        &src,
        r#"
        let mut buf = [0u8; 256];
        // Non-zero nested entries so tail_offset indices diverge: if the
        // `ng_idx` counter regresses (e.g. `*=` stays at 0), both children
        // call `tail_offset_0` and the second group reads the wrong dimension.
        let mut enc = MultiNestedEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap();
        enc.header(0u32);
        let enc = enc.parent(1, |parent| {
            parent.add(|entry| {
                entry.parent_field(42u16);
                entry.kind(EntryKind::A);
                entry.child_a(1, |a| { a.add(|e| { e.value_a(0xA000_0001u32); Ok(()) })?; Ok(()) })?;
                entry.child_b(2, |b| {
                    b.add(|e| { e.value_b(0xB000_0000_0000_0001u64); Ok(()) })?;
                    b.add(|e| { e.value_b(0xB000_0000_0000_0002u64); Ok(()) })?;
                    Ok(())
                })?;
                entry.note1(b"hello")?;
                entry.note2(b"world")?;
                Ok(())
            })?;
            Ok(())
        }).unwrap();
        let len = enc.encoded_length_with_header();

        let dec = MultiNestedDecoder::try_from(&buf[..len])?;
        // Field name `header` is not reserved (no inherent header() method) —
        // natural accessor keeps the schema name.
        assert_eq!(dec.header(), 0u32);

        let mut entries = dec.into_parent()?;
        let mut count = 0usize;
        while let Some(Ok(entry)) = entries.next() {
            assert_eq!(entry.parent_field(), 42u16);
            assert_eq!(entry.kind(), EntryKind::A);

            let child_a: Vec<_> = entry.child_a()?.collect();
            assert_eq!(child_a.len(), 1);
            assert_eq!(child_a[0].value_a(), 0xA000_0001u32);

            let child_b: Vec<_> = entry.child_b()?.collect();
            assert_eq!(child_b.len(), 2);
            assert_eq!(child_b[0].value_b(), 0xB000_0000_0000_0001u64);
            assert_eq!(child_b[1].value_b(), 0xB000_0000_0000_0002u64);

            assert_eq!(entry.note1()?, b"hello");
            assert_eq!(entry.note2()?, b"world");

            // Display exercises the entry_display_out_idx counters and the
            // primitive-field skip logic (constant / array).
            let s = entry.to_string();
            assert!(s.contains("parentField"), "Display missing parentField: {s}");
            assert!(s.contains("kind"), "Display missing kind: {s}");
            // The separator ", " confirms entry_display_out_idx advanced
            // beyond the first field (mutations like `*=` leave it at zero).
            assert!(s.contains(", "), "Display missing field separator: {s}");
            // A constant field (padding) is excluded from Display;
            // the `||` → `&&` mutation would include it.
            assert!(!s.contains("padding"), "Display should exclude constant field: {s}");

            // Call the constant accessor — a `||` mutation changes the return
            // type from u16 to &str and would not compile against this assert.
            assert_eq!(entry.padding(), 42u16);
            assert_eq!(entry.delim(), b',');

            count += 1;
        }
        assert_eq!(count, 1);
        "#,
    );

    Ok(())
}

/// Group entry decoders implement `Display`. Mutations that change the
/// field-skipping logic (e.g. `||` → `&&`) or the separator counter
/// (e.g. `+=` → `*=`) survive only because no test formats a group entry.
#[test]
fn group_entry_display_includes_fields() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "group_display");
    compile_and_run(
        "group_display",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let mut car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2020);
        car.available(BooleanType::T); car.code(Model::A);
        car.some_numbers([0u32; 4]); car.vehicle_code([0u8; 6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(2, |ff| {
            ff.add(|e| { e.speed(100).mpg(35.5f32).usage_description(b"city")?; Ok(()) })?;
            ff.add(|e| { e.speed(200).mpg(25.0f32).usage_description(b"hwy")?; Ok(()) })?;
            Ok(())
        }).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Ford").unwrap();
        let car = car.model(b"Mustang").unwrap();
        let car = car.activation_code(b"ABC").unwrap();
        let len = car.encoded_length_with_header();

        let dec = CarDecoder::try_from(&buf[..len])?;
        let mut fuel = dec.into_fuel_figures()?;
        let mut i = 0usize;
        while let Some(Ok(entry)) = fuel.next() {
            let s = entry.to_string();
            assert!(s.contains("speed"), "entry {} Display missing 'speed': {s}", i);
            assert!(s.contains("mpg"), "entry {} Display missing 'mpg': {s}", i);
            // The separator between fields is ", " when the output-index
            // counter advances correctly; mutations like `*=` keep the
            // counter at 0 which makes every separator the empty string.
            assert!(s.contains(", "), "entry {} Display missing field separator ', ': {s}", i);
            i += 1;
        }
        assert_eq!(i, 2);
        "#,
    );

    Ok(())
}
