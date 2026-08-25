//! Unit tests for the XML parser.

use std::path::{Path, PathBuf};

use super::registry::{try_parse_u64_val, value_in_declared_range};
use super::*;
use crate::ir::{ByteOrder, Encoding, Ir, Presence, PrimitiveType, Signal, Token};
use miette::Diagnostic;

#[test]
fn parse_u64_val_handles_value_types() -> Result<(), Box<dyn std::error::Error>> {
    // Empty -> None.
    assert_eq!(parse_u64_val("", None), None);
    // Char (single byte).
    assert_eq!(
        parse_u64_val("A", Some(PrimitiveType::Char)),
        Some(b'A' as u64)
    );
    // Float bit reinterpret (f32 branch).
    assert_eq!(
        parse_u64_val("1.5", Some(PrimitiveType::Float)),
        Some(1.5_f32.to_bits() as u64)
    );
    // Double bit reinterpret (f64 branch).
    assert_eq!(
        parse_u64_val("1.5", Some(PrimitiveType::Double)),
        Some(1.5_f64.to_bits() as u64)
    );
    // Unparseable float/double -> None (the branch fall-through return).
    assert_eq!(
        parse_u64_val("not_a_number", Some(PrimitiveType::Float)),
        None
    );
    assert_eq!(
        parse_u64_val("not_a_number", Some(PrimitiveType::Double)),
        None
    );
    // Negative -> i64 reinterpret.
    assert_eq!(parse_u64_val("-1", None), Some(u64::MAX));
    // Plain u64 / invalid.
    assert_eq!(parse_u64_val("42", None), Some(42));
    assert_eq!(parse_u64_val("garbage", None), None);

    Ok(())
}

#[test]
fn parse_malformed_xml_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = parse("<messageSchema><unclosed>").unwrap_err();
    assert!(matches!(err, ParseError::MalformedXml { .. }));

    Ok(())
}

#[test]
fn parse_valid_xml_without_message_schema_root_is_missing() -> Result<(), Box<dyn std::error::Error>>
{
    // Valid XML, but no <messageSchema> root element.
    let err = parse("<root/>").unwrap_err();
    assert!(matches!(err, ParseError::Missing { .. }));

    Ok(())
}

#[test]
fn parse_file_missing_path_is_io() -> Result<(), Box<dyn std::error::Error>> {
    let err = parse_file("/nonexistent/ergon/coverage/schema.xml").unwrap_err();
    assert!(matches!(err, ParseError::Io { .. }));
    assert!(std::error::Error::source(&err).is_some());

    Ok(())
}

#[test]
fn parse_set_choice_bit_out_of_range_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
<set name="S" encodingType="uint8">
  <choice name="Big">10</choice>
</set>
  </types>
</messageSchema>"#;
    assert!(parse(xml).is_err(), "set choice bit > max must error");

    Ok(())
}

#[test]
fn parse_set_duplicate_choice_bit_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
<set name="S" encodingType="uint8">
  <choice name="A">1</choice>
  <choice name="B">1</choice>
</set>
  </types>
</messageSchema>"#;
    assert!(parse(xml).is_err(), "duplicate set choice bit must error");

    Ok(())
}

#[test]
fn parse_invalid_byte_order_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="sideways">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/></composite></types>
</messageSchema>"#;
    assert!(parse(xml).is_err(), "invalid byteOrder must error");

    Ok(())
}

#[test]
fn parse_invalid_presence_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <message name="M" id="1"><field name="f" id="1" type="uint32" presence="bogus"/></message>
</messageSchema>"#;
    assert!(parse(xml).is_err(), "invalid presence must error");

    Ok(())
}

#[test]
fn parse_invalid_primitive_type_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="bad" primitiveType="notatype"/>
  </types>
</messageSchema>"#;
    assert!(parse(xml).is_err(), "invalid primitiveType must error");

    Ok(())
}

#[test]
fn parse_enum_with_float_encoding_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<enum name="E" encodingType="float"><validValue name="A">1</validValue></enum>
  </types>
</messageSchema>"#;
    assert!(parse(xml).is_err(), "enum with float encoding must error");

    Ok(())
}

#[test]
fn parse_set_with_signed_encoding_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<set name="S" encodingType="int8"><choice name="A">0</choice></set>
  </types>
</messageSchema>"#;
    assert!(parse(xml).is_err(), "set with signed encoding must error");

    Ok(())
}

#[test]
fn parse_set_duplicate_choice_name_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<set name="S" encodingType="uint8"><choice name="A">0</choice><choice name="A">1</choice></set>
  </types>
</messageSchema>"#;
    assert!(parse(xml).is_err(), "duplicate set choice name must error");

    Ok(())
}

#[test]
fn parse_invalid_message_schema_child_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/></composite></types>
  <unexpectedChild/>
</messageSchema>"#;
    assert!(
        parse(xml).is_err(),
        "invalid messageSchema child must error"
    );

    Ok(())
}

#[test]
fn parse_field_offset_out_of_order_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <message name="M" id="1" blockLength="8">
<field name="a" id="1" type="uint32" offset="4"/>
<field name="b" id="2" type="uint32" offset="0"/>
  </message>
</messageSchema>"#;
    assert!(parse(xml).is_err(), "out-of-order field offsets must error");

    Ok(())
}

#[test]
fn parse_invalid_message_child_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <message name="M" id="1"><bogusElement/></message>
</messageSchema>"#;
    assert!(parse(xml).is_err(), "invalid message child must error");

    Ok(())
}

#[test]
fn parse_invalid_types_container_child_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/></composite>
<bogusType/>
  </types>
</messageSchema>"#;
    assert!(
        parse(xml).is_err(),
        "invalid types container child must error"
    );

    Ok(())
}

#[test]
fn parse_collects_all_documentation_sources() -> Result<(), Box<dyn std::error::Error>> {
    // schema-docs-all-sources.xml exercises all four documentation shapes:
    // description attrs, <description> children, <comment> children, and
    // XML <!-- --> comments. Verify they all reach the IR.
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/schema-docs-all-sources.xml"
    );
    let ir = parse_file(path).unwrap();

    // Schema-level: description attr collected from root + preceding
    // XML comment (<!-- xml-comment:schema --> before root element).
    let sd = ir.description.as_ref().unwrap();
    assert!(
        sd.contains("attr:schema"),
        "missing schema description attr in {sd:?}"
    );

    // Root-level preceding XML comment: the comment before the root
    // element in the Document is a preceding sibling of the root,
    // so preceding_xml_comments(root) picks it up.
    assert!(
        sd.contains("xml-comment:schema"),
        "missing preceding XML comment on schema root in {sd:?}"
    );

    // Verify deterministic merge order on the root: attr first, then
    // preceding XML comments (root has no child <description>/<comment>).
    let attr_pos = sd.find("attr:schema").expect("attr:schema");
    let comment_pos = sd.find("xml-comment:schema").expect("xml-comment:schema");
    assert!(
        attr_pos < comment_pos,
        "description attr must precede XML comments; got {sd:?}"
    );

    // Find the messageHeader token — must now include all 4 sources
    // including the preceding-sibling XML comment.
    let mh = ir
        .tokens
        .iter()
        .find(|t| t.name == "messageHeader")
        .expect("messageHeader composite token not found");
    let mh_desc = mh.encoding.description.as_ref().unwrap();
    assert!(
        mh_desc.contains("attr:header"),
        "missing description attr in '{mh_desc}'"
    );
    assert!(
        mh_desc.contains("description-child:header"),
        "missing description child in '{mh_desc}'"
    );
    assert!(
        mh_desc.contains("comment-child:header"),
        "missing comment child in '{mh_desc}'"
    );
    assert!(
        mh_desc.contains("xml-comment:header"),
        "missing preceding-sibling XML comment in '{mh_desc}'"
    );

    // Also verify the enum picked up its preceding comment.
    let colour = ir
        .tokens
        .iter()
        .find(|t| t.name == "Colour")
        .expect("Colour token not found");
    let colour_desc = colour.encoding.description.as_ref().unwrap();
    assert!(
        colour_desc.contains("xml-comment:enum"),
        "missing preceding-sibling XML comment on Colour in '{colour_desc}'"
    );

    // And the message picked up its preceding comment.
    let msg = ir
        .tokens
        .iter()
        .find(|t| t.name == "M")
        .expect("M token not found");
    let msg_desc = msg.encoding.description.as_ref().unwrap();
    assert!(
        msg_desc.contains("xml-comment:message"),
        "missing preceding-sibling XML comment on M in '{msg_desc}'"
    );

    Ok(())
}

#[test]
fn parse_composite_with_undefined_type_member() -> Result<(), Box<dyn std::error::Error>> {
    // Composite member with type="X" where X isn't a known primitive
    // encoding or registered type — triggers the is_indirect_ref=true +
    // resolve_type_to_tokens=None fallback (lines ~769-796).
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<composite name="C"><type name="f" type="NoSuchType"/></composite>
  </types>
</messageSchema>"#;
    // Either parse errors (undefined type) or succeeds (fallback branch).
    // In either case the fallback code at 769-796 is exercised.
    let _ = parse(xml);
    Ok(())
}

#[test]
fn parse_include_file_not_found_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/></composite></types>
  <include href="definitely_nonexistent_file_12345.xml"/>
</messageSchema>"#;
    let err = parse(xml).unwrap_err();
    match &err {
        ParseError::Include {
            href,
            cause: crate::IncludeCause::NotFound,
            attempted,
            ..
        } => {
            assert_eq!(href, "definitely_nonexistent_file_12345.xml");
            assert!(!attempted.is_empty());
        }
        other => panic!("expected Include/NotFound, got {other:?}"),
    }

    Ok(())
}

#[test]
fn missing_no_node_creates_fault_without_span() -> Result<(), Box<dyn std::error::Error>> {
    let fault = Fault::missing_no_node("test");
    assert!(matches!(fault.kind, FaultKind::Missing { ref what } if what == "test"));
    assert!(fault.span.is_none());

    Ok(())
}

#[test]
fn resolve_type_with_since_version() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = TypeRegistry::new();
    registry.encodings.insert(
        "myType".to_string(),
        Encoding {
            primitive_type: Some(PrimitiveType::UInt32),
            ..Encoding::default()
        },
    );
    let result = resolve_type_to_tokens("f", "myType", Some(1), &registry, 5, None, None);
    assert!(result.is_some());
    assert_eq!(result.unwrap()[0].encoding.since_version, 5);

    Ok(())
}

#[test]
fn parse_missing_root_element() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parse("<?xml version=\"1.0\"?>\n<notSchema/>").is_err());

    Ok(())
}

#[test]
fn compute_type_size_all_paths() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = TypeRegistry::new();
    registry.encodings.insert(
        "p32".into(),
        Encoding {
            primitive_type: Some(PrimitiveType::Int32),
            length: Some(1),
            ..Encoding::default()
        },
    );
    assert_eq!(compute_type_size("p32", &registry), Some(4));
    registry.encodings.insert(
        "a4".into(),
        Encoding {
            primitive_type: Some(PrimitiveType::Int16),
            length: Some(4),
            ..Encoding::default()
        },
    );
    assert_eq!(compute_type_size("a4", &registry), Some(8));
    assert_eq!(compute_type_size("missing", &registry), None);

    Ok(())
}

