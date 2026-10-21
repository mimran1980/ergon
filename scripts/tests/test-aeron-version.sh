#!/usr/bin/env bash
# Prove check-aeron-version.sh fails, naming the culprit, when any one Aeron
# source drifts: a rusteron crate bundling another Aeron (in Cargo.lock or in
# a sample's own lock), a gitlink off the pin, a checked-out submodule at the
# wrong release or with local changes to tracked files, a built jar of another
# release, and the fail-closed cases (a built jar with no version; a pin file
# missing, malformed or empty; a Cargo.lock missing or unreadable; no gitlink;
# a checkout with no readable HEAD, status or version.txt, or no .git; a
# rusteron crate without aeron/version.txt; no Aeron-bundling rusteron; a
# rusteron crate cargo cannot locate offline or does not resolve from its
# lock). A consistent fixture passes, with and without the submodule checked
# out, with jars in every build/libs the check scans, with untracked and
# ignored files in the checkout and with a tracked file touched but unchanged
# (leaving aeron's index unwritten), so each failure comes from its one
# mutation.
# The fixture is a throwaway git repo whose rusteron crates are path crates,
# so cargo metadata runs offline.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-aeron-version.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

pin_sha=377943bcb8256fa5279fc4e5c6d9f2a076294222
other_sha=1111111111111111111111111111111111111111
git_q() { git -c user.name=fixture -c user.email=fixture@invalid -c commit.gpgsign=false "$@" >/dev/null; }

crate() { # dir name version [aeron-version]
    mkdir -p "$1/crates/$2/src"
    printf '[package]\nname = "%s"\nversion = "%s"\nedition = "2021"\npublish = false\n' "$2" "$3" >"$1/crates/$2/Cargo.toml"
    : >"$1/crates/$2/src/lib.rs"
    if [[ -n ${4:-} ]]; then
        mkdir -p "$1/crates/$2/aeron"
        printf '%s\n' "$4" >"$1/crates/$2/aeron/version.txt"
    fi
}

base="$work/base"
mkdir -p "$base/app/src" "$base/scripts"
printf '[workspace]\nmembers = ["app"]\nresolver = "2"\n' >"$base/Cargo.toml"
cat >"$base/app/Cargo.toml" <<'EOF'
[package]
name = "app"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
rusteron-client = { path = "../crates/rusteron-client" }
rusteron-media-driver = { path = "../crates/rusteron-media-driver" }
rusteron-code-gen = { path = "../crates/rusteron-code-gen" }
EOF
: >"$base/app/src/lib.rs"
crate "$base" rusteron-client 0.2.11 1.53.3
crate "$base" rusteron-media-driver 0.2.11 1.53.3
crate "$base" rusteron-code-gen 0.2.11 # bundles no Aeron, so it is not a source
cargo generate-lockfile --offline --quiet --manifest-path "$base/Cargo.toml"
printf '# fixture\n1.53.3 %s\n' "$pin_sha" >"$base/scripts/aeron-pin.txt"
git_q -C "$base" init -q
git_q -C "$base" update-index --add --cacheinfo "160000,$pin_sha,aeron"

fixture() { # name -> fresh copy of the consistent fixture
    rm -rf "${work:?}/$1"
    cp -R "$base" "$work/$1"
    echo "$work/$1"
}

expect_pass() { # label dir
    if ! out=$(AERON_VERSION_ROOT="$2" "$checker" 2>&1); then
        echo "$1: expected PASS, got:" >&2
        echo "$out" >&2
        exit 1
    fi
}

expect_fail() { # label dir needle...
    local label=$1 dir=$2
    shift 2
    if out=$(AERON_VERSION_ROOT="$dir" "$checker" 2>&1); then
        echo "$label: expected FAIL, got:" >&2
        echo "$out" >&2
        exit 1
    fi
    for needle in "$@"; do
        if ! grep -F -q -- "$needle" <<<"$out"; then
            echo "$label: failure does not name '$needle':" >&2
            echo "$out" >&2
            exit 1
        fi
    done
    echo "caught $label:"
    grep -E '^  - |version (1\.52|1\.54)|commit  1111' <<<"$out" | sed 's/^/    /'
}

# Positive control: consistent, submodule not checked out.
dir=$(fixture pass)
expect_pass "consistent fixture" "$dir"
grep -F -q 'aeron/ not checked out' <<<"$out" || { echo "pass output lacks the not-checked-out note: $out" >&2; exit 1; }

