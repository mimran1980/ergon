//! Mechanical guardrails for maintained head-to-head benchmark sources.

const PERF_PARITY: &str = include_str!("../benches/perf_parity_bench.rs");
const PERF_PARITY_EXTENDED: &str = include_str!("../benches/perf_parity_extended_bench.rs");
const GROUP_ENCODE: &str = include_str!("../benches/group_encode_bench.rs");
const GROUP_DECIMAL: &str = include_str!("../benches/group_encode_decimal_bench.rs");
const CLUSTER_CODEC: &str = include_str!("../../../cluster/benches/cluster_codec_bench.rs");
const README: &str = include_str!("../README.md");

const MAINTAINED: &[(&str, &str)] = &[
    ("perf_parity_bench.rs", PERF_PARITY),
    ("perf_parity_extended_bench.rs", PERF_PARITY_EXTENDED),
    ("group_encode_bench.rs", GROUP_ENCODE),
    ("group_encode_decimal_bench.rs", GROUP_DECIMAL),
    ("cluster_codec_bench.rs", CLUSTER_CODEC),
];

fn function_source<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let signature = format!("fn {name}(");
    let start = source.find(&signature)?;
    let rest = &source[start..];
    let tail = &rest[signature.len()..];
    let mut end = rest.len();
    for marker in ["\nfn ", "\npub fn ", "\npub(crate) fn ", "\npub unsafe fn "] {
        if let Some(offset) = tail.find(marker) {
            end = end.min(signature.len() + offset);
        }
    }
    Some(&rest[..end])
}

