#!/usr/bin/env bash
# One shard of ergo-sbe's integration test binaries for CI's `test` job (its
# compile-and-run suites are most of the job's time); scripts/ci-test-rest.sh
# runs everything else.
#
#   scripts/ci-test-shard.sh <shard> <shards>      run one shard (1-based)
#   scripts/ci-test-shard.sh --plan <shards>        print the plan, run nothing
#
# Binaries are split by greedy longest-first over scripts/ci-test-weights.tsv.
# The list comes from cargo metadata, so a new test binary always lands in a
# shard; one without a weight counts as 60 s.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

plan_only=0
if [[ "${1:-}" == "--plan" ]]; then
    plan_only=1
    shard=0
    shards=${2:?usage: $0 --plan <shards>}
else
    shard=${1:?usage: $0 <shard> <shards>}
    shards=${2:?usage: $0 <shard> <shards>}
fi
if ! [[ "$shards" =~ ^[0-9]+$ ]] || (( shards < 1 )); then
    echo "ci-test-shard: <shards> must be 1 or more" >&2
    exit 2
fi
if (( plan_only == 0 )) && { ! [[ "$shard" =~ ^[0-9]+$ ]] || (( shard < 1 || shard > shards )); }; then
    echo "ci-test-shard: <shard> must be 1..$shards" >&2
    exit 2
fi

# One line per sbe shard: the --test names it runs.
plan=$(cargo metadata --format-version 1 --no-deps | python3 -I -c '
import json, sys
shards = int(sys.argv[1])
weights = {}
for line in open(sys.argv[2]):
    if line.startswith("#") or not line.strip():
        continue
    name, secs = line.split("\t")
    weights[name] = float(secs)
pkg = [p for p in json.load(sys.stdin)["packages"] if p["name"] == "ergo-sbe"][0]
tests = sorted((t["name"] for t in pkg["targets"] if "test" in t["kind"]),
               key=lambda n: (-weights.get(n, 60.0), n))
bins = [[0.0, []] for _ in range(shards)]
for name in tests:
    b = min(bins, key=lambda b: b[0])
    b[0] += weights.get(name, 60.0)
    b[1].append(name)
for total, names in bins:
    print(f"{total:.0f} " + " ".join(names))
' "$shards" scripts/ci-test-weights.tsv)

if (( plan_only == 1 )); then
    i=0
    while read -r secs names; do
        i=$((i + 1))
        echo "shard $i/$shards: ~${secs} s on x86_64, $(wc -w <<<"$names") ergo-sbe test binaries"
    done <<<"$plan"
    exit 0
fi

names=$(sed -n "${shard}p" <<<"$plan" | cut -d' ' -f2-)
args=()
for n in $names; do args+=(--test "$n"); done
echo "ci-test-shard $shard/$shards: ergo-sbe ${names}"
exec cargo test -p ergo-sbe --all-features "${args[@]}"
