#!/bin/bash
# bench-run.sh <name> [VAR=value...]: one timed many_streams run under the shared bench lock,
# with vmstat, top and /proc/interrupts captured alongside. Output in ~/ergon-work/rusteron-readiness/<name>.
set -u
name=$1; shift
cd ~/ergon-work/rusteron-readiness/repo
cargo build --release -p rusteron-client --bench many_streams 2>&1 | grep -E '^error' -A15
out=$HOME/ergon-work/rusteron-readiness/$name
rm -rf "$out"; mkdir -p "$out"
exec 9>$HOME/ergon-work/.bench.lock
flock 9
vmstat -w 1 > "$out/vmstat.log" & v=$!
top -b -d 1 -w 200 | grep --line-buffered -A6 '^ *PID' > "$out/top.log" & t=$!
cat /proc/interrupts > "$out/irq-before.txt"
env "$@" MANY_STREAMS_OUT="$out" cargo bench -p rusteron-client --bench many_streams > "$out/stdout.txt" 2>&1
echo "exit $?" >> "$out/stdout.txt"
cat /proc/interrupts > "$out/irq-after.txt"
kill $v $t
grep -v '^warning\|manifest\|Finished\|Running' "$out/stdout.txt"