/// A gated function's own body must assert. An assert earlier in the file,
/// before the first timed case, does not count.
fn gated_function_preflight(source: &str, name: &str) -> Result<(), String> {
    let body = function_source(source, name).ok_or_else(|| format!("{name} missing"))?;
    if !body.contains("assert_eq!") {
        return Err(format!(
            "{name} has no assert_eq! inside the function; a file-level assert is not enough"
        ));
    }
    if !body.contains("BATCH_SIZE") {
        return Err(format!("{name} does not tie its preflight to BATCH_SIZE"));
    }
    Ok(())
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
fn gated_throughput_functions_assert_inside_the_function() -> Result<(), Box<dyn std::error::Error>>
{
    for name in ["bench_encode_throughput", "bench_throughput_batch"] {
        gated_function_preflight(PERF_PARITY, name)?;
    }
    // A file-level assert does not cover a gated function that has none.
    let file_level_only = "\nfn setup() {\n    assert_eq!(1, 1);\n}\nfn bench_encode_throughput() {\n    let _ = BATCH_SIZE;\n}\n";
    assert!(
        gated_function_preflight(file_level_only, "bench_encode_throughput").is_err(),
        "a file-level assert must not satisfy the per-function check"
    );
    let real =
        function_source(PERF_PARITY, "bench_encode_throughput").ok_or("bench_encode_throughput")?;
    let stripped = format!(
        "\nfn setup() {{ assert_eq!(1, 1); }}\n{}\nfn after() {{}}\n",
        real.replace("assert_eq!", "let _kept = ")
    );
    assert!(
        gated_function_preflight(&stripped, "bench_encode_throughput").is_err(),
        "removing the assert from bench_encode_throughput must fail the policy"
    );
    Ok(())
}

#[test]
fn throughput_preflight_matches_bytes_and_batch_totals() {
    use ergo_sbe_benchmarks::{
        THROUGHPUT_SLOT, sample_decode_throughput, sample_encode_throughput,
    };

    let encoded = sample_encode_throughput(10_000);
    assert_eq!(
        encoded.one_year,
        u64::from(ergo_sbe_benchmarks::THROUGHPUT_YEAR)
    );
    assert_eq!(encoded.one_ergo, encoded.one_tool);
    assert_eq!(encoded.batch_ergo, encoded.batch_tool);
    assert_eq!(
        &encoded.batch_ergo[..THROUGHPUT_SLOT],
        &encoded.one_ergo[..]
    );
    assert_eq!(encoded.year_total, 10_000 * encoded.one_year);
    assert_eq!(encoded.batch_ergo.len(), 10_000 * THROUGHPUT_SLOT);

    let baseline = include_bytes!("../benches/fixtures/car_example_baseline_data.sbe");
    let msg_len = baseline.len();
    let mut buf = vec![0u8; 10_000 * msg_len];
    for chunk in buf.chunks_mut(msg_len) {
        chunk.copy_from_slice(baseline);
    }
    let block_length = u16::from_le_bytes([baseline[0], baseline[1]]);
    let version = u16::from_le_bytes([baseline[6], baseline[7]]);
    let decoded = sample_decode_throughput(
        &buf,
        msg_len,
        10_000,
        usize::from(block_length),
        version,
        block_length,
        version,
    );
    assert_eq!(&buf[..msg_len], &baseline[..]);
    assert_eq!(decoded.ergo_serial, decoded.tool_serial);
    assert_eq!(decoded.ergo_year, decoded.tool_year);
    assert_eq!(decoded.ergo_serial, 10_000 * decoded.one_serial);
    assert_eq!(decoded.ergo_year, 10_000 * decoded.one_year);
}

#[test]
#[should_panic(expected = "a zero-length message would never advance")]
fn decode_preflight_rejects_zero_stride() {
    let baseline = include_bytes!("../benches/fixtures/car_example_baseline_data.sbe");
    let block_length = u16::from_le_bytes([baseline[0], baseline[1]]);
    let version = u16::from_le_bytes([baseline[6], baseline[7]]);
    // The backing frame is complete even before the fix, so this negative
    // test never attempts an out-of-bounds read on the unsound implementation.
    ergo_sbe_benchmarks::sample_decode_throughput(
        baseline,
        0,
        1,
        usize::from(block_length),
        version,
        block_length,
        version,
    );
}

#[test]
fn decode_preflight_rejects_missing_or_truncated_frames() {
    use ergo_sbe_benchmarks::sample_decode_throughput;

    let baseline = include_bytes!("../benches/fixtures/car_example_baseline_data.sbe");
    let block_length = u16::from_le_bytes([baseline[0], baseline[1]]);
    let version = u16::from_le_bytes([baseline[6], baseline[7]]);
    for (buf, stride, count) in [
        (&[][..], baseline.len(), 0),
        (&baseline[..8], baseline.len(), 1),
        (&baseline[..], baseline.len(), 2),
        (&baseline[..], usize::MAX, 2),
    ] {
        assert!(
            std::panic::catch_unwind(|| {
                sample_decode_throughput(
                    buf,
                    stride,
                    count,
                    usize::from(block_length),
                    version,
                    block_length,
                    version,
                )
            })
            .is_err()
        );
    }
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
    // Equal work, per the documented validation class. sbe-tool's `wrap` only
    // stores buffer/offset/block-length/version; ergon's bare `wrap` proves the
    // version-aware fixed extent on every message. Timing the validating
    // constructor against the unvalidating one charges ergon for a bounds proof
    // its reference never performs, so the timed region must use the unchecked
    // constructor — with the extent proven once, outside the measurement.
    assert!(
        source.contains("assert_stream_wrap_extent(&buf, msg_len, MICRO_BATCH_SIZE, bl_e, ver_e)"),
        "composite decode must prove the stream extent in an untimed preflight"
    );
    assert!(
        ergo.contains("CarDecoder::wrap_unchecked(buf, off, bl_e, ver_e)"),
        "Ergo composite decode must wrap each message at its absolute message_offset \
         using the unchecked constructor that matches sbe-tool's zero-check wrap"
    );
    assert!(
        !ergo.contains("CarDecoder::wrap(buf"),
        "a validating wrap in the timed region would not be equal work"
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
fn decode_scalar_clobber_is_symmetric_across_arms() -> Result<(), Box<dyn std::error::Error>> {
    // decode_scalar opaques the decoder *inside* the micro-batch loop (a
    // per-iteration memory clobber) rather than once outside it, so the
    // getter isn't drowned under pointer store/reload traffic. That shape
    // only stays fair if both arms — and the throwaway `warmup` arm that
    // absorbs the first-arm position penalty — pay the clobber the same
    // number of times per iteration.
    let source = get_source(PERF_PARITY, "bench_decode_scalar")?;
    let warmup = timed_arm_body(source, "warmup").ok_or("missing warmup arm")?;
    let ergo = timed_arm_body(source, "ergo-sbe").ok_or("missing Ergo scalar arm")?;
    let tool = timed_arm_body(source, "sbe-tool").ok_or("missing sbe-tool scalar arm")?;

    for (label, arm) in [("warmup", warmup), ("Ergo", ergo), ("sbe-tool", tool)] {
        assert_eq!(
            arm.matches("clobber_memory();").count(),
            1,
            "{label} decode_scalar arm must call clobber_memory() exactly once per loop body"
        );
    }

    Ok(())
}

#[test]
fn full_message_decode_arms_use_unchecked_wrap_and_include_ordered_path()
-> Result<(), Box<dyn std::error::Error>> {
    let source = get_source(PERF_PARITY, "bench_decode_consuming_full")?;
    assert!(
        source.contains("assert_decode_parity()")
            && source.contains("assert_ordered_decode_parity()"),
        "full-message decode must prove iterator and fused-visit value parity before timing"
    );
    for arm in ["ergo-sbe_random", "ergo-sbe_consuming", "ergo-sbe_ordered"] {
        let body = timed_arm_body(source, arm).ok_or_else(|| format!("missing {arm} arm"))?;
        assert!(
            body.contains("CarDecoder::wrap_unchecked"),
            "{arm} must use wrap_unchecked inside the timed region"
        );
        assert!(
            !body.contains("CarDecoder::wrap("),
            "{arm} must not pay a validating wrap in the timed region"
        );
    }
    let ordered = timed_arm_body(source, "ergo-sbe_ordered").ok_or("missing ordered arm")?;
    assert!(
        ordered.contains(".ordered()")
            && ordered.contains(".fuel_figures(")
            && ordered.contains("entry.ordered()")
            && ordered.contains(".usage_description(")
            && ordered.contains(".acceleration(")
            && !strip_line_comments(ordered).contains(".into_")
            && !ordered.contains("info.index"),
        "the ordered arm must walk recursively with ordered callbacks at every \
         tail — not staged into_* at any tail, and not extra index reads"
    );
    let consuming = timed_arm_body(source, "ergo-sbe_consuming").ok_or("missing consuming arm")?;
    assert!(
        consuming.contains("into_fuel_figures") && !consuming.contains(".ordered()"),
        "the consuming arm must exercise the staged lane"
    );
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

const BENCH_LIB: &str = include_str!("../src/lib.rs");

fn helper_names(arm: &str) -> Vec<String> {
    let code = strip_line_comments(arm);
    let bytes = code.as_bytes();
    let mut names = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'(' {
                let name = &code[start..i];
                if name.starts_with("throughput_") || name.starts_with("encode_scalar_bodies_") {
                    names.push(name.to_string());
                }
            }
        } else {
            i += 1;
        }
    }
    names
}

/// Inspect direct encoding work and the known codec helpers an arm calls.
/// Missing helpers must fail rather than silently evade the header policy.
fn encoding_work(source: &str, arm: &str) -> Result<String, String> {
    let mut work = arm.to_string();
    for name in helper_names(arm) {
        let helper_source = if name.starts_with("throughput_") {
            BENCH_LIB
        } else {
            source
        };
        let helper = function_source(helper_source, &name)
            .ok_or_else(|| format!("encoding helper {name} missing"))?;
        work.push_str(helper);
    }
    Ok(work)
}

#[test]
fn scalar_helper_header_work_cannot_evade_policy() -> Result<(), Box<dyn std::error::Error>> {
    let arm = "unsafe { encode_scalar_bodies_ergo(frames, serial, year) };";
    let work = encoding_work(PERF_PARITY, arm)?;
    assert!(arm_is_body_only_encode(&work));
    assert!(!arm_writes_message_header(&work));
    let changed = PERF_PARITY.replacen(
        "CarEncoder::wrap_unchecked(frame, 0)",
        "CarEncoder::wrap_and_apply_header_unchecked(frame, 0)",
        1,
    );
    let work = encoding_work(&changed, arm)?;
    assert!(arm_writes_message_header(&work));
    assert!(!arm_is_body_only_encode(&work));
    assert!(encoding_work("", arm).is_err());
    Ok(())
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
        let ergo = encoding_work(source, ergo)?;
        let tool = encoding_work(source, tool)?;
        let ergo_hdr = arm_writes_message_header(&ergo);
        let tool_hdr = arm_writes_message_header(&tool);
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
                    !ergo_hdr && arm_is_body_only_encode(&ergo),
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

#[test]
fn group_with_data_timed_arms_both_read_entry_fields_and_var_data()
-> Result<(), Box<dyn std::error::Error>> {
    let source = get_source(PERF_PARITY_EXTENDED, "bench_group_with_data")?;
    let ergo = timed_arm_body(source, "ergo-sbe").ok_or("missing ergo group-with-data arm")?;
    let tool = timed_arm_body(source, "sbe-tool").ok_or("missing sbe-tool group-with-data arm")?;
    assert!(
        PERF_PARITY_EXTENDED.contains("assert_group_with_data_value_parity"),
        "group-with-data must assert decoded value/bytes parity before timing"
    );
    assert!(
        ergo.contains("decode_group_with_data_ergon"),
        "ergon timed arm must enter the group/var-data decoder"
    );
    assert!(
        tool.contains("decode_group_with_data_tool"),
        "sbe-tool timed arm must enter the group/var-data decoder"
    );
    let ergo_dec = get_source(PERF_PARITY_EXTENDED, "decode_group_with_data_ergon")?;
    let tool_dec = get_source(PERF_PARITY_EXTENDED, "decode_group_with_data_tool")?;
    for (label, body) in [("ergon", ergo_dec), ("sbe-tool", tool_dec)] {
        assert!(
            body.contains("var_data_field"),
            "{label} group-with-data decode must read var-data bytes"
        );
        assert!(
            body.contains("tag_group"),
            "{label} group-with-data decode must read fixed entry fields"
        );
    }
    Ok(())
}

#[test]
fn optional_enum_nullify_arms_make_the_same_values_opaque() -> Result<(), Box<dyn std::error::Error>>
{
    // Both codecs compile one message to the same three member loads. A single
    // running total makes those loads a latency chain, which is not the
    // throughput the row gates: each arm keeps four independent totals and
    // calls the same per-message fold. Ergon's arm once black-boxed its
    // message offset while sbe-tool's passed a literal body offset. Every
    // value one arm makes opaque, the other must too.
    let source = get_source(PERF_PARITY_EXTENDED, "bench_optional_enum_nullify")?;
    let ergo = strip_line_comments(
        timed_arm_body(source, "ergo-sbe").ok_or("missing ergo optional-enum-nullify arm")?,
    );
    let tool = strip_line_comments(
        timed_arm_body(source, "sbe-tool").ok_or("missing sbe-tool optional-enum-nullify arm")?,
    );

    for (label, arm) in [("Ergo", &ergo), ("sbe-tool", &tool)] {
        assert_eq!(
            arm.matches("black_box(encoded)").count(),
            1,
            "{label} optional-enum-nullify arm must make the encoded buffer opaque once per message"
        );
        for member in [
            ".optional_enum()",
            ".required_enum_from_optional_type()",
            ".optional_counter()",
        ] {
            assert_eq!(
                arm.matches(member).count(),
                1,
                "{label} optional-enum-nullify arm must read {member} exactly once per message"
            );
        }
        assert_eq!(
            arm.matches(".wrapping_add(fold_message(encoded").count(),
            4,
            "{label} optional-enum-nullify arm must keep four independent totals"
        );
        assert!(
            arm.contains("for _ in 0..(AMP / INDEPENDENT_SUMS)"),
            "{label} optional-enum-nullify amplification must be split across the independent totals"
        );
        assert_eq!(
            arm.matches("black_box(s0.wrapping_add(s1).wrapping_add(s2).wrapping_add(s3))")
                .count(),
            1,
            "{label} must observe every independent total"
        );
    }
    assert!(
        PERF_PARITY_EXTENDED.contains("const INDEPENDENT_SUMS: usize = 4;"),
        "optional-enum-nullify independent-total count drifted from the four folds"
    );
    assert_eq!(
        ergo.matches("black_box(").count(),
        tool.matches("black_box(").count(),
        "optional-enum-nullify arms must make the same values opaque: a black_box on one \
         arm's offset or header field is harness work the other arm never pays"
    );
    Ok(())
}
