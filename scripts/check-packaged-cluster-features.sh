#!/usr/bin/env bash
# Fail-closed check that every advertised feature of a crate is present in
# `cargo package --list` and (with --unpack) actually builds from the archive.
#
# Default target: ergo-aeron-cluster. That crate must not advertise a
# `test-harness` feature or ship Java launcher sources.
set -euo pipefail

mode=""
package="ergo-aeron-cluster"
manifest=""
allow_dirty=("--allow-dirty")

usage() {
    echo "usage: $0 --list|--unpack [--package NAME] [--manifest PATH]" >&2
    exit 2
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --list) mode=list; shift ;;
        --unpack) mode=unpack; shift ;;
        --package) package="$2"; shift 2 ;;
        --manifest) manifest="$2"; shift 2 ;;
        --no-allow-dirty) allow_dirty=(); shift ;;
        *) usage ;;
    esac
done
[[ -n "$mode" ]] || usage

repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"

pkg_args=(-p "$package")
meta_args=(--format-version 1 --no-deps)
if [[ -n "$manifest" ]]; then
    pkg_args=(--manifest-path "$manifest")
    meta_args+=(--manifest-path "$manifest")
fi

metadata_json=$(cargo metadata "${meta_args[@]}")
read_meta() {
    python3 -c '
import json, sys
package = sys.argv[1]
field = sys.argv[2]
meta = json.load(sys.stdin)
for p in meta["packages"]:
    if p["name"] == package:
        if field == "features":
            print("\n".join(sorted(p.get("features", {}))))
        elif field == "version":
            print(p["version"])
        elif field == "manifest_dir":
            import os
            print(os.path.dirname(p["manifest_path"]))
        elif field == "target_directory":
            print(meta["target_directory"])
        sys.exit(0)
sys.exit(f"package {package} not in cargo metadata")
' "$package" "$1" <<<"$metadata_json"
}

features=$(read_meta features)
version=$(read_meta version)
target_directory=$(read_meta target_directory)

if [[ "$package" == "ergo-aeron-cluster" ]]; then
    if grep -qx 'test-harness' <<<"$features"; then
        echo "FAIL: $package advertises a test-harness feature" >&2
        exit 1
    fi
fi

list=$(cargo package "${pkg_args[@]}" --list "${allow_dirty[@]+"${allow_dirty[@]}"}")
if [[ "$package" == "ergo-aeron-cluster" ]]; then
    if grep -E 'test_support|ClusterLauncher\.java|(^|/)test-harness' <<<"$list"; then
        echo "FAIL: $package package list contains harness sources" >&2
        echo "$list" >&2
        exit 1
    fi
fi

echo "packaged $package files:"
echo "$list"
echo "advertised features: ${features:-<none>}"

if [[ "$mode" == "list" ]]; then
    echo "check-packaged-cluster-features: --list PASS"
    exit 0
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cargo package "${pkg_args[@]}" --no-verify "${allow_dirty[@]+"${allow_dirty[@]}"}" >/dev/null
crate_tar="$target_directory/package/${package}-${version}.crate"
if [[ ! -f "$crate_tar" ]]; then
    echo "FAIL: packed crate $crate_tar not found" >&2
    exit 1
fi
tar -xf "$crate_tar" -C "$work"
unpacked=$(find "$work" -mindepth 1 -maxdepth 1 -type d | head -n 1)
if ! (
    cd "$unpacked"
    # NOT --offline. This runs immediately after `cargo publish -p ergo-sbe`,
    # and the packaged cluster crate depends on that just-published version,
    # which is not in the local registry cache. `--offline` made this gate
    # structurally unable to pass at release time (0.1.22): the self-test
    # fixtures have no external deps, so it stayed green there.
    cargo check --all-features --all-targets
); then
    echo "FAIL: unpack-and-build of $package with all advertised features failed" >&2
    exit 1
fi
echo "check-packaged-cluster-features: --unpack PASS"
