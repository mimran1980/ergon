#!/usr/bin/env bash
# The io lab scripts ship a source snapshot to the VM. `ergon-io` takes
# `ergon-tune` by path and both crates `include_str!` their READMEs, so a
# snapshot that omits them builds nowhere: 2026-10-19's AWS run died on the
# receiver with "no matching package named `ergon-tune` found". Every lab
# script must snapshot the same set, and that set must be enough to build.
# The negative proof at the end re-runs the same checks on a snapshot missing
# `tune` and asserts they fail closed.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
benches="$repo_root/io/benches/scripts"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

# 1. Every lab script's snapshot line is the same, and it carries `tune` and `io`.
snap_line() {
    awk '/COPYFILE_DISABLE=1 tar/{f=1} f{print} f && /\)/{exit}' "$1" | tr '\n' ' '
}
want=$(snap_line "$benches/aws-flags.sh")
if [[ -z $want || $want != *'tune'* || $want != *'io'* ]]; then
    echo "aws-flags.sh snapshot must name the tune and io crates, got: $want" >&2
    exit 1
fi
# Only the scripts that ship a snapshot; aws-az-scan.sh, azure-plan.sh and
# vm-setup.sh run locally and never build on the VM.
for script in "$benches"/*.sh; do
    grep -q 'COPYFILE_DISABLE=1 tar' "$script" || continue
    have=$(snap_line "$script")
    if [[ $have != "$want" ]]; then
        echo "$(basename "$script") snapshots a different set from aws-flags.sh:" >&2
        echo "  $have" >&2
        exit 1
    fi
done

# Enough to build: every path dependency and include_str! target of the built
# crates resolves inside the snapshot, and cargo can read it. Prints the defect
# and returns 1; returns 0 when the tree is sound. No `exit` here so the
# negative proof can call it.
enough_to_build() {
    local root=$1 f manifest crate_dir dep resolved rs dir inc
    local need=(
        tune/Cargo.toml
        tune/README.md
        tune/src/lib.rs
        io/Cargo.toml
        io/README.md
        io/src/lib.rs
        io/benches/Cargo.toml
    )
    for f in "${need[@]}"; do
        if [[ ! -e $root/$f ]]; then
            echo "lab snapshot is missing $f" >&2
            return 1
        fi
    done

    # Path dependencies of the *built* crates. Only `tune/` and `io/` — the
    # root workspace manifest lists `sbe` and friends under
    # [workspace.dependencies], which the io lab never builds and which cargo
    # does not resolve for an excluded package like io/benches. `find` yields
    # manifests, so dirname to the crate root first.
    while IFS= read -r manifest; do
        crate_dir=$(dirname "$manifest")
        while IFS= read -r dep; do
            [[ -z $dep ]] && continue
            if [[ ! -d $crate_dir/$dep ]]; then
                echo "$(basename "$crate_dir")'s path dependency $dep is missing from the snapshot" >&2
                return 1
            fi
            resolved=$(cd "$crate_dir" && cd "$dep" && pwd)
            case $resolved in
            "$root"/*) ;;
            *)
                echo "$(basename "$crate_dir")'s path dependency $dep resolves outside the snapshot ($resolved)" >&2
                return 1
                ;;
            esac
        done < <(sed -n 's/.*path *= *"\([^"]*\)".*/\1/p' "$manifest")
    done < <(find "$root/tune" "$root/io" -name Cargo.toml -not -path '*/target/*')

    # Every include_str! in a built crate points at a file inside the snapshot.
    while IFS= read -r rs; do
        dir=$(dirname "$rs")
        while IFS= read -r inc; do
            [[ -z $inc ]] && continue
            if [[ ! -e $dir/$inc ]]; then
                echo "$rs include_str!(\"$inc\") is missing from the snapshot" >&2
                return 1
            fi
        done < <(sed -n 's/.*include_str!("\(.*\)").*/\1/p' "$rs")
    done < <(find "$root/tune/src" "$root/io/src" -name '*.rs')

    # Cargo agrees. `--no-deps` reads the manifests without touching the
    # registry, and this is the check that would have caught 2026-10-19's
    # "no matching package named `ergon-tune` found" before the run started.
    if ! command -v cargo >/dev/null; then
        echo "cargo is required for this test" >&2
        return 1
    fi
    if ! (cd "$root/io/benches" && cargo metadata --no-deps --format-version 1 >/dev/null); then
        echo "cargo cannot resolve the lab snapshot" >&2
        return 1
    fi
    return 0
}

# 2. The snapshot the lab scripts ship is enough to build.
(
    cd "$repo_root"
    COPYFILE_DISABLE=1 tar --no-xattrs --exclude target --exclude results \
        -czf "$fixture/src.tgz" Cargo.toml rust-toolchain.toml tune io
)
mkdir -p "$fixture/good"
tar -xzf "$fixture/src.tgz" -C "$fixture/good"
enough_to_build "$fixture/good"

# 3. Negative proof: the same checks on a snapshot missing `tune` must fail,
#    or they are not checking anything.
mkdir -p "$fixture/bad"
(
    cd "$repo_root"
    COPYFILE_DISABLE=1 tar --no-xattrs --exclude target --exclude results \
        -czf "$fixture/bad.tgz" Cargo.toml rust-toolchain.toml io
)
tar -xzf "$fixture/bad.tgz" -C "$fixture/bad"
if enough_to_build "$fixture/bad" 2>/dev/null; then
    echo "a snapshot without tune passed the checks; they prove nothing" >&2
    exit 1
fi

echo "test-lab-snapshot: PASS (all lab scripts ship one set; tune and io build inside it; a snapshot without tune fails closed)"
