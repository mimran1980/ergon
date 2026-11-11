#!/usr/bin/env bash
# Proof for scripts/bench-ab-judge.py (and bench-ab.sh's plan and arguments):
# a real slowdown FAILs, a row whose A/A control moved over 3% is UNSTABLE,
# a cross-build layout shift the old within-build-only rule failed now passes,
# and missing evidence fails closed.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
judge="$repo_root/scripts/bench-ab-judge.py"
driver="$repo_root/scripts/bench-ab.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

# make <dir> <row>=<base r1>,<base r2>,<rebuilt>,<branch> ...: two rounds per
# arm, 100 samples per run around the given p50 (ns per iteration).
make() {
    python3 -I - "$@" <<'PY'
import json, os, sys
out, rows = sys.argv[1], sys.argv[2:]
for spec in rows:
    name, vals = spec.split("=")
    b1, b2, rebuilt, branch = (float(v) for v in vals.split(","))
    runs = {"base-r1": b1, "base-r2": b2, "base-rebuilt-r1": rebuilt, "base-rebuilt-r2": rebuilt,
            "branch-r1": branch, "branch-r2": branch}
    for run, p50 in runs.items():
        d = os.path.join(out, "runs", run, "lto", "criterion", "repeats", "1", "group", name)
        os.makedirs(d, exist_ok=True)
        times = [p50 * (1 + (i - 50) * 1e-5) for i in range(100)]
        if run == "base-r2":
            times[99] = p50 * 50  # a host stall on the top sample: medians unmoved, means wrecked
        with open(os.path.join(d, "sample.json"), "w") as fh:
            json.dump({"iters": [1.0] * 100, "times": times}, fh)
# A warmup arm in one run only: the judge must skip it, as the gate does.
d = os.path.join(out, "runs", "branch-r1", "lto", "criterion", "repeats", "1", "group", "warmup_body_only")
os.makedirs(d, exist_ok=True)
with open(os.path.join(d, "sample.json"), "w") as fh:
    json.dump({"iters": [1.0], "times": [1e6]}, fh)
PY
}

# row <output> <scenario>: the branch line's old-rule and new verdicts.
row() { awk -F'|' -v s="group/$2" '$2 == s && $3 == "branch" { print $(NF-1) "/" $NF }' <<<"$1"; }

expect_row() {
    local got
    got=$(row "$1" "$2")
    if [ "$got" != "$3" ]; then
        echo "row $2: expected old/new verdict $3, got '$got'" >&2
        echo "$1" >&2
        exit 1
    fi
}

# Rows: base rounds 100/100.3 (within-build spread 0.3%).
pass=pass=100,100.3,100.2,100.4                 # +0.2%: inside noise
slow=slow=100,100.3,100.2,108                   # +7.8%, A/A 0.05%: a real regression
unstable=unstable=100,100.3,106,107             # A/A 5.8%: not judged
layout=layout=100,100.3,102,101.5               # +1.3% < A/A 1.85%: old rule's false FAIL
small=small=100,100.3,100.1,100.9               # +0.75% over the spread, under the 1% floor
capped=capped=100,106,103,107.5                 # +4.4%, inside a 5.8% spread: still over the 3% cap

make "$fixture/pass" "$pass" "$layout" "$small"
output=$(python3 -I "$judge" "$fixture/pass" 2>&1) && rc=0 || rc=$?
[ "$rc" = 0 ] || { echo "PASS fixture: expected rc 0, got $rc" >&2 && echo "$output" >&2 && exit 1; }
expect_row "$output" pass PASS/PASS
expect_row "$output" layout FAIL/PASS
expect_row "$output" small FAIL/PASS
[[ "$output" == *"verdict: PASS"* ]] || { echo "missing PASS verdict line" >&2 && echo "$output" >&2 && exit 1; }

# An UNSTABLE row is not a pass: with no FAIL the run exits 3, never 0.
make "$fixture/unstable" "$pass" "$unstable"
output=$(python3 -I "$judge" "$fixture/unstable" 2>&1) && rc=0 || rc=$?
[ "$rc" = 3 ] || { echo "UNSTABLE fixture: expected rc 3, got $rc" >&2 && echo "$output" >&2 && exit 1; }
expect_row "$output" unstable FAIL/UNSTABLE
[[ "$output" == *"verdict: UNSTABLE (1 row(s)"* ]] || { echo "missing UNSTABLE verdict line" >&2 && echo "$output" >&2 && exit 1; }