# (a) rusteron bumped to a release bundling another Aeron.
dir=$(fixture rusteron)
crate "$dir" rusteron-media-driver 0.2.12 1.54.0
cargo generate-lockfile --offline --quiet --manifest-path "$dir/Cargo.toml"
expect_fail "rusteron bundling another Aeron" "$dir" \
    'version 1.54.0     rusteron-media-driver 0.2.12' 'version 1.53.3     rusteron-client 0.2.11' \
    'version 1.53.3     scripts/aeron-pin.txt' 'Aeron versions disagree: 1.53.3, 1.54.0'

# (b) gitlink moved without the pin.
dir=$(fixture gitlink)
git_q -C "$dir" update-index --cacheinfo "160000,$other_sha,aeron"
expect_fail "gitlink off the pin" "$dir" \
    "commit  $other_sha  aeron gitlink (git index)" "commit  $pin_sha  scripts/aeron-pin.txt" \
    "aeron commits disagree: $other_sha, $pin_sha"

# (c) checked-out submodule: at the pin it passes; moved to another release it fails.
dir=$(fixture checkout)
mkdir -p "$dir/aeron"
git_q -C "$dir/aeron" init -q
printf '1.53.3\n' >"$dir/aeron/version.txt"
src=aeron-client/src/main/java/io/aeron/Aeron.java
mkdir -p "$dir/aeron/${src%/*}"
printf 'class Aeron {}\n' >"$dir/aeron/$src"
printf 'build/\n' >"$dir/aeron/.gitignore"
git_q -C "$dir/aeron" add version.txt "$src" .gitignore
git_q -C "$dir/aeron" commit -q --no-verify -m 1.53.3
at_pin=$(git -C "$dir/aeron" rev-parse HEAD)
printf '1.53.3 %s\n' "$at_pin" >"$dir/scripts/aeron-pin.txt"
git_q -C "$dir" update-index --add --cacheinfo "160000,$at_pin,aeron"
expect_pass "checked-out submodule at the pin" "$dir"
# Built jars: the harness puts every jar in these build/libs on its classpath
# and loads whichever aeron-all-*.jar it lists first, so a jar left behind by
# an earlier pin must fail. Sources/javadoc jars of the pin pass. Each module
# whose build/libs the harness reads must be scanned, so each must be listed.
modules=(aeron-all aeron-cluster aeron-archive aeron-samples)
for module in "${modules[@]}"; do
    mkdir -p "$dir/aeron/$module/build/libs"
    : >"$dir/aeron/$module/build/libs/$module-1.53.3.jar"
done
libs="$dir/aeron/aeron-all/build/libs"
: >"$libs/aeron-all-1.53.3-sources.jar"
expect_pass "built jars of the pinned release" "$dir"
for module in "${modules[@]}"; do
    jar="aeron/$module/build/libs/$module-1.53.3.jar"
    grep -F -q "$jar (built)" <<<"$out" || { echo "pass output does not list $jar: $out" >&2; exit 1; }
done
grep -F -q 'aeron/aeron-all/build/libs/aeron-all-1.53.3-sources.jar (built)' <<<"$out" || { echo "pass output does not list the sources jar: $out" >&2; exit 1; }
# Local edits to tracked files leave HEAD and version.txt on the pin, and a
# jar Gradle builds from them is still named for it, so a modified or staged
# tracked file fails, naming the files (the first five) and how to restore the
# pinned tree. Ignored and untracked files (the build/libs jars above, a
# scratch file) do not count.
: >"$dir/aeron/scratch.txt"
expect_pass "untracked and ignored files in the checkout" "$dir"
printf '// local edit\n' >>"$dir/aeron/$src"
expect_fail "checked-out submodule with a modified tracked file" "$dir" \
    'aeron/ has local changes to tracked files' " M $src" \
    'git submodule update --force aeron' 'just build-aeron-jars'
git_q -C "$dir/aeron" add "$src"
for n in 1 2 3 4 5 6; do printf '%s\n' "$n" >"$dir/aeron/new-$n.txt"; done
git_q -C "$dir/aeron" add new-*.txt
expect_fail "checked-out submodule with staged tracked files" "$dir" \
    "M  $src" 'A  new-1.txt' 'A  new-4.txt' '... and 2 more'
