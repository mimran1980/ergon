#!/usr/bin/env bash
# The book-content check must fail when with_domain_objects(true) sits inside
# a fence, including when it is not on the line after the opening fence.
# Prose that only mentions the call must pass. A line-oriented grep of the
# old pattern does not see this fixture, so putting that grep back fails here.
# Missing {{#include}} files and anchors, and dangling relative links, fail.
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

# An {{#include}} whose file or anchor is gone must fail: mdbook (0.4 and 0.5)
# renders an empty block and exits 0, and the SBE fence harness skips
# `rust,ignore` and non-Rust fences, so nothing else notices.
mkdir -p "$fixture/includes/ok" "$fixture/includes/anchor" "$fixture/includes/file"
cat > "$fixture/includes/code.rs" <<'EOF'
// ANCHOR: present
fn present() {}
// ANCHOR_END: present
EOF
cat > "$fixture/includes/ok/page.md" <<'EOF'
```rust,ignore
{{#include ../code.rs:present}}
```

```rust,ignore
{{#include ../code.rs}}
```

```rust,ignore
{{#include ../code.rs:2:3}}
```
EOF
cat > "$fixture/includes/anchor/page.md" <<'EOF'
```rust,ignore
{{#include ../code.rs:renamed}}
```
EOF
cat > "$fixture/includes/file/page.md" <<'EOF'
```yaml
{{#include ../moved.yaml}}
```
EOF
BOOK_CONTENT_ROOT="$fixture/includes/ok" "$checker" >"$fixture/include-ok.out"
grep -F -q 'check-book-content: PASS' "$fixture/include-ok.out"
for case in anchor file; do
    if BOOK_CONTENT_ROOT="$fixture/includes/$case" "$checker" >"$fixture/include-$case.out" 2>&1; then
        echo "expected an {{#include}} with a missing $case to fail" >&2
        cat "$fixture/include-$case.out" >&2
        exit 1
    fi
done
grep -F -q "anchor 'renamed' not found" "$fixture/include-anchor.out"
grep -F -q 'moved.yaml' "$fixture/include-file.out"

# A relative link to a page or file that is gone, or to a heading that is
# not on the page, must fail: mdbook builds both without a warning. A link
# inside code, an absolute URL and a repeated heading's -1 id pass.
for case in ok file anchor ref; do
    mkdir -p "$fixture/links/$case/deploy"
    printf '# Clocks\n\n## Set up time sync\n\n## Set up time sync\n' >"$fixture/links/$case/deploy/clocks.md"
done
cat > "$fixture/links/ok/deploy/page.md" <<'EOF'
[time sync](clocks.md#set-up-time-sync), [again](clocks.md#set-up-time-sync-1),
[html](clocks.html), [here](#same-page), [web](https://example.com/clock/overview.md)

## Same page

`[in code](../clock/overview.md)`

```text
[in a fence](../clock/wall-clock.md)
```
EOF
cat > "$fixture/links/file/deploy/page.md" <<'EOF'
Configuration: [time sync](../clock/wall-clock.md).
EOF
cat > "$fixture/links/anchor/deploy/page.md" <<'EOF'
[time sync](clocks.md#keep-the-wall-clock-right)
EOF
cat > "$fixture/links/ref/deploy/page.md" <<'EOF'
See [the clock][clock].

[clock]: ../clock/overview.md
EOF
BOOK_CONTENT_ROOT="$fixture/links/ok" "$checker" >"$fixture/links-ok.out"
grep -F -q 'check-book-content: PASS' "$fixture/links-ok.out"
for case in file anchor ref; do
    if BOOK_CONTENT_ROOT="$fixture/links/$case" "$checker" >"$fixture/links-$case.out" 2>&1; then
        echo "expected a dangling link ($case) to fail" >&2
        cat "$fixture/links-$case.out" >&2
        exit 1
    fi
done
grep -F -q 'link ../clock/wall-clock.md' "$fixture/links-file.out"
grep -F -q 'no heading #keep-the-wall-clock-right' "$fixture/links-anchor.out"
grep -F -q 'link ../clock/overview.md' "$fixture/links-ref.out"

"$checker" >"$fixture/real.out"
grep -F -q 'check-book-content: PASS' "$fixture/real.out"
echo "test-book-content: PASS"