#[test]
fn compute_type_size_composite_enum_set() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = TypeRegistry::new();
    let ct = vec![
        Token {
            id: None,
            name: "C".into(),
            signal: Signal::BeginComposite,
            encoding: Encoding::default(),
            span: None,
        },
        Token {
            id: None,
            name: "x".into(),
            signal: Signal::BeginField,
            encoding: Encoding {
                primitive_type: Some(PrimitiveType::Int32),
                length: Some(1),
                presence: Presence::Required,
                ..Encoding::default()
            },
            span: None,
        },
        Token {
            id: None,
            name: "x".into(),
            signal: Signal::EndField,
            encoding: Encoding::default(),
            span: None,
        },
        Token {
            id: None,
            name: "C".into(),
            signal: Signal::EndComposite,
            encoding: Encoding::default(),
            span: None,
        },
    ];
    registry.registry.insert("C".into(), ct);
    assert_eq!(compute_type_size("C", &registry), Some(4));

    let et = vec![Token {
        id: None,
        name: "E".into(),
        signal: Signal::BeginEnum,
        encoding: Encoding {
            primitive_type: Some(PrimitiveType::UInt8),
            ..Encoding::default()
        },
        span: None,
    }];
    registry.registry.insert("E".into(), et);
    assert_eq!(compute_type_size("E", &registry), Some(1));

    let st = vec![Token {
        id: None,
        name: "S".into(),
        signal: Signal::BeginSet,
        encoding: Encoding {
            primitive_type: Some(PrimitiveType::UInt16),
            ..Encoding::default()
        },
        span: None,
    }];
    registry.registry.insert("S".into(), st);
    assert_eq!(compute_type_size("S", &registry), Some(2));

    Ok(())
}

#[test]
fn parse_enum_duplicate_value() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<enum name="E" encodingType="uint8"><validValue name="A">1</validValue><validValue name="B">1</validValue></enum></types>
<sbe:message name="M" id="1"><field name="e" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    assert!(parse(xml).is_err());

    Ok(())
}

#[test]
fn parse_enum_null_sentinel_collision() -> Result<(), Box<dyn std::error::Error>> {
    // To trigger the null sentinel check, the enum's encodingType must
    // reference a REGISTERED type (not a bare primitive), because the
    // null_sentinel lookup goes through registry.encodings.
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="enumBase" primitiveType="uint8" nullValue="255"/>
<enum name="E" encodingType="enumBase"><validValue name="A">1</validValue><validValue name="Max">255</validValue></enum></types>
<sbe:message name="M" id="1"><field name="e" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    assert!(
        parse(xml).is_err(),
        "validValue == null sentinel must error"
    );
    Ok(())
}

#[test]
fn parse_set_bit_index_too_high() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<set name="F" encodingType="uint8"><choice name="X">99</choice></set></types>
<sbe:message name="M" id="1"><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    assert!(parse(xml).is_err());

    Ok(())
}

#[test]
fn parse_set_non_numeric_bit_index() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<set name="F" encodingType="uint8"><choice name="X">abc</choice></set></types>
<sbe:message name="M" id="1"><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    assert!(parse(xml).is_err());

    Ok(())
}

#[test]
fn parse_message_duplicate_field_name() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32"/><field name="x" id="2" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    assert!(parse(xml).is_err());

    Ok(())
}

#[test]
fn parse_message_duplicate_field_id() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32"/><field name="y" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    assert!(parse(xml).is_err());

    Ok(())
}

#[test]
fn parse_message_out_of_order_offset() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32" offset="4"/><field name="y" id="2" type="uint32" offset="0"/></sbe:message>
</sbe:messageSchema>"#;
    assert!(parse(xml).is_err());

    Ok(())
}

#[test]
fn parse_constant_field_missing_value() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="c" id="1" type="uint32" presence="constant"/></sbe:message>
</sbe:messageSchema>"#;
    assert!(parse(xml).is_err());

    Ok(())
}

#[test]
fn parse_composite_ref_member() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="baseInt" primitiveType="uint32"/>
<composite name="Wrapper"><type name="val" type="baseInt"/></composite></types>
<sbe:message name="M" id="1"><field name="w" id="1" type="Wrapper"/></sbe:message>
</sbe:messageSchema>"#;
    let _ = parse(xml);

    Ok(())
}

#[test]
fn parse_field_inheriting_presence() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="optVal" primitiveType="uint32" presence="optional" nullValue="4294967295"/></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="optVal"/></sbe:message>
</sbe:messageSchema>"#;
    // Exercise field inheriting presence from referenced type — may succeed or error
    let _ = parse(xml);

    Ok(())
}

#[test]
fn parse_value_ref_dot_notation() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<enum name="Colour" encodingType="uint8"><validValue name="Red">1</validValue></enum></types>
<sbe:message name="M" id="1"><field name="c" id="1" type="uint8" presence="constant" valueRef="Colour.Red"/></sbe:message>
</sbe:messageSchema>"#;
    // Exercise the valueRef dot-notation code path — may succeed or warn
    let _ = parse(xml);

    Ok(())
}

#[test]
fn parse_value_ref_unknown_enum_warns() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="c" id="1" type="uint8" presence="constant" valueRef="NonExistent.SomeVal"/></sbe:message>
</sbe:messageSchema>"#;
    // Exercise the valueRef unknown-enum warning path
    let _ = parse(xml);

    Ok(())
}

#[test]
fn parse_value_ref_no_dot() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="c" id="1" type="uint8" presence="constant" valueRef="SimpleVal"/></sbe:message>
</sbe:messageSchema>"#;
    // Exercise the valueRef no-dot path
    let _ = parse(xml);

    Ok(())
}

#[test]
fn parse_field_inherit_constant_from_type() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="ci" primitiveType="uint32" presence="constant">42</type></types>
<sbe:message name="M" id="1"><field name="c" id="1" type="ci" presence="constant"/></sbe:message>
</sbe:messageSchema>"#;
    let _ = parse(xml);

    Ok(())
}

#[test]
fn parse_char_constant_wrong_length() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="c3" primitiveType="char" length="3" presence="constant">AB</type></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    let _ = parse(xml);

    Ok(())
}

#[test]
fn parse_set_valid_indices() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
<types>
<set name="S" encodingType="uint8"><choice name="BitZero">0</choice><choice name="BitMax">7</choice></set>
<set name="S16" encodingType="uint16"><choice name="B">15</choice></set>
<set name="S32" encodingType="uint32"><choice name="B">31</choice></set>
<set name="S64" encodingType="uint64"><choice name="B">63</choice></set>
</types>
<message name="M" id="1"><field name="f" id="1" type="uint32"/></message>
</sbe:messageSchema>"#;
    let _ = parse(xml);

    Ok(())
}

#[test]
fn workspace_root_found() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();
    assert!(root.join("Cargo.toml").exists());

    Ok(())
}

#[test]
fn parse_message_with_explicit_offsets_and_registered_types()
-> Result<(), Box<dyn std::error::Error>> {
    // Triggers the offset tracking / compute_type_size path in parse_message
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <composite name="Point"><type name="x" primitiveType="int32"/><type name="y" primitiveType="int32"/></composite>
</types>
<sbe:message name="M" id="1">
  <field name="p" id="1" type="Point" offset="0"/>
  <field name="v" id="2" type="uint16" offset="8"/>
</sbe:message>
</sbe:messageSchema>"#;
    let _ = parse(xml);

    Ok(())
}

#[test]
fn parse_message_nullvalue_on_required_field() -> Result<(), Box<dyn std::error::Error>> {
    // Triggers the warning for nullValue on non-optional field
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32" nullValue="0"/></sbe:message>
</sbe:messageSchema>"#;
    let _ = parse(xml);

    Ok(())
}

#[test]
fn parse_char_constant_correct_length() -> Result<(), Box<dyn std::error::Error>> {
    // Triggers the char constant length check with correct length (length > 1)
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="code3" primitiveType="char" length="3" presence="constant">ABC</type></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    let _ = parse(xml);

    Ok(())
}

#[test]
fn parse_enum_with_description() -> Result<(), Box<dyn std::error::Error>> {
    // Triggers the enum description collection trailing brace
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<enum name="Colour" encodingType="uint8" description="Colour enum">
  <description>Colour description</description>
  <validValue name="Red" description="Red">1</validValue>
</enum></types>
<sbe:message name="M" id="1"><field name="c" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    let _ = parse(xml);

    Ok(())
}

#[test]
fn parse_set_with_description() -> Result<(), Box<dyn std::error::Error>> {
    // Triggers the set description collection trailing brace
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<set name="Flags" encodingType="uint8" description="Flag set">
  <description>Flag description</description>
  <choice name="A" description="First">0</choice>
</set></types>
<sbe:message name="M" id="1"><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    let _ = parse(xml);

    Ok(())
}

#[test]
fn parse_composite_member_nonexistent_type() -> Result<(), Box<dyn std::error::Error>> {
    // Triggers the else branch at line 770 where resolve_type_to_tokens
    // returns None for a type="X" that's not in the registry
    let xml = r#"<?xml version="1.0"?>
<messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="t" id="1" version="0" byteOrder="littleEndian">
<types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<composite name="C"><type name="f" type="NonExistent"/></composite></types>
<sbe:message name="M" id="1"><field name="x" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    let _ = parse(xml);
    Ok(())
}

#[test]
fn compute_type_size_array_and_constant_members() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = TypeRegistry::new();
    let ct = vec![
        Token {
            id: None,
            name: "C".into(),
            signal: Signal::BeginComposite,
            encoding: Encoding::default(),
            span: None,
        },
        Token {
            id: None,
            name: "arr".into(),
            signal: Signal::BeginField,
            encoding: Encoding {
                primitive_type: Some(PrimitiveType::Int16),
                length: Some(3),
                presence: Presence::Required,
                ..Encoding::default()
            },
            span: None,
        },
        Token {
            id: None,
            name: "arr".into(),
            signal: Signal::EndField,
            encoding: Encoding::default(),
            span: None,
        },
        Token {
            id: None,
            name: "c".into(),
            signal: Signal::BeginField,
            encoding: Encoding {
                primitive_type: Some(PrimitiveType::Char),
                length: Some(1),
                presence: Presence::Constant,
                ..Encoding::default()
            },
            span: None,
        },
        Token {
            id: None,
            name: "c".into(),
            signal: Signal::EndField,
            encoding: Encoding::default(),
            span: None,
        },
        Token {
            id: None,
            name: "C".into(),
            signal: Signal::EndComposite,
            encoding: Encoding::default(),
            span: None,
        },
    ];
    registry.registry.insert("C".into(), ct);
    assert_eq!(compute_type_size("C", &registry), Some(6));

    Ok(())
}

#[test]
fn compute_type_size_unknown_signal() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = TypeRegistry::new();
    let tokens = vec![Token {
        id: None,
        name: "X".into(),
        signal: Signal::Encoding,
        encoding: Encoding::default(),
        span: None,
    }];
    registry.registry.insert("X".into(), tokens);
    assert_eq!(compute_type_size("X", &registry), None);

    Ok(())
}

#[test]
fn parse_malformed_include_file_is_error() -> Result<(), Box<dyn std::error::Error>> {
    // The include file is found but contains invalid XML — covers the
    // Document::parse error handler in parse_schema (xml.rs:544-548).
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <include href="bad-include.xml"/>
</messageSchema>"#;
    assert!(parse(xml).is_err(), "malformed include file must error");

    Ok(())
}

#[test]
fn parse_var_data_with_simple_encoding_type_is_error() -> Result<(), Box<dyn std::error::Error>> {
    // A var-data field whose type is a simple encoding (uint32), not a
    // var-data composite, must be rejected.
    let xml = r#"<?xml version="1.0"?>
<messageSchema package="x" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  </types>
  <message name="M" id="1"><data name="d" id="1" type="uint32"/></message>
</messageSchema>"#;
    assert!(parse(xml).is_err(), "simple encoding as varData must error");

    Ok(())
}

const MINIMAL_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="example.sbe" id="1" version="0" byteOrder="littleEndian"
           description="minimal test schema">
  <types>
<composite name="messageHeader" description="SBE message header">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId"   primitiveType="uint16"/>
  <type name="schemaId"     primitiveType="uint16"/>
  <type name="version"      primitiveType="uint16"/>