if grep -F -q 'new-5.txt' <<<"$out"; then echo "staged-file failure lists more than five files: $out" >&2; exit 1; fi
git_q -C "$dir/aeron" reset -q --hard
expect_pass "checkout restored to the pin" "$dir"
# The check is read-only: a tracked file touched but unchanged passes, and
# aeron's index is left as it was, where a plain `git status` would rewrite it
# to record the new timestamp.
touch -t 202001010000 "$dir/aeron/$src"
cp "$dir/aeron/.git/index" "$work/index.before"
expect_pass "checked-out submodule with a touched, unchanged tracked file" "$dir"
cmp -s "$work/index.before" "$dir/aeron/.git/index" || { echo "the check rewrote aeron/.git/index" >&2; exit 1; }
# Fail closed when git cannot read the checkout's status (a corrupt index).
cp "$dir/aeron/.git/index" "$work/index.good"
printf 'not an index\n' >"$dir/aeron/.git/index"
expect_fail "checked-out submodule whose status git cannot read" "$dir" 'cannot read the status of aeron/'
cp "$work/index.good" "$dir/aeron/.git/index"
rm "$dir/aeron/scratch.txt"
# A jar whose name carries no version cannot be checked, so it fails.
: >"$libs/aeron-all.jar"
expect_fail "built jar without a version" "$dir" 'aeron/aeron-all/build/libs/aeron-all.jar is not named aeron-all-<version>.jar'
rm "$libs/aeron-all.jar"
: >"$libs/aeron-all-1.52.2.jar"
expect_fail "built jar of another release beside the pinned one" "$dir" \
    'version 1.52.2     aeron/aeron-all/build/libs/aeron-all-1.52.2.jar (built)' 'Aeron versions disagree: 1.52.2, 1.53.3'
for module in "${modules[@]}"; do rm -r "${dir:?}/aeron/$module"; done
printf '1.52.2\n' >"$dir/aeron/version.txt"
git_q -C "$dir/aeron" commit -q --no-verify -am 1.52.2
moved=$(git -C "$dir/aeron" rev-parse HEAD)
expect_fail "checked-out submodule at another release" "$dir" \
    'version 1.52.2     aeron/version.txt (checkout)' "commit  $moved  aeron/ checkout HEAD" \
    "commit  $at_pin  aeron gitlink (git index)" 'Aeron versions disagree: 1.52.2, 1.53.3'

# (c') same HEAD, but aeron/version.txt edited: the version is checked on its
# own, and the edit is a local change.
git_q -C "$dir/aeron" reset -q --hard "$at_pin"
printf '1.52.2\n' >"$dir/aeron/version.txt"
expect_fail "checked-out version.txt off the pin" "$dir" \
    'version 1.52.2     aeron/version.txt (checkout)' 'Aeron versions disagree: 1.52.2, 1.53.3' ' M version.txt'

# Fail closed on a checkout whose release cannot be read: at the pin but with
# no version.txt (a deleted tracked file, so a local change too), a repository
# with no HEAD, and files with no .git.
rm "$dir/aeron/version.txt"
expect_fail "checked-out submodule without version.txt" "$dir" \
    'aeron/ is checked out but has no version.txt' ' D version.txt'
dir=$(fixture no-head)
mkdir -p "$dir/aeron"
git_q -C "$dir/aeron" init -q
printf '1.53.3\n' >"$dir/aeron/version.txt"
expect_fail "checked-out submodule with no HEAD" "$dir" 'cannot read the HEAD of aeron/'
dir=$(fixture no-dotgit)
mkdir -p "$dir/aeron"
printf '1.53.3\n' >"$dir/aeron/version.txt"
expect_fail "aeron/ files without .git" "$dir" 'aeron/ has files but no .git'

# Fail closed: a bundling crate without aeron/version.txt.
dir=$(fixture no-stamp)
rm "$dir/crates/rusteron-client/aeron/version.txt"
expect_fail "rusteron crate without aeron/version.txt" "$dir" \
    'rusteron-client 0.2.11 has no' 'aeron/version.txt to read its Aeron version from'

# Fail closed when cargo cannot locate a rusteron crate offline: CI and a fresh
# machine have no registry until something is fetched. A registry dependency
# and an empty CARGO_HOME reproduce that without the network.
dir=$(fixture no-registry)
printf '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2021"\npublish = false\n\n[dependencies]\nrusteron-client = "=0.2.11"\n' >"$dir/app/Cargo.toml"
cat >"$dir/Cargo.lock" <<'EOF'
version = 4

[[package]]
name = "app"
version = "0.1.0"
dependencies = [
 "rusteron-client",
]

[[package]]
name = "rusteron-client"
version = "0.2.11"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "64491b3f092615e98d365c2398bd466aeba527e096bb8714545dc6d223d7eb06"
EOF
mkdir -p "$work/empty-cargo-home"
(
    export CARGO_HOME="$work/empty-cargo-home"
    expect_fail "rusteron crate not downloaded (no registry)" "$dir" \
        'cargo metadata --locked --offline failed for Cargo.lock' 'rusteron-client'
)

