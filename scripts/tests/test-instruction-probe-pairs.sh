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

# The ordered lane must retire strictly fewer instructions than the iterator
# walk it wraps. Equal is a failure, not a tie.
write_ordered_pair() {
    local dir=$1 iterator=$2 ordered=$3
    write_summary "$dir/no-lto" ergon decode_full_message "$iterator"
    write_summary "$dir/no-lto" sbe-tool decode_full_message "$iterator"
    write_summary "$dir/no-lto" ergon decode_full_message_ordered "$ordered"
    write_summary "$dir/no-lto" sbe-tool decode_full_message_ordered "$ordered"
}
cat >"$fixture/ordered.tsv" <<'EOF'
symbol	arm	pair	topic	operations
ergo_probe_decode_full_message	ergon	decode_full_message	decode	10000
tool_probe_decode_full_message	sbe-tool	decode_full_message	decode	10000
ergo_probe_decode_full_message_ordered	ergon	decode_full_message_ordered	decode	10000
tool_probe_decode_full_message_ordered	sbe-tool	decode_full_message_ordered	decode	10000
EOF

ordered_below="$fixture/ordered-below"
write_ordered_pair "$ordered_below" 20.0 19.5
python3 "$judge" "$ordered_below" "$fixture/ordered.tsv" >/dev/null

ordered_equal="$fixture/ordered-equal"
write_ordered_pair "$ordered_equal" 20.0 20.0
expect_failure "is not strictly below iterator" python3 "$judge" "$ordered_equal" "$fixture/ordered.tsv"

ordered_above="$fixture/ordered-above"
write_ordered_pair "$ordered_above" 20.0 20.5
expect_failure "is not strictly below iterator" python3 "$judge" "$ordered_above" "$fixture/ordered.tsv"

# Both probe drivers reject a missing or unknown profile before touching the
# host, so this half of their fail-closed contract is provable anywhere.
for driver in run-sbe-instruction-probes.sh run-cluster-instruction-probes.sh; do
    expect_failure "name at least one profile" bash "$root/scripts/$driver"
    expect_failure "unknown profile 'fast'" bash "$root/scripts/$driver" --profile fast
    expect_failure "usage:" bash "$root/scripts/$driver" --bogus
done

echo "test-instruction-probe-pairs: PASS"