# The gate can fail: a real slowdown fails the run.
make "$fixture/fail" "$pass" "$slow" "$layout" "$capped"
output=$(python3 -I "$judge" "$fixture/fail" 2>&1) && rc=0 || rc=$?
[ "$rc" = 1 ] || { echo "FAIL fixture: expected rc 1, got $rc" >&2 && echo "$output" >&2 && exit 1; }
expect_row "$output" slow FAIL/FAIL
expect_row "$output" capped FAIL/FAIL
[[ "$output" == *"verdict: FAIL (2 row(s)"* ]] || { echo "missing FAIL verdict line" >&2 && echo "$output" >&2 && exit 1; }
# The distribution columns, worked by hand: 200 samples of 108 * (1 + k * 1e-5),
# k = -50..49 twice; median 108 * (1 - 0.5e-5), nearest-rank p99 is k = 48.
grep -qxF 'lto|group/slow|branch|200|107.95|108.00|108.05|108.05|108.05|0.30|0.05|0.30|+7.84|FAIL|FAIL' <<<"$output" ||
    { echo "slow branch line: wrong n/min/p50/p99/p99.99/max or terms" >&2 && echo "$output" >&2 && exit 1; }
# A FAIL outranks an UNSTABLE row.
make "$fixture/both" "$slow" "$unstable"
python3 -I "$judge" "$fixture/both" >/dev/null 2>&1 && rc=0 || rc=$?
[ "$rc" = 1 ] || { echo "FAIL+UNSTABLE fixture: expected rc 1, got $rc" >&2 && exit 1; }

# Missing evidence fails closed: a row absent from one run, or a missing arm.
make "$fixture/missing" "$pass" "$layout"
mv "$fixture/missing/runs/branch-r2/lto/criterion/repeats/1/group/layout" "$fixture/missing-layout"
output=$(python3 -I "$judge" "$fixture/missing" 2>&1) && rc=0 || rc=$?
[ "$rc" = 2 ] && [[ "$output" == *"branch-r2 lacks 1 row(s)"* ]] || { echo "missing-row fixture: rc $rc" >&2 && echo "$output" >&2 && exit 1; }
mv "$fixture/missing/runs/base-rebuilt-r1" "$fixture/gone-r1"
mv "$fixture/missing/runs/base-rebuilt-r2" "$fixture/gone-r2"
output=$(python3 -I "$judge" "$fixture/missing" 2>&1) && rc=0 || rc=$?
[ "$rc" = 2 ] && [[ "$output" == *"arm base-rebuilt has no run"* ]] || { echo "missing-arm fixture: rc $rc" >&2 && echo "$output" >&2 && exit 1; }

# The driver's arm order: rounds 1-3 rotate, rounds 4-6 run the reversed
# rotations, and refuses bad arguments.
plan=$(bash "$driver" --print-plan --rounds 3)
expected=$'round 1: base base-rebuilt branch\nround 2: base-rebuilt branch base\nround 3: branch base base-rebuilt'
[ "$plan" = "$expected" ] || { echo "unexpected plan:" >&2 && echo "$plan" >&2 && exit 1; }
# 3 rounds: every arm once in every position.
[ "$(awk '{ for (i = 3; i <= NF; i++) print i, $i }' <<<"$plan" | sort | uniq -c | awk '{ print $1 }' | sort -u)" = 1 ] ||
    { echo "3 rounds do not put every arm in every position once" >&2 && exit 1; }
# 6 rounds: all six orders, every arm twice in every position and twice right
# after each other arm; and over 12 rounds no arm runs twice in a row across a
# round boundary.
plan=$(bash "$driver" --print-plan --rounds 6)
[ "$(cut -d: -f2 <<<"$plan" | sort -u | wc -l | tr -d ' ')" = 6 ] ||
    { echo "6 rounds do not run all six orders:" >&2 && echo "$plan" >&2 && exit 1; }
[ "$(awk '{ for (i = 3; i <= NF; i++) print i, $i }' <<<"$plan" | sort | uniq -c | awk '{ print $1 }' | sort -u)" = 2 ] ||
    { echo "6 rounds do not put every arm in every position twice:" >&2 && echo "$plan" >&2 && exit 1; }
