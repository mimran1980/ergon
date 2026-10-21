#!/usr/bin/env python3
"""One CSV line per measured run of the latency-pods lab (aws/eks-nri.sh).

latency-bench-csv.py --mode <mode> --rep <n> --rtt-file <path> [options] >> results-latency.csv

--rtt-file holds xping's stdout line:

    rtt,<label>,udp,p50,p90,p99,p99.9,p99.99,max,mean,samples,lost

(nanoseconds; `lost` is round trips that never echoed). The CSV schema is
fixed (bench-harness.md):

    mode,messages,p50_ns,p99_ns,p999_ns,p9999_ns,MAX_NS,loss,max_cause

- mode: the run's mode label. The only mode set is M0..M6:

    baseline|hugepages|partition|seccomp|devices|probe|all

- messages: round trips attempted (the xping loop's iterations: samples+loss)
- p50_ns,p99_ns,p999_ns,p9999_ns,MAX_NS: the histogram's quantiles and its
  max in nanoseconds. Every latency figure ships with its MAX_NS; the repo
  rule is that the max is then investigated to a cause.
- loss: round trips that never echoed (the xping `lost` field)
- max_cause: why the max happened, from latency-bench-attribute.py. Never empty:
  with no cause it is "OPEN: ..." and benchmarks.md lists it as open. Never
  dropped, never smoothed.

Reps are one line each, consecutive lines share the mode (rep 1 first). The
header is written with --header only. --markdown prints one benchmarks.md
table row instead of CSV (--mode still required). A mode that did not run
prints no line here at all: its reason belongs in benchmarks.md's
"Not measured / open".

--loss-detail NAK/retransmit/short_send puts the Aeron loss counters in the
markdown row's loss cell (CSV keeps the numeric echo loss; both numbers must
agree with the run's counters log).
"""
import argparse
import re
import sys

MODES = (
    "baseline",
    "hugepages",
    "partition",
    "seccomp",
    "devices",
    "probe",
    "all",
)
HEADER = "mode,messages,p50_ns,p99_ns,p999_ns,p9999_ns,MAX_NS,loss,max_cause"
RTT = re.compile(
    r"^rtt,(?P<label>[^,]*),udp,(?P<p50>\d+),(?P<p90>\d+),(?P<p99>\d+),"
    r"(?P<p999>\d+),(?P<p9999>\d+),(?P<max>\d+),(?P<mean>\d+),(?P<samples>\d+),(?P<lost>\d+)\s*$"
)


class Bad(Exception):
    pass


def parse_rtt_file(path):
    """The one rtt line in `path`. A file with none, or with two, is a bug:
    a run that printed nothing did not measure, and two lines are two runs."""
    if path == "-":
        text = sys.stdin.read()
    else:
        with open(path, encoding="utf-8") as f:
            text = f.read()
    rows = [RTT.match(ln) for ln in text.splitlines() if ln.startswith("rtt,")]
    rows = [m for m in rows if m]
    if not rows:
        raise Bad(f"{path}: no rtt line (the run measured nothing; do not invent numbers)")
    if len(rows) > 1:
        raise Bad(f"{path}: {len(rows)} rtt lines (one run per file; split them)")
    return rows[0]


def csv_field(text):
    """`text` as one CSV field (quoted when it holds a comma or a quote)."""
    if any(c in text for c in ',"\n\r'):
        return '"' + text.replace('"', '""') + '"'
    return text


def cause_text(cause):
    cause = " ".join(cause.split()).strip()
    if not cause:
        return "OPEN: no attribution input (latency-bench-attribute.py was not run)"
    return cause


def row(args, m):
    samples = int(m.group("samples"))
    lost = int(m.group("lost"))
    return [
        args.mode,
        str(samples + lost),
        m.group("p50"),
        m.group("p99"),
        m.group("p999"),
        m.group("p9999"),
        m.group("max"),
        str(lost),
        cause_text(args.cause),
    ]


def markdown(args, m, detail):
    samples = int(m.group("samples"))
    lost = int(m.group("lost"))
    loss = detail or str(lost)
    return (
        f"| {args.mode} | rep {args.rep} | {m.group('p50')} | {m.group('p90')} | "
        f"{m.group('p99')} | {m.group('p999')} | {m.group('p9999')} | **{m.group('max')}** | "
        f"{m.group('mean')} | {samples} | {loss} | {cause_text(args.cause)} |"
    )


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--mode", required=True, choices=MODES)
    p.add_argument("--rep", type=int, default=1, help="rep number, for --markdown only")
    p.add_argument("--rtt-file", default="", help="xping's stdout (a file, or - for stdin); required unless --header alone")
    p.add_argument("--cause", default="", help="the attributed cause of the max (latency-bench-attribute.py)")
    p.add_argument("--cause-file", default="", help="file holding the cause (overrides --cause)")
    p.add_argument("--loss-detail", default="", help="NAK/retransmit/short for the markdown loss cell")
    p.add_argument("--header", action="store_true", help="print the CSV header line")
    p.add_argument("--markdown", action="store_true", help="one benchmarks.md table row instead of CSV")
    args = p.parse_args()

    if args.header and not args.rtt_file and not args.markdown:
        print(HEADER)
        return 0
    if not args.rtt_file:
        print("latency-bench-csv: --rtt-file is required", file=sys.stderr)
        return 2
    if args.cause_file:
        with open(args.cause_file, encoding="utf-8") as f:
            args.cause = f.read()
    try:
        m = parse_rtt_file(args.rtt_file)
    except Bad as e:
        print(f"latency-bench-csv: {e}", file=sys.stderr)
        return 2
    if args.header:
        print(HEADER)
    if args.markdown:
        print(markdown(args, m, args.loss_detail))
    else:
        print(",".join(csv_field(c) for c in row(args, m)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
