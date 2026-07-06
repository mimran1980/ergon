//! Structural tests for upstream schema edge cases (Items 1 & 2 of todo 00).
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

mod common;
use common::{Paths, assert_source_ok, generate};

// ── Helpers ──────────────────────────────────────────────────────────────

/// Generate code from an sbe-tool test resource and verify structural validity.
fn assert_tool_schema(name: &str, filename: &str, expected: &[&str]) {
    let (_schema, src) = generate(&Paths::sbe_tool_test_resource(filename), name);
    assert_source_ok(&src, expected);
}

// ── Item 1: Port key Java test cases ─────────────────────────────────────

/// Schema extension (version 2): Car with `uuid`, `cupHolderCount`, `mpg (sinceVersion=2)`.
#[test]
fn extension_schema_types_exist() {
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
}

/// Null semantics: optional enum fields, optional enum encoding, optional composite.
#[test]
fn optional_enum_nullify_types_exist() {
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
}

/// Since-version filtering: types, fields, groups, composites, enums, sets with sinceVersion.
/// Note: plain <type name="TypeSince0" .../> definitions don't generate
/// standalone structs (they are inlined).  Only composites, enums, and sets
/// produce named types.
#[test]
fn since_version_filter_types_exist() {
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
            "EnumSince0Kind",
            "EnumSince4",
            "EnumSince4Kind",
            "EnumSince5",
            "EnumSince5Kind",
            "EnumWithSinceValues",
            "EnumWithSinceValuesKind",
            "SetSince0",
            "SetSince4",
            "SetSince5",
            "SetWithSinceChoices",
            "GroupSizeEncoding",
            "VarStringEncoding",
        ],
    );
}

/// Issue 895: optional float/double with NaN-as-null semantics.
#[test]
fn issue_895_optional_float_double_types_exist() {
    assert_tool_schema(
        "issue895",
        "issue895.xml",
        &["Issue895Decoder", "Issue895Encoder"],
    );
}

/// Issue 972: optional composite with versioned fields.
#[test]
fn issue_972_optional_composite_types_exist() {
    assert_tool_schema(
        "issue972",
        "issue972.xml",
        &["Issue972Decoder", "Issue972Encoder", "NewComposite"],
    );
}

// ── Item 2: Extract XML schemas with edge cases ──────────────────────────

/// Composite elements: enum, set, and nested composite inside a composite.
/// Note: inline enum/set/composite types (`EnumOne`, `SetOne`, Inner) are
/// embedded in the Outer composite and don't generate standalone structs.
#[test]
fn composite_elements_types_exist() {
    assert_tool_schema(
        "comp_el",
        "composite-elements-schema.xml",
        &["Outer(pub", "MsgDecoder", "MsgEncoder"],
    );
}

/// Explicit field offsets in composites and messages.
#[test]
fn composite_offsets_types_exist() {
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
}

/// Basic repeating group with custom dimensionType.
#[test]
fn basic_group_types_exist() {
    assert_tool_schema(
        "basic_grp",
        "basic-group-schema.xml",
        &[
            "TestMessage1Decoder",
            "TestMessage1Encoder",
            "GroupSizeEncoding",
        ],
    );
}

/// Triply-nested repeating groups.
#[test]
fn nested_group_types_exist() {
    assert_tool_schema(
        "nested_grp",
        "nested-group-schema.xml",
        &["TopDecoder", "TopEncoder", "GroupSizeEncoding"],
    );
}

/// Groups that contain var-data fields (single, multiple, nested).
#[test]
fn group_with_data_types_exist() {
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
}

/// Message with embedded length/count (group dimension inside composite) and var data.
#[test]
fn embedded_length_and_count_types_exist() {
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
}

/// Group with constant fields inside and outside messages.
/// Note: `PrimitiveConst` and `StrConst` are simple type aliases;
/// they don't generate standalone structs.
#[test]
fn group_with_constant_fields_types_exist() {
    assert_tool_schema(
        "const_flds",
        "group-with-constant-fields.xml",
        &[
            "ConstantsGaloreDecoder",
            "ConstantsGaloreEncoder",
            "ModelKind",
            "CompositeWithConst",
            "GroupSizeEncoding",
        ],
    );
}

/// Value-ref schemas: constant enum valueRef and constant type valueRef.
#[test]
fn value_ref_types_exist() {
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
}

/// Basic minimal schema (single uint32 field, no groups/var-data).
#[test]
fn basic_schema_types_exist() {
    assert_tool_schema(
        "basic",
        "basic-schema.xml",
        &["TestMessage50001Decoder", "TestMessage50001Encoder"],
    );
}

/// Types schema with various primitive types.
#[test]
fn basic_types_schema_types_exist() {
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
}

/// Block-length test schema with explicit blockLength on messages.
#[test]
fn block_length_schema_types_exist() {
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
}

// ── todo 101: Gap coverage tests ──────────────────────────────────────

/// UTF-16 var-data encoding: verify schema parses and generates valid Rust.
/// The generated code currently hardcodes `core::str::from_utf8` for var-data
/// accessors, so UTF-16 decoding will fail at runtime. This test asserts the
/// structural gap: valid schema, valid Rust output, no UTF-16-specific method.
#[test]
fn utf16_encoding_types_exist() {
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
}

/// Unit attribute: verify schema with `unit` on types parses successfully.
/// The `unit` attribute is currently NOT stored in the IR or emitted into
/// generated `field_meta`. This test asserts the schema parses and produces
/// valid Rust despite the gap. The generated code should include
/// `FIELD_MIN` / `FIELD_MAX` constants for types with `minValue`/`maxValue`.
#[test]
fn unit_attribute_types_exist() {
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
}

/// Group entry with versioned fields: verify schema parses and generates
/// valid Rust. The generated `EntryDecoder::skip()` currently ignores the
/// wire `block_len` from the dimension header and uses the compiled
/// `ENTRY_BLOCK_LENGTH` constant — a classic SBE versioning bug.
/// This test asserts structural validity; a full wire-parity test will
/// need a binary fixture encoded at an earlier schema version.
#[test]
fn group_extension_types_exist() {
    assert_tool_schema(
        "grp_ext",
        "group-extension-test-schema.xml",
        &[
            "GroupExtensionMessageDecoder",
            "GroupExtensionMessageEncoder",
            "EntriesDecoder",
            "EntriesEntryDecoder",
            "ENTRY_BLOCK_LENGTH",
        ],
    );
}
