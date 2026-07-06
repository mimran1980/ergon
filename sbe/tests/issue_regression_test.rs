//! Regression tests for all upstream issue schemas (issueXXX.xml).
//!
//! Each test verifies that an upstream SBE regression-test schema:
//!   1. Parses as valid XML (proves roxmltree can read it)
//!   2. Has expected SBE metadata (package, id, version)
//!   3. Generates through the Generator pipeline without panicking
//!
//! When the full XML-to-IR parser lands, these tests will be extended to
//! verify semantic parse, full codegen, and compile-and-run.

#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;

// ── Path helpers ──────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap();
    for ancestor in cwd.ancestors() {
        if ancestor.join("Cargo.toml").exists() && ancestor.join("sbe").exists() {
            return ancestor.to_path_buf();
        }
    }
    let fb = PathBuf::from("../..");
    if fb.join("Cargo.toml").exists() && fb.join("simple-binary-encoding").exists() {
        return fb;
    }
    panic!("Cannot find workspace root from {cwd:?}");
}

fn issue_schema(num: &str) -> PathBuf {
    workspace_root()
        .join("simple-binary-encoding")
        .join("sbe-tool")
        .join("src")
        .join("test")
        .join("resources")
        .join(format!("issue{num}.xml"))
}

/// Minimal schema metadata extracted from SBE XML.
struct SchemaMeta {
    package: String,
    id: u16,
    version: u16,
    byte_order: String,
}

/// Parse SBE XML using roxmltree and extract top-level metadata.
fn parse_xml_meta(xml: &str) -> SchemaMeta {
    let doc = roxmltree::Document::parse(xml).expect("XML must be well-formed");
    let root = doc
        .root()
        .children()
        .find(roxmltree::Node::is_element)
        .expect("must have root element");

    // Accept both <messageSchema> and <ns2:messageSchema> (roxmltree
    // returns the local name via tag_name().name()).
    assert_eq!(
        root.tag_name().name(),
        "messageSchema",
        "root element must be <messageSchema>"
    );

    let package = root.attribute("package").unwrap_or("(missing)").to_string();
    let id = root
        .attribute("id")
        .and_then(|v| v.parse().ok())
        .expect("id must be a valid u16");
    let version = root
        .attribute("version")
        .and_then(|v| v.parse().ok())
        .expect("version must be a valid u16");
    let byte_order = root
        .attribute("byteOrder")
        .unwrap_or("littleEndian")
        .to_string();

    SchemaMeta {
        package,
        id,
        version,
        byte_order,
    }
}