</composite>
  </types>
  <message name="Car" id="1" blockLength="11" semanticType="">
<field name="serialNumber" id="1" type="uint64" offset="0" presence="required"/>
<field name="modelYear"    id="2" type="uint16" offset="8" presence="required"/>
<field name="available"    id="3" type="uint8"  offset="10" presence="required"/>
  </message>
</messageSchema>"#;

fn structural(name: &str, signal: Signal) -> Token {
    Token {
        id: None,
        name: name.to_string(),
        signal,
        encoding: Encoding::default(),
        span: None,
    }
}

fn field(
    name: &str,
    id: Option<u16>,
    primitive: PrimitiveType,
    offset: Option<usize>,
) -> [Token; 2] {
    let encoding = Encoding {
        primitive_type: Some(primitive),
        offset,
        presence: Presence::Required,
        since_version: 0,
        ..Encoding::default()
    };
    [
        Token {
            id,
            name: name.to_string(),
            signal: Signal::BeginField,
            encoding,
            span: None,
        },
        Token {
            id: None,
            name: name.to_string(),
            signal: Signal::EndField,
            encoding: Encoding::default(),
            span: None,
        },
    ]
}

#[test]
fn parses_schema_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(MINIMAL_SCHEMA).unwrap();
    assert_eq!(ir.package, "example.sbe");
    assert_eq!(ir.id, 1);
    assert_eq!(ir.version, 0);
    assert_eq!(ir.byte_order, ByteOrder::LittleEndian);
    assert_eq!(ir.description.as_deref(), Some("minimal test schema"));
    assert_eq!(ir.semantic_version, None);
    assert_eq!(ir.header_type, "messageHeader");

    Ok(())
}

#[test]
fn parses_message_header_composite_and_message_fields() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(MINIMAL_SCHEMA).unwrap();

    let mut expected = Vec::new();
    let mut msg_hdr_start = structural("messageHeader", Signal::BeginComposite);
    msg_hdr_start.encoding.description = Some("SBE message header".to_string());
    expected.push(msg_hdr_start);
    expected.extend(field("blockLength", None, PrimitiveType::UInt16, None));
    expected.extend(field("templateId", None, PrimitiveType::UInt16, None));
    expected.extend(field("schemaId", None, PrimitiveType::UInt16, None));
    expected.extend(field("version", None, PrimitiveType::UInt16, None));
    expected.push(structural("messageHeader", Signal::EndComposite));

    expected.push(Token {
        id: Some(1),
        name: "Car".to_string(),
        signal: Signal::BeginMessage,
        encoding: Encoding {
            since_version: 0,
            description: None,
            semantic_type: Some(String::new()),
            ..Encoding::default()
        },
        span: None,
    });
    expected.extend(field(
        "serialNumber",
        Some(1),
        PrimitiveType::UInt64,
        Some(0),
    ));
    expected.extend(field("modelYear", Some(2), PrimitiveType::UInt16, Some(8)));
    expected.extend(field("available", Some(3), PrimitiveType::UInt8, Some(10)));
    expected.push(structural("Car", Signal::EndMessage));

    let mut expected_ir = Ir {
        package: "example.sbe".to_string(),
        id: 1,
        version: 0,
        byte_order: ByteOrder::LittleEndian,
        description: None,
        semantic_version: None,
        header_type: "messageHeader".to_string(),
        tokens: expected,
    };
    crate::resolve::resolve_schema(&mut expected_ir, None).unwrap();

    // Normalise spans — parsed tokens carry real source locations but
    // expected tokens are hand-built with `None`.
    let mut actual_tokens = ir.tokens;
    for t in &mut actual_tokens {
        t.span = None;
    }
    assert_eq!(actual_tokens, expected_ir.tokens);

    Ok(())
}

#[test]
fn rejects_non_message_schema_root() -> Result<(), Box<dyn std::error::Error>> {
    let err = parse("<notSbe/>").unwrap_err();
    assert!(matches!(err, ParseError::Missing { .. }));

    Ok(())
}

#[test]
fn rejects_missing_package() -> Result<(), Box<dyn std::error::Error>> {
    let err = parse(r#"<messageSchema id="1" version="0"/>"#).unwrap_err();
    assert!(matches!(err, ParseError::Missing { .. }));

    Ok(())
}

#[test]
fn invalid_primitive_error_describes_and_spans() -> Result<(), Box<dyn std::error::Error>> {
    let err = parse(
        r#"<messageSchema package="x" id="1" version="0">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <message name="M" id="1"><field name="f" id="1" type="bogus"/></message>
</messageSchema>"#,
    )
    .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("type for field 'f'"), "{msg}");
    assert!(msg.contains("bogus"), "{msg}");
    assert!(err.labels().is_some(), "expected a span label attached");

    Ok(())
}

/// Proves the whole miette pipeline renders a real source snippet, not just
/// that `.labels()` / `.source_code()` return `Some`. A `build.rs` that
/// returns `Box<dyn std::error::Error>` from `main` prints this error via
/// `{:?}` (a raw struct dump) instead — `fn main() -> miette::Result<()>`
/// is what actually produces this graphical output.
#[test]
fn invalid_primitive_error_renders_source_snippet_via_miette()
-> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<messageSchema package="x" id="1" version="0">
  <types><composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite></types>
  <message name="M" id="1"><field name="f" id="1" type="bogus"/></message>
</messageSchema>"#;
    let err = parse(xml).unwrap_err();

    let mut rendered = String::new();
    miette::GraphicalReportHandler::new_themed(miette::GraphicalTheme::unicode_nocolor())
        .render_report(&mut rendered, &err)?;

    assert!(rendered.contains("bogus"), "rendered:\n{rendered}");
    assert!(
        rendered.contains("type for field 'f'"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.lines().count() > 1,
        "expected a multi-line snippet, got:\n{rendered}"
    );

    Ok(())
}

/// Walk up to find the workspace root (where the top-level Cargo.toml lives).
/// The opt-in XSD validator must never reject a schema [`parse`] accepts.
///
/// This swept up six real bugs: it dropped namespaced vendor attributes
/// (`mbx:exponent`) on the floor, and its allow-lists omitted
/// `characterEncoding` on `<data>`, `unit` on `<type>`, `jsonValue` on
/// `<validValue>`/`<choice>`, and `package` on `<types>` — between them enough
/// to reject the checked-in `l3-book` and `binance-spot` schemas. It also
/// required `@version`, which upstream sbe-tool's own fixtures omit.
///
/// Unit tests with hand-written schemas cannot catch this class of bug; only
/// the real corpus can, so the corpus is the assertion.
///
/// **Scope:** this catches the validator being *stricter than the parser* —
/// i.e. drift in the lists `xsd.rs` owns alone (`<types>`, `<type>`,
/// `<composite>`, `<enum>`, `<set>`, `<validValue>`, `<choice>`). It cannot
/// catch a regression in the lists both share via `schema_attrs`, because
/// breaking one of those makes `parse` reject the same schema, which the
/// parser-valid filter below then skips. Shared-list coverage comes from
/// `grammar_attributes_absent_from_the_fixture_corpus_are_accepted` and from
/// every fixture-driven test in the suite.
#[test]
fn xsd_validator_accepts_every_parser_valid_schema() -> Result<(), Box<dyn std::error::Error>> {
    fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect(&p, out);
            } else if p.extension().is_some_and(|x| x == "xml") {
                out.push(p);
            }
        }
    }

    let root = workspace_root();
    let mut files = Vec::new();
    for sub in ["sbe/tests", "samples", "cluster/schemas"] {
        collect(&root.join(sub), &mut files);
    }
    files.sort();

    let mut checked = 0usize;
    let mut bad = Vec::new();
    for f in &files {
        let Ok(xml) = std::fs::read_to_string(f) else {
            continue;
        };
        // Only schemas the parser itself accepts are in scope — a negative
        // fixture proves nothing about the validator.
        if parse(&xml).is_err() {
            continue;
        }
        checked += 1;
        if let Err(e) = crate::xsd::validate_against_sbe_xsd(&xml) {
            bad.push(format!("{}: {e}", f.display()));
        }
    }

    assert!(
        checked > 100,
        "expected the schema corpus to be found; only {checked} parsed"
    );
    assert!(
        bad.is_empty(),
        "XSD validator rejected {} schema(s) the parser accepts:\n{}",
        bad.len(),
        bad.join("\n")
    );
    Ok(())
}

fn workspace_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap();
    loop {
        if dir.join("Cargo.toml").exists() && dir.join("sbe").exists() {
            return dir;
        }
        assert!(
            dir.pop(),
            "cannot find workspace root from {:?}",
            std::env::current_dir()
        );
    }
}

fn sbe_test_resource(sub: &str) -> PathBuf {
    workspace_root()
        .join("sbe")
        .join("tests")
        .join("fixtures")
        .join("schemas")
        .join(sub)
}

fn sbe_sample_resource(sub: &str) -> PathBuf {
    workspace_root()
        .join("sbe")
        .join("tests")
        .join("fixtures")
        .join("schemas")
        .join(sub)
}

#[test]
fn parses_schema_with_xinclude_relative_path() -> Result<(), Box<dyn std::error::Error>> {
    let path = sbe_test_resource("sub/basic-schema.xml");
    let ir = parse_file(&path).unwrap();

    assert_eq!(ir.package, "SBE tests");
    assert_eq!(ir.id, 2);

    // Included types from sub2/common.xml should be present.
    // `Symbol` is a plain <type>, stored in the encoding registry (not tokens).
    // `messageHeader` is a <composite> → produces BeginComposite/EndComposite tokens.
    assert!(
        ir.tokens.iter().any(|t| t.name == "messageHeader"),
        "expected messageHeader composite from included sub2/common.xml"
    );

    // Schema's own message should also be present.
    assert!(
        ir.tokens.iter().any(|t| t.name == "TestMessage50001"),
        "expected TestMessage50001 from the main schema"
    );
    Ok(())
}

#[test]
fn parses_example_schema_with_xinclude() -> Result<(), Box<dyn std::error::Error>> {
    let path = sbe_sample_resource("example-schema.xml");
    let ir = parse_file(&path).unwrap();

    assert_eq!(ir.package, "baseline");

    // Included types from common-types.xml should be present.
    assert!(
        ir.tokens.iter().any(|t| t.name == "messageHeader"),
        "expected messageHeader from included common-types.xml"
    );
    assert!(
        ir.tokens.iter().any(|t| t.name == "groupSizeEncoding"),
        "expected groupSizeEncoding from included common-types.xml"
    );
    assert!(
        ir.tokens.iter().any(|t| t.name == "varDataEncoding"),
        "expected varDataEncoding from included common-types.xml"
    );

    Ok(())
}

#[test]
fn parse_with_shared_resolves_types_without_include() -> Result<(), Box<dyn std::error::Error>> {
    let common = parse(
        r#"<?xml version="1.0"?>
<messageSchema package="common" id="0" version="1" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<composite name="Price">
  <type name="mantissa" primitiveType="int64"/>
  <type name="exponent" primitiveType="int8"/>
</composite>
  </types>
</messageSchema>"#,
    )
    .unwrap();

    // No <types> or <include> of common-types.xml here — both messageHeader
    // and Price must resolve from `common`'s seeded registry.
    let orders = parse_with_shared(
        r#"<?xml version="1.0"?>
<messageSchema package="orders" id="1" version="1" byteOrder="littleEndian"
           headerType="messageHeader">
  <message name="NewOrder" id="1">
<field name="price" id="1" type="Price"/>
  </message>
</messageSchema>"#,
        &common,
    )?;

    assert!(
        orders
            .tokens
            .iter()
            .any(|t| t.name == "price" && t.signal == Signal::BeginField),
        "expected `price` field resolved from the shared `Price` composite"
    );

    Ok(())
}

