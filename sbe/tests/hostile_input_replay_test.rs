//! Deterministic replay of parser and generated-code hostile-input corpora.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use common::{Paths, compile_and_run, generate};

/// Format a byte slice as a Rust `vec!` literal (e.g. `vec![0u8, 1u8, 255u8]`).
fn bytes_to_vec_literal(bytes: &[u8]) -> String {
    let inner = bytes
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("vec![{inner}]")
}

#[test]
fn every_schema_fixture_and_deterministic_parser_mutation_is_panic_free()
-> Result<(), Box<dyn std::error::Error>> {
    let schema_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/schemas");
    let mut paths = fs::read_dir(schema_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.extension().is_some_and(|extension| extension == "xml"));
    paths.sort();

    for path in paths {
        let bytes = fs::read(&path)?;
        let xml = std::str::from_utf8(&bytes)?;
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = ergo_sbe::parse(xml);
            }))
            .is_ok(),
            "parser panicked for complete fixture: {}",
            path.display()
        );

        let stride = (bytes.len() / 64).max(1);
        for cut in (0..bytes.len()).step_by(stride) {
            let prefix = &bytes[..cut];
            let result = catch_unwind(AssertUnwindSafe(|| {
                if let Ok(text) = std::str::from_utf8(prefix) {
                    let _ = ergo_sbe::parse(text);
                }
            }));
            assert!(
                result.is_ok(),
                "parser panicked for {} truncated at {cut}",
                path.display()
            );
        }
    }
    Ok(())
}

#[test]
fn generated_verify_dispatch_cursor_and_nested_decode_corpus_is_panic_free()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, source) = generate(&Paths::l3_orderbook_schema(), "hostile_replay");
    let fixture_vectors = [Paths::baseline_binary(), Paths::extension_binary()]
        .into_iter()
        .map(fs::read)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|bytes| bytes_to_vec_literal(&bytes))
        .collect::<Vec<_>>()
        .join(",");
    let body = format!(
        r#"
        let mut corpus = vec![
            vec![],
            vec![0],
            vec![0xff; 7],
            vec![0xff; 8],
            vec![0xff; 64],
            vec![0; 128],
            {fixture_vectors}
        ];
        for len in 0usize..=128 {{
            corpus.push((0..len).map(|index| (index as u8).wrapping_mul(37)).collect());
        }}
        for block_length in [0u16, 1, u16::MAX] {{
            for count in [0u16, 1, u16::MAX] {{
                let mut frame = vec![0u8; 32];
                frame[0..2].copy_from_slice(&block_length.to_le_bytes());
                frame[2..4].copy_from_slice(&L3BookDecoder::TEMPLATE_ID.to_le_bytes());
                frame[4..6].copy_from_slice(&L3BookDecoder::SCHEMA_ID.to_le_bytes());
                frame[6..8].copy_from_slice(&L3BookDecoder::SCHEMA_VERSION.to_le_bytes());
                frame[24..26].copy_from_slice(&block_length.to_le_bytes());
                frame[26..28].copy_from_slice(&count.to_le_bytes());
                corpus.push(frame);
            }}
        }}

        for (case, bytes) in corpus.iter().enumerate() {{
            let result = std::panic::catch_unwind(|| {{
                let verified = L3BookDecoder::verify(bytes);
                let _ = AnyMessage::decode(bytes, 0);
                let _ = AnyMessage::decode_frame(bytes, 0, bytes.len());
                for policy in [
                    FramingPolicy::LengthPrefixU16,
                    FramingPolicy::LengthPrefixU32,
                    FramingPolicy::Fixed(bytes.len()),
                ] {{
                    let mut cursor = FrameCursor::new(bytes, policy);
                    for _ in 0..256 {{
                        match cursor.next() {{
                            Some(Ok(_)) => {{}}
                            Some(Err(_)) | None => break,
                        }}
                    }}
                }}

                if verified.is_ok() {{
                    let message = L3BookDecoder::try_from(bytes.as_slice()).unwrap();
                    let mut bids = message.into_bids().unwrap();
                    while let Some(Ok(level)) = bids.next() {{
                        let mut orders = level.into_orders().unwrap();
                        while let Some(Ok(order)) = orders.next() {{
                            let _ = order.into_order_id();
                        }}
                        let _ = orders.finish();
                    }}
                    if let Ok(after_bids) = bids.finish() {{
                        if let Ok(mut asks) = after_bids.into_asks() {{
                            while let Some(Ok(level)) = asks.next() {{
                                let mut orders = level.into_orders().unwrap();
                                while let Some(Ok(order)) = orders.next() {{
                                    let _ = order.into_order_id();
                                }}
                                let _ = orders.finish();
                            }}
                            let _ = asks.finish();
                        }}
                    }}
                }}
            }});
            assert!(result.is_ok(), "hostile corpus case {{case}} panicked");
        }}
        "#
    );
    compile_and_run("hostile_replay", &source, &body);
    Ok(())
}
