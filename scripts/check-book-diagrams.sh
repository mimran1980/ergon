#!/usr/bin/env bash
# check-book-diagrams.sh: fail when a source file a book diagram was checked
# against has changed since the diagram was last stamped.
#
# book/diagrams.tsv has one row per diagram and source file:
#   diagram<TAB>source<TAB>blob
# where diagram is a fragment under book/src/diagrams/, source is a file its
# facts were verified against, and blob is that file's `git hash-object` when
# the diagram was last verified (all paths from the repository root).
#
# Fails, naming the diagram, when:
# * a listed source's blob differs (it changed) or the source is missing;
# * a listed diagram is missing;
# * a fragment under book/src/diagrams/ (any depth) has no rows;
# * a row is malformed.
#
# After re-verifying a diagram against the changed sources (and fixing it),
# record the new blobs with `--restamp DIAGRAM` (its path or its name, e.g.
# `cpu-quota`). Restamping without re-verifying defeats the check.
#
# Usage: check-book-diagrams.sh [--restamp DIAGRAM] [ROOT]   (default: repo root)
set -euo pipefail

restamp=
if [ "${1:-}" = "--restamp" ]; then
    restamp=${2:?--restamp needs a diagram}
    shift 2
fi
root=${1:-$(cd "$(dirname "$0")/.." && pwd)}
tsv=$root/book/diagrams.tsv
dir=book/src/diagrams
rc=0

fail() {
    echo "  FAIL: $*"
    rc=1
}

# row_ok LINE: a data row has exactly three fields and a 40-hex blob.
row_ok() {
    local d s b extra
    IFS=$'\t' read -r d s b extra <<<"$1"
    [ -n "$d" ] && [ -n "$s" ] && [ -z "$extra" ] && [[ $b =~ ^[0-9a-f]{40}$ ]]
}

if [ ! -f "$tsv" ]; then
    echo "check-book-diagrams: $tsv is missing"
    exit 1
fi

if [ -n "$restamp" ]; then
    case $restamp in */*) ;; *) restamp=$dir/$restamp.html ;; esac
    [ -f "$root/$restamp" ] || { echo "check-book-diagrams: $restamp does not exist"; exit 1; }
    out=$(mktemp "$tsv.XXXXXX")
    trap 'rm -f "$out"' EXIT
    stamped=0
    while IFS= read -r line || [ -n "$line" ]; do
        case $line in '#'* | '') printf '%s\n' "$line" >>"$out"; continue ;; esac
        row_ok "$line" || { echo "check-book-diagrams: malformed row: $line"; exit 1; }
        IFS=$'\t' read -r d s b <<<"$line"
        if [ "$d" = "$restamp" ]; then
            [ -f "$root/$s" ] || { echo "check-book-diagrams: $d: source $s is missing: fix its row first"; exit 1; }
            b=$(git hash-object -- "$root/$s")
            stamped=$((stamped + 1))
        fi
        printf '%s\t%s\t%s\n' "$d" "$s" "$b" >>"$out"
    done <"$tsv"
    [ "$stamped" -gt 0 ] || { echo "check-book-diagrams: no rows for $restamp in book/diagrams.tsv: add its sources first"; exit 1; }
    mv "$out" "$tsv"
    trap - EXIT
    echo "check-book-diagrams: restamped $restamp ($stamped source(s))"
    exit 0
fi

echo "=== book diagram staleness check ==="
listed=
rows=0
while IFS= read -r line || [ -n "$line" ]; do
    case $line in '#'* | '') continue ;; esac
    if ! row_ok "$line"; then
        fail "malformed row in book/diagrams.tsv: $line"
        continue
    fi
    IFS=$'\t' read -r d s b <<<"$line"
    rows=$((rows + 1))
    listed+=$d$'\n'
    if [ ! -f "$root/$d" ]; then
        fail "$d: listed diagram is missing"
    elif [ ! -f "$root/$s" ]; then
        fail "$d: source $s is missing: re-verify the diagram, fix its rows, then --restamp $d"
    else
        now=$(git hash-object -- "$root/$s")
        [ "$now" = "$b" ] ||
            fail "$d: $s changed since the diagram was verified: re-verify the diagram against it, then run scripts/check-book-diagrams.sh --restamp $d"
    fi
done <"$tsv"

while IFS= read -r f; do
    rel=${f#"$root"/}
    grep -Fqx -- "$rel" <<<"$listed" || fail "$rel: no rows in book/diagrams.tsv: list every source its facts were verified against"
done < <(find "$root/$dir" -type f -name '*.html' | sort)

if [ "$rc" -ne 0 ]; then
    echo "check-book-diagrams: FAIL"
    exit 1
fi
echo "check-book-diagrams: PASS ($rows source row(s))"
