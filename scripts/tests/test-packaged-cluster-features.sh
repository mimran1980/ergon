#!/usr/bin/env bash
# Prove check-packaged-cluster-features.sh fails closed when an advertised
# feature cannot be built from the packed crate.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-packaged-cluster-features.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/src"
cat >"$fixture/Cargo.toml" <<'EOF'
[package]
name = "packaged-feature-fixture"
version = "0.1.0"
edition = "2021"
publish = false

[features]
default = []
explode = []
EOF
cat >"$fixture/src/lib.rs" <<'EOF'
pub fn ok() -> u8 { 1 }
EOF
cat >"$fixture/build.rs" <<'EOF'
fn main() {
    if std::env::var_os("CARGO_FEATURE_EXPLODE").is_some() {
        panic!("advertised explode feature is not buildable from this package");
    }
}
EOF

if output=$("$checker" --unpack --package packaged-feature-fixture --manifest "$fixture/Cargo.toml" 2>&1); then
    echo "expected packaged-feature check to fail closed, got success:" >&2
    echo "$output" >&2
    exit 1
fi
if [[ "$output" != *"advertised explode feature is not buildable"* ]]; then
    echo "wrong failure; expected the explode build.rs panic, got:" >&2
    echo "$output" >&2
    exit 1
fi

ok_fixture=$(mktemp -d)
trap 'rm -rf "$fixture" "$ok_fixture"' EXIT
mkdir -p "$ok_fixture/src"
cat >"$ok_fixture/Cargo.toml" <<'EOF'
[package]
name = "packaged-feature-ok"
version = "0.1.0"
edition = "2021"
publish = false
EOF
cat >"$ok_fixture/src/lib.rs" <<'EOF'
pub fn ok() -> u8 { 1 }
EOF
if ! output=$("$checker" --unpack --package packaged-feature-ok --manifest "$ok_fixture/Cargo.toml" 2>&1); then
    echo "expected packaged-feature-ok unpack to succeed, got:" >&2
    echo "$output" >&2
    exit 1
fi

echo "test-packaged-cluster-features: PASS (fail-closed unpack + positive pack-and-build)"
