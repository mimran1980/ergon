#!/usr/bin/env bash
# snapshot-bench-repeat.sh <criterion-dir> <k> — keep invocation k's estimates.
#
# The runner takes several independent invocations of the same binary, because
# one invocation's confidence interval is not the harness's resolution:
# identical code walked 3.5 % between invocations on 2026-10-20 while each
# run's own ratio CI was ±0.5 %. Criterion overwrites `<group>/<arm>/new/` on
# every run, so each invocation is copied out to `repeats/<k>/` before the
# next one starts. The gate grades the envelope of those copies.
#
# Only estimates this invocation produced are kept. The Criterion home is
# shared (`target/criterion` also holds `ergon-clock-bench`'s `clock_*` groups)
# and Criterion leaves `new/` in place for groups a run does not touch, so
# "everything under the home" would fold another binary's leftovers into this
# run's envelope and widen it with evidence the invocation never produced.
# The runner touches `<dir>/.bench-since` immediately before `cargo bench`; an
# estimate belongs to the invocation when it is not older than that marker.
# Without the marker there is no way to tell, so the snapshot is refused.
#
# Only `estimates.json` is kept: that is the whole of what the gate reads, and
# copying Criterion's sample trees would multiply the artifact size for data
# nothing consumes.
set -euo pipefail

dir="${1:?criterion dir}"
k="${2:?repeat index}"
case "$k" in
'' | *[!0-9]*)
    echo "snapshot-bench-repeat: repeat index must be a positive integer, got '$k'" >&2
    exit 2
    ;;
esac
[ -d "$dir" ] || {
    echo "snapshot-bench-repeat: no such criterion dir: $dir" >&2
    exit 2
    }

marker="$dir/.bench-since"
[ -f "$marker" ] || {
    echo "snapshot-bench-repeat: no producer marker $marker — touch it before cargo bench" \
        "so a snapshot cannot mix in estimates from an earlier run of a different bench" >&2
    exit 2
    }

dest="$dir/repeats/$k"
[ -e "$dest" ] && {
    echo "snapshot-bench-repeat: repeat $k already snapshotted under $dir" >&2
    exit 2
    }

copied=0
while IFS= read -r estimates; do
    # `.../<group>/<arm>/new/estimates.json` -> `repeats/<k>/<group>/<arm>/estimates.json`
    arm_dir=$(dirname "$(dirname "$estimates")")
    rel=${arm_dir#"$dir"/}
    [ "$rel" = "$arm_dir" ] && continue
    mkdir -p "$dest/$rel"
    cp "$estimates" "$dest/$rel/estimates.json"
    copied=$((copied + 1))
done < <(find "$dir" -path "*/new/estimates.json" -not -path "$dir/repeats/*" -newer "$marker" | sort)

if [ "$copied" -eq 0 ]; then
    echo "snapshot-bench-repeat: no Criterion estimates under $dir produced after the" \
        "producer marker — nothing snapshotted" >&2
    exit 1
fi
echo "snapshotted $copied estimate(s) as repeat $k under $dir/repeats"