# Fail closed when cargo succeeds but does not resolve a lock's rusteron: a
# Cargo.lock left in a workspace member is ignored by cargo, which resolves
# the member from the workspace's lock, so the Aeron of the rusteron it lists
# cannot be read.
dir=$(fixture stranded-lock)
printf 'version = 4\n\n[[package]]\nname = "app"\nversion = "0.1.0"\ndependencies = [\n "rusteron-client",\n]\n\n[[package]]\nname = "rusteron-client"\nversion = "0.2.10"\n' >"$dir/app/Cargo.lock"
expect_fail "lock cargo does not resolve from (stranded in a workspace member)" "$dir" \
    'cargo metadata did not resolve rusteron-client 0.2.10 from app/Cargo.lock'

# Fail closed: Cargo.lock missing, unreadable or without rusteron; no gitlink;
# the pin file missing, malformed, holding two pins or none.
dir=$(fixture no-lock)
rm "$dir/Cargo.lock"
expect_fail "Cargo.lock missing" "$dir" 'Cargo.lock is missing'
dir=$(fixture bad-lock)
printf 'not toml [[\n' >"$dir/Cargo.lock"
expect_fail "Cargo.lock unreadable" "$dir" 'cannot read Cargo.lock'
dir=$(fixture no-rusteron)
printf '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2021"\npublish = false\n' >"$dir/app/Cargo.toml"
cargo generate-lockfile --offline --quiet --manifest-path "$dir/Cargo.toml"
expect_fail "Cargo.lock without rusteron" "$dir" 'Cargo.lock lists no rusteron crate'
dir=$(fixture code-gen-only)
printf '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2021"\npublish = false\n\n[dependencies]\nrusteron-code-gen = { path = "../crates/rusteron-code-gen" }\n' >"$dir/app/Cargo.toml"
cargo generate-lockfile --offline --quiet --manifest-path "$dir/Cargo.toml"
expect_fail "Cargo.lock with no Aeron-bundling rusteron" "$dir" 'no rusteron crate in Cargo.lock bundles Aeron'
dir=$(fixture no-gitlink)
git_q -C "$dir" update-index --force-remove aeron
expect_fail "aeron gitlink missing" "$dir" 'no aeron gitlink in the git index'
dir=$(fixture no-pin)
rm "$dir/scripts/aeron-pin.txt"
expect_fail "pin file missing" "$dir" 'scripts/aeron-pin.txt is missing'
malformed='scripts/aeron-pin.txt must hold exactly one "<version> <40-hex commit>" line'
printf '1.53.3 %s\n' "${pin_sha:0:12}" >"$dir/scripts/aeron-pin.txt"
expect_fail "pin with a short commit" "$dir" "$malformed"
printf '1.53.3 %s\n1.53.3 %s\n' "$pin_sha" "$pin_sha" >"$dir/scripts/aeron-pin.txt"
expect_fail "pin file with two pins" "$dir" "$malformed"
printf '# no pin\n' >"$dir/scripts/aeron-pin.txt"
expect_fail "pin file without a pin" "$dir" 'scripts/aeron-pin.txt holds no pin line'

# A standalone crate's own Cargo.lock (the samples' are gitignored and resolved
# by `just build`) counts too: sharing the root's rusteron passes, resolving a
# rusteron that bundles another Aeron fails, naming that lock.
dir=$(fixture sample)
mkdir -p "$dir/samples/demo/src"
: >"$dir/samples/demo/src/lib.rs"
sample_manifest() { # dir rusteron-client-path
    printf '[package]\nname = "demo"\nversion = "0.1.0"\nedition = "2021"\npublish = false\n\n[workspace]\n\n[dependencies]\nrusteron-client = { path = "%s" }\n' "$2" >"$1/samples/demo/Cargo.toml"
    cargo generate-lockfile --offline --quiet --manifest-path "$1/samples/demo/Cargo.toml"
}
sample_manifest "$dir" ../../crates/rusteron-client
expect_pass "sample lock on the same rusteron" "$dir"
grep -F -q 'samples/demo/Cargo.lock' <<<"$out" || { echo "pass output does not list the sample's lock: $out" >&2; exit 1; }
crate "$dir/samples/demo/vendor" rusteron-client 0.2.12 1.54.0
sample_manifest "$dir" vendor/crates/rusteron-client
expect_fail "sample lock bundling another Aeron" "$dir" \
    'version 1.54.0     rusteron-client 0.2.12 (samples/demo/Cargo.lock)' 'Aeron versions disagree: 1.53.3, 1.54.0'

echo "test-aeron-version: PASS (7 consistent fixtures pass; 25 drifts caught)"
