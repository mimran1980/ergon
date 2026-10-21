#!/usr/bin/env bash
# check-book-content.sh — grep book code fences for known stale/incorrect API
# patterns that the allowlist alone doesn't catch, and check every relative
# link and {{#include}} resolves. Called from release-check, CI and book-ci.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
book_src=${BOOK_CONTENT_ROOT:-$root/book/src}
rc=0

check_pattern() {
    local pattern="$1"
    local message="$2"
    if grep -rn "$pattern" "$book_src/" --include='*.md' | grep -v 'check-book-content\|allowlist' > /dev/null 2>&1; then
        echo "  FAIL: $message"
        grep -rn "$pattern" "$book_src/" --include='*.md' | grep -v 'check-book-content\|allowlist'
        rc=1
    fi
}

echo "=== book content check ==="

# Deprecated chrono: NaiveDateTime::from_timestamp_opt
check_pattern 'NaiveDateTime::from_timestamp_opt' \
    'NaiveDateTime::from_timestamp_opt is deprecated — use DateTime::from_timestamp(...)'

# Deprecated chrono: NaiveDateTime::timestamp (not DateTime<Utc>::timestamp)
check_pattern 'naive.*\.timestamp()' \
    'NaiveDateTime::timestamp() is deprecated — use .and_utc().timestamp()'

# Old API inside a fence. A line-oriented grep cannot see this: the call is
# not required to sit on the line after the opening fence, and macOS grep
# has no multiline mode. Prose mentioning the call is not a failure.
if ! python3 - "$book_src" <<'PY'
import pathlib, sys

root = pathlib.Path(sys.argv[1])
failed = False
for path in sorted(root.rglob("*.md")):
    lines = path.read_text(errors="replace").splitlines()
    in_fence = False
    body = []
    start = 0
    for number, line in enumerate(lines, 1):
        if line.lstrip().startswith("```"):
            if in_fence:
                if "with_domain_objects(true)" in "\n".join(body):
                    print(f"  FAIL: {path}:{start}: with_domain_objects(true) inside a fence")
                    failed = True
                body = []
                in_fence = False
            else:
                in_fence = True
                start = number
                body = []
        elif in_fence:
            body.append(line)
    if in_fence and "with_domain_objects(true)" in "\n".join(body):
        print(f"  FAIL: {path}:{start}: with_domain_objects(true) inside a fence")
        failed = True
sys.exit(1 if failed else 0)
PY
then
    rc=1
fi

# Every {{#include}} / {{#rustdoc_include}} must name a file that exists and,
# for a named anchor, an ANCHOR / ANCHOR_END pair in it. mdbook renders a
# missing one as an empty block and still exits 0, and the SBE fence harness
# skips `rust,ignore` and non-Rust fences, so a renamed anchor or moved file
# would otherwise blank a book example silently.
if ! python3 - "$book_src" <<'PY'
import pathlib, re, sys

root = pathlib.Path(sys.argv[1])
directive = re.compile(r"(?<!\\)\{\{#(?:include|rustdoc_include)\s+([^}\s]+)\s*\}\}")
lines_only = re.compile(r"\d*(:\d*)?")
failed = False
for page in sorted(root.rglob("*.md")):
    for number, line in enumerate(page.read_text(errors="replace").splitlines(), 1):
        for match in directive.finditer(line):
            target, _, anchor = match.group(1).partition(":")
            source = page.parent / target
            where = f"{page}:{number}: {{{{#include {match.group(1)}}}}}"
            if not source.is_file():
                print(f"  FAIL: {where}: {source} does not exist")
                failed = True
                continue
            if not anchor or lines_only.fullmatch(anchor):
                continue
            name = re.escape(anchor)
            text = source.read_text(errors="replace").splitlines()
            start = [i for i, l in enumerate(text) if re.search(rf"ANCHOR:\s*{name}(?![\w-])", l)]
            end = [i for i, l in enumerate(text) if re.search(rf"ANCHOR_END:\s*{name}(?![\w-])", l)]
            if not start:
                print(f"  FAIL: {where}: anchor '{anchor}' not found in {source}")
                failed = True
            elif not [e for e in end if e > start[0]]:
                print(f"  FAIL: {where}: anchor '{anchor}' has no ANCHOR_END after it in {source}")
                failed = True
sys.exit(1 if failed else 0)
PY
then
    rc=1
fi

