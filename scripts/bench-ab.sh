#!/usr/bin/env bash
# bench-ab.sh BASE BRANCH [--suite sbe|cluster] [--rounds N] [--out DIR] [--floor PCT]
# bench-ab.sh --print-plan [--rounds N]
# bench-ab.sh BASE BRANCH --salt-only [--suite sbe|cluster] [--out DIR]
#   (build base and base-rebuilt, run the salt check, stop; BRANCH is not built)
#
# Base-vs-branch A/B for the gated benchmark suites, judged against an A/A
# control built from unchanged code. One build's round-to-round spread is not
# the noise an A/B must clear: rebuilding identical code moves constant data
# and code addresses, and rows whose timed loops were byte-identical have moved
# 6-12% between builds. So there are three arms, each a detached worktree with
# its own target directory (wt/arm0, wt/arm1, wt/arm2: equal-length paths):
#   base          BASE as committed
#   base-rebuilt  BASE plus a layout salt in the suite's bench sources: an
#                 unused static (moves constant data) and an unused
#                 Bencher::iter instance that rustc emits before the suite's
#                 own, so the instances after it start at other addresses.
#                 No behaviour change.
#   branch        BRANCH as committed
# After the builds a salt check compares base and base-rebuilt symbol tables:
# both must hold the same timed Bencher::iter symbols (bar the salt's), and
# every one in every bench binary must have moved by other than whole cache
# lines of this host. A timed loop LLVM merged into an identical one (A/A
# control copies, warmup arms) runs that one's code and has no symbol of its own. The tool tries the salt sizes in
# BENCH_AB_SALT_SIZES in turn, rebuilding base-rebuilt for each, and refuses the
# run (exit 4) when none passes.
# Every round runs all three, rounds 1-3 rotating the order and rounds 4-6
# reversing it (see order()), each run holding the
# Mac bench lock (BENCH_LOCK, default <main checkout>/tmp/.mac-bench.lock) and
# logging uptime before and after. A run takes the suite's profiles and
# BENCH_REPEATS invocations (default 5) as `just bench` / `just bench-cluster`
# do, but runs the arm's bench binaries, copied to bin/<slot>/<profile>/,
# directly: same cwd and argv, and an env of only HOME, PATH, an equal-length
# CRITERION_HOME, DYLD_FALLBACK_LIBRARY_PATH and LD_LIBRARY_PATH (the binary's
# directory, holding the Aeron library the cluster bench loads) and the cluster
# arm order; arm paths differ only in the equal-length slot name. Neither cargo, the arm's runner nor its gate runs. Each run's
# Criterion repeats are copied to <out>/runs/<arm>-r<k>/ as soon as it ends,
# then scripts/bench-ab-judge.py grades the table.
#
# Builds happen before round 1, outside the lock, under BENCH_AB_BUILD_WRAPPER
# when set (e.g. a build-slot limiter). The tool deletes nothing: the worktrees
# and results stay under <out> (default target/bench-ab/<stamp>-<suite>) and are
# listed at the end. Exit: the judge's (0 PASS, 1 FAIL, 2 missing evidence,
# 3 UNSTABLE rows, not a pass); 2 on usage (including --rounds not a positive
# multiple of 3, or BENCH_AB_SALT_SIZES not positive integers), a failed build
# or binary copy, and
# 2 when the judge passed but a bench binary exited non-zero; 4 when no salt
# size passes the salt check.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SELF="$REPO_ROOT/scripts/bench-ab.sh"
ARMS=(base base-rebuilt branch)
# Salt sizes (black_box calls in the salt body) tried in turn; see salt_worktree.
SALT_SIZES=${BENCH_AB_SALT_SIZES-"6 8 3 4 32 1 2 5 7 10"}

usage() {
    sed -n '2,5p' "$SELF" | sed 's/^# //' >&2
    exit 2
}

