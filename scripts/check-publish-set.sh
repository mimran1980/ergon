#!/usr/bin/env bash
# The crates the release publishes are a closed, ordered set:
#   - a published crate depends (normal or build) on no unpublished workspace
#     crate, every path dependency carries a version (cargo publish refuses
#     one without), and a requirement on a published sibling names that
#     sibling's own version (the version the release publishes it at);
#   - `just _release-post` and the CI release workflow each publish every
#     crate that does not say `publish = false`, once, each after (or in one
#     multi-package `cargo publish` with) the crates it depends on;
#   - `just release-check` dry-runs each one with its published dependencies
#     in the same invocation: the release publishes them all at one new
#     version, so a dependency is never on crates.io yet when its dependents
#     are verified;
#   - api/public-api-baseline.toml lists exactly the published crates.
# Usage: check-publish-set.sh [--metadata FILE] [--justfile FILE]
#                             [--workflow FILE] [--api-manifest FILE]
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
metadata=""
justfile="$root/justfile"
workflow="$root/.github/workflows/release.yml"
api_manifest="$root/api/public-api-baseline.toml"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --metadata) metadata="$2"; shift 2 ;;
        --justfile) justfile="$2"; shift 2 ;;
        --workflow) workflow="$2"; shift 2 ;;
        --api-manifest) api_manifest="$2"; shift 2 ;;
        *) echo "usage: $0 [--metadata FILE] [--justfile FILE] [--workflow FILE] [--api-manifest FILE]" >&2; exit 2 ;;
    esac
done

if [[ -z "$metadata" ]]; then
    metadata=$(mktemp)
    trap 'rm -f "$metadata"' EXIT
    (cd "$root" && cargo metadata --format-version 1 --no-deps --offline) > "$metadata"
fi

python3 - "$metadata" "$justfile" "$workflow" "$api_manifest" <<'PY'
import json, re, shlex, sys, tomllib

metadata_path, justfile_path, workflow_path, api_path = sys.argv[1:]

# Workspace members never released although their manifests do not say
# publish = false: sbe-tool writes these reference crates' manifests verbatim
# (regenerate-sbe-benchmark-reference.sh --check keeps them byte for byte).
NOT_RELEASED = {
    "sbe_tool_extension",
    "sbe_tool_group_with_data",
    "sbe_tool_optional_enum_nullify",
}
# Not dry-run in release-check, with the gate that stands in for it.
DRY_RUN_EXEMPT = {
    "ergo-aeron-cluster": "its build verifies against ergo-sbe from crates.io: "
    "check-packaged-cluster-features.sh --unpack runs after ergo-sbe is published",
}

packages = json.load(open(metadata_path))["packages"]
members = {p["name"] for p in packages}
versions = {p["name"]: p["version"] for p in packages}
published = [p["name"] for p in packages
             if p.get("publish") != [] and p["name"] not in NOT_RELEASED]
errors = []

# Published workspace dependencies of each published crate (normal and build:
# cargo strips a path-only dev-dependency from the published manifest).
needs = {}
for p in packages:
    if p["name"] not in published:
        continue
    needs[p["name"]] = set()
    for d in p["dependencies"]:
        if d.get("kind") == "dev":
            continue
        if d.get("path") and d["req"] == "*":
            errors.append(f"{p['name']}: path dependency {d['name']} has no version requirement")
        if d["name"] in members:
            if d["name"] in published:
                needs[p["name"]].add(d["name"])
                # The release publishes the sibling at its current version;
                # an older requirement is a lower bound the published
                # dependant does not meet (a bump that missed this line).
                want = versions[d["name"]]
                if d["req"] not in ("*", f"^{want}", f"={want}"):
                    errors.append(f"{p['name']} requires {d['name']} {d['req']}, "
                                  f"but the release publishes {d['name']} {want}: "
                                  f"make it version = \"{want}\"")
            else:
                errors.append(f"{p['name']} depends on {d['name']}, which is not published (publish = false)")


def recipe(name):
    """Command lines of one justfile recipe's body."""
    lines = open(justfile_path).read().splitlines()
    head = re.compile(rf"^@?{re.escape(name)}(\s[^:]*)?:(?!=)")
    for i, line in enumerate(lines):
        if head.match(line):
            body = []
            for line in lines[i + 1:]:
                if line and not line[0].isspace():
                    break
                body.append(line.strip().lstrip("@-"))
            return body
    errors.append(f"justfile has no {name} recipe")
    return []


def workflow_commands():
    """Shell lines of the workflow's run steps, continuations joined."""
    joined, pending = [], ""
    for line in open(workflow_path).read().splitlines():
        text = re.sub(r"^\s*(-\s+)?run:\s*\|?", "", line).strip()
        if text.endswith("\\"):
            pending += text[:-1] + " "
            continue
        joined.append(pending + text)
        pending = ""
    return joined


def publishes(label, commands, dry):
    """Package groups of the cargo publish (or cargo release) lines, in order."""
    groups = []
    for line in commands:
        try:
            words = shlex.split(line, comments=True)
        except ValueError:
            continue
        if words[:2] not in (["cargo", "publish"], ["cargo", "release"]):
            continue
        if words[1] == "publish":
            is_dry = "--dry-run" in words or "-n" in words
        else:  # cargo-release only acts with --execute
            is_dry = "--execute" not in words and "-x" not in words
        if dry != is_dry:
            continue
        group, args = [], iter(words[2:])
        for w in args:
            if w in ("-p", "--package"):
                group.append(next(args, ""))
            elif w.startswith("--package="):
                group.append(w.split("=", 1)[1])
            elif w in ("--workspace", "--all", "--exclude"):
                group = []
                break
        if not group:
            errors.append(f"{label}: `{line}` names no package (-p NAME each, not --workspace)")
            continue
        groups.append(group)
    return groups


def check_order(label, groups):
    position = {}
    for i, group in enumerate(groups):
        for crate in group:
            if crate in position:
                errors.append(f"{crate} is published more than once by {label}")
            position.setdefault(crate, i)
            if crate not in published:
                errors.append(f"{crate} is published by {label} but publish = false")
    for crate in published:
        if crate not in position:
            errors.append(f"{crate} is not published by {label}")
            continue
        for dep in sorted(needs[crate]):
            if dep in position and position[dep] > position[crate]:
                errors.append(f"{crate} is published before its dependency {dep} in {label}")


release = publishes("_release-post", recipe("_release-post"), dry=False)
check_order("_release-post", release)
check_order(".github/workflows/release.yml",
            publishes("release.yml", workflow_commands(), dry=False))

dry_run = set()
for group in publishes("release-check", recipe("release-check"), dry=True):
    dry_run.update(group)
    for crate in group:
        for dep in sorted(needs.get(crate, ())):
            if dep not in group:
                errors.append(f"dry run of {crate} without its dependency {dep} "
                              f"(add -p {dep} to the same cargo publish)")
for crate in published:
    if crate not in dry_run and crate not in DRY_RUN_EXEMPT:
        errors.append(f"{crate} has no dry run in release-check")

api = set(tomllib.load(open(api_path, "rb")).get("metadata", {}).get("packages", []))
for crate in published:
    if crate not in api:
        errors.append(f"{crate} is published but not in api/public-api-baseline.toml [metadata].packages")
for crate in sorted(api - set(published)):
    errors.append(f"{crate} is in api/public-api-baseline.toml but not published")

if errors:
    for e in errors:
        print(f"FAIL: {e}", file=sys.stderr)
    sys.exit(1)
print("release order: " + "; ".join(" ".join(g) for g in release))
print(f"check-publish-set: PASS ({len(published)} published crates)")
PY
