use crate::ir::{ByteOrder, Presence, PrimitiveType};
use crate::structured_ir::{MemberType, SchemaElements, parse_composite_members};

/// Pre-compute the exact schema-declared message-header wire image. Composite
/// offsets may introduce padding and blockLength may use another unsigned
/// primitive width; every multi-octet member follows schema byteOrder.
pub(crate) fn message_header_template(
    elements: &SchemaElements,
    header_type: &str,
    header_size: usize,
    byte_order: ByteOrder,
    block_length: usize,
    template_id: u16,
    schema_id: u16,
    schema_version: u16,
) -> Vec<u8> {
    let header = elements
        .composites
        .iter()
        .find(|composite| composite[0].name == header_type)
        .unwrap_or_else(|| panic!("resolved message header composite '{header_type}' is missing"));
    let members = parse_composite_members(header);
    let mut bytes = vec![0u8; header_size];

    for (field_name, value) in [
        ("blockLength", block_length as u64),
        ("templateId", u64::from(template_id)),
        ("schemaId", u64::from(schema_id)),
        ("version", u64::from(schema_version)),
    ] {
        let member = members
            .iter()
            .find(|member| member.name == field_name)
            .unwrap_or_else(|| panic!("message header is missing required field '{field_name}'"));
        let MemberType::Primitive {
            prim,
            length,
            presence,
            ..
        } = member.member_type
        else {
            panic!("message header field '{field_name}' is not a primitive integer");
        };
        assert_eq!(
            length.unwrap_or(1),
            1,
            "message header field '{field_name}' must be scalar"
        );
        if presence == Presence::Constant {
            continue;
        }
        assert_eq!(
            presence,
            Presence::Required,
            "message header field '{field_name}' must be required or constant"
        );

        let offset = member.offset;
        match prim {
            PrimitiveType::UInt8 => {
                bytes[offset] = u8::try_from(value).unwrap_or_else(|_| {
                    panic!("message header field '{field_name}' value {value} exceeds uint8")
                });
            }
            PrimitiveType::UInt16 => {
                let encoded = u16::try_from(value).unwrap_or_else(|_| {
                    panic!("message header field '{field_name}' value {value} exceeds uint16")
                });
                let encoded = match byte_order {
                    ByteOrder::LittleEndian => encoded.to_le_bytes(),
                    ByteOrder::BigEndian => encoded.to_be_bytes(),
                };
                bytes[offset..offset + 2].copy_from_slice(&encoded);
            }
            PrimitiveType::UInt32 => {
                let encoded = u32::try_from(value).unwrap_or_else(|_| {
                    panic!("message header field '{field_name}' value {value} exceeds uint32")
                });
                let encoded = match byte_order {
                    ByteOrder::LittleEndian => encoded.to_le_bytes(),
                    ByteOrder::BigEndian => encoded.to_be_bytes(),
                };
                bytes[offset..offset + 4].copy_from_slice(&encoded);
            }
            PrimitiveType::UInt64 => {
                let encoded = match byte_order {
                    ByteOrder::LittleEndian => value.to_le_bytes(),
                    ByteOrder::BigEndian => value.to_be_bytes(),
                };
                bytes[offset..offset + 8].copy_from_slice(&encoded);
            }
            _ => panic!("message header field '{field_name}' must be an unsigned integer"),
        }
    }

    bytes
}
