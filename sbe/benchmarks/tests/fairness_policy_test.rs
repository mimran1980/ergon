//! Mechanical guardrails for maintained head-to-head benchmark sources.

const PERF_PARITY: &str = include_str!("../benches/perf_parity_bench.rs");
const GROUP_ENCODE: &str = include_str!("../benches/group_encode_bench.rs");
const GROUP_DECIMAL: &str = include_str!("../benches/group_encode_decimal_bench.rs");
const CLUSTER_CODEC: &str = include_str!("../../../cluster/benches/cluster_codec_bench.rs");
const README: &str = include_str!("../README.md");

const MAINTAINED: &[(&str, &str)] = &[
    ("perf_parity_bench.rs", PERF_PARITY),
    ("group_encode_bench.rs", GROUP_ENCODE),
    ("group_encode_decimal_bench.rs", GROUP_DECIMAL),
    ("cluster_codec_bench.rs", CLUSTER_CODEC),
];

fn function_source<'a>(source: &'a str, name: &str) -> &'a str {
    let signature = format!("fn {name}(");
    let start = source
        .find(&signature)
        .unwrap_or_else(|| panic!("missing benchmark function {name}"));
    let rest = &source[start..];
    let end = rest[signature.len()..]
        .find("\nfn ")
        .map_or(rest.len(), |offset| signature.len() + offset);
    &rest[..end]
}

#[test]
fn maintained_benches_use_std_black_box() {
    for (name, source) in MAINTAINED {
        assert!(
            source.contains("use std::hint::black_box;"),
            "{name} must use std::hint::black_box"
        );
        assert!(
            !source.contains("criterion::{Criterion, Throughput, black_box")
                && !source.contains("criterion::black_box"),
            "{name} must not use Criterion's fallback black_box"
        );
    }
}

#[test]
fn maintained_bench_sources_have_a_correctness_preflight() -> Result<(), Box<dyn std::error::Error>>
{
    for (name, source) in MAINTAINED {
        let Some(first_timed_case) = source.find(".bench_") else {
            return Err(std::io::Error::other(format!("{name} has no Criterion benchmark")).into());
        };
        let setup = &source[..first_timed_case];
        assert!(
            setup.contains("assert_eq!") || setup.contains("assert_wire_parity"),
            "{name} must assert exact correctness before its first timed case"
        );
    }
    Ok(())
}

#[test]
fn full_message_wire_parity_is_checked_before_criterion_runs() {
    let source = function_source(PERF_PARITY, "bench_wire_parity_encode_full_message");
    let preflight = source
        .find("assert_full_message_encode_wire_parity();")
        .expect("full-message benchmark must call its wire preflight");
    let timing = source
        .find(".bench_function")
        .expect("full-message benchmark has no Criterion case");
    assert!(
        preflight < timing,
        "full-message byte parity must be established before timing"
    );
}

#[test]
fn cluster_connect_encode_writes_equal_fixed_fields_and_observes_both_lengths() {
    let source = function_source(CLUSTER_CODEC, "bench_encode_connect_request_ergo");
    for (ergo_field, tool_setter) in [
        ("correlation_id: 0", ".correlation_id(0)"),
        ("response_stream_id: 102", ".response_stream_id(102)"),
        ("version: Some(0)", ".version(0)"),
    ] {
        assert_eq!(
            source.matches(ergo_field).count(),
            1,
            "Cluster connect Ergo fixed block must write {ergo_field} once"
        );
        assert_eq!(
            source.matches(tool_setter).count(),
            1,
            "Cluster connect sbe-tool arm must call {tool_setter} once"
        );
    }
    assert!(
        source.contains(".fixed(black_box(&fixed))"),
        "Cluster connect must use the required chainable fixed stage and obscure its input"
    );
    assert_eq!(
        source.matches("black_box(len);").count(),
        2,
        "Cluster connect benchmark must observe both encoded lengths"
    );
}

#[test]
fn benchmark_documentation_keeps_the_sceptical_disclaimer_and_lto_result() {
    let normalized = README
        .replace('>', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for phrase in [
        "notoriously difficult and easy to get wrong",
        "more likely to expose a benchmark mistake",
        "sbe-tool performed well with and without LTO",
        "pre-fix ergon performed well with LTO but became slower than sbe-tool without LTO",
    ] {
        assert!(
            normalized.contains(phrase),
            "benchmark README lost required disclosure: {phrase:?}"
        );
    }
}
