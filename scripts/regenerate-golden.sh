#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
golden="$root/sbe/tests/golden/car_example.rs"

# The generator gates var-data accessors on its OWN build features
# (`#[cfg(feature = "compact_str")]` and friends in codegen/tail_stages.rs), so
# generated output is a function of the feature set, not just GenerationConfig.
# Every producer and checker of the golden must therefore agree on one set.
#
# That set is ergo-sbe's DEFAULT features, not --all-features: the golden is
# `include!`d by sbe/tests/allocation_count_test.rs, which compiles under
# whatever features the test run selects. A default-flavoured golden references
# only paths that always exist, so it compiles under both plain `cargo test` and
# `cargo test --all-features`. An all-features golden would reference
# `ergo_sbe::compact_str` and fail the default build.
#
# Trade-off: the golden does not snapshot the feature-gated accessors. Those are
# covered by the feature-specific tests instead.
GOLDEN_FEATURES=()

case "${1:-}" in
    "")
        cargo run --quiet --manifest-path "$root/Cargo.toml" ${GOLDEN_FEATURES[@]+"${GOLDEN_FEATURES[@]}"} \
            -p ergo-sbe --example regenerate_golden -- "$golden"
        echo "golden regeneration: updated $golden"
        ;;
    --check)
        candidate=$(mktemp)
        trap 'rm -f "$candidate"' EXIT
        cargo run --quiet --manifest-path "$root/Cargo.toml" ${GOLDEN_FEATURES[@]+"${GOLDEN_FEATURES[@]}"} \
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
