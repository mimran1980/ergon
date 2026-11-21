#!/usr/bin/env bash
# check-book-diagrams.sh must pass a freshly stamped fixture and fail, naming
# the diagram, for each way a diagram can go stale or escape the check: a
# changed source, a missing source, a missing diagram, an unlisted diagram and
# a malformed row. --restamp must refuse a diagram with no rows, and must
# restamp only the diagram it names.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-book-diagrams.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
zero=0000000000000000000000000000000000000000

# make_root DIR: two diagrams, each with its sources, stamped.
make_root() {
    local dir=$1
    mkdir -p "$dir/book/src/diagrams" "$dir/src"
    printf 'fn a() {}\n' > "$dir/src/a.rs"
    printf 'fn b() {}\n' > "$dir/src/b.rs"
    printf '<div>one</div>\n' > "$dir/book/src/diagrams/one.html"
    printf '<div>two</div>\n' > "$dir/book/src/diagrams/two.html"
    printf '# diagram\tsource\tblob\nbook/src/diagrams/one.html\tsrc/a.rs\t%s\nbook/src/diagrams/one.html\tsrc/b.rs\t%s\nbook/src/diagrams/two.html\tsrc/b.rs\t%s\n' \
        "$zero" "$zero" "$zero" > "$dir/book/diagrams.tsv"
    bash "$checker" --restamp one "$dir" >/dev/null
    bash "$checker" --restamp book/src/diagrams/two.html "$dir" >/dev/null
}

# expect_fail NAME MESSAGE [ARGS...]: the checker must fail on $fixture/NAME and print MESSAGE.
expect_fail() {
    local name=$1 message=$2
    shift 2
    if bash "$checker" "$@" "$fixture/$name" >"$fixture/$name.out" 2>&1; then
        echo "expected '$name' to fail" >&2
        cat "$fixture/$name.out" >&2
        exit 1
    fi
    if ! grep -F -q -- "$message" "$fixture/$name.out"; then
        echo "'$name' failed without naming the problem ('$message'):" >&2
        cat "$fixture/$name.out" >&2
        exit 1
    fi
}

make_root "$fixture/good"
bash "$checker" "$fixture/good" >"$fixture/good.out"
grep -F -q 'check-book-diagrams: PASS (3 source row(s))' "$fixture/good.out"

make_root "$fixture/changed"
printf '// edited\n' >> "$fixture/changed/src/a.rs"
expect_fail changed 'book/src/diagrams/one.html: src/a.rs changed since the diagram was verified'

make_root "$fixture/gone"
rm "$fixture/gone/src/a.rs"
expect_fail gone 'book/src/diagrams/one.html: source src/a.rs is missing'

make_root "$fixture/nodiagram"
rm "$fixture/nodiagram/book/src/diagrams/two.html"
expect_fail nodiagram 'book/src/diagrams/two.html: listed diagram is missing'

make_root "$fixture/unlisted"
printf '<div>three</div>\n' > "$fixture/unlisted/book/src/diagrams/three.html"
expect_fail unlisted 'book/src/diagrams/three.html: no rows in book/diagrams.tsv'

make_root "$fixture/nested"
mkdir -p "$fixture/nested/book/src/diagrams/sub"
printf '<div>four</div>\n' > "$fixture/nested/book/src/diagrams/sub/four.html"
expect_fail nested 'book/src/diagrams/sub/four.html: no rows in book/diagrams.tsv'

make_root "$fixture/malformed"
printf 'book/src/diagrams/one.html\tsrc/a.rs\n' >> "$fixture/malformed/book/diagrams.tsv"
expect_fail malformed 'malformed row'

make_root "$fixture/norows"
printf '<div>three</div>\n' > "$fixture/norows/book/src/diagrams/three.html"
expect_fail norows 'no rows for book/src/diagrams/three.html' --restamp three

# A shared source changes: restamping one diagram leaves the other failing.
make_root "$fixture/partial"
printf '// edited\n' >> "$fixture/partial/src/b.rs"
bash "$checker" --restamp one "$fixture/partial" >/dev/null
expect_fail partial 'book/src/diagrams/two.html: src/b.rs changed'
if grep -F -q 'one.html' "$fixture/partial.out"; then
    echo "restamping 'one' did not clear its own rows:" >&2
    cat "$fixture/partial.out" >&2
    exit 1
fi
bash "$checker" --restamp two "$fixture/partial" >/dev/null
bash "$checker" "$fixture/partial" >/dev/null

echo "test-book-diagrams: PASS"
