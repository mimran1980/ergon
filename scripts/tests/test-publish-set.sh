#!/usr/bin/env bash
# check-publish-set.sh must pass a consistent publish set and fail, naming the
# crate, on each way a release can break: a published crate depending on an
# unpublished one, on a path without a version or on an older version of a
# published sibling, a crate published before its dependency or not at all, a
# dry run that leaves a dependency out, and a published crate missing from the
# public-API gate.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-publish-set.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

# Workspace at 0.1.30: core (published), util (published, depends on core,
# exactly), app (published, depends on both), tool (unpublished, depends on
# app).
write_metadata() {
    python3 - "$fixture/metadata.json" "$@" <<'PY'
import json, sys

out, *edits = sys.argv[1:]
dep = lambda name, req="^0.1.30", kind=None: {
    "name": name, "req": req, "kind": kind, "path": f"/ws/{name}"}
packages = {
    "core": {"publish": None, "dependencies": [
        {"name": "libc", "req": "^0.2", "kind": None}]},
    "util": {"publish": None, "dependencies": [dep("core", "=0.1.30")]},
    "app": {"publish": None, "dependencies": [
        dep("core"), dep("util"), dep("tool", "*", "dev")]},
    "tool": {"publish": [], "dependencies": [dep("app", "*")]},
}
for edit in edits:
    if edit == "unpublish-util":
        packages["util"]["publish"] = []
    elif edit == "unversioned-core":
        packages["app"]["dependencies"][0]["req"] = "*"
    elif edit == "build-dep-tool":
        packages["app"]["dependencies"].append(dep("tool", "^0.1.30", "build"))
    elif edit == "stale-util":
        packages["app"]["dependencies"][1]["req"] = "^0.1.29"
    elif edit == "add-io":
        packages["io"] = {"publish": None, "dependencies": [dep("util")]}
    else:
        sys.exit(f"unknown edit {edit}")
json.dump({"packages": [{"name": n, "version": "0.1.30", **p}
                       for n, p in packages.items()]},
          open(out, "w"))
PY
}

write_justfile() {
    local dry=$1 publish=$2
    {
        printf '%s\n' 'release-check: test'
        printf '%s\n' "$dry" | sed 's/^/    /'
        printf '%s\n' '    @echo ok' ''
        printf '%s\n' '_release-post:' '    just release-check'
        printf '%s\n' "$publish" | sed 's/^/    /'
        printf '%s\n' '    just _tag' '' '_tag:' '    cargo publish -p tool'
    } > "$fixture/justfile"
}

write_workflow() {
    {
        printf '%s\n' 'jobs:' '  release:' '    steps:' '      - name: Publish core' \
            '        run: |' \
            '          cargo release "${{ github.event.inputs.release_type }}" \' \
            '            -p core --execute --no-confirm' \
            '      - name: Dry run is not a release' \
            '        run: cargo release patch -p tool'
        printf '%s\n' "$1" | sed 's/^/      - run: /'
    } > "$fixture/release.yml"
}

write_api() {
    printf '[metadata]\nbaseline_tag = "v0.1.26"\npackages = [%s]\n' "$1" \
        > "$fixture/api.toml"
}

good_dry='cargo publish -p core -p util -p app --dry-run --allow-dirty'
good_publish=$'cargo publish -p core\ncargo publish -p util -p app'

run() {
    "$checker" --metadata "$fixture/metadata.json" \
        --justfile "$fixture/justfile" --api-manifest "$fixture/api.toml" \
        --workflow "$fixture/release.yml" \
        >"$fixture/out" 2>&1
}

expect_failure() {
    local expected=$1
    if run; then
        echo "expected check-publish-set to fail with: $expected" >&2
        cat "$fixture/out" >&2
        exit 1
    fi
    if ! grep -qF -- "$expected" "$fixture/out"; then
        echo "failure did not say: $expected" >&2
        cat "$fixture/out" >&2
        exit 1
    fi
}

reset() {
    write_metadata
    write_justfile "$good_dry" "$good_publish"
    write_api '"core", "util", "app"'
    write_workflow 'cargo publish -p util -p app'
}

