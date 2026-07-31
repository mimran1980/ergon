#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
config=${MUTATION_CONFIG_FILE:-"$repo_root/.cargo/mutants.toml"}

if [[ ! -f "$config" ]]; then
    echo "mutation config: missing $config" >&2
    exit 1
fi

# cargo-mutants already creates `mutants.out` by default. A relative `output`
# config value is resolved inside that directory by current cargo-mutants,
# silently producing `mutants.out/mutants.out` and leaving the ratchet looking
# at an incomplete outer directory.
if rg -n '^[[:space:]]*output[[:space:]]*=' "$config" >/dev/null; then
    echo "mutation config: do not set output in $config; use cargo-mutants' default mutants.out" >&2
    exit 1
fi

for required_scope in \
    parse_with_context \
    get_token_block_size \
    get_dimension_info \
    generate_direct \
    generate_group_decoder
do
    if ! rg -F "\"$required_scope\"" "$config" >/dev/null; then
        echo "mutation config: required critical-path scope is missing: $required_scope" >&2
        exit 1
    fi
done

echo "mutation config: PASS (canonical output and all critical-path scopes)"
