#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
coverage_checker="$repo_root/scripts/check-coverage-ratchet.sh"
mutation_checker="$repo_root/scripts/check-mutation-ratchet.sh"
mutation_config_checker="$repo_root/scripts/check-mutation-config.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

expect_failure() {
    local expected=$1
    shift
    if output=$("$@" 2>&1); then
        echo "expected ratchet failure containing: $expected" >&2
        exit 1
    fi
    if [[ "$output" != *"$expected"* ]]; then
        echo "wrong ratchet failure; expected '$expected', got:" >&2
        echo "$output" >&2
        exit 1
    fi
}

cat >"$fixture/coverage-baseline.env" <<'EOF'
MIN_REGION_COVERAGE=90.00
MIN_FUNCTION_COVERAGE=90.00
MIN_LINE_COVERAGE=90.00
EOF
cat >"$fixture/coverage-pass.txt" <<'EOF'
TOTAL 100 5 95.00% 100 4 96.00% 100 3 97.00% 0 0 -
EOF
cat >"$fixture/coverage-low.txt" <<'EOF'
TOTAL 100 11 89.00% 100 4 96.00% 100 3 97.00% 0 0 -
EOF

COVERAGE_BASELINE_FILE="$fixture/coverage-baseline.env" \
    "$coverage_checker" "$fixture/coverage-pass.txt"
expect_failure "below baseline" \
    env COVERAGE_BASELINE_FILE="$fixture/coverage-baseline.env" \
    "$coverage_checker" "$fixture/coverage-low.txt"
expect_failure "no TOTAL row" \
    env COVERAGE_BASELINE_FILE="$fixture/coverage-baseline.env" \
    "$coverage_checker" /dev/null

cat >"$fixture/mutation-baseline.env" <<'EOF'
MIN_MUTANT_OUTCOMES=1
MAX_MISSED_MUTANTS=0
MAX_TIMEOUT_MUTANTS=0
EOF
mkdir "$fixture/mutants-pass"
touch \
    "$fixture/mutants-pass/missed.txt" \
    "$fixture/mutants-pass/timeout.txt" \
    "$fixture/mutants-pass/unviable.txt"
printf '%s\n' "src/lib.rs:1: caught" >"$fixture/mutants-pass/caught.txt"
printf '%s\n' '{}' >"$fixture/mutants-pass/mutants.json"
printf '%s\n' '{}' >"$fixture/mutants-pass/outcomes.json"

MUTATION_BASELINE_FILE="$fixture/mutation-baseline.env" \
    "$mutation_checker" "$fixture/mutants-pass"
expect_failure "output directory is missing" \
    env MUTATION_BASELINE_FILE="$fixture/mutation-baseline.env" \
    "$mutation_checker" "$fixture/not-created"

cp -R "$fixture/mutants-pass" "$fixture/mutants-empty"
: >"$fixture/mutants-empty/caught.txt"
expect_failure "contains no mutant outcomes" \
    env MUTATION_BASELINE_FILE="$fixture/mutation-baseline.env" \
    "$mutation_checker" "$fixture/mutants-empty"

cp -R "$fixture/mutants-pass" "$fixture/mutants-missed"
printf '%s\n' "src/lib.rs:2: missed" >"$fixture/mutants-missed/missed.txt"
expect_failure "missed exceeds baseline" \
    env MUTATION_BASELINE_FILE="$fixture/mutation-baseline.env" \
    "$mutation_checker" "$fixture/mutants-missed"

cat >"$fixture/mutation-minimum-two.env" <<'EOF'
MIN_MUTANT_OUTCOMES=2
MAX_MISSED_MUTANTS=0
MAX_TIMEOUT_MUTANTS=0
EOF
expect_failure "below baseline" \
    env MUTATION_BASELINE_FILE="$fixture/mutation-minimum-two.env" \
    "$mutation_checker" "$fixture/mutants-pass"

cat >"$fixture/mutants-good.toml" <<'EOF'
examine_re = [
    "parse_with_context",
    "get_token_block_size",
    "generate_direct",
    "generate_group_decoder",
]
EOF
MUTATION_CONFIG_FILE="$fixture/mutants-good.toml" "$mutation_config_checker"

cat >"$fixture/mutants-nested-output.toml" <<'EOF'
output = "mutants.out"
EOF
expect_failure "do not set output" \
    env MUTATION_CONFIG_FILE="$fixture/mutants-nested-output.toml" \
    "$mutation_config_checker"

cat >"$fixture/mutants-missing-scope.toml" <<'EOF'
examine_re = [
    "parse_with_context",
    "get_token_block_size",
    "generate_direct",
]
EOF
expect_failure "required critical-path scope is missing" \
    env MUTATION_CONFIG_FILE="$fixture/mutants-missing-scope.toml" \
    "$mutation_config_checker"

echo "quality ratchet self-test: PASS"
