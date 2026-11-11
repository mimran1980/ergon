#!/usr/bin/env bash
# Prove the instruction-probe pair judge fails when ergon Ir/op is higher.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
judge="$root/scripts/compare-sbe-probe-pairs.py"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

expect_failure() {
    local expected=$1
    shift
    if output=$("$@" 2>&1); then
        echo "expected judge failure containing: $expected" >&2
        exit 1
    fi
    if [[ "$output" != *"$expected"* ]]; then
        echo "wrong judge failure; expected '$expected', got:" >&2
        echo "$output" >&2
        exit 1
    fi
}

write_summary() {
    local dir=$1 arm=$2 pair=$3 ir=$4
    mkdir -p "$dir"
    cat >"$dir/${arm}_${pair}.summary.json" <<EOF
{
  "arm": "$arm",
  "pair": "$pair",
  "profile": "no-lto",
  "instructions_per_operation": $ir
}
EOF
}

cat >"$fixture/probes.tsv" <<'EOF'
symbol	arm	pair	topic	operations
ergo_probe_x	ergon	decode_x	decode	10000
tool_probe_x	sbe-tool	decode_x	decode	10000
ergo_probe_y	ergon	encode_y	encode	10000
tool_probe_y	sbe-tool	encode_y	encode	10000
EOF

pass="$fixture/pass"
write_summary "$pass/no-lto" ergon decode_x 10.0
write_summary "$pass/no-lto" sbe-tool decode_x 10.0
python3 "$judge" "$pass" "$fixture/probes.tsv" >/dev/null

equal_under="$fixture/under"
write_summary "$equal_under/no-lto" ergon decode_x 9.5
write_summary "$equal_under/no-lto" sbe-tool decode_x 10.0
python3 "$judge" "$equal_under" "$fixture/probes.tsv" >/dev/null

over="$fixture/over"
write_summary "$over/no-lto" ergon decode_x 10.001
write_summary "$over/no-lto" sbe-tool decode_x 10.0
expect_failure "exceeds sbe-tool" python3 "$judge" "$over" "$fixture/probes.tsv"

missing="$fixture/missing"
write_summary "$missing/no-lto" ergon decode_x 10.0
expect_failure "missing arm" python3 "$judge" "$missing" "$fixture/probes.tsv"

# The ordered lane must retire strictly fewer instructions than random access
# (each field found from the message start). Equal is a failure, not a tie.
# The staged walk is set below ordered, so judging ordered against it instead
# of random access fails the "below" case.
write_ordered_pair() {
    local dir=$1 random=$2 ordered=$3
    write_summary "$dir/no-lto" ergon decode_full_message 19.0
    write_summary "$dir/no-lto" sbe-tool decode_full_message 19.0
    write_summary "$dir/no-lto" ergon decode_full_message_random "$random"
    write_summary "$dir/no-lto" ergon decode_full_message_ordered "$ordered"
    write_summary "$dir/no-lto" sbe-tool decode_full_message_ordered "$ordered"
}
cat >"$fixture/ordered.tsv" <<'EOF'
symbol	arm	pair	topic	operations
ergo_probe_decode_full_message	ergon	decode_full_message	decode	10000
tool_probe_decode_full_message	sbe-tool	decode_full_message	decode	10000
ergo_probe_decode_full_message_ordered	ergon	decode_full_message_ordered	decode	10000
tool_probe_decode_full_message_ordered	sbe-tool	decode_full_message_ordered	decode	10000
ergo_probe_decode_full_message_random	ergon	decode_full_message_random	lane	10000
EOF

ordered_below="$fixture/ordered-below"
write_ordered_pair "$ordered_below" 20.0 19.5
python3 "$judge" "$ordered_below" "$fixture/ordered.tsv" >/dev/null

ordered_equal="$fixture/ordered-equal"
write_ordered_pair "$ordered_equal" 20.0 20.0
expect_failure "is not strictly below random access" python3 "$judge" "$ordered_equal" "$fixture/ordered.tsv"

ordered_above="$fixture/ordered-above"
write_ordered_pair "$ordered_above" 20.0 20.5
expect_failure "is not strictly below random access" python3 "$judge" "$ordered_above" "$fixture/ordered.tsv"

