#!/usr/bin/env bash
# ensure-fix-sbe-conformance.sh must land on the pinned commit even when the
# remote has moved on, and fail on every way the suite can be somewhere else;
# run-fix-sbe-conformance.sh must go through it and never clone by branch.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
ensure="$repo_root/scripts/ensure-fix-sbe-conformance.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

expect_failure() {
    local expected=$1
    shift
    if output=$("$@" 2>&1); then
        echo "expected conformance-pin failure containing: $expected" >&2
        exit 1
    fi
    if [[ "$output" != *"$expected"* ]]; then
        echo "wrong conformance-pin failure; expected '$expected', got:" >&2
        echo "$output" >&2
        exit 1
    fi
}

git_fix() { git -c user.name=t -c user.email=t@example.invalid -C "$fixture/upstream" "$@"; }

# An upstream with two commits; the pin names the first.
git init -q "$fixture/upstream"
echo one >"$fixture/upstream/file"
git_fix add file
git_fix commit -qm one
pinned=$(git_fix rev-parse HEAD)
echo two >"$fixture/upstream/file"
git_fix commit -qam two
moved=$(git_fix rev-parse HEAD)
[[ "$pinned" != "$moved" ]]
printf '# fixture\nfile://%s/upstream %s\n' "$fixture" "$pinned" >"$fixture/pin.txt"

ensure_fixture() { "$ensure" --pin "$fixture/pin.txt" "$@"; }

# 1. A fresh directory lands on the pin, not on the remote's newer HEAD.
ensure_fixture "$fixture/fresh" >/dev/null
[[ "$(git -C "$fixture/fresh" rev-parse HEAD)" == "$pinned" ]]

# 2. A checkout of the moved remote is brought back to the pin.
git clone -q "file://$fixture/upstream" "$fixture/stale"
[[ "$(git -C "$fixture/stale" rev-parse HEAD)" == "$moved" ]]
ensure_fixture "$fixture/stale" >/dev/null
[[ "$(git -C "$fixture/stale" rev-parse HEAD)" == "$pinned" ]]

# 3. A supplied suite at another commit is refused, not moved.
git clone -q "file://$fixture/upstream" "$fixture/supplied"
expect_failure "the pin is $pinned" ensure_fixture --verify-only "$fixture/supplied"
[[ "$(git -C "$fixture/supplied" rev-parse HEAD)" == "$moved" ]]
ensure_fixture --verify-only "$fixture/fresh" >/dev/null

# 4. Not a checkout.
mkdir "$fixture/plain"
echo x >"$fixture/plain/file"
expect_failure "is not a git checkout" ensure_fixture --verify-only "$fixture/plain"
expect_failure "exists and is not a git checkout" ensure_fixture "$fixture/plain"

# 5. A pin that is not a full commit.
printf 'file://%s/upstream main\n' "$fixture" >"$fixture/bad-pin.txt"
expect_failure "is not a full commit sha" "$ensure" --pin "$fixture/bad-pin.txt" "$fixture/x"
printf '# nothing\n' >"$fixture/empty-pin.txt"
expect_failure "no '<url> <commit>' line" "$ensure" --pin "$fixture/empty-pin.txt" "$fixture/x"

# 6. The real pin is well formed and the runner uses the helper, never a bare clone.
grep -qE '^https://[^ ]+ [0-9a-f]{40}$' "$repo_root/scripts/fix-sbe-conformance-pin.txt"
runner="$repo_root/scripts/run-fix-sbe-conformance.sh"
grep -q 'ensure-fix-sbe-conformance.sh' "$runner"
grep -q 'fix-sbe-conformance-pom.patch' "$runner"
grep -q '^+++ b/pom.xml' "$repo_root/scripts/fix-sbe-conformance-pom.patch"
if grep -qE 'git clone' "$runner"; then
    echo "run-fix-sbe-conformance.sh must not clone by branch" >&2
    exit 1
fi

echo "test-fix-sbe-conformance-pin: PASS"