reset
run || { cat "$fixture/out" >&2; echo "consistent fixture must pass" >&2; exit 1; }
grep -qF 'release order: core; util app' "$fixture/out" \
    || { cat "$fixture/out" >&2; echo "pass did not print the order" >&2; exit 1; }

# A published crate's normal or build dependency on an unpublished crate.
reset; write_metadata unpublish-util
expect_failure 'app depends on util, which is not published'
reset; write_metadata build-dep-tool
expect_failure 'app depends on tool, which is not published'

# A path dependency without a version: cargo publish refuses it.
reset; write_metadata unversioned-core
expect_failure 'app: path dependency core has no version requirement'

# A requirement older than the published sibling it names: the release
# publishes both at one version, so app's published lower bound would admit a
# util without what app uses (a workspace bump that missed the line).
reset; write_metadata stale-util
expect_failure 'app requires util ^0.1.29, but the release publishes util 0.1.30'

# Release order: a crate published before its dependency, or never.
reset; write_justfile "$good_dry" $'cargo publish -p util -p app\ncargo publish -p core'
expect_failure 'util is published before its dependency core'
reset; write_justfile "$good_dry" $'cargo publish -p core\ncargo publish -p app'
expect_failure 'util is not published by _release-post'
reset; write_justfile "$good_dry" $'cargo publish -p core\ncargo publish -p util -p app\ncargo publish -p util'
expect_failure 'util is published more than once by _release-post'
reset; write_justfile "$good_dry" $'cargo publish -p core -p tool\ncargo publish -p util -p app'
expect_failure 'tool is published by _release-post but publish = false'
reset; write_justfile "$good_dry" $'cargo publish --workspace\ncargo publish -p util -p app'
expect_failure 'names no package'
reset; write_justfile "$good_dry" $'cargo publish -p core --workspace\ncargo publish -p util -p app'
expect_failure 'names no package'
# -n is cargo publish's --dry-run: that line publishes nothing.
reset; write_justfile "$good_dry" $'cargo publish -p core\ncargo publish -p util -p app -n'
expect_failure 'util is not published by _release-post'

# The CI release workflow publishes the same set in a valid order.
reset; write_workflow 'cargo publish -p app'
expect_failure 'util is not published by .github/workflows/release.yml'
reset; write_workflow $'cargo publish -p app\ncargo publish -p util'
expect_failure 'app is published before its dependency util'

# Dry runs: each crate's published dependencies in the same invocation (the
# release publishes them all at one new version, so none is on crates.io).
reset; write_justfile 'cargo publish -p core -p app --dry-run' "$good_publish"
expect_failure 'dry run of app without its dependency util'
reset; write_justfile 'cargo publish -p core --dry-run' "$good_publish"
expect_failure 'util has no dry run in release-check'
# Separate invocations do not help: util is not on crates.io when app's runs.
reset; write_justfile $'cargo publish -p core -p util --dry-run\ncargo publish -p app --dry-run' "$good_publish"
expect_failure 'dry run of app without its dependency util'

# The public-API gate lists every published crate and nothing else.
reset; write_api '"core", "app"'
expect_failure 'util is published but not in api/public-api-baseline.toml'
reset; write_api '"core", "util", "app", "tool"'
expect_failure 'tool is in api/public-api-baseline.toml but not published'

# A second dependant of one published sibling (ergon-io and ergon-md-dpdk
# both need ergon-tune): in the same multi-package publish and dry run as
# that sibling it passes; dry-run on its own, it fails.
with_io() {
    write_metadata add-io
    write_api '"core", "util", "io", "app"'
    write_workflow 'cargo publish -p util -p io -p app'
    write_justfile "$1" $'cargo publish -p core\ncargo publish -p util -p io -p app'
}
reset; with_io 'cargo publish -p core -p util -p io -p app --dry-run --allow-dirty'
run || { cat "$fixture/out" >&2; echo "a second dependant published with its dependency must pass" >&2; exit 1; }
grep -qF 'release order: core; util io app' "$fixture/out" \
    || { cat "$fixture/out" >&2; echo "pass did not print the order with io" >&2; exit 1; }
reset; with_io $'cargo publish -p core -p util -p app --dry-run\ncargo publish -p io --dry-run'
expect_failure 'dry run of io without its dependency util'

echo "test-publish-set: PASS"
