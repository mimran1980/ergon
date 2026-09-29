#!/usr/bin/env bash
# The book-content check must fail when with_domain_objects(true) sits inside
# a fence, including when it is not on the line after the opening fence.
# Prose that only mentions the call must pass. A line-oriented grep of the
# old pattern does not see this fixture, so putting that grep back fails here.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-book-content.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/bad" "$fixture/prose"
cat > "$fixture/bad/page.md" <<'EOF'
# Generator

```rust
let cfg = GenerationConfig::default();
cfg.with_domain_objects(true);
```
EOF

cat > "$fixture/prose/page.md" <<'EOF'
The old call `with_domain_objects(true)` is obsolete.

```rust
let cfg = GenerationConfig::default();
cfg.with_domain_objects(DomainVarData::Bytes);
```
EOF

if BOOK_CONTENT_ROOT="$fixture/bad" "$checker" >"$fixture/bad.out" 2>&1; then
    echo "expected a fenced with_domain_objects(true) to fail" >&2
    cat "$fixture/bad.out" >&2
    exit 1
fi
if ! grep -F -q 'with_domain_objects(true) inside a fence' "$fixture/bad.out"; then
    echo "fence failure did not name the call:" >&2
    cat "$fixture/bad.out" >&2
    exit 1
fi

BOOK_CONTENT_ROOT="$fixture/prose" "$checker" >"$fixture/prose.out"
grep -F -q 'check-book-content: PASS' "$fixture/prose.out"

"$checker" >"$fixture/real.out"
grep -F -q 'check-book-content: PASS' "$fixture/real.out"
echo "test-book-content: PASS"
