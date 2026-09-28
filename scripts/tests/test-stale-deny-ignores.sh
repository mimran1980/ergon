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

printf '%s\n' 'ignore = [' ']' > "$fixture/empty/deny.toml"
"$checker" "$fixture/empty" >/dev/null

"$checker" "$repo_root" >/dev/null
echo "test-stale-deny-ignores: PASS"