# Every relative link must name a file that exists and, with a #fragment into
# a page, a heading (or explicit id) on it. mdbook builds a link to a deleted
# page or a renamed heading without a warning, so a removed chapter would
# otherwise leave dead links behind. Links in code, absolute URLs and
# {{#include}} (checked above) are not links.
if ! python3 - "$book_src" <<'PY'
import html, pathlib, re, sys
from urllib.parse import unquote

root = pathlib.Path(sys.argv[1])
inline = re.compile(r"!?\[(?:[^\[\]]|\[[^\]]*\])*\]\(\s*<?([^)\s>]+)>?(?:\s+\"[^\"]*\")?\s*\)")
refdef = re.compile(r"^\s{0,3}\[[^\]]+\]:\s*<?(\S+?)>?(?:\s|$)")
html_href = re.compile(r"""(?:href|src)\s*=\s*["']([^"']+)["']""")
scheme = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")
fence_mark = ("```", "~~~")


def slug(text):
    """mdbook's heading id: markup dropped, lower case, alphanumerics, '-'
    and '_' kept, whitespace to '-'."""
    text = re.sub(r"<[^>]+>", "", text).replace("`", "")
    text = html.unescape(re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text))
    return "".join(c if c.isalnum() or c in "-_" else "-" if c.isspace() else ""
                   for c in text.strip().lower())


ids_of = {}


def ids(page):
    """Every id a link can name on <page>: its headings' (a repeat gets -1,
    -2, ...), explicit {#id}s and HTML id/name attributes."""
    if page not in ids_of:
        found, seen, fence = set(), {}, False
        for line in page.read_text(errors="replace").splitlines():
            if line.lstrip().startswith(fence_mark):
                fence = not fence
                continue
            if fence:
                continue
            heading = re.match(r"^\s{0,3}#{1,6}\s+(.*?)\s*#*\s*$", line)
            if heading:
                text = heading.group(1)
                explicit = re.search(r"\{#([^}\s]+)[^}]*\}\s*$", text)
                if explicit:
                    found.add(explicit.group(1))
                    text = text[: explicit.start()]
                base = slug(text)
                n = seen.get(base, 0)
                found.add(base if n == 0 else f"{base}-{n}")
                seen[base] = n + 1
            found.update(re.findall(r"""\b(?:id|name)\s*=\s*["']([^"']+)["']""", line))
        ids_of[page] = found
    return ids_of[page]


failed = False
for page in sorted(root.rglob("*.md")):
    fence = False
    for number, raw in enumerate(page.read_text(errors="replace").splitlines(), 1):
        if raw.lstrip().startswith(fence_mark):
            fence = not fence
            continue
        if fence:
            continue
        line = re.sub(r"(`+)(?:(?!\1).)*?\1", "", raw)
        targets = [m.group(1) for pattern in (inline, refdef, html_href) for m in pattern.finditer(line)]
        for target in targets:
            if scheme.match(target) or target.startswith(("//", "{{")):
                continue
            path, _, fragment = target.partition("#")
            path = unquote(path)
            where = f"{page}:{number}: link {target}"
            if path.startswith("/"):
                print(f"  FAIL: {where}: absolute path; link relative to the page")
                failed = True
                continue
            dest = page.parent / path if path else page
            if path.endswith(".html") and dest.with_suffix(".md").is_file():
                dest = dest.with_suffix(".md")
            if not dest.exists():
                print(f"  FAIL: {where}: {dest} does not exist")
                failed = True
            elif fragment and dest.suffix == ".md" and fragment not in ids(dest):
                print(f"  FAIL: {where}: no heading #{fragment} in {dest}")
                failed = True
sys.exit(1 if failed else 0)
PY
then
    rc=1
fi

# CString::new in code fences — must use c"…" literals
check_pattern 'CString::new' \
    'CString::new is forbidden — use c"…" literals or cformat!'

# Stale generation-config default model (T-8).
check_pattern 'All knobs default to `true`' \
    'GenerationConfig knobs do not all default to true — list enabled vs disabled defaults'

# Stale ParseError variants that were never shipped (T-7).
check_pattern 'ParseError::Unsupported' \
    'ParseError has no Unsupported variant'
check_pattern 'schema_parse::unsupported' \
    'ParseError has no unsupported diagnostic code'

# Trust-boundary drift: wrap does not return Result.
check_pattern 'wrap` return `Result' \
    'bare wrap panics if short — it does not return Result'

# Copied point estimates (quote a run id instead).
check_pattern '22-23%' \
    'do not copy bulk_add latency percentages into the book'


if [ $rc -eq 0 ]; then
    echo "check-book-content: PASS"
else
    echo "check-book-content: FAIL — fix the patterns above"
fi
exit $rc
