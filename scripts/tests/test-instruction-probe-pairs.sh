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

echo "test-instruction-probe-pairs: PASS"