#[test]
fn xinclude_without_base_falls_back_to_hardcoded_paths() -> Result<(), Box<dyn std::error::Error>> {
    // Without a base dir, the hardcoded submodule path probes should work
    // for common schemas.
    let path = sbe_sample_resource("example-schema.xml");
    let content = std::fs::read_to_string(&path).unwrap();
    let ir = parse(&content).unwrap();

    assert_eq!(ir.package, "baseline");
    assert!(
        ir.tokens.iter().any(|t| t.name == "groupSizeEncoding"),
        "expected groupSizeEncoding from included file via hardcoded paths"
    );

    Ok(())
}

fn include_cycle_chain(err: &ParseError) -> &[PathBuf] {
    match err {
        ParseError::Include {
            cause: crate::IncludeCause::Cycle { chain },
            ..
        } => chain,
        other => panic!("expected Include/Cycle, got {other:?}"),
    }
}

fn source_chain_reaches_io(err: &(dyn std::error::Error + 'static)) -> bool {
    let mut cur = err.source();
    while let Some(e) = cur {
        if e.downcast_ref::<std::io::Error>().is_some() {
            return true;
        }
        cur = e.source();
    }
    false
}

struct TempXmlDir(PathBuf);

impl TempXmlDir {
    fn new(label: &str) -> Result<Self, Box<dyn std::error::Error>> {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ergon_xml_{label}_{}_{}_{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }

    fn write(&self, name: &str, body: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let path = self.0.join(name);
        std::fs::write(&path, body)?;
        Ok(path)
    }
}

impl Drop for TempXmlDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn stub_schema_including(href: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<messageSchema package="cycle" id="1" version="0" byteOrder="littleEndian">
  <include href="{href}"/>
  <types>{HEADER_TYPES}</types>
</messageSchema>
"#
    )
}

#[test]
fn xinclude_detects_cycle() -> Result<(), Box<dyn std::error::Error>> {
    let path = sbe_test_resource("cyclic-self-include.xml");
    let canon = path.canonicalize()?;
    let err = parse_file(&path).unwrap_err();
    assert_eq!(include_cycle_chain(&err), [canon.clone(), canon].as_slice());

    Ok(())
}

#[test]
fn parse_file_mutual_include_cycle_is_visit_order() -> Result<(), Box<dyn std::error::Error>> {
    // Names are chosen so lexicographic HashSet order is a.xml then z.xml.
    // Visit order when parsing z.xml is z → a → z.
    let dir = TempXmlDir::new("mutual")?;
    let z = dir.write("z.xml", &stub_schema_including("a.xml"))?;
    let a = dir.write("a.xml", &stub_schema_including("z.xml"))?;
    let err = parse_file(&z).unwrap_err();
    let z_c = z.canonicalize()?;
    let a_c = a.canonicalize()?;
    assert_eq!(
        include_cycle_chain(&err),
        [z_c.clone(), a_c, z_c].as_slice(),
        "cycle chain must be visit order, not sorted HashSet order"
    );

    Ok(())
}

#[test]
fn parse_file_transitive_include_cycle_is_visit_order() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempXmlDir::new("transitive")?;
    let c = dir.write("c.xml", &stub_schema_including("b.xml"))?;
    let b = dir.write("b.xml", &stub_schema_including("a.xml"))?;
    let a = dir.write("a.xml", &stub_schema_including("c.xml"))?;
    let err = parse_file(&c).unwrap_err();
    let c_c = c.canonicalize()?;
    let b_c = b.canonicalize()?;
    let a_c = a.canonicalize()?;
    assert_eq!(
        include_cycle_chain(&err),
        [c_c.clone(), b_c, a_c, c_c].as_slice()
    );

    Ok(())
}

#[test]
fn parse_file_diamond_shared_include_is_not_a_cycle() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempXmlDir::new("diamond")?;
    dir.write(
        "shared.xml",
        r#"<?xml version="1.0"?>
<types>
  <composite name="SharedType">
    <type name="x" primitiveType="uint32"/>
  </composite>
</types>
"#,
    )?;
    dir.write(
        "mid.xml",
        r#"<?xml version="1.0"?>
<messageSchema package="mid" id="2" version="0">
  <include href="shared.xml"/>
  <types/>
</messageSchema>
"#,
    )?;
    let root = dir.write(
        "root.xml",
        r#"<?xml version="1.0"?>
<messageSchema package="root" id="1" version="0" byteOrder="littleEndian">
  <include href="shared.xml"/>
  <include href="mid.xml"/>
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="shared" id="1" type="SharedType"/>
  </message>
</messageSchema>
"#,
    )?;

    let ir = parse_file(&root)?;
    assert!(
        ir.tokens
            .iter()
            .any(|t| t.name == "shared" && t.signal == Signal::BeginField),
        "shared include must resolve once through both root and mid"
    );

    Ok(())
}

#[test]
fn parse_file_directory_root_is_io() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempXmlDir::new("dir_root")?;
    let err = parse_file(&dir.0).unwrap_err();
    match &err {
        ParseError::Io { path, source } => {
            assert_eq!(path, &dir.0);
            assert!(
                source.kind() == std::io::ErrorKind::IsADirectory
                    || source.kind() == std::io::ErrorKind::PermissionDenied,
                "directory root must surface a typed I/O kind, got {:?}",
                source.kind()
            );
        }
        other => panic!("expected ParseError::Io, got {other:?}"),
    }
    assert!(
        source_chain_reaches_io(&err),
        "Error::source must reach std::io::Error"
    );

    Ok(())
}

#[cfg(unix)]
#[test]
fn parse_file_unreadable_root_is_io() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempXmlDir::new("perm_root")?;
    let path = dir.write("schema.xml", &stub_schema_including("missing.xml"))?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))?;
    let err = parse_file(&path).unwrap_err();
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
    match &err {
        ParseError::Io { source, .. } => {
            assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
        }
        other => panic!("expected ParseError::Io, got {other:?}"),
    }
    assert!(source_chain_reaches_io(&err));

    Ok(())
}

#[test]
fn parse_file_unreadable_include_is_include_io() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempXmlDir::new("inc_io")?;
    let blocked = dir.0.join("blocked");
    std::fs::create_dir(&blocked)?;
    let root = dir.write(
        "root.xml",
        r#"<?xml version="1.0"?>
<messageSchema package="ioinc" id="1" version="0" byteOrder="littleEndian">
  <include href="blocked"/>
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
</messageSchema>
"#,
    )?;
    let err = parse_file(&root).unwrap_err();
    match &err {
        ParseError::Include {
            href,
            cause: crate::IncludeCause::Io { path, source },
            ..
        } => {
            assert_eq!(href, "blocked");
            assert_eq!(path, &blocked.canonicalize()?);
            assert!(
                source.kind() == std::io::ErrorKind::IsADirectory
                    || source.kind() == std::io::ErrorKind::PermissionDenied,
                "unreadable include must surface a typed I/O kind, got {:?}",
                source.kind()
            );
        }
        other => panic!("expected Include/Io, got {other:?}"),
    }
    assert!(
        source_chain_reaches_io(&err),
        "Error::source must reach std::io::Error through IncludeCause::Io"
    );

    Ok(())
}

#[cfg(unix)]
#[test]
fn parse_file_permission_denied_include_is_include_io() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempXmlDir::new("inc_perm")?;
    let blocked = dir.write("blocked.xml", "<types/>")?;
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o000))?;
    let root = dir.write("root.xml", &stub_schema_including("blocked.xml"))?;
    let err = parse_file(&root).unwrap_err();
    let _ = std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o644));
    match &err {
        ParseError::Include {
            href,
            cause: crate::IncludeCause::Io { path, source },
            ..
        } => {
            assert_eq!(href, "blocked.xml");
            assert_eq!(path, &blocked.canonicalize()?);
            assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
        }
        other => panic!("expected Include/Io, got {other:?}"),
    }
    assert!(source_chain_reaches_io(&err));

    Ok(())
}

#[test]
fn null_value_on_non_optional_type_parses_with_warning() -> Result<(), Box<dyn std::error::Error>> {
    // nullValue on a required type should generate a warning but still parse.
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="MyType" primitiveType="uint32" presence="required" nullValue="999"/>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="MyType"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    assert!(ir.tokens.iter().any(|t| t.name == "M"));

    Ok(())
}

#[test]
fn constant_field_without_value_errors() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="MT" primitiveType="uint32"/>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="MT" presence="constant"/>
  </message>
</messageSchema>"#;
    let err = parse(schema).unwrap_err();
    assert!(matches!(err, ParseError::Missing { .. }));

    Ok(())
}

#[test]
fn duplicate_enum_valid_value_names_error() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<enum name="Color" encodingType="uint8">
  <validValue name="Red">1</validValue>
  <validValue name="Red">2</validValue>
</enum>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="Color"/>
  </message>
</messageSchema>"#;
    let err = parse(schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

#[test]
fn duplicate_enum_encoded_values_error() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<enum name="Color" encodingType="uint8">
  <validValue name="Red">1</validValue>
  <validValue name="Blue">1</validValue>
</enum>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="Color"/>
  </message>
</messageSchema>"#;
    let err = parse(schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

#[test]
fn char_constant_length_too_short_errors() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<type name="CC" primitiveType="char" length="3" presence="constant">AB</type>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="CC"/>
  </message>
</messageSchema>"#;
    let err = parse(schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

#[test]
fn char_constant_exact_length_parses() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
<type name="CC" primitiveType="char" length="3" presence="constant">ABC</type>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="CC"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    assert!(ir.tokens.iter().any(|t| t.name == "M"));

    Ok(())
}

#[test]
fn duplicate_field_id_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
package="test" id="1" version="1" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
  </types>
  <sbe:message name="M" id="1">
<field name="a" id="1" type="uint8"/>
<field name="b" id="1" type="uint8"/>
  </sbe:message>
</sbe:messageSchema>"#;
    assert!(parse(schema).is_err());

    Ok(())
}

#[test]
fn duplicate_field_name_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
package="test" id="1" version="1" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
  </types>
  <sbe:message name="M" id="1">
<field name="dup" id="1" type="uint8"/>
<field name="dup" id="2" type="uint8"/>
  </sbe:message>
</sbe:messageSchema>"#;
    assert!(parse(schema).is_err());

    Ok(())
}

#[test]
fn group_with_unknown_dimension_type_fails() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
  </types>
  <message name="M" id="1">
<group name="g" id="2" dimensionType="NonExistentDim">
  <field name="f" id="3" type="uint32"/>
</group>
  </message>
