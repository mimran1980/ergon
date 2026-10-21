#!/usr/bin/env bash
# One CSV line per measured rep of the latency-pods lab.
#
#   latency-bench-line.sh --mode <mode> --rep <n> --run-dir <dir> [--header]
#   latency-bench-line.sh --mode <mode> --rep <n> --run-dir <dir> --markdown
#
# <dir> is one rep's collected inputs (latency-bench-podrun.sh writes them, the lab
# script fetches them). Reads rtt-line.txt, attributes the max with
# latency-bench-attribute.py, prints exactly one CSV line (or one benchmarks.md
# table row with --markdown) on stdout. Appends nothing; the lab script
# redirects. The header is printed only with --header and no other output.
#
# Columns (fixed, bench-harness.md):
#   mode,messages,p50_ns,p99_ns,p999_ns,p9999_ns,MAX_NS,loss,max_cause
#
# Reps are one line each, consecutive lines share the mode (rep 1 first). A
# mode that did not run prints no line at all: its reason belongs in
# benchmarks.md "Not measured / open".
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
mode= rep=1 run_dir= header=0 markdown=0 loss_detail=

while [[ $# -gt 0 ]]; do
    case $1 in
    --mode)
        mode=${2:?--mode needs a value}
        shift 2
        ;;
    --rep)
        rep=${2:?--rep needs a value}
        shift 2
        ;;
    --run-dir)
        run_dir=${2:?--run-dir needs a value}
        shift 2
        ;;
    --loss-detail)
        loss_detail=${2:?--loss-detail needs a value}
        shift 2
        ;;
    --header)
        header=1
        shift
        ;;
    --markdown)
        markdown=1
        shift
        ;;
    *)
        echo "latency-bench-line: unknown arg $1" >&2
        exit 2
        ;;
    esac
done

csv=$here/latency-bench-csv.py
attr=$here/latency-bench-attribute.py
rtt_file=$run_dir/rtt-line.txt

if [[ $header -eq 1 && -z $run_dir ]]; then
    python3 "$csv" --header --mode baseline
    exit 0
fi

[[ -n $mode ]] || {
    echo "latency-bench-line: --mode is required" >&2
    exit 2
}
[[ -n $run_dir && -f $rtt_file ]] || {
    echo "latency-bench-line: --run-dir must hold rtt-line.txt (got '$run_dir')" >&2
    exit 2
}

# The attributed cause of the max: never empty, never dropped. A run with no
# attribution input still gets a line, with OPEN: as the cause.
cause=$(python3 "$attr" --run-dir "$run_dir" --mode "$mode" --rep "$rep" --cause-only 2>>"$run_dir/attribute.log" ||
    echo "OPEN: attribution failed (see attribute.log)")

if [[ $markdown -eq 1 ]]; then
    python3 "$csv" --mode "$mode" --rep "$rep" --rtt-file "$rtt_file" --cause "$cause" ${loss_detail:+--loss-detail "$loss_detail"} --markdown
else
    python3 "$csv" --mode "$mode" --rep "$rep" --rtt-file "$rtt_file" --cause "$cause"
fi
