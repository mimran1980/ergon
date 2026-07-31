#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
baseline=${COVERAGE_BASELINE_FILE:-"$repo_root/.github/coverage-baseline.env"}
summary=${1:-}
temporary_summary=

cleanup() {
    if [[ -n "$temporary_summary" ]]; then
        rm -f "$temporary_summary"
    fi
}
trap cleanup EXIT

if [[ ! -f "$baseline" ]]; then
    echo "coverage ratchet: baseline is missing: $baseline" >&2
    exit 1
fi

if [[ -z "$summary" ]]; then
    temporary_summary=$(mktemp)
    summary=$temporary_summary
    (
        cd "$repo_root"
        cargo llvm-cov -p ergo-sbe --all-features --summary-only -- --test-threads=1
    ) | tee "$summary"
fi

# shellcheck disable=SC1090
source "$baseline"
for variable in MIN_REGION_COVERAGE MIN_FUNCTION_COVERAGE MIN_LINE_COVERAGE; do
    value=${!variable:-}
    if [[ ! "$value" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
        echo "coverage ratchet: $variable is missing or not numeric in $baseline" >&2
        exit 1
    fi
done

total_line=$(awk '$1 == "TOTAL" { line = $0 } END { print line }' "$summary")
if [[ -z "$total_line" ]]; then
    echo "coverage ratchet: no TOTAL row in $summary" >&2
    exit 1
fi

read -r region function line < <(
    awk '$1 == "TOTAL" {
        gsub(/%/, "", $4);
        gsub(/%/, "", $7);
        gsub(/%/, "", $10);
        print $4, $7, $10
    }' <<<"$total_line"
)
for metric in "$region" "$function" "$line"; do
    if [[ ! "$metric" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
        echo "coverage ratchet: malformed TOTAL row in $summary: $total_line" >&2
        exit 1
    fi
done

check_metric() {
    local name=$1
    local actual=$2
    local minimum=$3
    if ! awk -v actual="$actual" -v minimum="$minimum" \
        'BEGIN { exit !(actual + 0.000001 >= minimum) }'; then
        echo "coverage ratchet: $name $actual% is below baseline $minimum%" >&2
        exit 1
    fi
    echo "coverage ratchet: $name $actual% >= $minimum%"
}

check_metric regions "$region" "$MIN_REGION_COVERAGE"
check_metric functions "$function" "$MIN_FUNCTION_COVERAGE"
check_metric lines "$line" "$MIN_LINE_COVERAGE"
