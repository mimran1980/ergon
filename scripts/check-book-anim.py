#!/usr/bin/env python3
"""Check the book's animations (book/src/anim, runtime in anim.js).

Fails when:

* a page embeds `<div class="ergon-anim" data-anim="SLUG">` and
  book/src/anim/SLUG.js does not exist;
* a module book/src/anim/SLUG.js is embedded on no page (anim.js, the
  runtime, is exempt);
* a module does not call Anim.register('SLUG', ...) with its own file name;
* any .js under book/src/anim fails `node --check` (node missing is a
  failure, not a skip);
* any file under book/src/anim names an external URL (animations work
  offline); the SVG namespace URI, which is never fetched, is allowed;
* a module's leading comment has no `Sources:` line followed by at least one
  `path:line` citation of the code it shows;
* the runtime sets `disabled` on a control (a disabled button that has focus
  drops it to <body>, so the next arrow key changes chapter; use aria-disabled);
* the runtime (anim.js, anim.css) is missing or not registered in book.toml.

Generated pages are scanned like any other, so a page exported into this book
with an embed fails here until its module comes with it.

Usage: check-book-anim.py [BOOK_DIR]   (default: book/)
"""
import pathlib
import re
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
book = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "book"
src = book / "src"
anim = src / "anim"
RUNTIME = "anim.js"

div = re.compile(r"<div\b[^>]*>", re.IGNORECASE)
is_anim = re.compile(r"""\bclass\s*=\s*["'][^"']*\bergon-anim\b""")
slug_attr = re.compile(r"""\bdata-anim\s*=\s*["']([^"']*)["']""")
register = re.compile(r"""Anim\.register\(\s*['"]([^'"]+)['"]""")
url = re.compile(r"""(?:https?:)?//[A-Za-z0-9.-]+\.[A-Za-z]{2,}[/\w.-]*|\burl\(\s*['"]?(?:https?:)?//""")
ALLOWED_URLS = {"http://www.w3.org/2000/svg"}
citation = re.compile(r"[\w./-]+\.\w+:\d+")

failures = []


def fail(message):
    failures.append(message)


def header(text):
    """The leading // comment block of a module."""
    lines = []
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("//"):
            lines.append(stripped[2:].strip())
        elif stripped:
            break
    return lines


if not anim.is_dir():
    fail(f"{anim}: missing animation directory")
else:
    for name in (RUNTIME, "anim.css"):
        if not (anim / name).is_file():
            fail(f"{anim / name}: runtime file missing")
    toml = (book / "book.toml").read_text() if (book / "book.toml").is_file() else ""
    for name in ("src/anim/anim.js", "src/anim/anim.css"):
        if f'"{name}"' not in toml:
            fail(f"{book / 'book.toml'}: {name} is not registered (additional-js / additional-css)")

runtime = anim / RUNTIME
if runtime.is_file():
    for number, line in enumerate(runtime.read_text(errors="replace").splitlines(), 1):
        if re.search(r"\.disabled\s*=|setAttribute\(\s*['\"]disabled['\"]", line):
            fail(f"{runtime}:{number}: sets `disabled` on a control; use aria-disabled so focus stays put")

embedded = {}
for page in sorted(src.rglob("*.md")):
    text = page.read_text(errors="replace")
    # Whole-page match, so an embed whose tag spans several lines is still seen.
    for match in div.finditer(text):
        tag = match.group(0)
        number = text.count("\n", 0, match.start()) + 1
        if not is_anim.search(tag):
            continue
        found = slug_attr.search(tag)
        if not found or not found.group(1):
            fail(f"{page}:{number}: ergon-anim embed has no data-anim")
            continue
        slug = found.group(1)
        embedded.setdefault(slug, f"{page}:{number}")
        if not (anim / f"{slug}.js").is_file():
            fail(f"{page}:{number}: embeds '{slug}' but {anim / (slug + '.js')} does not exist")

modules = sorted(p for p in anim.glob("*.js") if p.name != RUNTIME) if anim.is_dir() else []
for module in modules:
    slug = module.stem
    text = module.read_text(errors="replace")
    if slug not in embedded:
        fail(f"{module}: module '{slug}' is not embedded on any page")
    registered = register.findall(text)
    if registered != [slug]:
        fail(f"{module}: must call Anim.register('{slug}', ...) exactly once (found {registered})")
    lines = header(text)
    at = next((i for i, l in enumerate(lines) if l.startswith("Sources:")), None)
    if at is None or not any(citation.search(l) for l in lines[at:]):
        fail(f"{module}: leading comment needs a 'Sources:' line with path:line citations")

if anim.is_dir():
    for path in sorted(anim.rglob("*")):
        if not path.is_file():
            continue
        for number, line in enumerate(path.read_text(errors="replace").splitlines(), 1):
            for match in url.finditer(line):
                if match.group(0) not in ALLOWED_URLS:
                    fail(f"{path}:{number}: external URL {match.group(0)!r}: animations must work offline")

    scripts = sorted(anim.rglob("*.js"))
    node = shutil.which("node")
    if scripts and node is None:
        fail("node not found: install Node.js to syntax-check book/src/anim/*.js")
    elif node is not None:
        for script in scripts:
            result = subprocess.run([node, "--check", str(script)], capture_output=True, text=True)
            if result.returncode != 0:
                detail = (result.stderr.strip().splitlines() or ["syntax error"])[-1]
                fail(f"{script}: node --check failed: {detail}")

for message in failures:
    print(f"  FAIL: {message}")
if failures:
    print("check-book-anim: FAIL")
    sys.exit(1)
print(f"check-book-anim: PASS ({len(modules)} module(s), {len(embedded)} embedded)")
