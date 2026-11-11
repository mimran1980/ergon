#!/usr/bin/env bash
# check-workflow-paths.sh must pass the real workflow and fail, naming the gap,
# when a filter drops an input of the job, when push and pull_request drift
# apart, and when a trigger has no filter.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-workflow-paths.sh"
workflow=".github/workflows/cluster-compatibility.yml"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

expect_failure() {
    local expected=$1
    shift
    if output=$("$@" 2>&1); then
        echo "expected workflow-paths failure containing: $expected" >&2
        exit 1
    fi
    if [[ "$output" != *"$expected"* ]]; then
        echo "wrong workflow-paths failure; expected '$expected', got:" >&2
        echo "$output" >&2
        exit 1
    fi
}

mkdir -p "$fixture/.github/workflows"
"$checker" --root "$repo_root" >/dev/null

# Drop each required input from both filters in turn.
required=$(sed -n "/^  push:/,/^jobs:/p" "$repo_root/$workflow" | sed -n "/paths:/,/^  pull_request:/p" | grep -E "^      - " | sed -E "s/^      - '(.*)'$/\1/")
[[ -n "$required" ]]
while IFS= read -r entry; do
    grep -vxF "      - '$entry'" "$repo_root/$workflow" >"$fixture/$workflow"
    expect_failure "filter misses ['$entry']" "$checker" --root "$fixture"
done <<<"$required"

# Only pull_request loses an entry: the two filters drift apart.
awk -v drop="      - 'Cargo.lock'" '/^  pull_request:/ {pr=1} !(pr && $0 == drop)' \
    "$repo_root/$workflow" >"$fixture/$workflow"
expect_failure "push and pull_request filters differ" "$checker" --root "$fixture"

# A trigger with no filter runs on every path, so nothing can be missed; the
# check still refuses it, because the job is meant to be path-scoped.
awk '/^  pull_request:/ {print; print "    branches: [main]"; skip=1; next} skip && /^jobs:/ {skip=0; print "" } !skip' \
    "$repo_root/$workflow" >"$fixture/$workflow"
expect_failure "pull_request has no paths filter" "$checker" --root "$fixture"

echo "test-workflow-paths: PASS"