# order R: the arms of round R (1-based), from a 6-round cycle. Rounds 1-3 are
# ARMS rotated left by 0, 1, 2: every arm runs once in every position, but
# within a round each arm always follows the same other arm. Rounds 4-6 are the
# reversed rotations, ordered so no arm runs twice in a row across a round
# boundary; after 6 rounds every arm has run twice in every position and twice
# right after each other arm.
ORDERS=("0 1 2" "1 2 0" "2 0 1" "2 1 0" "1 0 2" "0 2 1")
order() {
    local i a=()
    for i in ${ORDERS[$((($1 - 1) % 6))]}; do a+=("${ARMS[$i]}"); done
    echo "${a[*]}"
}

plan() {
    local r
    for r in $(seq 1 "$1"); do echo "round $r: $(order "$r")"; done
}

# The suite's bench sources, which the salt goes into.
salt_files() {
    case "$1" in
    sbe) echo sbe/benchmarks/benches/perf_parity_bench.rs sbe/benchmarks/benches/perf_parity_extended_bench.rs ;;
    cluster) echo cluster/benches/cluster_codec_bench.rs ;;
    esac
}

# salt_worktree WT SUITE SIZE: insert the salt before the first item of each
# file (after its inner attributes and docs), so the code and data that follow
# it move. SIZE black_box calls in the salt's timed body set how far; the
# caller tries several.
salt_worktree() {
    local wt="$1" suite="$2" size="$3" f
    for f in $(salt_files "$suite"); do
        python3 -I - "$wt/$f" "$size" <<'PY'
import re, sys
path, size = sys.argv[1], int(sys.argv[2])
body = "\n".join(f"            std::hint::black_box({i}_u32);" for i in range(size))
with open(path, encoding="utf-8") as fh:
    lines = fh.read().split("\n")
at = next(i for i, l in enumerate(lines) if re.match(r"(use|mod|fn|static|const|pub|struct|enum|impl) ", l))
lines.insert(at, """// bench-ab layout salt (base-rebuilt arm only). rustc emits upstream generic
// instances first, sorted by symbol name, so this unused Bencher::iter instance
// (a closure in a closure in a 10-letter fn) lands before the suite's own and
// moves every timed loop after it; the static moves the data.
#[used]
static BENCH_AB_LAYOUT_SALT: [u8; 1531] = [0x5a; 1531];
#[used]
static BENCH_AB_LAYOUT_SALT_FN: fn(&mut criterion::Bencher<'_>) = aa_salt_ab;
fn aa_salt_ab(b: &mut criterion::Bencher<'_>) {
    let mut outer = || {
        b.iter(|| {
""" + body + """
        });
    };
    outer();
}""")
with open(path, "w", encoding="utf-8") as fh:
    fh.write("\n".join(lines))
PY
    done
}

# The host's cache line in bytes: 128 on Apple Silicon, usually 64 elsewhere,
# and 64 when the host reports nothing usable.
cache_line() {
    local line
    line=$(sysctl -n hw.cachelinesize 2>/dev/null || getconf LEVEL1_DCACHE_LINESIZE 2>/dev/null || true)
    case "$line" in '' | *[!0-9]* | 0) line=64 ;; esac
    echo "$line"
}

