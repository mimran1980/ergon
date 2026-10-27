#!/usr/bin/env bash
# The book-content check must fail when with_domain_objects(true) sits inside
# a fence, including when it is not on the line after the opening fence.
# Prose that only mentions the call must pass. A line-oriented grep of the
# old pattern does not see this fixture, so putting that grep back fails here.
# Missing {{#include}} files and anchors, dangling relative links, and dead
# absolute links into the repository or book, fail.
# check-repo-links: fixtures (the dead links below are written on purpose)
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-book-content.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
# Fixture books link into an empty repository, so only the fixture is checked.
mkdir -p "$fixture/empty-repo"
export LINK_REPO_ROOT="$fixture/empty-repo"

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

# Absolute links into the repository (GitHub blob/tree URLs) and the book,
# relative links in Markdown outside the book, and benchmark anchors cited in
# source must resolve against the working tree. A git submodule, or any
# other checkout inside the tree (a worktree), is not scanned. mdbook and
# GitHub both render a dead one without a warning.
make_repo() {
    local repo="$fixture/abs/$1"
    mkdir -p "$repo/book/src/guide" "$repo/docs" "$repo/src" "$repo/vendor/docs"
    printf '[submodule "vendor"]\n\tpath = vendor\n' >"$repo/.gitmodules"
    printf '# Summary\n\n[Intro](./intro.md)\n- [Guide](guide/page.md)\n' >"$repo/book/src/SUMMARY.md"
    printf '# Intro\n' >"$repo/book/src/intro.md"
    printf '# Guide\n\n## Tune the host\n' >"$repo/book/src/guide/page.md"
    printf '# Bench\n\n## SMT sibling cost (x86)\n\n## `uring-sqpoll`, own core\n' >"$repo/docs/BENCHMARKS.md"
    printf 'fn a() {}\nfn b() {}\n' >"$repo/src/a.rs"
    printf '[gone](https://github.com/mimran1980/ergon/blob/main/missing.md)\n' >"$repo/vendor/docs/x.md"
    # Another checkout inside the tree (a git worktree: a .git file) is its
    # own repository, never scanned as part of this one.
    mkdir -p "$repo/wt/docs"
    printf 'gitdir: /elsewhere\n' >"$repo/wt/.git"
    printf '[gone](https://github.com/mimran1980/ergon/blob/main/missing.md)\n' >"$repo/wt/docs/x.md"
}
make_repo ok
repo="$fixture/abs/ok"
cat >"$repo/README.md" <<'EOF'
[cost](https://github.com/mimran1980/ergon/blob/main/docs/BENCHMARKS.md#smt-sibling-cost-x86),
[own](https://github.com/mimran1980/ergon/blob/main/docs/BENCHMARKS.md#uring-sqpoll-own-core),
[docs](https://github.com/mimran1980/ergon/tree/main/docs), [lines](https://github.com/mimran1980/ergon/blob/main/src/a.rs#L1-L2),
[book](https://mimran1980.github.io/ergon/), [tune](https://mimran1980.github.io/ergon/guide/page.html#tune-the-host),
[relative](docs/BENCHMARKS.md#smt-sibling-cost-x86)
EOF
printf '// See docs/BENCHMARKS.md#smt-sibling-cost-x86 and BENCHMARKS.md#uring-sqpoll-own-core.\n' >"$repo/docs/cite.rs"
LINK_REPO_ROOT="$repo" BOOK_CONTENT_ROOT="$repo/book/src" "$checker" >"$fixture/abs-ok.out"
grep -F -q 'check-book-content: PASS' "$fixture/abs-ok.out"

make_repo file
printf '[gone](https://github.com/mimran1980/ergon/blob/main/docs/BENCHMARK.md)\n' >"$fixture/abs/file/README.md"
make_repo anchor
printf '[gone](https://github.com/mimran1980/ergon/blob/main/docs/BENCHMARKS.md#experiment-log)\n' >"$fixture/abs/anchor/book/src/guide/link.md"
make_repo code
printf '/// Evidence: docs/BENCHMARKS.md#round-6-recommended-configuration\nfn c() {}\n' >"$fixture/abs/code/src/c.rs"
make_repo book
printf 'See https://mimran1980.github.io/ergon/guide/page.html#tune-the-nic.\n' >"$fixture/abs/book/docs/README.md"
make_repo relative
printf '[log](BENCHMARKS.md#experiment-log)\n' >"$fixture/abs/relative/docs/README.md"
for case in file anchor code book relative; do
    repo="$fixture/abs/$case"
    if LINK_REPO_ROOT="$repo" BOOK_CONTENT_ROOT="$repo/book/src" "$checker" >"$fixture/abs-$case.out" 2>&1; then
        echo "expected a dead absolute link ($case) to fail" >&2
        cat "$fixture/abs-$case.out" >&2
        exit 1
    fi
    if grep -F -q 'vendor/' "$fixture/abs-$case.out"; then
        echo "a submodule's links were checked ($case)" >&2
        exit 1
    fi
    if grep -F -q 'wt/' "$fixture/abs-$case.out"; then
        echo "a nested checkout's links were checked ($case)" >&2
        exit 1
    fi
done
grep -F -q 'docs/BENCHMARK.md does not exist' "$fixture/abs-file.out"
grep -F -q 'no heading #experiment-log' "$fixture/abs-anchor.out"
grep -F -q 'src/c.rs:1: docs/BENCHMARKS.md#round-6-recommended-configuration: no heading' "$fixture/abs-code.out"
grep -F -q 'no heading #tune-the-nic' "$fixture/abs-book.out"
grep -F -q 'docs/README.md:1: BENCHMARKS.md#experiment-log: no heading' "$fixture/abs-relative.out"

env -u LINK_REPO_ROOT "$checker" >"$fixture/real.out"
grep -F -q 'check-book-content: PASS' "$fixture/real.out"
echo "test-book-content: PASS"
