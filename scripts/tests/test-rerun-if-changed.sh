#!/usr/bin/env bash
# A missing literal rerun-if-changed path fails. The aeron build directory
# does not: it lives under the submodule and is produced outside git.
# Creating the missing file makes the same fixture pass. This does not use
# git ls-files, so the fixture can stay untracked.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-rerun-if-changed.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/app"
cat > "$fixture/.gitmodules" <<'EOF'
[submodule "aeron"]
	path = aeron
EOF
cat > "$fixture/app/build.rs" <<'EOF'
fn main() {
    println!("cargo::rerun-if-changed=missing.txt");
    println!("cargo::rerun-if-changed=../aeron/aeron-all/build/libs");
    println!("cargo::rerun-if-changed={}", "not-a-literal");
}
EOF

if RERUN_ROOT="$fixture" "$checker" >"$fixture/out" 2>&1; then
    echo "expected the missing literal path to fail" >&2
    cat "$fixture/out" >&2
    exit 1
fi
if ! grep -F -q 'watches missing.txt' "$fixture/out"; then
    echo "failure did not name missing.txt:" >&2
    cat "$fixture/out" >&2
    exit 1
fi
if grep -F -q 'aeron/aeron-all/build/libs' "$fixture/out"; then
    echo "submodule path was not exempt:" >&2
    cat "$fixture/out" >&2
    exit 1
fi

printf 'present\n' > "$fixture/app/missing.txt"
RERUN_ROOT="$fixture" "$checker" >/dev/null

# The real tree has no RERUN_ROOT, so this sees tracked build.rs files only.
"$checker" >/dev/null
echo "test-rerun-if-changed: PASS"