# salt_check BASE_BIN REBUILT_BIN: the A/A control only samples code placement
# if the salt moved the timed code within a cache line: every Bencher::iter
# instance (where each timed closure is inlined) must sit at an address that is
# not a whole number of cache lines away in base-rebuilt. Fails, naming the
# unmoved symbols, when any did not, or when there is nothing to compare.
salt_check() {
    local a b only
    a=$(nm "$1" | awk '$3 ~ /7Bencher[0-9]+iter/ && $3 !~ /aa_salt_ab/ { print $3, $1 }' | LC_ALL=C sort)
    b=$(nm "$2" | awk '$3 ~ /7Bencher[0-9]+iter/ && $3 !~ /aa_salt_ab/ { print $3, $1 }' | LC_ALL=C sort)
    # Bar the salt's own, both binaries must hold the same timed symbols: one
    # missing from either side would go unchecked.
    only=$(LC_ALL=C join -v 1 -v 2 <(printf '%s\n' "$a") <(printf '%s\n' "$b") | awk 'NF { print "  in one binary only: " $1 }')
    if [ -n "$only" ] || [ "$(grep -c . <<<"$a")" != "$(grep -c . <<<"$b")" ]; then
        printf 'salt check %s: base has %d timed Bencher::iter symbols, base-rebuilt %d (bar the salt); they must match\n%s\n' \
            "$(basename "$2")" "$(grep -c . <<<"$a")" "$(grep -c . <<<"$b")" "$only"
        return 1
    fi
    LC_ALL=C join <(printf '%s\n' "$a") <(printf '%s\n' "$b") | awk -v bin="$(basename "$2")" -v line="$(cache_line)" '
        # The low 16 bits of each address are enough for an offset within a line.
        function lo(s,   v, i) { v = 0; s = tolower(substr(s, length(s) - 3)); for (i = 1; i <= 4; i++) v = v * 16 + index("0123456789abcdef", substr(s, i, 1)) - 1; return v }
        NF == 3 { n++; d = (lo($3) - lo($2) + 65536) % line; if (d) moved++; else unmoved = unmoved "\n  unmoved: " $1; seen[d] = 1 }
        END {
            for (d in seen) list = list " " d
            printf "salt check %s: %d of %d timed Bencher::iter symbols moved within a %d-byte line (shifts mod %d:%s)%s\n", bin, moved, n, line, line, list, unmoved
            exit (n > 0 && moved == n ? 0 : 1)
        }'
}

# Build an arm's benches with the exact environment its suite run uses, so the
# timed run finds them fresh and holds the lock only to measure.
prebuild() {
    local wt="$1" suite="$2" w=()
    [ -n "${BENCH_AB_BUILD_WRAPPER:-}" ] && read -r -a w <<<"$BENCH_AB_BUILD_WRAPPER"
    cd "$wt"
    case "$suite" in
    sbe)
        ${w[@]+"${w[@]}"} env -u CARGO_TARGET_DIR cargo bench --no-run -p ergo-sbe-benchmarks \
            --bench perf_parity_bench --bench perf_parity_extended_bench
        ${w[@]+"${w[@]}"} env CARGO_TARGET_DIR="$wt/target/bench-shared/no-lto/target" CARGO_PROFILE_BENCH_LTO=false \
            CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 cargo bench --no-run -p ergo-sbe-benchmarks \
            --bench perf_parity_bench --bench perf_parity_extended_bench
        ;;
    cluster)
        ${w[@]+"${w[@]}"} env -u CARGO_TARGET_DIR cargo bench --no-run -p ergo-aeron-cluster --bench cluster_codec_bench
        ${w[@]+"${w[@]}"} env CARGO_TARGET_DIR="$wt/target/bench-no-lto" CARGO_PROFILE_BENCH_LTO=false \
            CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 cargo bench --no-run -p ergo-aeron-cluster --bench cluster_codec_bench
        ;;
    esac
}

# An arm's directory name under wt/, bin/ and home/: equal length for every
# arm, so no path the binaries embed or are run with differs in size.
slot() {
    local i
    for i in 0 1 2; do if [ "${ARMS[$i]}" = "$1" ]; then echo "arm$i"; fi; done
}

# run_bin OUT BIN CRITERION_HOME [ARM_ORDER]: every arm's binary runs with the
# same cwd, argv and environment, bar equal-length paths.
# The binary's own directory also holds the shared Aeron library the cluster
# bench loads (cargo would point the loader at its build directory instead).
run_bin() {
    local lib
    lib=$(dirname "$2")
    (cd "$1" && env -i HOME="$HOME" PATH=/usr/bin:/bin CRITERION_HOME="$3" ${4:+ERGON_BENCH_ARM_ORDER="$4"} \
        DYLD_FALLBACK_LIBRARY_PATH="$lib" LD_LIBRARY_PATH="$lib" "$2" --bench --noplot)
}

