#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
output=${1:-"$repo_root/mutants.out"}
baseline=${MUTATION_BASELINE_FILE:-"$repo_root/.github/mutation-baseline.env"}

if [[ ! -f "$baseline" ]]; then
    echo "mutation ratchet: baseline is missing: $baseline" >&2
    exit 1
fi
if [[ ! -d "$output" ]]; then
    echo "mutation ratchet: output directory is missing: $output" >&2
    exit 1
fi

required_files=(
    mutants.json
    outcomes.json
    caught.txt
    missed.txt
    timeout.txt
    unviable.txt
)
for file in "${required_files[@]}"; do
    if [[ ! -f "$output/$file" ]]; then
        echo "mutation ratchet: incomplete run; missing $output/$file" >&2
        exit 1
    fi
done

# shellcheck disable=SC1090
source "$baseline"
for variable in MIN_MUTANT_OUTCOMES MAX_MISSED_MUTANTS MAX_TIMEOUT_MUTANTS; do
    value=${!variable:-}
    if [[ ! "$value" =~ ^[0-9]+$ ]]; then
        echo "mutation ratchet: $variable is missing or not an integer in $baseline" >&2
        exit 1
    fi
done

count_nonempty_lines() {
    awk 'NF { count += 1 } END { print count + 0 }' "$1"
}

caught=$(count_nonempty_lines "$output/caught.txt")
missed=$(count_nonempty_lines "$output/missed.txt")
timed_out=$(count_nonempty_lines "$output/timeout.txt")
unviable=$(count_nonempty_lines "$output/unviable.txt")
total=$((caught + missed + timed_out + unviable))

if ((total == 0)); then
    echo "mutation ratchet: output contains no mutant outcomes" >&2
    exit 1
fi
if ((total < MIN_MUTANT_OUTCOMES)); then
    echo "mutation ratchet: $total outcomes is below baseline $MIN_MUTANT_OUTCOMES" >&2
    exit 1
fi
if ((missed > MAX_MISSED_MUTANTS)); then
    echo "mutation ratchet: $missed missed exceeds baseline $MAX_MISSED_MUTANTS" >&2
    exit 1
fi
if ((timed_out > MAX_TIMEOUT_MUTANTS)); then
    echo "mutation ratchet: $timed_out timeouts exceeds baseline $MAX_TIMEOUT_MUTANTS" >&2
    exit 1
fi

echo "mutation ratchet: $total outcomes ($caught caught, $unviable unviable)"
echo "mutation ratchet: outcomes $total >= $MIN_MUTANT_OUTCOMES"
echo "mutation ratchet: missed $missed <= $MAX_MISSED_MUTANTS"
echo "mutation ratchet: timeouts $timed_out <= $MAX_TIMEOUT_MUTANTS"
