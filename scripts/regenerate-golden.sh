#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
golden="$root/sbe/tests/golden/car_example.rs"

case "${1:-}" in
    "")
        cargo run --quiet --manifest-path "$root/Cargo.toml" \
            -p ergo-sbe --example regenerate_golden -- "$golden"
        echo "golden regeneration: updated $golden"
        ;;
    --check)
        candidate=$(mktemp)
        trap 'rm -f "$candidate"' EXIT
        cargo run --quiet --manifest-path "$root/Cargo.toml" \
            -p ergo-sbe --example regenerate_golden -- "$candidate" >/dev/null
        if ! cmp -s "$golden" "$candidate"; then
            echo "golden regeneration: generated source differs from $golden" >&2
            diff_status=0
            diff -u "$golden" "$candidate" || diff_status=$?
            if [[ $diff_status -gt 1 ]]; then
                echo "golden regeneration: diff command failed with status $diff_status" >&2
            fi
            echo "run: just update-golden" >&2
            exit 1
        fi
        echo "golden regeneration: PASS"
        ;;
    *)
        echo "usage: $0 [--check]" >&2
        exit 2
        ;;
esac
