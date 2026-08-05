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

fn function_source<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let signature = format!("fn {name}(");
    let start = source.find(&signature)?;
    let rest = &source[start..];
    let end = rest[signature.len()..]
        .find("\nfn ")
        .map_or(rest.len(), |offset| signature.len() + offset);
    Some(&rest[..end])
}

fn get_source(source: &'static str, fn_name: &str) -> Result<&'static str, String> {
    let fn_name_owned = fn_name.to_string();
    function_source(source, fn_name)
        .ok_or_else(|| format!("missing benchmark function {fn_name_owned}"))
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
fn composite_decode_streams_equal_fields_from_equal_message_offsets()
-> Result<(), Box<dyn std::error::Error>> {
    let source = get_source(PERF_PARITY, "bench_decode_composite")?;
    let ergo = timed_arm_body(source, "ergo-sbe_engine").ok_or("missing Ergo composite arm")?;
    let tool = timed_arm_body(source, "sbe-tool_engine").ok_or("missing sbe-tool composite arm")?;

    assert!(
        source.contains("replicate_baseline(MICRO_BATCH_SIZE)"),
        "composite decode must traverse a prebuilt contiguous message stream"
    );
    assert!(
        ergo.contains("CarDecoder::wrap(buf, off, bl_e, ver_e)"),
        "Ergo composite decode must wrap each message at its absolute message_offset"
    );
    assert!(
        tool.contains("sbe_tool_car_body_decoder(buf, off, bl, ver)"),
        "sbe-tool composite decode must wrap the same message at its equivalent message_offset"
    );

    for (label, arm) in [("Ergo", ergo), ("sbe-tool", tool)] {
        assert_eq!(
            arm.matches(".capacity()").count(),
            1,
            "{label} composite arm must read capacity exactly once per message"
        );
        assert_eq!(
            arm.matches(".num_cylinders()").count(),
            1,
            "{label} composite arm must read num_cylinders exactly once per message"
        );
        assert!(
            arm.contains("off += msg_len;"),
            "{label} composite arm must advance by the same framed-message length"
        );
        assert!(
            arm.contains("black_box((total_capacity, total_cylinders))"),
            "{label} composite arm must observe the same two-field checksum"
        );
    }

    Ok(())
}

#[test]
fn full_message_wire_parity_is_checked_before_criterion_runs()
-> Result<(), Box<dyn std::error::Error>> {
    let source = get_source(PERF_PARITY, "bench_wire_parity_encode_full_message")?;
    let preflight = source
        .find("assert_full_message_encode_wire_parity();")
        .ok_or("full-message benchmark must call its wire preflight")?;
    let timing = source
        .find(".bench_function")
        .ok_or("full-message benchmark has no Criterion case")?;
    assert!(
        preflight < timing,
        "full-message byte parity must be established before timing"
    );
    Ok(())
}

#[test]
fn cluster_connect_encode_writes_equal_fixed_fields_and_observes_both_lengths()
-> Result<(), Box<dyn std::error::Error>> {
    let source = get_source(CLUSTER_CODEC, "bench_encode_connect_request_ergo")?;
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
    Ok(())
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
        // Header equal-work rule must stay explicit — mixed arms are the
        // classic fairness bug (one codec writes MessageHeader, the other skips).
        "both write it, or both skip it",
        "never mix",
    ] {
        assert!(
            normalized.contains(phrase),
            "benchmark README lost required disclosure: {phrase:?}"
        );
    }
}

/// Timed arm body from a Criterion `bench_function("label", …)` call.
fn timed_arm_body<'a>(fn_source: &'a str, label: &str) -> Option<&'a str> {
    let needle = format!("\"{label}\"");
    let start = fn_source.find(&needle)?;
    let rest = &fn_source[start..];
    // Next sibling arm or group.finish ends this arm.
    let mut end = rest.len();
    for marker in [
        "\n    g.bench_function",
        "\n    group.bench_function",
        "\n        group.bench_with_input",
        "\n    group.bench_with_input",
        "\n    g.finish",
        "\n    group.finish",
    ] {
        if let Some(i) = rest[needle.len()..].find(marker) {
            end = end.min(needle.len() + i);
        }
    }
    Some(&rest[..end])
}