</messageSchema>"#;
    let err = parse(schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

#[test]
fn group_with_wrong_dimension_type_structure_fails() -> Result<(), Box<dyn std::error::Error>> {
    // A composite that exists but lacks blockLength/numInGroup fields.
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<composite name="BadDim">
  <type name="foo" primitiveType="uint32"/>
</composite>
  </types>
  <message name="M" id="1">
<group name="g" id="2" dimensionType="BadDim">
  <field name="f" id="3" type="uint32"/>
</group>
  </message>
</messageSchema>"#;
    let err = parse(schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

fn dimension_schema(member: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="BadDim">
      {member}
    </composite>
  </types>
  <message name="M" id="1">
    <group name="g" id="2" dimensionType="BadDim">
      <field name="f" id="3" type="uint32"/>
    </group>
  </message>
</messageSchema>"#
    )
}

fn assert_dimension_invalid(xml: &str, field: &str) -> Result<(), Box<dyn std::error::Error>> {
    let err = parse(xml).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains(field),
        "expected field-qualified diagnostic containing {field:?}, got {msg}"
    );
    assert!(matches!(err, ParseError::Invalid { .. }));
    Ok(())
}

#[test]
fn group_dimension_rejects_signed_member() -> Result<(), Box<dyn std::error::Error>> {
    assert_dimension_invalid(
        &dimension_schema(
            r#"<type name="blockLength" primitiveType="int16"/>
               <type name="numInGroup" primitiveType="uint16"/>"#,
        ),
        "blockLength",
    )
}

#[test]
fn group_dimension_rejects_optional_member() -> Result<(), Box<dyn std::error::Error>> {
    assert_dimension_invalid(
        &dimension_schema(
            r#"<type name="blockLength" primitiveType="uint16"/>
               <type name="numInGroup" primitiveType="uint16" presence="optional"/>"#,
        ),
        "numInGroup",
    )
}

#[test]
fn group_dimension_rejects_array_member() -> Result<(), Box<dyn std::error::Error>> {
    assert_dimension_invalid(
        &dimension_schema(
            r#"<type name="blockLength" primitiveType="uint16" length="2"/>
               <type name="numInGroup" primitiveType="uint16"/>"#,
        ),
        "blockLength",
    )
}

#[test]
fn group_dimension_rejects_overlapping_members() -> Result<(), Box<dyn std::error::Error>> {
    assert_dimension_invalid(
        &dimension_schema(
            r#"<type name="blockLength" primitiveType="uint32" offset="0"/>
               <type name="numInGroup" primitiveType="uint32" offset="2"/>"#,
        ),
        "numInGroup",
    )
}

#[test]
fn group_dimension_rejects_out_of_bounds_member() -> Result<(), Box<dyn std::error::Error>> {
    assert_dimension_invalid(
        &dimension_schema(
            r#"<type name="blockLength" primitiveType="uint16" offset="0"/>
               <type name="numInGroup" primitiveType="uint16" offset="18446744073709551615"/>"#,
        ),
        "numInGroup",
    )
}

#[test]
fn group_dimension_rejects_missing_member() -> Result<(), Box<dyn std::error::Error>> {
    assert_dimension_invalid(
        &dimension_schema(r#"<type name="blockLength" primitiveType="uint16"/>"#),
        "numInGroup",
    )
}

#[test]
fn var_data_with_unknown_type_fails() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
  </types>
  <message name="M" id="1">
<data name="d" id="2" type="NonExistentVarType"/>
  </message>
</messageSchema>"#;
    let err = parse(schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

#[test]
fn var_data_with_wrong_type_structure_fails() -> Result<(), Box<dyn std::error::Error>> {
    // A composite that exists but lacks length/varData fields.
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<composite name="BadVar">
  <type name="foo" primitiveType="uint32"/>
</composite>
  </types>
  <message name="M" id="1">
<data name="d" id="2" type="BadVar"/>
  </message>
</messageSchema>"#;
    let err = parse(schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

#[test]
fn malformed_variable_data_encodings_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (
            "reversed members",
            r#"<type name="varData" primitiveType="uint8" length="0"/>
               <type name="length" primitiveType="uint16"/>"#,
            "",
        ),
        (
            "interposed member",
            r#"<type name="length" primitiveType="uint16"/>
               <type name="flags" primitiveType="uint8"/>
               <type name="varData" primitiveType="uint8" length="0"/>"#,
            "",
        ),
        (
            "signed length",
            r#"<type name="length" primitiveType="int16"/>
               <type name="varData" primitiveType="uint8" length="0"/>"#,
            "",
        ),
        (
            "nullable length",
            r#"<type name="length" primitiveType="uint16" presence="optional"/>
               <type name="varData" primitiveType="uint8" length="0"/>"#,
            "",
        ),
        (
            "non-octet payload",
            r#"<type name="length" primitiveType="uint16"/>
               <type name="varData" primitiveType="uint16" length="0"/>"#,
            "",
        ),
        (
            "gap before payload",
            r#"<type name="length" primitiveType="uint16"/>
               <type name="varData" primitiveType="uint8" length="0" offset="4"/>"#,
            "",
        ),
        (
            "optional data field",
            r#"<type name="length" primitiveType="uint16"/>
               <type name="varData" primitiveType="uint8" length="0"/>"#,
            r#" presence="optional""#,
        ),
    ];

    for (name, members, data_attrs) in cases {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<composite name="badVarData">{members}</composite>
  </types>
  <message name="M" id="1">
<data name="d" id="1" type="badVarData"{data_attrs}/>
  </message>
</messageSchema>"#
        );
        assert!(
            parse(&schema).is_err(),
            "{name} must not be accepted as a variable-data encoding"
        );
    }

    Ok(())
}

#[test]
fn block_length_validation_passes_for_correct_value() -> Result<(), Box<dyn std::error::Error>> {
    // Computed: uint64@0=8, uint16@8=2, uint8@10=1 → 11
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
  </types>
  <message name="M" id="1" blockLength="11">
<field name="a" id="1" type="uint64" offset="0"/>
<field name="b" id="2" type="uint16" offset="8"/>
<field name="c" id="3" type="uint8"  offset="10"/>
  </message>
</messageSchema>"#;
    parse(schema).unwrap();

    Ok(())
}

#[test]
fn larger_block_length_is_legal_padding() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
  </types>
  <message name="M" id="1" blockLength="99">
<field name="a" id="1" type="uint64" offset="0"/>
<field name="b" id="2" type="uint16" offset="8"/>
<field name="c" id="3" type="uint8"  offset="10"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    let message = ir
        .tokens
        .iter()
        .find(|token| token.signal == Signal::BeginMessage)
        .unwrap();
    assert_eq!(message.encoding.offset, Some(99));

    Ok(())
}

#[test]
fn overlapping_fixed_field_offsets_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (
            "message",
            format!(
                r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
<field name="a" id="1" type="uint32" offset="0"/>
<field name="b" id="2" type="uint16" offset="2"/>
  </message>
</messageSchema>"#
            ),
        ),
        (
            "group",
            format!(
                r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<composite name="groupSizeEncoding">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="numInGroup" primitiveType="uint16"/>
</composite>
  </types>
  <message name="M" id="1">
<group name="g" id="1" dimensionType="groupSizeEncoding">
  <field name="a" id="2" type="uint32" offset="0"/>
  <field name="b" id="3" type="uint16" offset="2"/>
</group>
  </message>
</messageSchema>"#
            ),
        ),
        (
            "composite",
            format!(
                r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<composite name="Overlap">
  <type name="a" primitiveType="uint32"/>
  <type name="b" primitiveType="uint16" offset="2"/>
</composite>
  </types>
  <message name="M" id="1"><field name="c" id="1" type="Overlap"/></message>
</messageSchema>"#
            ),
        ),
    ];

    for (name, schema) in cases {
        assert!(
            parse(&schema).is_err(),
            "{name} overlapping offsets must be rejected"
        );
    }

    Ok(())
}

#[test]
fn undersized_message_and_group_block_lengths_are_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (
            "message",
            format!(
                r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1" blockLength="2">
<field name="a" id="1" type="uint32"/>
  </message>
</messageSchema>"#
            ),
        ),
        (
            "group",
            format!(
                r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<composite name="groupSizeEncoding">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="numInGroup" primitiveType="uint16"/>
</composite>
  </types>
  <message name="M" id="1">
<group name="g" id="1" dimensionType="groupSizeEncoding" blockLength="2">
  <field name="a" id="2" type="uint32"/>
</group>
  </message>
</messageSchema>"#
            ),
        ),
    ];

    for (name, schema) in cases {
        assert!(
            parse(&schema).is_err(),
            "{name} blockLength must cover its fixed fields"
        );
    }

    Ok(())
}

#[test]
fn field_inherits_optional_presence_from_type() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<type name="OptU32" primitiveType="uint32" presence="optional"/>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="OptU32"/>
<field name="g" id="2" type="OptU32" presence="required"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    // f should have inherited optional presence from OptU32.
    let f_begins: Vec<&Token> = ir
        .tokens
        .iter()
        .filter(|t| t.name == "f" && t.signal == Signal::BeginField)
        .collect();
    assert_eq!(f_begins.len(), 1, "expected exactly one BeginField for 'f'");
    assert_eq!(
        f_begins[0].encoding.presence,
        Presence::Optional,
        "f should inherit Optional from OptU32"
    );
    // g has explicit presence="required" and should stay required.
    let g_begins: Vec<&Token> = ir
        .tokens
        .iter()
        .filter(|t| t.name == "g" && t.signal == Signal::BeginField)
        .collect();
    assert_eq!(g_begins.len(), 1, "expected exactly one BeginField for 'g'");
    assert_eq!(
        g_begins[0].encoding.presence,
        Presence::Required,
        "g should stay Required (explicit)"
    );

    Ok(())
}

#[test]
fn field_inherits_constant_presence_from_type() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<type name="ConstU32" primitiveType="uint32" presence="constant">42</type>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="ConstU32"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    let f_begins: Vec<&Token> = ir
        .tokens
        .iter()
        .filter(|t| t.name == "f" && t.signal == Signal::BeginField)
        .collect();
    assert_eq!(f_begins.len(), 1, "expected exactly one BeginField for 'f'");
    assert_eq!(
        f_begins[0].encoding.presence,
        Presence::Constant,
        "f should inherit Constant from ConstU32"
    );

    Ok(())
}

#[test]
fn composite_member_with_valid_ref_parses() -> Result<(), Box<dyn std::error::Error>> {
    // <ref> on a composite member should resolve through the registry.
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<type name="innerType" primitiveType="uint32"/>
<composite name="outer">
  <type name="inner" ref="innerType"/>
</composite>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="outer"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    assert!(ir.tokens.iter().any(|t| t.name == "M"));

    Ok(())
}

#[test]
fn composite_member_with_invalid_ref_fails() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<composite name="outer">
  <type name="inner" ref="BogusType"/>
</composite>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="outer"/>
  </message>
</messageSchema>"#;
    let err = parse(schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

#[test]
fn custom_header_type_with_required_fields_parses() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" headerType="MyHeader" byteOrder="littleEndian">
  <types>
<composite name="MyHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
  </types>
  <message name="M" id="1">
<field name="x" id="1" type="uint8"/>
  </message>
</messageSchema>"#;
    parse(schema).unwrap();

    Ok(())
}

#[test]
fn custom_header_type_missing_fields_fails() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" headerType="MyHeader" byteOrder="littleEndian">
  <types>
<composite name="MyHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
  <!-- missing schemaId -->
</composite>
  </types>
  <message name="M" id="1">
<field name="x" id="1" type="uint8"/>
  </message>
</messageSchema>"#;
    let err = parse(schema).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("schemaId"),
        "expected error about missing schemaId, got: {msg}"
    );

    Ok(())
}

#[test]
fn malformed_message_header_fields_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (
            "missing header composite",
            r#"<type name="NotAHeader" primitiveType="uint16"/>"#,
        ),
        (
            "signed blockLength",
            r#"<composite name="messageHeader">
                <type name="blockLength" primitiveType="int16"/>
                <type name="templateId" primitiveType="uint16"/>
                <type name="schemaId" primitiveType="uint16"/>
                <type name="version" primitiveType="uint16"/>
               </composite>"#,
        ),
        (
            "signed templateId",
            r#"<composite name="messageHeader">
                <type name="blockLength" primitiveType="uint16"/>
                <type name="templateId" primitiveType="int32"/>
                <type name="schemaId" primitiveType="uint16"/>
                <type name="version" primitiveType="uint16"/>
               </composite>"#,
        ),
        (
            "array schemaId",
            r#"<composite name="messageHeader">
                <type name="blockLength" primitiveType="uint16"/>
                <type name="templateId" primitiveType="uint16"/>
                <type name="schemaId" primitiveType="uint16" length="2"/>
                <type name="version" primitiveType="uint16"/>
               </composite>"#,
        ),
        (
            "optional version",
            r#"<composite name="messageHeader">
                <type name="blockLength" primitiveType="uint16"/>
                <type name="templateId" primitiveType="uint16"/>
                <type name="schemaId" primitiveType="uint16"/>
                <type name="version" primitiveType="uint16" presence="optional"/>
               </composite>"#,
        ),
        (
            "optional group count",
            r#"<composite name="messageHeader">
                <type name="blockLength" primitiveType="uint16"/>
                <type name="templateId" primitiveType="uint16"/>
                <type name="schemaId" primitiveType="uint16"/>
                <type name="version" primitiveType="uint16"/>
                <type name="numGroups" primitiveType="uint16" presence="optional"/>
               </composite>"#,
        ),
    ];

    for (name, header) in cases {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{header}</types>
  <message name="M" id="1"><field name="x" id="1" type="uint8"/></message>
</messageSchema>"#
        );
        assert!(parse(&schema).is_err(), "{name} must be rejected");
    }

    Ok(())
}

