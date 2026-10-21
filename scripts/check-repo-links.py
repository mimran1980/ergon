#!/usr/bin/env python3
"""Check absolute links into this repository and its book.

The book's relative links are checked by check-book-content.sh. This script
checks the rest:

* relative links in every other Markdown file (GitHub's rules);

* GitHub links into this repository (blob/main/PATH and tree/main/PATH,
  optionally with a fragment): PATH must exist in the working tree (a file
  for blob, a directory for tree), and a fragment must be a heading of the
  Markdown file (GitHub's anchor rules) or a line range (L10 or L10-L20)
  inside a source file;
* links into the published book (mimran1980.github.io/ergon/PAGE.html): PAGE
  must be a book source page and a fragment one of its headings (mdbook's
  rules); the bare site root is the first chapter in SUMMARY.md;
* in source files (Rust, shell, Python, YAML, TOML, C, Go, justfiles), a
  benchmark write-up cited with a fragment (a path ending in BENCHMARKS.md
  or BENCHMARK.md, then a hash and an anchor): the heading must exist in the
  file the path names (a bare name resolves upward from the citing file).

Scanned: the book source and every Markdown and source file under the
repository, except .git, target, node_modules, git submodules and a file
whose first 10 lines say "check-repo-links: fixtures" (a self-test that
writes dead links on purpose). Links are
checked inside code too: a reader follows a URL wherever it is printed.

Usage: check-repo-links.py REPO_ROOT BOOK_SRC
"""

import html
import pathlib
import re
import sys
from urllib.parse import unquote

REPO_URL = re.compile(
    r"https://github\.com/mimran1980/ergon/(blob|tree)/main(/[^\s)\]>\"'`|]*)?"
)
BOOK_URL = re.compile(r"https://mimran1980\.github\.io/ergon(/[^\s)\]>\"'`|]*)?")
BENCH_REF = re.compile(r"([\w./-]*BENCHMARKS?\.md)#([\w-]+)")
INLINE = re.compile(r"!?\[(?:[^\[\]]|\[[^\]]*\])*\]\(\s*<?([^)\s>]+)>?(?:\s+\"[^\"]*\")?\s*\)")
REFDEF = re.compile(r"^\s{0,3}\[[^\]]+\]:\s*<?(\S+?)>?(?:\s|$)")
HTML_HREF = re.compile(r"""(?:href|src)\s*=\s*["']([^"']+)["']""")
SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")
SOURCE_SUFFIXES = {".rs", ".sh", ".py", ".toml", ".yaml", ".yml", ".c", ".h", ".go", ".just"}
SOURCE_NAMES = {"justfile", "Justfile"}
SKIP_DIRS = {".git", "target", "node_modules"}
FENCE = ("```", "~~~")
FIXTURES = "check-repo-links: " + "fixtures"


def strip_markup(text):
    text = re.sub(r"<[^>]+>", "", text).replace("`", "")
    text = re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", text)
    return html.unescape(text)


def github_slug(text):
    """GitHub's heading anchor: rendered text, lower case, letters, digits,
    '-' and '_' kept, each space to '-', other punctuation dropped."""
    text = strip_markup(text).strip().lower()
    return "".join(c if c.isalnum() or c in "-_" else "-" if c == " " else "" for c in text)


def mdbook_slug(text):
    """mdbook's heading id (as check-book-content.sh computes it)."""
    text = strip_markup(text).strip().lower()
    return "".join(c if c.isalnum() or c in "-_" else "-" if c.isspace() else "" for c in text)


_ids = {}


def heading_ids(page, slug):
    key = (page, slug)
    if key not in _ids:
        found, seen, fence = set(), {}, False
        for line in page.read_text(errors="replace").splitlines():
            if line.lstrip().startswith(FENCE):
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
        _ids[key] = found
    return _ids[key]


def check_fragment(dest, fragment, slug):
    """None when <fragment> names something in <dest>, else the reason."""
    if not fragment:
        return None
    if dest.suffix.lower() in (".md", ".markdown"):
        if fragment.lower() in heading_ids(dest, slug):
            return None
        return f"no heading #{fragment} in {dest}"
    lines = re.fullmatch(r"L(\d+)(?:-L(\d+))?", fragment)
    if lines:
        count = len(dest.read_text(errors="replace").splitlines())
        last = int(lines.group(2) or lines.group(1))
        if 1 <= int(lines.group(1)) <= last <= count:
            return None
        return f"#{fragment} is past the end of {dest} ({count} lines)"
    return f"#{fragment} is not a line range (#L<n>) and {dest} is not Markdown"


def submodules(repo):
    gitmodules = repo / ".gitmodules"
    if not gitmodules.is_file():
        return set()
    return {
        (repo / m.group(1).strip()).resolve()
        for m in re.finditer(r"^\s*path\s*=\s*(.+)$", gitmodules.read_text(), re.M)
    }