fn strip_line_comments(src: &str) -> String {
    src.lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn arm_writes_message_header(arm: &str) -> bool {
    // Ignore // comments — body-only arms may mention header(0) in notes.
    let code = strip_line_comments(arm);
    // wrap_and_apply_header / wrap_and_apply_header_unchecked write the
    // MessageHeader; bare wrap / wrap_unchecked do not.
    code.contains("wrap_and_apply_header") || code.contains(".header(")
}

fn arm_is_body_only_encode(arm: &str) -> bool {
    // Body-only: wraps without applying/writing the MessageHeader.
    // Prefer wrap_unchecked when matching sbe-tool's zero-check wrap (equal work).
    let code = strip_line_comments(arm);
    let has_wrap = code.contains("wrap_unchecked(")
        || code.contains("::wrap(")
        || code.contains(".wrap(")
        || code.contains("Encoder::wrap(");
    // Exclude wrap_and_apply_header* false positives: those contain "wrap(" after
    // stripping only if we match too loosely — header writer check covers them.
    has_wrap && !arm_writes_message_header(arm)
}

/// Maintained encode pairs must not mix "writes `MessageHeader`" with "body only".
#[test]
#[allow(clippy::too_many_lines)] // reasonable for a header-mode inventory table
fn encode_parity_arms_do_not_mix_header_writes() -> Result<(), Box<dyn std::error::Error>> {
    // (source, function, ergo_label, tool_label, expected_mode)
    // expected_mode: "header" = both write MessageHeader; "body" = neither does.
    let pairs: &[(&str, &str, &str, &str, &str)] = &[
        (
            PERF_PARITY,
            "bench_encode_scalar",
            "ergo-sbe_header_and_body",
            "sbe-tool_header_and_body",
            "header",
        ),
        (
            PERF_PARITY,
            "bench_encode_scalar",
            "ergo-sbe_header_only",
            "sbe-tool_header_only",
            "header",
        ),
        (
            PERF_PARITY,
            "bench_encode_scalar",
            "ergo-sbe_body_only",
            "sbe-tool_body_only",
            "body",
        ),
        (
            PERF_PARITY,
            "bench_encode_throughput",
            "ergo-sbe",
            "sbe-tool",
            "header",
        ),
        (
            PERF_PARITY,
            "bench_wire_parity_encode_full_message",
            "ergo-sbe",
            "sbe-tool",
            "header",
        ),
        // Cluster encode gates are body-only: match sbe-tool wrap(…, 8) without
        // .header(0). Ergon uses wrap, not wrap_and_apply_header.
        (
            CLUSTER_CODEC,
            "bench_encode_msg_header_ergo",
            "ergo-sbe",
            "sbe-tool",
            "body",
        ),
        (
            CLUSTER_CODEC,
            "bench_encode_keep_alive_ergo",
            "ergo-sbe",
            "sbe-tool",
            "body",
        ),
        (
            CLUSTER_CODEC,
            "bench_encode_connect_request_ergo",
            "ergo-sbe",
            "sbe-tool",
            "body",
        ),
        (
            CLUSTER_CODEC,
            "bench_claim_shaped_write",
            "ergo-sbe",
            "sbe-tool",
            "body",
        ),
    ];

    for (source, fn_name, ergo_label, tool_label, mode) in pairs {
        let fn_src =
            function_source(source, fn_name).ok_or_else(|| format!("{fn_name}: not found"))?;
        let ergo = timed_arm_body(fn_src, ergo_label)
            .ok_or_else(|| format!("{fn_name}/{ergo_label}: timed arm not found"))?;
        let tool = timed_arm_body(fn_src, tool_label)
            .ok_or_else(|| format!("{fn_name}/{tool_label}: timed arm not found"))?;
        let ergo_hdr = arm_writes_message_header(ergo);
        let tool_hdr = arm_writes_message_header(tool);
        match *mode {
            "header" => {
                assert!(
                    ergo_hdr,
                    "{fn_name}/{ergo_label} must write MessageHeader \
                     (wrap_and_apply_header or equivalent)"
                );
                assert!(
                    tool_hdr,
                    "{fn_name}/{tool_label} must write MessageHeader via .header(…) — \
                     wrap(…, 8) alone is body-only and mixes work with ergon's \
                     wrap_and_apply_header"
                );
            }
            "body" => {
                assert!(
                    !ergo_hdr && arm_is_body_only_encode(ergo),
                    "{fn_name}/{ergo_label} must be body-only (wrap without header write)"
                );
                assert!(
                    !tool_hdr,
                    "{fn_name}/{tool_label} must not call .header(…) in a body-only pair"
                );
            }
            other => return Err(format!("unknown mode {other}").into()),
        }
        // Hard rule: never one-sided header work inside a gated pair.
        assert_eq!(
            ergo_hdr, tool_hdr,
            "{fn_name}: mixed header work — {ergo_label} header={ergo_hdr}, \
             {tool_label} header={tool_hdr}. Both arms must write the MessageHeader \
             or both must skip it."
        );
    }
    Ok(())
}

/// Group encode sbe-tool arm must apply the header like ergon `add_closure`.
#[test]
fn group_encode_sbe_tool_arm_writes_header_like_ergon() -> Result<(), Box<dyn std::error::Error>> {
    let src = function_source(GROUP_ENCODE, "bench_group_encode")
        .ok_or("bench_group_encode not found")?;
    // ergon add_closure path
    assert!(
        src.contains("wrap_and_apply_header"),
        "group encode ergon arms must use wrap_and_apply_header"
    );
    // sbe-tool arm must not be body-only
    let tool = timed_arm_body(src, "sbe-tool").ok_or("sbe-tool arm not found")?;
    assert!(
        arm_writes_message_header(tool),
        "group encode sbe-tool arm must call .header(…) so it matches ergon \
         wrap_and_apply_header; body-only wrap would under-work the reference"
    );
    // Do not invent frame length as `8 + encoded_length()` after a real header write —
    // prefer get_limit() (absolute end after wrap@8).
    let tool_code = strip_line_comments(tool);
    assert!(
        !tool_code.contains("encoded_length() + 8")
            && !tool_code.contains("8 + enc.encoded_length()"),
        "group encode sbe-tool arm must not invent header length as 8 + encoded_length()"
    );
    assert!(
        tool_code.contains("get_limit()"),
        "group encode sbe-tool arm should use get_limit() for full-wire length"
    );
    Ok(())
}

/// Gated diagnostic benches must not claim `sbe-tool` ratios with mixed work.
#[test]
fn diagnostic_benches_are_not_mixed_sbe_tool_ratios() {
    // These are ergon-only or DTO-vs-DTO; if they gain a sbe-tool arm later,
    // it must go through the encode_parity_arms_do_not_mix_header_writes table.
    let (name, source) = ("group_encode_decimal_bench.rs", GROUP_DECIMAL);
    let has_tool_bench = source.contains("bench_function(\"sbe-tool\"")
        || source.contains("BenchmarkId::new(\"sbe-tool\"");
    assert!(
        !has_tool_bench,
        "{name}: diagnostic suite gained an sbe-tool arm — register it in \
         encode_parity_arms_do_not_mix_header_writes with an explicit mode"
    );
}
