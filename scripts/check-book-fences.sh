#!/usr/bin/env bash
# check-book-fences.sh — inventory every `rust,ignore` fence in book/ and
# report the file:line and snippet. Does NOT parse the fence content (that
# requires an SBE-aware tool); it lists what a human must re-verify before
# every release per CLAUDE.md "rust,ignore recheck" policy.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
count=0
echo "=== rust,ignore fence inventory ==="
while IFS= read -r -d '' file; do
    # Count fences per file
    fences=$(grep -c '```rust,ignore' "$file" 2>/dev/null || true)
    if [ "$fences" -gt 0 ] 2>/dev/null; then
        echo ""
        echo "--- $file ($fences fence(s)) ---"
        grep -n '```rust,ignore' "$file" | while read -r line; do
            echo "  $line"
        done
        count=$((count + fences))
    fi
done < <(
    find "$root/book" -name '*.md' -print0 2>/dev/null
    find "$root/docs" -name '*.md' -print0 2>/dev/null
    find "$root" -maxdepth 1 -name '*.md' -print0 2>/dev/null
)

echo ""
echo "Total: $count rust,ignore fences"
echo "Re-verify before every release (CLAUDE.md § release process)."
echo ""
if [ $# -gt 0 ] && [ "$1" = "--ci" ]; then
    # CI mode: report fence count, exit 0 (the CI check just ensures
    # the inventory doesn't rot — actual verification is manual at
    # release time per CLAUDE.md).
    exit 0
fi