def walk(repo):
    skip = submodules(repo)
    stack = [repo]
    while stack:
        here = stack.pop()
        for entry in sorted(here.iterdir()):
            if entry.is_dir():
                if entry.name in SKIP_DIRS or entry.resolve() in skip or entry.is_symlink():
                    continue
                stack.append(entry)
            elif entry.is_file():
                yield entry


def main():
    repo = pathlib.Path(sys.argv[1]).resolve()
    book = pathlib.Path(sys.argv[2]).resolve()
    markdown, sources = set(book.rglob("*.md")), set()
    for path in walk(repo):
        if path.suffix == ".md":
            markdown.add(path)
        elif path.suffix in SOURCE_SUFFIXES or path.name in SOURCE_NAMES:
            sources.add(path)

    failures = []

    def fail(where, url, reason):
        failures.append(f"  FAIL: {where}: {url}: {reason}")

    def check_repo_url(where, match):
        kind, rest = match.group(1), (match.group(2) or "/").rstrip(".,;:")
        path, _, fragment = rest.partition("#")
        dest = repo / unquote(path.strip("/"))
        url = match.group(0).rstrip(".,;:")
        if "{" in path:
            return
        if not dest.exists():
            fail(where, url, f"{dest.relative_to(repo)} does not exist")
        elif kind == "blob" and not dest.is_file():
            fail(where, url, "blob/ names a directory; use tree/")
        elif kind == "tree" and not dest.is_dir():
            fail(where, url, "tree/ names a file; use blob/")
        elif fragment:
            reason = check_fragment(dest, fragment, github_slug)
            if reason:
                fail(where, url, reason)

    def check_book_url(where, match):
        rest = (match.group(1) or "/").rstrip(".,;:")
        path, _, fragment = rest.partition("#")
        path = unquote(path.strip("/"))
        url = match.group(0).rstrip(".,;:")
        if path in ("", "index.html"):
            summary = book / "SUMMARY.md"
            first = re.search(r"\]\(\s*(?:\./)?([^)\s]+\.md)\s*\)", summary.read_text()) if summary.is_file() else None
            candidates = [book / first.group(1)] if first else [book / "README.md"]
        elif path.endswith(".html"):
            candidates = [book / (path[: -len(".html")] + ".md")]
        else:
            candidates = [book / path]
        dest = next((c for c in candidates if c.exists()), None)
        if dest is None:
            fail(where, url, f"no book page {candidates[0]}")
        elif fragment and dest.suffix == ".md":
            reason = check_fragment(dest, fragment, mdbook_slug)
            if reason:
                fail(where, url, reason)

    def check_bench_ref(source, where, match):
        name, fragment = match.group(1), match.group(2)
        dest = None
        if "/" in name:
            dest = repo / name.lstrip("./")
        else:
            here = source.parent
            while True:
                if (here / name).is_file():
                    dest = here / name
                    break
                if here == repo or here == here.parent:
                    break
                here = here.parent
        if dest is None or not dest.is_file():
            fail(where, match.group(0), f"{name} not found")
            return
        reason = check_fragment(dest, fragment, github_slug)
        if reason:
            fail(where, match.group(0), reason)

    def check_relative(page, where, target):
        path, _, fragment = target.partition("#")
        path = unquote(path)
        if path.startswith("/"):
            fail(where, target, "root-relative link; link relative to the file")
            return
        dest = page.parent / path if path else page
        if not dest.exists():
            fail(where, target, f"{dest} does not exist")
        elif fragment and dest.is_file():
            reason = check_fragment(dest, fragment, github_slug)
            if reason:
                fail(where, target, reason)

    for path in sorted(markdown):
        in_book = book in path.parents
        fence = False
        for number, line in enumerate(path.read_text(errors="replace").splitlines(), 1):
            where = f"{path}:{number}"
            if line.lstrip().startswith(FENCE):
                fence = not fence
            elif not in_book and not fence:
                # The book's relative links follow mdbook's rules and are
                # checked by check-book-content.sh; elsewhere GitHub renders.
                prose = re.sub(r"(`+)(?:(?!\1).)*?\1", "", line)
                for pattern in (INLINE, REFDEF, HTML_HREF):
                    for match in pattern.finditer(prose):
                        target = match.group(1)
                        if not SCHEME.match(target) and not target.startswith(("//", "{{")):
                            check_relative(path, where, target)
            for match in REPO_URL.finditer(line):
                check_repo_url(where, match)
            for match in BOOK_URL.finditer(line):
                check_book_url(where, match)

    for path in sorted(sources):
        try:
            text = path.read_text()
        except (UnicodeDecodeError, OSError):
            continue
        if FIXTURES in "\n".join(text.splitlines()[:10]):
            continue
        for number, line in enumerate(text.splitlines(), 1):
            where = f"{path}:{number}"
            urls = []
            for match in REPO_URL.finditer(line):
                urls.append(match.span())
                check_repo_url(where, match)
            for match in BOOK_URL.finditer(line):
                urls.append(match.span())
                check_book_url(where, match)
            for match in BENCH_REF.finditer(line):
                if not any(start <= match.start() < end for start, end in urls):
                    check_bench_ref(path, where, match)

    for failure in failures:
        print(failure)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