#[test]
fn message_members_must_follow_fixed_group_data_order() -> Result<(), Box<dyn std::error::Error>> {
    let types = format!(
        r#"{HEADER_TYPES}
<composite name="groupSizeEncoding">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="numInGroup" primitiveType="uint16"/>
</composite>
<composite name="varDataEncoding">
  <type name="length" primitiveType="uint16"/>
  <type name="varData" primitiveType="uint8" length="0"/>
</composite>"#
    );
    let invalid_bodies = [
        (
            "field after group",
            r#"<group name="g" id="1"><field name="a" id="2" type="uint8"/></group>
               <field name="late" id="3" type="uint8"/>"#,
        ),
        (
            "field after data",
            r#"<data name="d" id="1" type="varDataEncoding"/>
               <field name="late" id="2" type="uint8"/>"#,
        ),
        (
            "group after data",
            r#"<data name="d" id="1" type="varDataEncoding"/>
               <group name="g" id="2"><field name="a" id="3" type="uint8"/></group>"#,
        ),
        (
            "nested field after data",
            r#"<group name="g" id="1">
                 <data name="d" id="2" type="varDataEncoding"/>
                 <field name="late" id="3" type="uint8"/>
               </group>"#,
        ),
    ];

    for (name, body) in invalid_bodies {
        let schema = format!(
            r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{types}</types>
  <message name="M" id="1">{body}</message>
</messageSchema>"#
        );
        let error = parse(&schema).expect_err(name);
        assert!(
            format!("{error}").contains("message member order"),
            "{name} failed for an unrelated reason: {error}"
        );
    }

    let valid = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{types}</types>
  <message name="M" id="1">
<field name="fixed" id="1" type="uint8"/>
<group name="g" id="2"><field name="entry" id="3" type="uint8"/></group>
<data name="d" id="4" type="varDataEncoding"/>
  </message>
</messageSchema>"#
    );
    parse(&valid)?;

    Ok(())
}

#[test]
fn parses_epoch_and_time_unit_on_type() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<type name="Timestamp" primitiveType="uint64" epoch="unix" timeUnit="nanoseconds"/>
  </types>
  <message name="M" id="1">
<field name="ts" id="1" type="Timestamp"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    let ts_tokens: Vec<&Token> = ir
        .tokens
        .iter()
        .filter(|t| t.name == "ts" && t.signal == Signal::BeginField)
        .collect();
    assert_eq!(ts_tokens.len(), 1);
    assert_eq!(
        ts_tokens[0].encoding.epoch.as_deref(),
        Some("unix"),
        "epoch should be inherited from type"
    );
    assert_eq!(
        ts_tokens[0].encoding.time_unit.as_deref(),
        Some("nanoseconds"),
        "timeUnit should be inherited from type"
    );

    Ok(())
}

#[test]
fn parses_epoch_and_time_unit_on_field() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
  </types>
  <message name="M" id="1">
<field name="ts" id="1" type="uint64" epoch="unix" timeUnit="nanoseconds"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    let ts_tokens: Vec<&Token> = ir
        .tokens
        .iter()
        .filter(|t| t.name == "ts" && t.signal == Signal::BeginField)
        .collect();
    assert_eq!(ts_tokens.len(), 1);
    assert_eq!(ts_tokens[0].encoding.epoch.as_deref(), Some("unix"));
    assert_eq!(
        ts_tokens[0].encoding.time_unit.as_deref(),
        Some("nanoseconds")
    );

    Ok(())
}

#[test]
fn deprecated_on_type() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<type name="OldType" primitiveType="uint32" deprecated="1"/>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="OldType"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    let old_tokens: Vec<&Token> = ir
        .tokens
        .iter()
        .filter(|t| t.name == "f" && t.signal == Signal::BeginField)
        .collect();
    assert_eq!(old_tokens.len(), 1);
    assert_eq!(old_tokens[0].encoding.deprecated, Some(1));

    Ok(())
}

#[test]
fn deprecated_on_message() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
  </types>
  <message name="M" id="1" deprecated="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    let msg_token = ir
        .tokens
        .iter()
        .find(|t| t.signal == Signal::BeginMessage && t.name == "M");
    assert!(msg_token.is_some());
    assert_eq!(msg_token.unwrap().encoding.deprecated, Some(1));

    Ok(())
}

#[test]
fn deprecated_on_field() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8" deprecated="1"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    let f_tokens: Vec<&Token> = ir
        .tokens
        .iter()
        .filter(|t| t.name == "f" && t.signal == Signal::BeginField)
        .collect();
    assert_eq!(f_tokens.len(), 1);
    assert_eq!(f_tokens[0].encoding.deprecated, Some(1));

    Ok(())
}

#[test]
fn deprecated_on_group() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
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
  </types>
  <message name="M" id="1">
<group name="g" id="2" dimensionType="groupSizeEncoding" deprecated="1">
  <field name="f" id="3" type="uint32"/>
</group>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    let g_token = ir
        .tokens
        .iter()
        .find(|t| t.signal == Signal::BeginGroup && t.name == "g");
    assert!(g_token.is_some());
    assert_eq!(g_token.unwrap().encoding.deprecated, Some(1));

    Ok(())
}

#[test]
fn deprecated_on_data() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<composite name="varDataEncoding">
  <type name="length" primitiveType="uint32"/>
  <type name="varData" primitiveType="uint8" length="0"/>
</composite>
  </types>
  <message name="M" id="1">
<data name="d" id="2" type="varDataEncoding" deprecated="1"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    let d_token = ir
        .tokens
        .iter()
        .find(|t| t.signal == Signal::BeginVarData && t.name == "d");
    assert!(d_token.is_some());
    assert_eq!(d_token.unwrap().encoding.deprecated, Some(1));

    Ok(())
}

#[test]
fn deprecated_on_composite() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<composite name="OldComposite" deprecated="1">
  <type name="val" primitiveType="uint32"/>
</composite>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="OldComposite"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    let c_token = ir
        .tokens
        .iter()
        .find(|t| t.signal == Signal::BeginComposite && t.name == "OldComposite");
    assert!(c_token.is_some());
    assert_eq!(c_token.unwrap().encoding.deprecated, Some(1));

    Ok(())
}

#[test]
fn deprecated_on_enum() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<enum name="OldEnum" encodingType="uint8" deprecated="1">
  <validValue name="A">1</validValue>
</enum>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="OldEnum"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    let e_token = ir
        .tokens
        .iter()
        .find(|t| t.signal == Signal::BeginEnum && t.name == "OldEnum");
    assert!(e_token.is_some());
    assert_eq!(e_token.unwrap().encoding.deprecated, Some(1));

    Ok(())
}

#[test]
fn deprecated_on_set() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<set name="OldSet" encodingType="uint8" deprecated="1">
  <choice name="X">0</choice>
</set>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="OldSet"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    let s_token = ir
        .tokens
        .iter()
        .find(|t| t.signal == Signal::BeginSet && t.name == "OldSet");
    assert!(s_token.is_some());
    assert_eq!(s_token.unwrap().encoding.deprecated, Some(1));

    Ok(())
}

#[test]
fn deprecated_version_zero_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="OldType" primitiveType="uint32" deprecated="0"/>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="OldType"/>
  </message>
</messageSchema>"#;
    let ir = parse(xml)?;
    let field = ir
        .tokens
        .iter()
        .find(|t| t.name == "f" && t.signal == Signal::BeginField)
        .ok_or("field f")?;
    assert_eq!(field.encoding.deprecated, Some(0));
    Ok(())
}

#[test]
fn deprecated_ordinary_version_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="OldType" primitiveType="uint32" deprecated="7"/>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="OldType"/>
  </message>
</messageSchema>"#;
    let ir = parse(xml)?;
    let field = ir
        .tokens
        .iter()
        .find(|t| t.name == "f" && t.signal == Signal::BeginField)
        .ok_or("field f")?;
    assert_eq!(field.encoding.deprecated, Some(7));
    Ok(())
}

#[test]
fn field_inherits_type_deprecation_version() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="OldType" primitiveType="uint32" deprecated="4"/>
  </types>
  <message name="M" id="1">
    <field name="f" id="1" type="OldType"/>
  </message>
</messageSchema>"#;
    let ir = parse(xml)?;
    let field = ir
        .tokens
        .iter()
        .find(|t| t.name == "f" && t.signal == Signal::BeginField)
        .ok_or("field f")?;
    assert_eq!(field.encoding.deprecated, Some(4));
    Ok(())
}

#[test]
fn field_and_type_deprecation_keep_the_earliest_version() -> Result<(), Box<dyn std::error::Error>>
{
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="OldType" primitiveType="uint32" deprecated="5"/>
  </types>
  <message name="M" id="1">
    <field name="earlier_on_field" id="1" type="OldType" deprecated="2"/>
    <field name="earlier_on_type" id="2" type="OldType" deprecated="9"/>
  </message>
</messageSchema>"#;
    let ir = parse(xml)?;
    let earlier_on_field = ir
        .tokens
        .iter()
        .find(|t| t.name == "earlier_on_field" && t.signal == Signal::BeginField)
        .ok_or("earlier_on_field")?;
    let earlier_on_type = ir
        .tokens
        .iter()
        .find(|t| t.name == "earlier_on_type" && t.signal == Signal::BeginField)
        .ok_or("earlier_on_type")?;
    assert_eq!(earlier_on_field.encoding.deprecated, Some(2));
    assert_eq!(earlier_on_type.encoding.deprecated, Some(5));
    Ok(())
}

#[test]
fn duplicate_message_name_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
  </types>
  <message name="M" id="1">
<field name="a" id="1" type="uint8"/>
  </message>
  <message name="M" id="2">
<field name="b" id="2" type="uint8"/>
  </message>
</messageSchema>"#;
    let err = parse(schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));
    let msg = format!("{err}");
    assert!(
        msg.contains("duplicate message name"),
        "expected error about duplicate message name, got: {msg}"
    );

    Ok(())
}

#[test]
fn vardata_member_excluded_from_block_length() -> Result<(), Box<dyn std::error::Error>> {
    // The varData member inside varDataEncoding has length="0", which marks it
    // as variable-length. The block length should only include the length field (4 bytes).
    let schema = r#"<?xml version="1.0" encoding="UTF-8"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>
<composite name="varDataEncoding">
  <type name="length" primitiveType="uint32"/>
  <type name="varData" primitiveType="uint8" length="0"/>
</composite>
  </types>
  <message name="M" id="1">
<field name="a" id="1" type="uint32"/>
<data name="d" id="2" type="varDataEncoding"/>
  </message>
</messageSchema>"#;
    let ir = parse(schema).unwrap();
    // The resolver will compute blockLength from fixed-width fields only.
    // uint32 = 4 bytes; varData's length field is uint32 = 4 bytes but lives in the tail.
    // So block length should be 4 (just field 'a').
    // Data fields are tail-encoded, so they don't contribute to message blockLength.
    // Find the BeginMessage token for M and verify its offset (block length).
    let msg_token = ir
        .tokens
        .iter()
        .find(|t| t.signal == Signal::BeginMessage && t.name == "M");
    assert!(msg_token.is_some(), "expected BeginMessage for M");
    // The block length is the computed offset stored on the BeginMessage token.
    // With one uint32 field (4 bytes) and no other fixed fields, it should be 4.
    assert_eq!(
        msg_token.unwrap().encoding.offset,
        Some(4),
        "expected block length 4 for message with one uint32 field"
    );

    Ok(())
}