pairs=$(awk '{ for (i = 4; i <= NF; i++) print $(i - 1) ">" $i }' <<<"$plan" | sort | uniq -c | awk '{ print $1 }' | sort -u)
[ "$pairs" = 2 ] || { echo "unbalanced predecessors over 6 rounds:" >&2 && echo "$plan" >&2 && exit 1; }
repeats=$(bash "$driver" --print-plan --rounds 12 | awk '$3 == last { print } { last = $NF }')
[ -z "$repeats" ] || { echo "an arm runs twice in a row across a round boundary: $repeats" >&2 && exit 1; }
for args in "--rounds 1 a b" "--suite nope a b" "only-one-commit" "a b --rounds" "a b --suite" "a b --out" "a b --floor"; do
    # shellcheck disable=SC2086
    bash "$driver" $args >/dev/null 2>&1 && rc=0 || rc=$?
    [ "$rc" = 2 ] || { echo "driver accepted '$args' (rc $rc)" >&2 && exit 1; }
done
# Rounds that are not whole 3-round rotations lose position balance: refused.
for n in 0 2 4 5 7 03 x; do
    output=$(bash "$driver" --rounds "$n" a b 2>&1) && rc=0 || rc=$?
    [ "$rc" = 2 ] && [[ "$output" == *"multiple of 3"* ]] || { echo "driver accepted --rounds $n (rc $rc): $output" >&2 && exit 1; }
done
# Salt sizes must be positive integers, and at least one: refused up front
# with exit 2, never read as a FAIL (1) or a void A/A control (4).
for sizes in "" "x" "6 x 8" "   " "6 0" "-1" "4.5" "*"; do
    output=$(BENCH_AB_SALT_SIZES="$sizes" bash "$driver" a b 2>&1) && rc=0 || rc=$?
    [ "$rc" = 2 ] && [[ "$output" == *"BENCH_AB_SALT_SIZES must be"* ]] ||
        { echo "driver accepted BENCH_AB_SALT_SIZES='$sizes' (rc $rc): $output" >&2 && exit 1; }
done

# The salt check passes only when a timed Bencher::iter symbol moved within a
# cache line between base and base-rebuilt (the salt's own instance does not
# count), and exits 4 when none did: an A/A control that never moves the timed
# code, or moves it by whole lines only, is refused. The shifts are chosen to
# hold for 64- and 128-byte lines alike: 0xe8 (moved) and 0x80 (whole lines).
mkdir -p "$fixture/bin"
cat >"$fixture/bin/nm" <<'EOF'
#!/bin/sh
cat "$1.nm"
EOF
chmod +x "$fixture/bin/nm"
iter=__RINvMNtCskszXIvg4u5n_9criterion7bencherNtB3_7Bencher4iteruNCNCNvCs1LNPWYmMVFW_17perf_parity_bench
printf '%s\n' "0000000100001f4c t ${iter}18bench_decode_array00EB10_" "0000000100002030 t ${iter}22bench_decode_composite00EB10_" \
    "0000000100009000 t _main" >"$fixture/base.nm"
printf '%s\n' "0000000100001e00 t ${iter}10aa_salt_ab00EB10_" "0000000100001f4c t ${iter}18bench_decode_array00EB10_" \
    "0000000100002030 t ${iter}22bench_decode_composite00EB10_" "0000000100009100 t _main" >"$fixture/unmoved.nm"
printf '%s\n' "0000000100001e00 t ${iter}10aa_salt_ab00EB10_" "0000000100002034 t ${iter}18bench_decode_array00EB10_" \
    "0000000100002118 t ${iter}22bench_decode_composite00EB10_" >"$fixture/moved.nm"
printf '%s\n' "0000000100001f00 t ${iter}10aa_salt_ab00EB10_" "0000000100001fcc t ${iter}18bench_decode_array00EB10_" \
    "00000001000020b0 t ${iter}22bench_decode_composite00EB10_" >"$fixture/lines.nm"