// ── Schema metadata assertions ────────────────────────────────────────
// Each issue schema exercises a specific edge case. The test confirms
// it is valid XML with the expected SBE attributes.

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue435_enum_ref_composite_ref_set_ref() {
    let xml = fs::read_to_string(issue_schema("435")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue435");
    assert_eq!(meta.id, 435);
    assert_eq!(meta.version, 0);
    assert_eq!(meta.byte_order, "bigEndian");
    assert!(xml.contains("setRef"), "should contain set definition");
    assert!(xml.contains("enumRef"), "should contain enum definition");
    assert!(xml.contains("exampleRef"), "should contain composite ref");
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue472_optional_uint64() {
    let xml = fs::read_to_string(issue_schema("472")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue472");
    assert_eq!(meta.id, 472);
    assert!(
        xml.contains("presence=\"optional\""),
        "should have optional field"
    );
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue483_all_presence_types() {
    let xml = fs::read_to_string(issue_schema("483")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue483");
    assert_eq!(meta.id, 483);
    assert!(xml.contains("presence=\"required\""), "required field");
    assert!(xml.contains("presence=\"constant\""), "constant field");
    assert!(xml.contains("presence=\"optional\""), "optional field");
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue488_variable_length_data() {
    let xml = fs::read_to_string(issue_schema("488")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue488");
    assert_eq!(meta.id, 488);
    assert!(
        xml.contains("varDataEncoding"),
        "should contain varDataEncoding"
    );
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue496_nested_composites() {
    let xml = fs::read_to_string(issue_schema("496")).unwrap();
    let meta = parse_xml_meta(&xml);
    // Note: this file re-uses the issue488 package/id
    assert_eq!(meta.id, 488);
    assert!(xml.contains("compositeOne"), "compositeOne");
    assert!(xml.contains("compositeTwo"), "compositeTwo");
    assert!(xml.contains("compositeThree"), "compositeThree");
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue505_constant_fields() {
    let xml = fs::read_to_string(issue_schema("505")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue505");
    assert_eq!(meta.id, 505);
    assert!(xml.contains("presence=\"constant\""), "constant fields");
    // Multiple constant field patterns
    assert!(xml.contains(">C<"), "char constant C"); // idSourceOne
    assert!(xml.contains(">D<"), "char constant D"); // idSourceTwo
    assert!(xml.contains(">EF<"), "char constant EF"); // idSourceThree
    assert!(xml.contains(">GH<"), "char constant GH"); // idSourceFour
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue560_constant_enum_ref() {
    let xml = fs::read_to_string(issue_schema("560")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue560");
    assert_eq!(meta.id, 560);
    assert!(
        xml.contains("valueRef=\"Model.C\""),
        "constant enum valueRef"
    );
    assert!(
        xml.contains("groupSizeEncoding"),
        "groupSizeEncoding composite"
    );
    assert!(
        xml.contains("varStringEncoding"),
        "varStringEncoding composite"
    );
    assert!(xml.contains("varDataEncoding"), "varDataEncoding composite");
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue567_valid_group_uint32_dimension() {
    let xml = fs::read_to_string(issue_schema("567-valid")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.id, 1);
    assert!(
        xml.contains("numInGroup") && xml.contains("maxValue"),
        "valid group dimension has maxValue"
    );
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue567_invalid_group_dimension() {
    let xml = fs::read_to_string(issue_schema("567-invalid")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.id, 1);
    // uint32 numInGroup WITHOUT maxValue — invalid by SBE spec.
    // Our parser should reject this later.
    // Use a targeted regex: the numInGroup type element should lack maxValue.
    assert!(xml.contains("primitiveType=\"uint32\""));
    // Extract the groupSizeEncoding block
    if let Some(start) = xml.find("groupSizeEncoding") {
        let after = &xml[start..];
        let end = after.find("</composite>").unwrap_or(after.len());
        let gse = &after[..end];
        assert!(
            !gse.contains("maxValue"),
            "invalid schema's groupSizeEncoding lacks maxValue constraint"
        );
    } else {
        panic!("groupSizeEncoding not found");
    }
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue661_set_with_since_version() {
    let xml = fs::read_to_string(issue_schema("661")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue661");
    assert_eq!(meta.id, 661);
    assert_eq!(meta.version, 1);
    assert!(
        xml.contains("sinceVersion=\"1\""),
        "field with sinceVersion"
    );
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue827_set_uint64_encoding() {
    let xml = fs::read_to_string(issue_schema("827")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue827");
    assert_eq!(meta.id, 827);
    assert!(xml.contains("encodingType=\"uint64\""), "uint64 encoding");
    assert!(xml.contains("Bit35"), "bit 35 position");
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue835_large_fix_schema() {
    let xml = fs::read_to_string(issue_schema("835")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "simple");
    assert_eq!(meta.id, 1);
    assert_eq!(meta.version, 9);
    // ns2 namespace — parser must not reject this
    assert!(
        xml.contains("ns2:messageSchema"),
        "should use ns2 namespace"
    );
    assert!(
        xml.len() > 10_000,
        "schema should be large (got {})",
        xml.len()
    );
    assert!(
        xml.contains("MDIncrementalRefreshOrderBook47"),
        "should contain the group message"
    );
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue847_composite_ref_in_header() {
    let xml = fs::read_to_string(issue_schema("847")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue847");
    assert_eq!(meta.id, 1);
    assert!(xml.contains("name=\"c1\""), "ref inside messageHeader");
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue848_composite_ref_to_composite() {
    let xml = fs::read_to_string(issue_schema("848")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue848");
    assert_eq!(meta.id, 1);
    assert!(xml.contains("name=\"c1\""), "ref to Comp1 inside Comp2");
    assert!(xml.contains("Comp2"), "Comp2 composite");
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue849_deeply_nested_composites() {
    let xml = fs::read_to_string(issue_schema("849")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue849");
    assert_eq!(meta.id, 1);
    assert!(xml.contains("Comp1"), "Comp1");
    assert!(xml.contains("Comp2"), "Comp2");
    assert!(xml.contains("Comp3"), "Comp3");
    assert!(xml.contains("Comp4"), "Comp4");
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue889_enum_optional_encoding() {
    let xml = fs::read_to_string(issue_schema("889")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue889");
    assert_eq!(meta.id, 1);
    assert!(
        xml.contains("encodingType=\"uInt8NULL\""),
        "enum with optional encoding type"
    );
    assert!(xml.contains("LotType"), "LotType enum");
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue895_optional_float_double() {
    let xml = fs::read_to_string(issue_schema("895")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue895");
    assert_eq!(meta.id, 895);
    assert!(xml.contains("presence=\"optional\""), "optional fields");
    assert!(
        xml.contains("type=\"float\"") && xml.contains("type=\"double\""),
        "float and double types"
    );
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue910_keyword_field_names() {
    let xml = fs::read_to_string(issue_schema("910")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue910");
    assert_eq!(meta.id, 910);
    // "yield" is a Rust keyword — codegen must handle it
    assert!(xml.contains("yield"), "field named 'yield'");
    assert!(xml.contains("varDataEncoding"), "varDataEncoding");
    assert!(xml.contains("groupSizeEncoding"), "groupSizeEncoding");
    // 8 messages
    let msg_count = xml.matches("message name=").count();
    assert_eq!(msg_count, 8, "should have 8 messages, found {msg_count}");
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue967_composite_optional_constant() {
    let xml = fs::read_to_string(issue_schema("967")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue967");
    assert_eq!(meta.id, 1);
    assert_eq!(meta.version, 13);
    // PRICENULL9 has optional mantissa + constant exponent
    assert!(
        xml.contains("presence=\"optional\"") && xml.contains("presence=\"constant\""),
        "both optional and constant presence in composite"
    );
    assert!(
        xml.contains("sinceVersion=\"12\""),
        "field with sinceVersion"
    );
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue972_composite_optional_fields() {
    let xml = fs::read_to_string(issue_schema("972")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue972");
    assert_eq!(meta.id, 972);
    assert_eq!(meta.version, 2);
    assert!(
        xml.contains("presence=\"optional\""),
        "optional fields in composite"
    );
    assert!(
        xml.contains("nullValue=\"0\""),
        "null value on optional fields"
    );
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue984_group_char_arrays() {
    let xml = fs::read_to_string(issue_schema("984")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue984");
    assert_eq!(meta.id, 984);
    assert_eq!(meta.version, 3);
    assert!(xml.contains("String4"), "char[4] type");
    assert!(xml.contains("String5"), "char[5] type");
    assert!(xml.contains("String6"), "char[6] type");
    assert!(xml.contains("sinceVersion=\"2\""), "sinceVersion on field");
    assert!(
        xml.contains("dimensionType=\"groupSize\""),
        "custom dimensionType"
    );
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue987_composite_offset_attributes() {
    let xml = fs::read_to_string(issue_schema("987")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue987");
    assert_eq!(meta.id, 987);
    assert_eq!(meta.version, 1);
    assert!(
        xml.contains("offset=\"4\""),
        "composite with explicit offset"
    );
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue1007_enum_keyword_values() {
    let xml = fs::read_to_string(issue_schema("1007")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue1007");
    assert_eq!(meta.id, 1007);
    // ValidValue names "false" and "true" are Rust keywords
    assert!(xml.contains("name=\"false\""), "validValue named 'false'");
    assert!(xml.contains("name=\"true\""), "validValue named 'true'");
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue1028_set_since_version_in_composite() {
    let xml = fs::read_to_string(issue_schema("1028")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue1028");
    assert_eq!(meta.id, 1028);
    assert_eq!(meta.version, 4);
    assert!(xml.contains("sinceVersion=\"4\""), "sinceVersion on set");
    assert!(xml.contains("EventIndicator"), "EventIndicator set");
    assert!(
        xml.contains("OutboundBusinessHeader"),
        "OutboundBusinessHeader composite"
    );
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue1057_set_and_primitive_in_composite() {
    let xml = fs::read_to_string(issue_schema("1057")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue1057");
    assert_eq!(meta.id, 1057);
    assert_eq!(meta.version, 4);
    assert!(xml.contains("SessionID"), "primitive type in composite");
    assert!(xml.contains("EventIndicator"), "set type in composite");
    assert!(
        xml.contains("OutboundBusinessHeader"),
        "OutboundBusinessHeader composite"
    );
}

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue1066_optional_versioned_field() {
    let xml = fs::read_to_string(issue_schema("1066")).unwrap();
    let meta = parse_xml_meta(&xml);
    assert_eq!(meta.package, "issue1066");
    assert_eq!(meta.id, 1066);
    assert_eq!(meta.version, 2);
    assert!(xml.contains("sinceVersion=\"2\""), "sinceVersion on field");
    assert!(xml.contains("presence=\"optional\""), "optional on field");
}

// ── Codegen pipeline smoke test ───────────────────────────────────────

#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn issue435_codegen_pipeline() {
    let xml = fs::read_to_string(issue_schema("435")).unwrap();
    let meta = parse_xml_meta(&xml);
    let schema = ergosbe::Schema::new(&meta.package, meta.id, meta.version);
    let generator = ergosbe::Generator::new(ergosbe::GenerationConfig::new("issue435"));
    let modules = generator.generate(&schema);
    let module = modules.modules().next().unwrap();
    assert_eq!(module.path, "issue435.rs");
    assert!(module.source.contains(&meta.package));
    assert!(module.source.contains(&meta.id.to_string()));
}

// ── Bulk checks ───────────────────────────────────────────────────────

/// All issue schemas parse as valid XML.
#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn all_issue_schemas_valid_xml() {
    for num in &[
        "435",
        "472",
        "483",
        "488",
        "496",
        "505",
        "560",
        "567-valid",
        "567-invalid",
        "661",
        "827",
        "835",
        "847",
        "848",
        "849",
        "889",
        "895",
        "910",
        "967",
        "972",
        "984",
        "987",
        "1007",
        "1028",
        "1057",
        "1066",
    ] {
        let path = issue_schema(num);
        let xml = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read issue{num}.xml: {e}"));
        let meta = parse_xml_meta(&xml);
        assert!(!meta.package.is_empty(), "issue{num}: package should exist");
        assert!(
            !meta.byte_order.is_empty(),
            "issue{num}: byteOrder should exist"
        );
    }
}

/// Codegen pipeline works for every issue schema.
#[test]
#[ignore = "requires removed simple-binary-encoding submodule"]
fn all_issue_schemas_codegen() {
    for num in &[
        "435",
        "472",
        "483",
        "488",
        "496",
        "505",
        "560",
        "567-valid",
        "661",
        "827",
        "835",
        "847",
        "848",
        "849",
        "889",
        "895",
        "910",
        "967",
        "972",
        "984",
        "987",
        "1007",
        "1028",
        "1057",
        "1066",
    ] {
        let path = issue_schema(num);
        let xml = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read issue{num}.xml: {e}"));
        let meta = parse_xml_meta(&xml);
        let schema = ergosbe::Schema::new(&meta.package, meta.id, meta.version);
        let generator =
            ergosbe::Generator::new(ergosbe::GenerationConfig::new(format!("issue{num}")));
        let modules = generator.generate(&schema);
        let module = modules
            .modules()
            .next()
            .unwrap_or_else(|| panic!("issue{num}: expected at least one module"));
        assert_eq!(module.path, format!("issue{num}.rs"));
        assert!(
            module.source.contains(&meta.id.to_string()),
            "issue{num}: source should contain schema id {}",
            meta.id
        );
    }
}