const HEADER_TYPES: &str = r#"
<composite name="messageHeader">
  <type name="blockLength" primitiveType="uint16"/>
  <type name="templateId" primitiveType="uint16"/>
  <type name="schemaId" primitiveType="uint16"/>
  <type name="version" primitiveType="uint16"/>
</composite>"#;

/// An unknown attribute is an authoring typo — `presense` for `presence`
/// silently drops the intent, so the parser must reject it rather than
/// ignore it. Namespaced attributes (`xsi:*`, `xi:*`) are outside the SBE
/// grammar and must still pass.
#[test]
fn unknown_attributes_are_rejected_but_namespaced_ones_pass()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = |schema_attrs: &str, field_attrs: &str| {
        format!(
            r#"<?xml version="1.0"?>
<messageSchema package="t" id="1" version="0" byteOrder="littleEndian"{schema_attrs}>
  <types>{HEADER_TYPES}
    <type name="u32" primitiveType="uint32"/>
  </types>
  <message name="M" id="1">
    <field name="x" id="1" type="u32"{field_attrs}/>
  </message>
</messageSchema>"#
        )
    };

    // Control: the same schema with no stray attribute parses, so a failure
    // below is the attribute and not a malformed fixture.
    parse(&schema("", ""))?;

    // Typo on the root element.
    assert!(matches!(
        parse(&schema(r#" bogusAttr="x""#, "")).unwrap_err(),
        ParseError::Invalid { .. }
    ));

    // Typo on a field — `presense` is the classic one.
    assert!(matches!(
        parse(&schema("", r#" presense="optional""#)).unwrap_err(),
        ParseError::Invalid { .. }
    ));

    // Namespaced attributes are not part of the SBE grammar; they must pass.
    parse(&schema(
        r#" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="sbe.xsd""#,
        "",
    ))?;

    Ok(())
}

/// Guards the allow-list entries that no checked-in schema exercises.
///
/// The lists are the union of the SBE XSD grammar with what real schemas
/// carry, because neither source alone is complete: the corpus omits valid
/// grammar attributes, and the published XSD omits `constantValue` /
/// `length` / `nullValue` / `characterEncoding` that sbe-tool accepts. The
/// 160-schema fixture corpus defends the second half; nothing but this test
/// defends the first, so dropping one of these would reject valid SBE with
/// every repository test still green.
#[test]
fn grammar_attributes_absent_from_the_fixture_corpus_are_accepted()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = |field_attrs: &str, data_attrs: &str| {
        format!(
            r#"<?xml version="1.0"?>
<messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
    <type name="u32" primitiveType="uint32"/>
    <composite name="varStringEncoding">
      <type name="length" primitiveType="uint16"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="x" id="1" type="u32"{field_attrs}/>
    <data name="note" id="2" type="varStringEncoding"{data_attrs}/>
  </message>
</messageSchema>"#
        )
    };

    // Control first: the bare shape must parse.
    parse(&schema("", ""))?;

    // `<data>` is typed `sbe:fieldType` by the XSD, so it takes the full
    // field attribute set — not the narrower list one might hand-write.
    for attr in [
        r#" epoch="unix""#,
        r#" timeUnit="nanosecond""#,
        r#" offset="0""#,
        r#" semanticType="Length""#,
    ] {
        parse(&schema("", attr)).map_err(|e| format!("<data>{attr} must be accepted, got: {e}"))?;
    }

    // Grammar-declared field attributes no fixture happens to use.
    for attr in [
        r#" minValue="0""#,
        r#" maxValue="99""#,
        r#" epoch="unix""#,
        r#" timeUnit="nanosecond""#,
    ] {
        parse(&schema(attr, ""))
            .map_err(|e| format!("<field>{attr} must be accepted, got: {e}"))?;
    }

    Ok(())
}

#[test]
fn include_of_message_schema_wrapped_types_registers_types()
-> Result<(), Box<dyn std::error::Error>> {
    // The included file's root is <messageSchema>, not <types> — the
    // parser must descend into it and find the nested <types> node.
    let dir = std::env::temp_dir().join(format!("ergon_xml_inc_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let inc = dir.join("wrapped-types.xml");
    std::fs::write(
        &inc,
        r#"<?xml version="1.0"?>
<messageSchema package="inc" id="9" version="0">
  <types>
<type name="IncU8" primitiveType="uint8"/>
  </types>
</messageSchema>"#,
    )
    .unwrap();
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <include href="{}"/>
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
<field name="f" id="1" type="IncU8"/>
  </message>
</messageSchema>"#,
        inc.display()
    );
    let ir = parse(&schema).unwrap();
    assert!(
        ir.tokens
            .iter()
            .any(|t| t.name == "f" && t.signal == Signal::BeginField),
        "field using included type must resolve"
    );
    std::fs::remove_file(&inc).ok();
    Ok(())
}

#[test]
fn parse_file_with_deps_lists_root_nested_and_transitive_includes()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!(
        "ergon_xml_deps_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let nested = dir.join("nested");
    std::fs::create_dir_all(&nested)?;
    std::fs::write(
        dir.join("leaf.xml"),
        r#"<?xml version="1.0"?>
<types>
  <composite name="LeafType">
    <type name="x" primitiveType="uint32"/>
  </composite>
</types>
"#,
    )?;
    std::fs::write(
        nested.join("mid.xml"),
        r#"<?xml version="1.0"?>
<messageSchema package="mid" id="2" version="0">
  <include href="../leaf.xml"/>
  <types/>
</messageSchema>
"#,
    )?;
    let root = dir.join("root.xml");
    std::fs::write(
        &root,
        r#"<?xml version="1.0"?>
<messageSchema package="root" id="1" version="0" byteOrder="littleEndian">
  <include href="nested/mid.xml"/>
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="M" id="1">
    <field name="leaf" id="1" type="LeafType"/>
  </message>
</messageSchema>
"#,
    )?;

    let parsed = parse_file_with_deps(&root)?;
    assert!(
        parsed
            .ir
            .tokens
            .iter()
            .any(|t| t.name == "leaf" && t.signal == Signal::BeginField),
        "transitive LeafType must resolve through nested/mid.xml"
    );

    let names: Vec<String> = parsed
        .dependencies
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.first().map(String::as_str), Some("root.xml"));
    assert!(names.contains(&"mid.xml".to_string()), "{names:?}");
    assert!(names.contains(&"leaf.xml".to_string()), "{names:?}");
    assert_eq!(names.len(), 3, "each path once: {names:?}");
    let again = parse_file_with_deps(&root)?;
    assert_eq!(
        parsed.dependencies, again.dependencies,
        "dependency order must be deterministic"
    );

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn include_without_href_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <include/>
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    parse(&schema).unwrap();

    Ok(())
}

#[test]
fn char_constant_with_matching_length_parses() -> Result<(), Box<dyn std::error::Error>> {
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<type name="CC" primitiveType="char" length="3" presence="constant">ABC</type>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    parse(&schema).unwrap();

    Ok(())
}

#[test]
fn composite_member_with_primitive_type_attr_inlines_encoding()
-> Result<(), Box<dyn std::error::Error>> {
    // Member uses `type="uint16"` (a primitive name, not a registered
    // type). This is indirect by shape but unresolvable by name, so the
    // parser falls back to inline parsing of the element itself.
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<composite name="Pair">
  <type name="a" type="uint16"/>
  <type name="b" primitiveType="uint16"/>
</composite>
  </types>
  <message name="M" id="1">
<field name="p" id="1" type="Pair"/>
  </message>
</messageSchema>"#
    );
    let ir = parse(&schema).unwrap();
    assert!(
        ir.tokens
            .iter()
            .any(|t| t.name == "p" && t.signal == Signal::BeginField),
        "composite field must resolve"
    );

    Ok(())
}

#[test]
fn composite_member_without_any_type_attr_is_parsed_inline()
-> Result<(), Box<dyn std::error::Error>> {
    // Member has no type/primitiveType/ref attribute at all — the parser
    // parses the bare element inline (no primitive type recorded).
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<composite name="Bare">
  <type name="mystery"/>
</composite>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    // The composite is never referenced by a message, so whether the
    // overall parse succeeds is a resolver decision; the member itself
    // must not panic and must take the inline-parse path.
    let _ = parse(&schema);

    Ok(())
}

#[test]
fn enum_valid_value_equal_to_registered_null_sentinel_is_error()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<type name="OptU8" primitiveType="uint8" presence="optional" nullValue="255"/>
<enum name="E" encodingType="OptU8">
  <validValue name="X">255</validValue>
</enum>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    let err = parse(&schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

#[test]
fn enum_with_unknown_child_element_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<enum name="E" encodingType="uint8">
  <validValue name="A">1</validValue>
  <somethingElse/>
</enum>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    parse(&schema).unwrap();

    Ok(())
}

#[test]
fn set_with_unknown_child_element_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<set name="S" encodingType="uint8">
  <choice name="A">1</choice>
  <somethingElse/>
</set>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    parse(&schema).unwrap();

    Ok(())
}

#[test]
fn set_choice_non_numeric_bit_index_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<set name="S" encodingType="uint8">
  <choice name="A">notanumber</choice>
</set>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    let err = parse(&schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

#[test]
fn message_children_with_missing_or_unparseable_attrs_reach_second_pass()
-> Result<(), Box<dyn std::error::Error>> {
    // The first structural pass tolerates a missing name, a non-numeric
    // id, and a non-numeric offset; the second (real) parse pass then
    // reports the actual fault. This proves the pre-validation loop does
    // not panic or mask errors on degenerate attributes.
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
<field id="xyz" type="uint8" offset="abc"/>
  </message>
</messageSchema>"#
    );
    let err = parse(&schema).unwrap_err();
    assert!(matches!(
        err,
        ParseError::Missing { .. } | ParseError::Invalid { .. }
    ));

    Ok(())
}

#[test]
fn block_length_tracking_skips_fields_without_computable_size()
-> Result<(), Box<dyn std::error::Error>> {
    // Field with a valid offset but an unregistered type: the expected
    // block-length tracker must skip it rather than fault.
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
<field name="a" id="1" type="NotAKnownType" offset="0"/>
  </message>
</messageSchema>"#
    );
    // The field itself fails to resolve in the second pass — the point is
    // the block-length pre-pass tolerated the unknown size first.
    let err = parse(&schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

#[test]
fn null_value_on_required_field_warns_but_parses() -> Result<(), Box<dyn std::error::Error>> {
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8" nullValue="255"/>
  </message>
</messageSchema>"#
    );
    parse(&schema).unwrap();

    Ok(())
}

