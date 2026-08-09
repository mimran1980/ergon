#!/usr/bin/env bash
# check-book-fences.sh — enforce that every `rust,ignore` fence in book/
# is accounted for in the allowlist, and that every allowlist entry still
# exists at the declared line. Additions, removals, or line moves fail.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
allowlist="$root/scripts/book-fence-allowlist.txt"

# Collect actual fences: "path:line" sorted
actual=$(mktemp)
trap 'rm -f "$actual"' EXIT
find "$root/book" -name '*.md' -print0 | while IFS= read -r -d '' f; do
    rel="${f#"$root/"}"
    grep -n '```rust,ignore' "$f" 2>/dev/null | while IFS=: read -r ln _; do
        echo "$rel:$ln"
    done || true   # grep exits 1 on no-match; pipefail would otherwise abort the whole find pipeline
done | sort > "$actual"

# Collect allowlisted fences (skip comment/blank lines)
allowed=$(mktemp)
trap 'rm -f "$actual" "$allowed"' EXIT
grep -v '^\s*#' "$allowlist" | grep -v '^\s*$' | while IFS= read -r line; do
    echo "${line%% *}"
done | sort > "$allowed"

# Fences present but not allowlisted (additions)
added=$(comm -23 "$actual" "$allowed" || true)
# Allowlisted but not present (removals or stale line numbers)
removed=$(comm -13 "$actual" "$allowed" || true)

rc=0
if [ -n "$added" ]; then
    echo "ERROR: new unallowlisted rust,ignore fence(s):"
    echo "$added" | sed 's/^/  /'
    echo "Add each to scripts/book-fence-allowlist.txt with a rationale."
    rc=1
fi
if [ -n "$removed" ]; then
    echo "ERROR: allowlist entry no longer matches a fence (removed or moved):"
    echo "$removed" | sed 's/^/  /'
    echo "Update or remove the entry in scripts/book-fence-allowlist.txt."
    rc=1
fi
if [ $rc -eq 0 ]; then
    echo "check-book-fences: PASS ($(wc -l < "$actual") fences, all allowlisted)"
fi
exit $rc
