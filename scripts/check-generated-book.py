#!/usr/bin/env python3
"""Check the book's generated pages are as generated.

A generated page's first line is an HTML comment carrying the sha256 of the
rest of the page. book/generated/manifest.tsv lists every generated page with
that hash, and book/generated/summary.fragment holds the SUMMARY.md entries
for them, between `<!-- generated:begin NAME -->` / `<!-- generated:end
NAME -->` markers. This check fails when:

* a listed page is missing, has no header, or its body no longer matches the
  hash (it was edited by hand: refresh it with the export instead);
* the manifest's hash differs from the page's;
* a page under book/src carries the header but is not in the manifest;
* the fragment was edited, or SUMMARY.md's marker blocks differ from it;
* a listed page is not in the fragment, or the fragment names one not listed;
* a listed page is outside GENERATED (the export's destinations), so no
  hand-written page can be exempted by recomputing its hash.

With no manifest there are no generated pages, and the check passes.

Usage: check-generated-book.py [--list] [BOOK_DIR]   (default: book/)
--list prints the generated pages (paths under book/src, one a line) only
when every check passes, and prints nothing otherwise; check-book-fences.sh
and check-book-content.sh exempt exactly those pages.
"""
import hashlib
import pathlib
import re
import sys

HEADER = re.compile(r"^<!-- Generated page: edits here are overwritten by the next export\. sha256:([0-9a-f]{64}) -->$")
MARKER = re.compile(r"^<!-- generated:(begin|end) ([a-z0-9-]+) -->$")
# Where generated pages may live under book/src: a trailing slash is a
# directory.
GENERATED = ("io/", "md-dpdk/", "transports/", "findings.md",
             "samples/k8s-quickstart.md", "samples/low-latency-k8s.md")
ENTRY = re.compile(r"^\s*- \[.*\]\((?:\./)?([^)]+)\)\s*$")


def split(text):
    """(hash in the header or None, body after the header line)."""
    first, _, body = text.partition("\n")
    m = HEADER.match(first)
    return (m.group(1) if m else None), body


def blocks(text):
    """{name: content between its markers}, or an error string."""
    found, inside, lines = {}, None, []
    for line in text.splitlines():
        m = MARKER.match(line.strip())
        if m and m.group(1) == "begin":
            if inside is not None:
                return f"begin {m.group(2)} inside {inside}"
            inside, lines = m.group(2), []
        elif m:
            if m.group(2) != inside:
                return f"end {m.group(2)} without its begin"
            found[inside] = "\n".join(lines).strip("\n")
            inside = None
        elif inside is not None:
            lines.append(line)
    if inside is not None:
        return f"no end marker for {inside}"
    return found


def check(book):
    src = book / "src"
    manifest = book / "generated" / "manifest.tsv"
    fragment = book / "generated" / "summary.fragment"
    errors, verified = [], []
    if not manifest.is_file():
        if fragment.exists():
            errors.append(f"{fragment}: present without {manifest}")
        stray = [p for p in src.rglob("*.md") if split(p.read_text(errors="replace"))[0]]
        errors += [f"{p}: generated header but no {manifest}" for p in stray]
        return errors, verified
    listed = {}
    for number, line in enumerate(manifest.read_text().splitlines(), 1):
        if not line.strip() or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) != 2 or not re.fullmatch(r"[0-9a-f]{64}", parts[1]):
            errors.append(f"{manifest}:{number}: expected 'path<TAB>sha256'")
            continue
        listed[parts[0]] = parts[1]
    for rel, digest in listed.items():
        if not any(rel == g or (g.endswith("/") and rel.startswith(g)) for g in GENERATED):
            errors.append(f"book/src/{rel}: not a path the export writes ({', '.join(GENERATED)})")
            continue
        page = src / rel
        if not page.is_file():
            errors.append(f"book/src/{rel}: listed in the manifest but missing")
            continue
        header, body = split(page.read_text())
        actual = hashlib.sha256(body.encode()).hexdigest()
        if header is None:
            errors.append(f"book/src/{rel}: no generated header")
        elif header != actual:
            errors.append(f"book/src/{rel}: edited by hand (body sha256 {actual[:12]}, header {header[:12]}); regenerate it")
        elif header != digest:
            errors.append(f"book/src/{rel}: header sha256 {header[:12]} differs from the manifest's {digest[:12]}")
        else:
            verified.append(rel)
    for page in sorted(src.rglob("*.md")):
        rel = page.relative_to(src).as_posix()
        if rel not in listed and split(page.read_text(errors="replace"))[0]:
            errors.append(f"book/src/{rel}: generated header but not in the manifest")
    if not fragment.is_file():
        errors.append(f"{fragment}: missing")
        return errors, verified
    header, body = split(fragment.read_text())
    if header is None or header != hashlib.sha256(body.encode()).hexdigest():
        errors.append(f"{fragment}: edited by hand or no header; regenerate it")
    want, have = blocks(body), blocks((src / "SUMMARY.md").read_text())
    for name, value in (("fragment", want), ("SUMMARY.md", have)):
        if isinstance(value, str):
            errors.append(f"{name}: {value}")
    if isinstance(want, str) or isinstance(have, str):
        return errors, verified
    for name in sorted(set(want) | set(have)):
        if want.get(name) != have.get(name):
            if name not in want:
                errors.append(f"SUMMARY.md: the generated:{name} block is not in {fragment.name}; "
                              "delete its markers and content")
            else:
                errors.append(f"SUMMARY.md: the generated:{name} block differs from {fragment.name}; regenerate it")
    named = {m.group(1) for text in want.values() for line in text.splitlines()
             for m in [ENTRY.match(line)] if m}
    for rel in sorted(set(listed) - named):
        errors.append(f"book/src/{rel}: in the manifest but not in {fragment.name}")
    for rel in sorted(named - set(listed)):
        errors.append(f"{fragment.name}: names {rel}, which is not in the manifest")
    return errors, verified


def main(argv):
    listing = "--list" in argv
    args = [a for a in argv if a != "--list"]
    book = pathlib.Path(args[0]) if args else pathlib.Path(__file__).resolve().parent.parent / "book"
    errors, verified = check(book)
    if listing and not errors:
        for rel in verified:
            print(f"book/src/{rel}")
    for e in errors:
        print(f"check-generated-book: {e}", file=sys.stderr)
    if errors:
        print("check-generated-book: FAIL", file=sys.stderr)
        return 1
    if not listing:
        print(f"check-generated-book: PASS ({len(verified)} generated pages, hashes match)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