# One arm-round, called by flock while it holds the bench lock: the suite's
# profiles and BENCH_REPEATS invocations as `just bench` / `just bench-cluster`
# take them, run from the arm's copied binaries (not through cargo, the arm's
# runner or its gate).
run_one() {
    local out="$1" arm="$2" r="$3" suite="$4" s
    s=$(slot "$arm")
    local bin="$out/bin/$s" home="$out/home/$s-r$r" log="$out/logs/$arm-r$r.log" dest="$out/runs/$arm-r$r"
    local rc=0 p k b order crit snap="$REPO_ROOT/scripts/snapshot-bench-repeat.sh" profiles=(no-lto lto)
    [ "$suite" = cluster ] && profiles=(lto no-lto)
    echo "=== $arm r$r start $(date -u +%FT%TZ) head=$(git -C "$out/wt/$s" rev-parse --short HEAD) uptime: $(uptime)" >>"$out/ab.log"
    for p in "${profiles[@]}"; do
        crit="$home/$p/criterion"
        mkdir -p "$crit"
        for k in $(seq 1 "${BENCH_REPEATS:-5}"); do
            echo "=== $p invocation $k" >>"$log"
            case "$suite" in
            sbe)
                touch "$crit/.bench-since"
                for b in perf_parity_bench perf_parity_extended_bench; do
                    run_bin "$out" "$bin/$p/$b" "$crit" >>"$log" 2>&1 || rc=$?
                done
                "$snap" "$crit" "$k" >>"$log" 2>&1 || rc=$?
                ;;
            cluster)
                for order in forward reversed; do
                    touch "$crit/.bench-since"
                    run_bin "$out" "$bin/$p/cluster_codec_bench" "$crit" "$order" >>"$log" 2>&1 || rc=$?
                    "$snap" "$crit" "$k" "$([ "$order" = forward ] && echo repeats || echo repeats-reversed)" >>"$log" 2>&1 || rc=$?
                done
                ;;
            esac
        done
        mkdir -p "$dest/$p/criterion"
        cp -R "$crit"/repeats* "$dest/$p/criterion/" || echo "=== $arm r$r: no $p repeats to copy" >>"$out/ab.log"
    done
    echo "=== $arm r$r end rc=$rc $(date -u +%FT%TZ) uptime: $(uptime)" >>"$out/ab.log"
    return "$rc"
}

if [ "${1:-}" = --run-one ]; then
    shift
    run_one "$@"
    exit $?
fi
if [ "${1:-}" = --salt-check ]; then
    salt_check "${2:?base binary}" "${3:?base-rebuilt binary}" || exit 4
    exit 0
fi

suite=sbe rounds=3 out="" floor=1.0 print_plan=0 salt_only=0 commits=() failed=()
while [ $# -gt 0 ]; do
    case "$1" in
    --salt-only) salt_only=1 && shift ;;
    # [ $# -ge 2 ]: a value flag given last would fail `shift 2`, exit 1 (FAIL).
    --suite) [ $# -ge 2 ] || usage && suite=$2 && shift 2 ;;
    --rounds) [ $# -ge 2 ] || usage && rounds=$2 && shift 2 ;;
    --out) [ $# -ge 2 ] || usage && out=$2 && shift 2 ;;
    --floor) [ $# -ge 2 ] || usage && floor=$2 && shift 2 ;;
    --print-plan) print_plan=1 && shift ;;
    -*) usage ;;
    *) commits+=("$1") && shift ;;
    esac
done
case "$suite" in sbe | cluster) ;; *) usage ;; esac
# Position balance needs whole 3-round rotations; a partial one would put some
# arms first more often than others.
case "$rounds" in '' | *[!0-9]* | 0*) rounds=x ;; esac
[ "$rounds" != x ] && [ $((rounds % 3)) = 0 ] ||
    { echo "bench-ab: --rounds must be a positive multiple of 3 (3, 6, 9, ...) so every arm runs equally often in every position" >&2 && exit 2; }
