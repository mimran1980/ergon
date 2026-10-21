#!/usr/bin/env bash
# The stale-ignore check must fail when an advisory names a crate that no
# lockfile contains, and pass once that ignore is gone.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-stale-deny-ignores.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/empty/pkg"
printf '%s\n' '[package]' 'name = "empty"' 'version = "0.0.0"' > "$fixture/empty/pkg/Cargo.toml"
printf '%s\n' 'version = 3' '[[package]]' 'name = "empty"' 'version = "0.0.0"' > "$fixture/empty/Cargo.lock"
cat > "$fixture/empty/deny.toml" <<'EOF'
[advisories]
ignore = [
    { id = "RUSTSEC-2026-0235", reason = "rkyv is an optional rust_decimal feature that this workspace never enables" },
]
EOF

if "$checker" "$fixture/empty" >"$fixture/out" 2>&1; then
    echo "expected the stale rkyv ignore to fail" >&2
    cat "$fixture/out" >&2
    exit 1
fi
if ! grep -q 'ignores rkyv' "$fixture/out"; then
    echo "failure did not name rkyv:" >&2
    cat "$fixture/out" >&2
    exit 1
fi

expect_failure() {
    local expected=$1
    if "$checker" "$fixture/empty" >"$fixture/out" 2>&1; then
        echo "expected stale-ignore failure containing: $expected" >&2
        exit 1
    fi
    grep -q "$expected" "$fixture/out" || { cat "$fixture/out" >&2; exit 1; }
}

# TOML key order, quoting, and representation must not bypass the check.
for entry in \
    '{ reason = "An optional feature", id = "RUSTSEC-2026-0235" }' \
    "{ id = 'RUSTSEC-2026-0235', reason = 'An optional feature' }" \
    '"RUSTSEC-2026-0235"'; do
    printf '[advisories]\nignore = [%s]\n' "$entry" > "$fixture/empty/deny.toml"
    expect_failure 'ignores rkyv'
done

# A real package entry permits the ignore; a comment mentioning it does not.
printf '\n# name = "rkyv"\n' >> "$fixture/empty/Cargo.lock"
expect_failure 'ignores rkyv'
printf '\n[[package]]\nname = "rkyv"\nversion = "0.7.0"\n' >> "$fixture/empty/Cargo.lock"
"$checker" "$fixture/empty" >/dev/null

printf '[advisories]\nignore = ["RUSTSEC-9999-9999"]\n' > "$fixture/empty/deny.toml"
expect_failure 'no crate mapping'

printf '%s\n' '[advisories]' 'ignore = [' ']' > "$fixture/empty/deny.toml"
"$checker" "$fixture/empty" >/dev/null

"$checker" "$repo_root" >/dev/null
echo "test-stale-deny-ignores: PASS"
