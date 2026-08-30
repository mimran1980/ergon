#!/usr/bin/env bash
# Prove the CI network/coverage/fuzz traps stay closed.
#
# 1. The coverage job must have a timeout long enough for instrumented
#    compile-and-run tests (30m cancelled the 0.1.23 run).
# 2. Scratch-crate cargo must not force --offline / CARGO_NET_OFFLINE:
#    llvm-cov and a cold CI cache then fail downloading transitives
#    (bit-set, ahash) that are not in the workspace lock.
# 3. Nightly cargo-fuzz must pin the GNU triple. cargo-fuzz's Linux default
#    is musl+ASAN, which fails on a minimal nightly without that target.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-ci-network-and-coverage.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

expect_failure() {
    local expected=$1
    shift
    if output=$("$@" 2>&1); then
        echo "expected CI-network check failure containing: $expected" >&2
        exit 1
    fi
    if [[ "$output" != *"$expected"* ]]; then
        echo "wrong CI-network check failure; expected '$expected', got:" >&2
        echo "$output" >&2
        exit 1
    fi
}

mkdir -p "$fixture/.github/workflows" "$fixture/sbe/tests/common"

cat >"$fixture/.github/workflows/ci.yml" <<'EOF'
  coverage:
    timeout-minutes: 90
  test:
    timeout-minutes: 90
EOF
cat >"$fixture/.github/workflows/nightly.yml" <<'EOF'
        run: cd sbe/fuzz && cargo +nightly fuzz run --target x86_64-unknown-linux-gnu schema_parse
EOF
cat >"$fixture/sbe/tests/common/mod.rs" <<'EOF'
    cmd.env_remove("CARGO_NET_OFFLINE");
    let out = Command::new("cargo").args(["run"]);
EOF
cat >"$fixture/sbe/tests/common/encoded_length_matrix.rs" <<'EOF'
    let output = Command::new("cargo").args(["test"]);
EOF
cat >"$fixture/sbe/tests/proptest_roundtrip.rs" <<'EOF'
    let out = Command::new("cargo").args(["test"]);
EOF

"$checker" --root "$fixture"

cat >"$fixture/.github/workflows/ci.yml" <<'EOF'
  coverage:
    timeout-minutes: 30
EOF
expect_failure "coverage timeout-minutes must be >= 90" \
    "$checker" --root "$fixture"

cat >"$fixture/.github/workflows/ci.yml" <<'EOF'
  coverage:
    timeout-minutes: 90
EOF
cat >"$fixture/sbe/tests/proptest_roundtrip.rs" <<'EOF'
    .args(["test", "--offline"])
    .env("CARGO_NET_OFFLINE", "true")
EOF
expect_failure "must not force cargo offline" \
    "$checker" --root "$fixture"

cat >"$fixture/sbe/tests/proptest_roundtrip.rs" <<'EOF'
    let out = Command::new("cargo").args(["test"]);
EOF
cat >"$fixture/.github/workflows/nightly.yml" <<'EOF'
        run: cd sbe/fuzz && cargo +nightly fuzz run schema_parse -- -max_total_time=600
EOF
expect_failure "must pin x86_64-unknown-linux-gnu" \
    "$checker" --root "$fixture"

echo "CI network/coverage self-test: PASS"

# Real tree — this is the gate the policy job runs.
"$checker" --root "$repo_root"
echo "CI network/coverage live tree: PASS"