output=$(PATH="$fixture/bin:$PATH" bash "$driver" --salt-check "$fixture/base" "$fixture/moved" 2>&1) && rc=0 || rc=$?
[ "$rc" = 0 ] && [[ "$output" == *"2 of 2 timed"* ]] || { echo "salt check (moved): rc $rc" >&2 && echo "$output" >&2 && exit 1; }
output=$(PATH="$fixture/bin:$PATH" bash "$driver" --salt-check "$fixture/base" "$fixture/unmoved" 2>&1) && rc=0 || rc=$?
[ "$rc" = 4 ] && [[ "$output" == *"0 of 2 timed"* ]] || { echo "salt check (unmoved): expected rc 4, got $rc" >&2 && echo "$output" >&2 && exit 1; }
# Moved by whole lines only (0x80): the timed loops keep their in-line offsets.
output=$(PATH="$fixture/bin:$PATH" bash "$driver" --salt-check "$fixture/base" "$fixture/lines" 2>&1) && rc=0 || rc=$?
[ "$rc" = 4 ] && [[ "$output" == *"0 of 2 timed"* ]] || { echo "salt check (whole lines): expected rc 4, got $rc" >&2 && echo "$output" >&2 && exit 1; }
# Partly moved: one symbol within a line, one by whole lines. Every timed loop
# must move, so this is refused and the unmoved symbol named.
printf '%s\n' "0000000100001e00 t ${iter}10aa_salt_ab00EB10_" "0000000100002034 t ${iter}18bench_decode_array00EB10_" \
    "00000001000020b0 t ${iter}22bench_decode_composite00EB10_" >"$fixture/partial.nm"
output=$(PATH="$fixture/bin:$PATH" bash "$driver" --salt-check "$fixture/base" "$fixture/partial" 2>&1) && rc=0 || rc=$?
[ "$rc" = 4 ] && [[ "$output" == *"1 of 2 timed"* && "$output" == *"unmoved: ${iter}22bench_decode_composite"* ]] ||
    { echo "salt check (partial): expected rc 4 naming the unmoved symbol, got $rc" >&2 && echo "$output" >&2 && exit 1; }
# A timed symbol in one binary only would go unchecked: refused, and named,
# even though the one symbol both hold moved.
printf '%s\n' "0000000100001e00 t ${iter}10aa_salt_ab00EB10_" "0000000100002034 t ${iter}18bench_decode_array00EB10_" >"$fixture/dropped.nm"
output=$(PATH="$fixture/bin:$PATH" bash "$driver" --salt-check "$fixture/base" "$fixture/dropped" 2>&1) && rc=0 || rc=$?
[ "$rc" = 4 ] && [[ "$output" == *"base has 2 timed Bencher::iter symbols, base-rebuilt 1"* && "$output" == *"in one binary only: ${iter}22bench_decode_composite"* ]] ||
    { echo "salt check (symbol in one binary only): expected rc 4 naming it, got $rc" >&2 && echo "$output" >&2 && exit 1; }
# The line size: a +0x40 shift moves within a 128-byte line but not a 64-byte
# one, and a host reporting no usable size (empty, 0) is taken as 64.
printf '%s\n' "0000000100001f8c t ${iter}18bench_decode_array00EB10_" "0000000100002070 t ${iter}22bench_decode_composite00EB10_" >"$fixture/half.nm"
printf '#!/bin/sh\necho 128\n' >"$fixture/bin/sysctl"
chmod +x "$fixture/bin/sysctl"
PATH="$fixture/bin:$PATH" bash "$driver" --salt-check "$fixture/base" "$fixture/half" >/dev/null 2>&1 && rc=0 || rc=$?
[ "$rc" = 0 ] || { echo "salt check (+64, 128-byte line): expected rc 0, got $rc" >&2 && exit 1; }
printf '#!/bin/sh\nexit 1\n' >"$fixture/bin/sysctl"
printf '#!/bin/sh\necho 0\n' >"$fixture/bin/getconf"
chmod +x "$fixture/bin/getconf"
output=$(PATH="$fixture/bin:$PATH" bash "$driver" --salt-check "$fixture/base" "$fixture/half" 2>&1) && rc=0 || rc=$?
[ "$rc" = 4 ] && [[ "$output" == *"64-byte line"* ]] ||
    { echo "salt check (+64, line size 0 taken as 64): expected rc 4, got $rc" >&2 && echo "$output" >&2 && exit 1; }
: >"$fixture/empty.nm"
PATH="$fixture/bin:$PATH" bash "$driver" --salt-check "$fixture/empty" "$fixture/empty" >/dev/null 2>&1 && rc=0 || rc=$?
[ "$rc" = 4 ] || { echo "salt check (no symbols): expected rc 4, got $rc" >&2 && exit 1; }

echo "test-bench-ab: PASS"
