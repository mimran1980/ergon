#!/usr/bin/env bash
# The Cluster Compatibility workflow only runs when a changed file matches its
# `paths` filter, so a filter that misses a file the job builds from is a gate
# that cannot fire (0.1.27 changed only sbe/src/codegen/ and it stayed silent).
# This checks that
#   - `push` and `pull_request` carry the same non-empty filter, and
#   - the filter names everything the job compiles or runs: the workspace and
#     member manifests, the lockfile, the toolchain pin, the cluster crate,
#     its schemas, the Java harness, the SBE sources it generates from, the
#     recipes it calls, and the Aeron submodule (its gitlink).
# Usage: check-workflow-paths.sh [--root DIR]
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
while [[ $# -gt 0 ]]; do
    case "$1" in
        --root) root=$2; shift 2 ;;
        *) echo "usage: $0 [--root DIR]" >&2; exit 2 ;;
    esac
done

python3 -I - "$root/.github/workflows/cluster-compatibility.yml" <<'PY'
import re, sys

path = sys.argv[1]
REQUIRED = [
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/**",
    "sbe/Cargo.toml",
    "sbe/src/**",
    "cluster/**",
    "cluster/schemas/**",
    "cluster-test-harness/**",
    "justfile",
    "just/aeron.just",
    ".gitmodules",
    "aeron",
    ".github/workflows/cluster-compatibility.yml",
]

try:
    lines = open(path).read().splitlines()
except OSError as err:
    sys.exit(f"workflow paths: cannot read {path}: {err}")

filters = {}
trigger = None
in_paths = False
in_on = False
for line in lines:
    if re.match(r"^on:\s*$", line):
        in_on, trigger, in_paths = True, None, False
        continue
    if in_on and re.match(r"^\S", line):
        in_on = False
    if not in_on:
        continue
    m = re.match(r"^  (\w+):\s*$", line)
    if m:
        trigger, in_paths = m.group(1), False
        continue
    if trigger and re.match(r"^    paths:\s*$", line):
        in_paths = True
        filters[trigger] = []
        continue
    if in_paths:
        m = re.match(r"^      - ['\"]?([^'\"#]+?)['\"]?\s*(#.*)?$", line)
        if m:
            filters[trigger].append(m.group(1))
        elif line.strip() and not line.lstrip().startswith("#"):
            in_paths = False

errors = []
for trig in ("push", "pull_request"):
    if not filters.get(trig):
        errors.append(f"{trig} has no paths filter")
if not errors:
    if set(filters["push"]) != set(filters["pull_request"]):
        diff = sorted(set(filters["push"]) ^ set(filters["pull_request"]))
        errors.append(f"push and pull_request filters differ: {diff}")
    for trig in ("push", "pull_request"):
        missing = [p for p in REQUIRED if p not in filters[trig]]
        if missing:
            errors.append(f"{trig} filter misses {missing}")
if errors:
    for e in errors:
        print(f"workflow paths: {path}: {e}", file=sys.stderr)
    sys.exit(1)
print("workflow paths: cluster-compatibility.yml filters cover the job's inputs")
PY