n=0
# Digits and spaces only, checked before the unquoted loops can glob it.
case "$SALT_SIZES" in *[!0-9\ ]*) ;; *)
    for size in $SALT_SIZES; do
        case "$size" in 0*) n=0 && break ;; esac
        n=$((n + 1))
    done
    ;;
esac
[ "$n" -gt 0 ] ||
    { echo "bench-ab: BENCH_AB_SALT_SIZES must be a space-separated list of positive integers (got '${BENCH_AB_SALT_SIZES:-}')" >&2 && exit 2; }
if [ "$print_plan" = 1 ]; then
    plan "$rounds"
    exit 0
fi
[ "${#commits[@]}" = 2 ] || usage
command -v flock >/dev/null || { echo "bench-ab: flock is required" >&2 && exit 2; }

base=$(git -C "$REPO_ROOT" rev-parse --verify "${commits[0]}^{commit}")
branch=$(git -C "$REPO_ROOT" rev-parse --verify "${commits[1]}^{commit}")
main_checkout=$(cd "$(git -C "$REPO_ROOT" rev-parse --path-format=absolute --git-common-dir)/.." && pwd)
lock="${BENCH_LOCK:-$main_checkout/tmp/.mac-bench.lock}"
mkdir -p "$(dirname "$lock")"
out="${out:-$REPO_ROOT/target/bench-ab/$(date -u +%Y%m%dT%H%M%SZ)-$suite}"
mkdir -p "$(dirname "$out")"
# A fresh directory only: never write over an earlier run's evidence.
mkdir "$out"
out=$(cd "$out" && pwd)
mkdir "$out/wt" "$out/logs" "$out/runs"

{
    echo "bench-ab suite=$suite rounds=$rounds base=$base branch=$branch BENCH_REPEATS=${BENCH_REPEATS:-default}"
    echo "lock=$lock"
    plan "$rounds"
} | tee "$out/ab.log"

# build ARM LOG: build an arm's worktree, logging to logs/LOG.
build() {
    echo "=== build $1 ($2) $(date -u +%FT%TZ)" | tee -a "$out/ab.log"
    if ! (prebuild "$out/wt/$(slot "$1")" "$suite") >"$out/logs/$2.log" 2>&1; then
        echo "bench-ab: $1 failed to build; see $out/logs/$2.log" >&2
        exit 2
    fi
}

# The bench executables a build log lists, as cargo printed them: relative to
# the arm's worktree, e.g. target/bench-shared/no-lto/target/release/deps/<bench>-<hash>.
bins() { sed -n 's/^ *Executable .* (\(.*\))$/\1/p' "$out/logs/$1.log"; }

for arm in "${ARMS[@]}"; do
    commit=$base
    [ "$arm" = branch ] && commit=$branch
    git -C "$REPO_ROOT" worktree add --detach "$out/wt/$(slot "$arm")" "$commit" >/dev/null
    [ "$arm" = base-rebuilt ] || { [ "$arm" = branch ] && [ "$salt_only" = 1 ]; } || build "$arm" "build-$arm"
done

