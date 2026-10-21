#!/usr/bin/env python3
"""Why each latency spike (and each rep's max) happened, for one NRI lab rep.

eks-nri-attribute.py <spikes.csv> [options]

  --stalls <file>        stalls-ping.csv or stalls-pong.csv, repeatable
  --gapspin <file>       gapspin log ("CLOCK_REALTIME_s.ns gap_us" per gap), repeatable
  --probe-every <sec>    the pod ran an exec liveness probe every <sec> (M5)
  --vmstat <before> <after>   /proc/vmstat around the run (pgfault/pgmajfault)
  --threshold-us <n>     spikes to explain (default 200)

A spike of at least the threshold overlapping:
  * a client loop stall (stalls-*.csv) is a client stall, named by who and how long;
  * a gapspin gap is a host stall (the CPU did not run the spinner);
  * a probe kick window (probe-every) is a liveness probe kick;
otherwise, with major faults having grown, a page-fault burst; else OPEN.
The rep's max is always explained or listed as OPEN — never dropped.

Prints one line per spike: <due_ns>,<latency_ns>,<cause> and a final
MAX,<due_ns>,<latency_ns>,<cause> for the largest latency.
"""
import argparse
import sys


def start_realtime(path):
    with open(path) as f:
        for line in f:
            if line.startswith("# start_realtime_ns="):
                return int(line.split("=", 1)[1])
    return None


def read_spikes(path):
    rows = []
    with open(path) as f:
        for line in f:
            if line.startswith("#") or line.startswith("stream,"):
                continue
            parts = line.strip().split(",")
            if len(parts) == 3:
                rows.append((int(parts[1]), int(parts[2])))
    return rows


def read_stalls(path, start):
    spans = []
    with open(path) as f:
        for line in f:
            if line.startswith("#") or line.startswith("end_ns"):
                continue
            parts = line.strip().split(",")
            if len(parts) == 2:
                end, gap = int(parts[0]), int(parts[1])
                spans.append((end - gap, end, f"client stall {path.split('/')[-1]} {gap / 1000:.0f}us"))
    return spans


def read_gapspin(path, start):
    spans = []
    with open(path) as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            parts = line.strip().split()
            if len(parts) != 2:
                continue
            sec_ns, gap_us = parts[0], float(parts[1])
            s, _, ns = sec_ns.partition(".")
            end_abs = int(s) * 10**9 + int(ns.ljust(9, "0")[:9])
            if start is not None:
                end = end_abs - start
            else:
                end = end_abs
            gap_ns = int(gap_us * 1000)
            if gap_ns > 0:
                spans.append((end - gap_ns, end, f"host stall {path.split('/')[-1]} {gap_us:.0f}us"))
    return spans


def overlaps(a0, a1, b0, b1):
    return a0 < b1 and b0 < a1


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("spikes")
    ap.add_argument("--stalls", action="append", default=[])
    ap.add_argument("--gapspin", action="append", default=[])
    ap.add_argument("--probe-every", type=float, default=0.0)
    ap.add_argument("--probe-start", type=int, default=None, help="realtime ns of the pod's start")
    ap.add_argument("--vmstat", nargs=2, default=None)
    ap.add_argument("--threshold-us", type=float, default=200.0)
    args = ap.parse_args()

    threshold_ns = int(args.threshold_us * 1000)
    start = start_realtime(args.spikes)
    rows = read_spikes(args.spikes)
    spans = []
    for f in args.stalls:
        spans += read_stalls(f, start)
    for f in args.gapspin:
        spans += read_gapspin(f, start)

    maj = 0
    if args.vmstat:
        def vm(path):
            out = {}
            with open(path) as f:
                for line in f:
                    k, _, v = line.partition(" ")
                    out[k] = int(v)
            return out
        b, a = vm(args.vmstat[0]), vm(args.vmstat[1])
        maj = a.get("pgmajfault", 0) - b.get("pgmajfault", 0)

    explained = []
    for due, lat in sorted((d, l) for d, l in rows if l >= threshold_ns):
        s0, s1 = due, due + lat
        cause = "OPEN"
        for b0, b1, what in spans:
            if overlaps(s0, s1, b0, b1):
                cause = what
                break
        if cause == "OPEN" and args.probe_every > 0 and start is not None and args.probe_start is not None:
            rel_kick = ((due + start - args.probe_start) / 1e9) % args.probe_every
            window = min(0.15, args.probe_every / 2)
            if rel_kick < window or rel_kick > args.probe_every - window:
                cause = f"liveness probe kick (every {args.probe_every:g}s)"
        if cause == "OPEN" and maj > 0:
            cause = f"page faults (majfault +{maj} over the run)"
        explained.append((due, lat, cause))
        print(f"{due},{lat},{cause}")

    if rows:
        due, lat = max(rows, key=lambda r: r[1])
        if lat < threshold_ns:
            cause = "below threshold"
        else:
            cause = next(c for d, l, c in explained if d == due and l == lat)
        print(f"MAX,{due},{lat},{cause}")
    else:
        print("MAX,,,no spikes recorded")


if __name__ == "__main__":
    sys.exit(main())
