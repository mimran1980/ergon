//! Compile/run coverage for the generated zero-allocation metadata registry.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, compile_and_run, generate};

#[test]
fn nested_descriptors_are_complete_and_share_message_identity_constants()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "metadata_nested");
    compile_and_run(
        "metadata_nested",
        &src,
        r#"
        assert_eq!(MESSAGE_DESCRIPTORS.len(), 1);
        let message = message_descriptor(L3BookDecoder::SCHEMA_ID, L3BookDecoder::TEMPLATE_ID)
            .expect("known message descriptor");
        assert_eq!(message.schema_id, L3BookEncoder::SCHEMA_ID);
        assert_eq!(message.template_id, L3BookEncoder::TEMPLATE_ID);
        assert_eq!(message.name, "L3Book");
        assert_eq!(message.block_length, L3BookDecoder::BLOCK_LENGTH);
        assert_eq!(message.description, Some("L3 orderbook snapshot"));
        assert_eq!(message.fields.len(), 2);
        assert_eq!(message.fields[0].name, "timestamp");
        assert_eq!(message.fields[0].id, 1);
        assert_eq!(message.fields[0].offset, 0);
        assert_eq!(message.fields[0].encoded_length, 8);
        assert_eq!(message.fields[0].field_type, "u64");
        assert_eq!(message.fields[0].presence, "required");

        let bids = &message.groups[0];
        assert_eq!(bids.name, "bids");
        assert_eq!(bids.id, 10);
        assert_eq!(bids.dimension_type, "groupSize");
        assert_eq!(bids.block_length, 16);
        assert_eq!(bids.fields[1].name, "qty");
        assert_eq!(bids.groups.len(), 1);

        let orders = &bids.groups[0];
        assert_eq!(orders.name, "orders");
        assert_eq!(orders.block_length, 8);
        assert_eq!(orders.var_data.len(), 1);
        assert_eq!(orders.var_data[0].name, "orderId");
        assert_eq!(orders.var_data[0].id, 15);
        assert_eq!(orders.var_data[0].length_type, "u32");
        assert_eq!(orders.var_data[0].character_encoding, Some("UTF-8"));
        assert_eq!(orders.var_data[0].maximum, Some(u32::MAX as usize - 1));

        assert!(message_descriptor(L3BookDecoder::SCHEMA_ID, u16::MAX).is_none());
        assert!(message_descriptor(u16::MAX, L3BookDecoder::TEMPLATE_ID).is_none());
    "#,
    );
    Ok(())
}

#[test]
fn versioned_members_are_reflected_in_static_descriptors() -> Result<(), Box<dyn std::error::Error>>
{
    let (_schema, src) = generate(&Paths::versioned_domain_schema(), "metadata_versioned");
    compile_and_run(
        "metadata_versioned",
        &src,
        r#"
        let message = message_descriptor(VersionedDecoder::SCHEMA_ID, VersionedDecoder::TEMPLATE_ID)
            .expect("known versioned message");
        assert_eq!(message.fields.len(), 3);
        assert_eq!(message.fields[0].name, "active");
        assert_eq!(message.fields[0].since_version, 1);
        assert_eq!(message.fields[1].name, "extra");
        assert_eq!(message.fields[1].since_version, 2);
        assert_eq!(message.fields[2].name, "count");
        assert_eq!(message.fields[2].since_version, 0);
    "#,
    );
    Ok(())
}

#[test]
fn duplicate_template_ids_in_different_schemas_are_schema_qualified()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema_a, source_a) = generate(&Paths::l3_orderbook_schema(), "schema_a");
    let (_schema_b, source_b) = generate(&Paths::versioned_domain_schema(), "schema_b");
    let source =
        format!("pub mod schema_a {{\n{source_a}\n}}\npub mod schema_b {{\n{source_b}\n}}\n");
    compile_and_run(
        "metadata_duplicate_ids",
        &source,
        r#"
        let a = schema_a::message_descriptor(42, 1).expect("schema 42/template 1");
        let b = schema_b::message_descriptor(201, 1).expect("schema 201/template 1");
        assert_eq!(a.name, "L3Book");
        assert_eq!(b.name, "Versioned");
        assert!(schema_a::message_descriptor(201, 1).is_none());
        assert!(schema_b::message_descriptor(42, 1).is_none());
    "#,
    );
    Ok(())
}