# The A/A control samples code placement only if the salt moved every timed
# loop within a cache line, in every bench binary and profile. How far the salt
# moves code depends on its size and on the alignment padding after it, so try
# a bounded set of sizes, rebuilding base-rebuilt each time, and refuse the run
# (exit 4) when none moves them all.
rebuilt="$out/wt/$(slot base-rebuilt)"
salt_size=""
read -r -a salted <<<"$(salt_files "$suite")"
for size in $SALT_SIZES; do
    git -C "$rebuilt" checkout -q -- "${salted[@]}"
    salt_worktree "$rebuilt" "$suite" "$size"
    build base-rebuilt "build-base-rebuilt-s$size"
    ok=1 unmoved=""
    while IFS=$'\t' read -r a b; do
        salt_check "$out/wt/$(slot base)/$a" "$rebuilt/$b" >"$out/logs/salt-check.tmp" ||
            { ok=0 && unmoved+="$(cat "$out/logs/salt-check.tmp")"$'\n'; }
        sed "s/^/salt size $size: /" "$out/logs/salt-check.tmp" >>"$out/ab.log"
        head -n 1 "$out/logs/salt-check.tmp" | sed "s/^/salt size $size: /"
    done < <(paste <(bins build-base) <(bins "build-base-rebuilt-s$size"))
    [ "$(bins build-base | wc -l)" -gt 0 ] || { echo "bench-ab: no bench executables in the build logs" >&2 && exit 2; }
    if [ "$ok" = 1 ]; then
        salt_size=$size
        break
    fi
done
if [ -z "$salt_size" ]; then
    printf 'bench-ab: no salt size in (%s) moved every timed loop within a cache line; the A/A control would be void. Every size is logged in %s; the last left these unmoved:\n%s' \
        "$SALT_SIZES" "$out/ab.log" "$unmoved" >&2
    exit 4
fi
echo "=== salt size $salt_size moved every timed Bencher::iter symbol within a cache line in every bench binary (loops LLVM merged into an identical one share its symbol)" | tee -a "$out/ab.log"
if [ "$salt_only" = 1 ]; then
    echo "results: $out (ab.log, logs/)"
    echo "worktrees left in place: $out/wt/arm0 (base), arm1 (base-rebuilt), arm2 (branch, not built)"
    exit 0
fi

# Copy each arm's executables to bin/<slot>/<profile>/<bench>, the paths
# run_one runs them from, with the Aeron client library rusteron-client built
# beside them, if any.
for arm in "${ARMS[@]}"; do
    log="build-$arm"
    [ "$arm" = base-rebuilt ] && log="build-base-rebuilt-s$salt_size"
    wt="$out/wt/$(slot "$arm")"
    while IFS= read -r f; do
        p=lto
        case "$f" in target/bench-shared/no-lto/* | target/bench-no-lto/*) p=no-lto ;; esac
        d="$out/bin/$(slot "$arm")/$p"
        n=${f##*/}
        { mkdir -p "$d" && cp "$wt/$f" "$d/${n%-*}"; } || { echo "bench-ab: cannot copy $wt/$f" >&2 && exit 2; }
        for l in "$wt/${f%/deps/*}"/build/rusteron-client-*/out/build/lib/libaeron.{dylib,so}; do
            if [ -f "$l" ]; then cp "$l" "$d/" || { echo "bench-ab: cannot copy $l" >&2 && exit 2; }; fi
        done
    done < <(bins "$log")
done

for r in $(seq 1 "$rounds"); do
    for arm in $(order "$r"); do
        echo "=== $arm r$r wait-lock $(date -u +%FT%TZ)" >>"$out/ab.log"
        flock "$lock" "$SELF" --run-one "$out" "$arm" "$r" "$suite" || failed+=("$arm-r$r")
        tail -n 1 "$out/ab.log"
    done
done

rc=0
python3 -I "$REPO_ROOT/scripts/bench-ab-judge.py" "$out" --floor "$floor" | tee "$out/table.txt" || rc=${PIPESTATUS[0]}
# A failed arm run (a bench binary crash or a failed snapshot) never reads as a pass.
if [ "${#failed[@]}" -gt 0 ]; then
    echo "bench-ab: run(s) exited non-zero: ${failed[*]} (see $out/logs)" >&2
    [ "$rc" = 0 ] && rc=2
fi
echo ""
echo "=== runs (uptime per run) ==="
grep -E '^=== .* (start|end) ' "$out/ab.log"
echo ""
echo "results: $out (table.txt, ab.log, logs/, runs/)"
echo "worktrees left in place: $out/wt/arm0 (base), arm1 (base-rebuilt), arm2 (branch)"
exit "$rc"