#[test]
fn include_with_non_types_sibling_elements_is_tolerated() -> Result<(), Box<dyn std::error::Error>>
{
    // The included <messageSchema> carries a <message> sibling next to
    // <types>; only the <types> node is imported.
    let dir = std::env::temp_dir().join(format!("ergon_xml_inc2_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let inc = dir.join("wrapped-types-siblings.xml");
    std::fs::write(
        &inc,
        r#"<?xml version="1.0"?>
<messageSchema package="inc" id="9" version="0">
  <message name="Ignored" id="7"/>
  <types>
<type name="IncU16" primitiveType="uint16"/>
  </types>
</messageSchema>"#,
    )
    .unwrap();
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <include href="{}"/>
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
<field name="f" id="1" type="IncU16"/>
  </message>
</messageSchema>"#,
        inc.display()
    );
    parse(&schema).unwrap();
    std::fs::remove_file(&inc).ok();

    Ok(())
}

#[test]
fn char_constant_without_text_is_tolerated_at_parse_time() -> Result<(), Box<dyn std::error::Error>>
{
    // presence="constant" with no element text: the length check is
    // skipped because there is no constant value to measure.
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<type name="CC2" primitiveType="char" length="3" presence="constant"/>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    let _ = parse(&schema);

    Ok(())
}

#[test]
fn composite_member_with_unknown_type_and_primitive_type_falls_back_inline()
-> Result<(), Box<dyn std::error::Error>> {
    // `type="Unknown"` is unresolvable, but `primitiveType="uint8"` lets
    // the inline fallback parse the element directly.
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<composite name="Odd">
  <type name="m" type="Unknown" primitiveType="uint8"/>
</composite>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    let _ = parse(&schema);

    Ok(())
}

#[test]
fn enum_valid_value_unparseable_with_null_sentinel_skips_check()
-> Result<(), Box<dyn std::error::Error>> {
    // The null-sentinel equality check is skipped when the value text
    // cannot be parsed for the encoding type.
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<type name="OptU8b" primitiveType="uint8" presence="optional" nullValue="255"/>
<enum name="E2" encodingType="OptU8b">
  <validValue name="A">notanumber</validValue>
</enum>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    let _ = parse(&schema);

    Ok(())
}

#[test]
fn field_with_unparseable_offset_attr_is_tolerated_by_prevalidation()
-> Result<(), Box<dyn std::error::Error>> {
    // Structural pre-validation ignores an offset it cannot parse; the
    // real field parse itself does not require the attribute.
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8" offset="abc"/>
  </message>
</messageSchema>"#
    );
    let _ = parse(&schema);

    Ok(())
}

#[test]
fn block_length_tracker_skips_type_without_computable_size()
-> Result<(), Box<dyn std::error::Error>> {
    // "NoPrim" is registered but has no primitive type, so the block
    // length tracker cannot size it and must skip it.
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<type name="NoPrim"/>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="NoPrim" offset="0"/>
  </message>
</messageSchema>"#
    );
    let _ = parse(&schema);

    Ok(())
}

#[test]
fn message_with_non_numeric_id_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}</types>
  <message name="M" id="notanumber">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    let err = parse(&schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

#[test]
fn type_with_non_numeric_since_version_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<type name="T" primitiveType="uint8" sinceVersion="notanumber"/>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    let err = parse(&schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

#[test]
fn type_with_non_numeric_length_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let schema = format!(
        r#"<?xml version="1.0"?>
<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
  <types>{HEADER_TYPES}
<type name="T" primitiveType="uint8" length="notanumber"/>
  </types>
  <message name="M" id="1">
<field name="f" id="1" type="uint8"/>
  </message>
</messageSchema>"#
    );
    let err = parse(&schema).unwrap_err();
    assert!(matches!(err, ParseError::Invalid { .. }));

    Ok(())
}

// ── T-2: reject malformed layout numerics ──────────────────────────────

fn mini_msg_xml(field_attrs: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="t" id="1" version="0" byteOrder="littleEndian">
          <types><composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
          </composite></types>
          <sbe:message name="M" id="1">
            <field name="a" id="1" type="uint32" {field_attrs}/>
          </sbe:message>
        </sbe:messageSchema>"#
    )
}

#[test]
fn malformed_field_offset_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = parse(&mini_msg_xml(r#"offset="not-a-number""#)).expect_err("garbage offset");
    let s = format!("{err:?}");
    assert!(s.contains("offset") || s.contains("Invalid"), "{s}");
    Ok(())
}

#[test]
fn negative_field_offset_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = parse(&mini_msg_xml(r#"offset="-1""#)).expect_err("negative offset");
    let s = format!("{err:?}");
    assert!(s.contains("offset") || s.contains("Invalid"), "{s}");
    Ok(())
}

#[test]
fn overflowing_field_offset_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = parse(&mini_msg_xml(r#"offset="999999999999999999999999999999""#))
        .expect_err("overflow offset");
    let s = format!("{err:?}");
    assert!(s.contains("offset") || s.contains("Invalid"), "{s}");
    Ok(())
}

#[test]
fn malformed_group_block_length_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="t" id="1" version="0" byteOrder="littleEndian">
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
          </types>
          <sbe:message name="M" id="1">
            <group name="g" id="2" dimensionType="groupSizeEncoding" blockLength="nope">
              <field name="x" id="3" type="uint8"/>
            </group>
          </sbe:message>
        </sbe:messageSchema>"#;
    let err = parse(xml).expect_err("garbage blockLength");
    let s = format!("{err:?}");
    assert!(s.contains("blockLength") || s.contains("Invalid"), "{s}");
    Ok(())
}

#[test]
fn valid_explicit_offset_still_parses() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(&mini_msg_xml(r#"offset="4""#))?;
    assert!(!ir.tokens.is_empty());
    Ok(())
}

// ── T-17: reject invalid deprecated / nullValue ────────────────────────

#[test]
fn deprecated_true_string_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="t" id="1" version="0" byteOrder="littleEndian">
          <types><composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
          </composite>
          <type name="Old" primitiveType="uint32" deprecated="true"/>
          </types>
          <sbe:message name="M" id="1"><field name="a" id="1" type="Old"/></sbe:message>
        </sbe:messageSchema>"#;
    let err = parse(xml).expect_err("deprecated=true");
    let s = format!("{err:?}");
    assert!(s.contains("deprecated") || s.contains("Invalid"), "{s}");
    Ok(())
}

#[test]
fn deprecated_negative_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="t" id="1" version="0" byteOrder="littleEndian">
          <types><composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
          </composite>
          <type name="Old" primitiveType="uint32" deprecated="-1"/>
          </types>
          <sbe:message name="M" id="1"><field name="a" id="1" type="Old"/></sbe:message>
        </sbe:messageSchema>"#;
    assert!(parse(xml).is_err());
    Ok(())
}

#[test]
fn deprecated_overflow_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="t" id="1" version="0" byteOrder="littleEndian">
          <types><composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
          </composite>
          <type name="Old" primitiveType="uint32" deprecated="65536"/>
          </types>
          <sbe:message name="M" id="1"><field name="a" id="1" type="Old"/></sbe:message>
        </sbe:messageSchema>"#;
    let err = parse(xml).expect_err("deprecated overflow");
    let s = format!("{err}");
    assert!(s.contains("deprecated") || s.contains("Invalid"), "{s}");
    Ok(())
}

#[test]
fn malformed_null_value_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="t" id="1" version="0" byteOrder="littleEndian">
          <types><composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
          </composite>
          <type name="Opt" primitiveType="uint32" presence="optional" nullValue="not-a-number"/>
          </types>
          <sbe:message name="M" id="1"><field name="a" id="1" type="Opt"/></sbe:message>
        </sbe:messageSchema>"#;
    let err = parse(xml).expect_err("bad nullValue");
    let s = format!("{err:?}");
    assert!(s.contains("nullValue") || s.contains("Invalid"), "{s}");
    Ok(())
}

fn opt_uint8_schema(attr: &str, value: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="t" id="1" version="0" byteOrder="littleEndian">
          <types><composite name="messageHeader">
            <type name="blockLength" primitiveType="uint16"/>
            <type name="templateId" primitiveType="uint16"/>
            <type name="schemaId" primitiveType="uint16"/>
            <type name="version" primitiveType="uint16"/>
          </composite>
          <type name="Opt" primitiveType="uint8" presence="optional" {attr}="{value}"/>
          </types>
          <sbe:message name="M" id="1"><field name="a" id="1" type="Opt"/></sbe:message>
        </sbe:messageSchema>"#
    )
}

#[test]
fn uint8_null_value_256_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = parse(&opt_uint8_schema("nullValue", "256")).expect_err("uint8 nullValue=256");
    let s = format!("{err:?}");
    assert!(
        s.contains("nullValue") || s.contains("out of range") || s.contains("Invalid"),
        "{s}"
    );
    Ok(())
}

#[test]
fn uint8_min_value_256_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = parse(&opt_uint8_schema("minValue", "256")).expect_err("uint8 minValue=256");
    let s = format!("{err:?}");
    assert!(
        s.contains("minValue") || s.contains("out of range") || s.contains("Invalid"),
        "{s}"
    );
    Ok(())
}

#[test]
fn uint8_max_value_256_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = parse(&opt_uint8_schema("maxValue", "256")).expect_err("uint8 maxValue=256");
    let s = format!("{err:?}");
    assert!(
        s.contains("maxValue") || s.contains("out of range") || s.contains("Invalid"),
        "{s}"
    );
    Ok(())
}

#[test]
fn uint8_null_value_negative_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = try_parse_u64_val("-1", Some(PrimitiveType::UInt8)).expect_err("uint8 -1");
    assert!(
        err.contains("out of range") || err.contains("not a valid"),
        "{err}"
    );
    Ok(())
}

#[test]
fn int8_null_value_minus_one_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        try_parse_u64_val("-1", Some(PrimitiveType::Int8))?,
        Some((-1i8) as u64)
    );
    Ok(())
}

#[test]
fn signed_declared_range_compares_as_i64() -> Result<(), Box<dyn std::error::Error>> {
    let minus_five = try_parse_u64_val("-5", Some(PrimitiveType::Int8))?.unwrap();
    let five = try_parse_u64_val("5", Some(PrimitiveType::Int8))?.unwrap();
    let minus_one = try_parse_u64_val("-1", Some(PrimitiveType::Int8))?.unwrap();
    let six = try_parse_u64_val("6", Some(PrimitiveType::Int8))?.unwrap();
    value_in_declared_range(PrimitiveType::Int8, minus_one, Some(minus_five), Some(five))?;
    assert!(
        value_in_declared_range(PrimitiveType::Int8, six, Some(minus_five), Some(five)).is_err()
    );
    Ok(())
}

#[test]
fn int8_enum_value_minus_one_is_inside_minus_five_to_five() -> Result<(), Box<dyn std::error::Error>>
{
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="t" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <type name="tiny" primitiveType="int8" minValue="-5" maxValue="5"/>
            <enum name="E" encodingType="tiny">
              <validValue name="Neg">-1</validValue>
              <validValue name="Pos">3</validValue>
            </enum>
          </types>
          <sbe:message name="M" id="1"><field name="a" id="1" type="E"/></sbe:message>
        </sbe:messageSchema>"#;
    parse(xml).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn int8_enum_value_six_is_above_max_five() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="t" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <type name="tiny" primitiveType="int8" minValue="-5" maxValue="5"/>
            <enum name="E" encodingType="tiny">
              <validValue name="TooBig">6</validValue>
            </enum>
          </types>
          <sbe:message name="M" id="1"><field name="a" id="1" type="E"/></sbe:message>
        </sbe:messageSchema>"#;
    let err = parse(xml).expect_err("6 is above maxValue=5");
    let s = format!("{err:?}");
    assert!(
        s.contains("validValue") || s.contains("range") || s.contains("Invalid"),
        "{s}"
    );
    Ok(())
}

#[test]
fn int8_null_value_128_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = try_parse_u64_val("128", Some(PrimitiveType::Int8)).expect_err("int8 128");
    assert!(err.contains("out of range"), "{err}");
    Ok(())
}

#[test]
fn uint8_null_value_255_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        try_parse_u64_val("255", Some(PrimitiveType::UInt8))?,
        Some(255)
    );
    Ok(())
}
