#!/usr/bin/env bash
# Put the FIX SBE conformance suite at the commit in the pin file, or fail.
#
#   ensure-fix-sbe-conformance.sh [--pin FILE] [--verify-only] DIR
#
# Without --verify-only, a missing DIR is fetched at the pinned commit and an
# existing checkout is moved to it (git refuses if that would overwrite local
# changes). With --verify-only, DIR must already be a git checkout at the
# pinned commit: used for a suite the caller supplies. Either way the script
# ends by comparing HEAD with the pin and exits 1 on a mismatch.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
pin="$root/scripts/fix-sbe-conformance-pin.txt"
verify_only=0
while [[ $# -gt 1 ]]; do
    case "$1" in
        --pin) pin=$2; shift 2 ;;
        --verify-only) verify_only=1; shift ;;
        *) break ;;
    esac
done
[[ $# -eq 1 ]] || { echo "usage: $0 [--pin FILE] [--verify-only] DIR" >&2; exit 2; }
dir=$1

read -r url sha < <(grep -vE '^[[:space:]]*(#|$)' "$pin") || {
    echo "conformance pin: no '<url> <commit>' line in $pin" >&2; exit 1; }
[[ "$sha" =~ ^[0-9a-f]{40}$ ]] || { echo "conformance pin: '$sha' is not a full commit sha" >&2; exit 1; }

if [[ "$verify_only" -eq 0 ]]; then
    if [[ ! -d "$dir/.git" ]]; then
        [[ ! -e "$dir" || -z "$(ls -A "$dir" 2>/dev/null)" ]] || {
            echo "conformance pin: $dir exists and is not a git checkout" >&2; exit 1; }
        git init -q "$dir"
        git -C "$dir" remote add origin "$url"
    fi
    if [[ "$(git -C "$dir" rev-parse HEAD 2>/dev/null || true)" != "$sha" ]]; then
        echo "--- fetching fix-sbe-conformance $sha into $dir ---"
        git -C "$dir" fetch -q --depth 1 origin "$sha"
        git -C "$dir" checkout -q --detach "$sha"
    fi
fi

head=$(git -C "$dir" rev-parse HEAD 2>/dev/null) || {
    echo "conformance pin: $dir is not a git checkout; the suite must be at $sha" >&2; exit 1; }
if [[ "$head" != "$sha" ]]; then
    echo "conformance pin: $dir is at $head, the pin is $sha ($url)" >&2
    exit 1
fi
echo "fix-sbe-conformance at pinned $sha"