# Documented exceptions, in the real artifact layout (<root>/<profile>/<probe>.summary.json)
# against the real manifest: each pair at its recorded Ir/op passes, and one
# more ergon instruction, or one fewer sbe-tool instruction, over the 10,000
# operations fails. A pair without an exception still fails one instruction
# above sbe-tool.
real_manifest="$root/sbe/benchmarks/probes.tsv"
write_probe() {
    local dir=$1 profile=$2 symbol=$3 arm=$4 pair=$5 ir=$6
    mkdir -p "$dir/$profile"
    cat >"$dir/$profile/$symbol.summary.json" <<EOF
{"arm": "$arm", "pair": "$pair", "profile": "$profile", "operations": 10000, "instructions_per_operation": $ir}
EOF
}
exceptions=$(cd "$root/scripts" && python3 -c '
import importlib.util
spec = importlib.util.spec_from_file_location("judge", "compare-sbe-probe-pairs.py")
judge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(judge)
for (pair, profile), (ergo, tool) in sorted(judge.IR_EXCEPTIONS.items()):
    print(pair, profile, repr(ergo), repr(tool))
')
[ -n "$exceptions" ] || { echo "no documented exceptions to test" >&2; exit 1; }
for want in decode_full_message decode_full_message_ordered extended_group_with_data; do
    for profile in no-lto lto; do
        grep -qx "$want $profile .*" <<<"$exceptions" || { echo "missing exception $want/$profile" >&2; exit 1; }
    done
done
while read -r pair profile ergo tool; do
    ergo_symbol=$(awk -F'\t' -v p="$pair" '$2=="ergon" && $3==p {print $1}' "$real_manifest")
    tool_symbol=$(awk -F'\t' -v p="$pair" '$2=="sbe-tool" && $3==p {print $1}' "$real_manifest")
    [ -n "$ergo_symbol" ] && [ -n "$tool_symbol" ] || { echo "exception $pair is not a manifest pair" >&2; exit 1; }
    at="$fixture/exc-at-$pair-$profile"
    write_probe "$at" "$profile" "$ergo_symbol" ergon "$pair" "$ergo"
    write_probe "$at" "$profile" "$tool_symbol" sbe-tool "$pair" "$tool"
    python3 "$judge" "$at" "$real_manifest" >/dev/null
    worse="$fixture/exc-worse-$pair-$profile"
    write_probe "$worse" "$profile" "$ergo_symbol" ergon "$pair" "$(python3 -c "print(repr($ergo + 0.0001))")"
    write_probe "$worse" "$profile" "$tool_symbol" sbe-tool "$pair" "$tool"
    expect_failure "beyond its documented exception" python3 "$judge" "$worse" "$real_manifest"
    cheaper="$fixture/exc-cheaper-tool-$pair-$profile"
    write_probe "$cheaper" "$profile" "$ergo_symbol" ergon "$pair" "$ergo"
    write_probe "$cheaper" "$profile" "$tool_symbol" sbe-tool "$pair" "$(python3 -c "print(repr($tool - 0.0001))")"
    expect_failure "beyond its documented exception" python3 "$judge" "$cheaper" "$real_manifest"
done <<<"$exceptions"
plain="$fixture/exc-none"
write_probe "$plain" lto ergo_probe_decode_composite ergon decode_composite 40.0001
write_probe "$plain" lto tool_probe_decode_composite sbe-tool decode_composite 40.0
expect_failure "exceeds sbe-tool 40.0000" python3 "$judge" "$plain" "$real_manifest"

# Both probe drivers reject a missing or unknown profile before touching the
# host, so this half of their fail-closed contract is provable anywhere.
for driver in run-sbe-instruction-probes.sh run-cluster-instruction-probes.sh; do
    expect_failure "name at least one profile" bash "$root/scripts/$driver"
    expect_failure "unknown profile 'fast'" bash "$root/scripts/$driver" --profile fast
    expect_failure "usage:" bash "$root/scripts/$driver" --bogus
done

echo "test-instruction-probe-pairs: PASS"
