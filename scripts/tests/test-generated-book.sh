#!/usr/bin/env bash
# check-generated-book.py must pass an untouched generated book and fail
# when a page or the SUMMARY fragment is edited by hand, a page or manifest
# entry is missing, a page carries the header without a manifest entry, or
# SUMMARY.md's marker block drifts from the fragment, or a listed page lies
# outside the export's paths. `--list` must print nothing while any check
# fails (check-book-fences.sh and check-book-content.sh's API greps exempt
# only what it lists).
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-generated-book.py"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

make_book() {
    rm -rf "$work/book"
    python3 - "$work/book" <<'PY'
import hashlib, pathlib, sys
book = pathlib.Path(sys.argv[1])
head = "<!-- Generated page: edits here are overwritten by the next export. sha256:{} -->\n"
def generated(body):
    return head.format(hashlib.sha256(body.encode()).hexdigest()) + body, hashlib.sha256(body.encode()).hexdigest()
(book / "src/io").mkdir(parents=True)
(book / "generated").mkdir()
page, digest = generated("# UDP\n\n```rust,ignore\nfn f() { CString::new(x); }\n```\n")
(book / "src/io/udp.md").write_text(page)
(book / "src/intro.md").write_text("# Intro\n")
block = "<!-- generated:begin ergon-io -->\n\n# ergon-io\n\n- [UDP](./io/udp.md)\n\n<!-- generated:end ergon-io -->\n"
(book / "generated/summary.fragment").write_text(generated(block)[0])
(book / "src/SUMMARY.md").write_text("# Summary\n\n[Intro](./intro.md)\n\n" + block)
(book / "generated/manifest.tsv").write_text(f"# comment\nio/udp.md\t{digest}\n")
PY
}

expect_fail() {
    local what=$1 needle=$2
    if python3 "$checker" "$work/book" >"$work/out" 2>&1; then
        echo "expected failure: $what" >&2
        cat "$work/out" >&2
        exit 1
    fi
    if ! grep -F -q -- "$needle" "$work/out"; then
        echo "$what: failure did not say '$needle':" >&2
        cat "$work/out" >&2
        exit 1
    fi
}

make_book
python3 "$checker" "$work/book" | grep -F -q 'PASS (1 generated pages'
[[ $(python3 "$checker" --list "$work/book") == "book/src/io/udp.md" ]]

content() {
    mkdir -p "$work/empty-repo"
    BOOK_CONTENT_ROOT="$work/book/src" LINK_REPO_ROOT="$work/empty-repo" \
        "$repo_root/scripts/check-book-content.sh" >"$work/content.out" 2>&1
}
make_book
content || { echo "a verified generated page must be exempt from the API greps" >&2; cat "$work/content.out" >&2; exit 1; }

make_book
printf 'an edit\n' >>"$work/book/src/io/udp.md"
expect_fail 'edited page' 'io/udp.md: edited by hand'
if content || ! grep -F -q 'CString::new is forbidden' "$work/content.out"; then
    echo "an edited generated page must lose the exemption" >&2
    cat "$work/content.out" >&2
    exit 1
fi
listed=$(python3 "$checker" --list "$work/book" 2>/dev/null || true)
if grep -q udp <<<"$listed"; then
    echo "--list must leave an edited page out" >&2
    exit 1
fi

make_book
sed -i.bak 's/\t[0-9a-f]*$/\t0000000000000000000000000000000000000000000000000000000000000000/' "$work/book/generated/manifest.tsv"
expect_fail 'manifest hash' "differs from the manifest's"

make_book
rm "$work/book/src/io/udp.md"
expect_fail 'missing page' 'listed in the manifest but missing'

make_book
cp "$work/book/src/io/udp.md" "$work/book/src/io/stray.md"
expect_fail 'page outside the manifest' 'io/stray.md: generated header but not in the manifest'

make_book
printf -- '- [More](./io/more.md)\n' >>"$work/book/generated/summary.fragment"
expect_fail 'edited fragment' 'summary.fragment: edited by hand'

make_book
sed -i.bak 's/\[UDP\]/[Datagrams]/' "$work/book/src/SUMMARY.md"
expect_fail 'SUMMARY drift' 'generated:ergon-io block differs'

make_book
mkdir -p "$work/book/src/guide"
cp "$work/book/src/io/udp.md" "$work/book/src/guide/fake.md"
digest=$(cut -f2 <(grep -v '^#' "$work/book/generated/manifest.tsv"))
printf 'guide/fake.md\t%s\n' "$digest" >>"$work/book/generated/manifest.tsv"
expect_fail 'generated page outside the export paths' 'guide/fake.md: not a path the export writes'
if [[ -n $(python3 "$checker" --list "$work/book" 2>/dev/null) ]]; then
    echo "--list must print nothing while any check fails" >&2
    exit 1
fi

make_book
printf '<!-- generated:begin old-part -->\n- [Old](./old.md)\n<!-- generated:end old-part -->\n' >>"$work/book/src/SUMMARY.md"
expect_fail 'marker block no longer exported' 'generated:old-part block is not in summary.fragment; delete its markers'

make_book
rm "$work/book/generated/manifest.tsv" "$work/book/generated/summary.fragment"
expect_fail 'header without a manifest' 'generated header but no'

rm -rf "$work/book" && mkdir -p "$work/book/src"
printf '# Summary\n' >"$work/book/src/SUMMARY.md"
python3 "$checker" "$work/book" | grep -F -q 'PASS (0 generated pages'

echo "test-generated-book: PASS"
