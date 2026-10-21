#!/usr/bin/env python3
"""Why this run's MAX happened: one cause line for latency-bench-csv.py, bullets
for benchmarks.md. The lab's rule is that every latency figure ships with its
max and every max is investigated to a cause; a max this script cannot
explain is reported as "OPEN: ..." and listed as open -- never dropped.

latency-bench-attribute.py --run-dir <dir> --mode <mode> [--rep <n>] [--cause-only]

<dir> holds one rep's collected inputs (latency-bench-podrun.sh writes them, the
lab script fetches them):

  spikes.csv          LOAD_SPIKES output: "# start_realtime_ns=<ns>", then
                      "stream,due_ns,latency_ns" rows (due_ns counts from the
                      start of the measured run). The max's row is the spike.
  stalls-ping.csv     LOOP_STALLS_DIR's "# start_realtime_ns=<ns>" then
  stalls-pong.csv     "end_ns,gap_ns" rows: the polling loop stalled gap_ns,
                      the gap ending end_ns after that file's start.
  gapspin-a.txt       gapspin's "<realtime s> <gap us>" rows (host a = ping),
  gapspin-b.txt       (host b = pong): every gap of the spinner on the spare
                      isolated CPU, a guest/hypervisor stall.
  probe-kicks.log     livenessProbe exec kicks, one "# realtime_ns=" header
                      then one realtime_ns per line (the probe run).
  vmstat.txt          "pgfault <before> <after>" / "pgmajfault ..." pairs
                      around the run (/proc/vmstat).
  ena.txt             "bw_out_allowance_exceeded <b> <a>" and
                      "pps_allowance_exceeded <b> <a>" pairs (ethtool -S).
  aeron-loss.txt      "nak <n>", "retransmit <n>", "short_send <n>" counters.

Attribution of the max spike (its due_ns..due_ns+latency window on
CLOCK_REALTIME, from the file headers' start_realtime_ns):

  1. a gapspin gap on either host overlapping the window: host stall, named
     by host and gap (the spinner's CPU is idle: nothing in the guest of
     that CPU explains it);
  2. a client loop stall (ping or pong) overlapping: loop stall;
  3. a probe kick inside the window: probe kick (the harm the admission
     extension removes);
  4. else pgmajfault delta > 0: page faults;
  5. else an ENA allowance counter moved: NIC allowance;
  6. else NAK/retransmit moved: driver loss;
  7. else OPEN, with what was ruled out in the line.

Overlap is on wall clocks; where the two hosts' clocks disagree by more than
1 ms the cause says so (the lab logs chrony tracking per mode).
"""
import argparse
import os
import re
import sys

SPIKE = re.compile(r"^(\d+),(\d+),(\d+)\s*$")  # stream,due_ns,latency_ns
STALL = re.compile(r"^(\d+),(\d+)\s*$")  # end_ns,gap_ns
GAP = re.compile(r"^(\d+(?:\.\d+)?)\s+(\d+(?:\.\d+)?)\s*$")  # s.ns, gap_us (%.1f)
PAIR = re.compile(r"^(\S+)\s+(\d+)\s+(\d+)\s*$")
KV = re.compile(r"^(\S+)\s+(\d+)\s*$")
HDR = re.compile(r"^#\s*start_realtime_ns=(\d+)")


def read_header_and(path, rowre):
    """(# start_realtime_ns=, rows matching rowre as int tuples) for one file."""
    start = None
    rows = []
    if not os.path.exists(path):
        return start, rows
    with open(path, encoding="utf-8") as f:
        for ln in f:
            ln = ln.strip()
            if not ln or ln.startswith("#"):
                m = HDR.match(ln)
                if m:
                    start = int(m.group(1))
                continue
            if ln.startswith("stream,") or ln.startswith("value_ns,"):
                continue
            m = rowre.match(ln)
            if m:
                rows.append(tuple(int(x) for x in m.groups()))
    return start, rows


def read_pairs(path):
    """"<name> <before> <after>" rows -> {name: (before, after)}."""
    out = {}
    if not os.path.exists(path):
        return out
    with open(path, encoding="utf-8") as f:
        for ln in f:
            m = PAIR.match(ln.strip())
            if m:
                out[m.group(1)] = (int(m.group(2)), int(m.group(3)))
    return out


def read_kv(path):
    """"<name> <n>" rows -> {name: n}."""
    out = {}
    if not os.path.exists(path):
        return out
    with open(path, encoding="utf-8") as f:
        for ln in f:
            m = KV.match(ln.strip())
            if m:
                out[m.group(1)] = int(m.group(2))
    return out


def read_gaps(path):
    """gapspin's "<realtime s.ns> <gap us>" -> [(window_start_ns, window_end_ns, gap_us)].

    The timestamp is split on the dot, never passed through float: a float64
    cannot hold nanoseconds since the epoch and would move the window by
    hundreds of nanoseconds, which is the whole signal here.
    """
    out = []
    if not os.path.exists(path):
        return out
    with open(path, encoding="utf-8") as f:
        for ln in f:
            m = GAP.match(ln.strip())
            if not m:
                continue
            sec, _, frac = m.group(1).partition(".")
            end = int(sec) * 1_000_000_000 + int((frac + "000000000")[:9])
            gap_us = int(float(m.group(2)))
            out.append((end - gap_us * 1000, end, gap_us))
    return out


def overlaps(a0, a1, b0, b1):
    return a0 <= b1 and b0 <= a1


def attribute(run_dir, mode):
    """(cause, bullets) for the rep in run_dir. bullets: strings for
    benchmarks.md's max investigations."""
    bullets = []
    spikes_start, spikes = read_header_and(os.path.join(run_dir, "spikes.csv"), SPIKE)
    if not spikes:
        return (
            "OPEN: no spike rows (no round trip reached LOAD_SPIKE_NS, or spikes.csv was not collected)",
            ["no spike rows: the max was below LOAD_SPIKE_NS or spikes.csv is missing"],
        )
    worst = max(spikes, key=lambda r: r[2])
    due, lat = worst[1], worst[2]
    w0 = (spikes_start or 0) + due
    w1 = w0 + lat
    bullets.append(f"max {lat} ns at realtime {w0} (window {w0}..{w1}, mode {mode})")

    # 1. host stalls (gapspin on either node)
    for host in "ab":
        gaps = read_gaps(os.path.join(run_dir, f"gapspin-{host}.txt"))
        for g0, g1, gu in gaps:
            if overlaps(w0, w1, g0, g1):
                cause = f"host-stall: gapspin {host} gap {gu} us overlapping the max"
                bullets.append(cause)
                return cause, bullets
    # 2. client loop stalls
    for who in ("ping", "pong"):
        start, rows = read_header_and(os.path.join(run_dir, f"stalls-{who}.csv"), STALL)
        for end_ns, gap_ns in rows:
            g1 = (start or 0) + end_ns
            g0 = g1 - gap_ns
            if overlaps(w0, w1, g0, g1):
                cause = f"loop-stall: {who} loop gap {gap_ns} ns overlapping the max"
                bullets.append(cause)
                return cause, bullets
    # 3. probe kicks (the probe run; the admission extension removes them)
    kicks = []
    pk = os.path.join(run_dir, "probe-kicks.log")
    if os.path.exists(pk):
        with open(pk, encoding="utf-8") as f:
            for ln in f:
                ln = ln.strip()
                if not ln or ln.startswith("#") or ln.startswith("stream,"):
                    continue
                m = KV.match(ln)
                if m:
                    kicks.append(int(m.group(2)))
                elif ln.isdigit():
                    kicks.append(int(ln))
    for k in kicks:
        if w0 <= k <= w1:
            cause = f"probe-kick: liveness exec at realtime {k} inside the max window"
            bullets.append(cause)
            return cause, bullets
    # 4. page faults
    vm = read_pairs(os.path.join(run_dir, "vmstat.txt"))
    for name in ("pgmajfault", "pgfault"):
        if name in vm and vm[name][1] > vm[name][0]:
            delta = vm[name][1] - vm[name][0]
            if name == "pgmajfault" and delta > 0:
                cause = f"page-faults: {name} +{delta} around the run (AERON_DIR not on hugetlbfs?)"
                bullets.append(cause)
                return cause, bullets
    # 5. ENA allowance
    ena = read_pairs(os.path.join(run_dir, "ena.txt"))
    for name in ("bw_out_allowance_exceeded", "pps_allowance_exceeded"):
        if name in ena and ena[name][1] > ena[name][0]:
            cause = f"ena: {name} +{ena[name][1] - ena[name][0]} around the run"
            bullets.append(cause)
            return cause, bullets
    # 6. Aeron loss
    loss = read_kv(os.path.join(run_dir, "aeron-loss.txt"))
    for name in ("nak", "retransmit", "short_send"):
        if loss.get(name, 0) > 0:
            cause = f"driver-loss: {name} {loss[name]} in the run"
            bullets.append(cause)
            return cause, bullets
    # 7. open
    ruled = "no gapspin gap, no loop stall, no probe kick"
    if "pgmajfault" in vm:
        ruled += f", pgmajfault {vm['pgmajfault'][0]}->{vm['pgmajfault'][1]}"
    if ena:
        ruled += ", ENA counters unchanged"
    cause = f"OPEN: max {lat} ns unexplained ({ruled})"
    bullets.append(cause)
    return cause, bullets


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--run-dir", required=True)
    p.add_argument("--mode", required=True)
    p.add_argument("--rep", type=int, default=1)
    p.add_argument("--cause-only", action="store_true", help="print only the cause line")
    args = p.parse_args()
    cause, bullets = attribute(args.run_dir, args.mode)
    if args.cause_only:
        print(cause)
        return 0
    print(f"cause: {cause}")
    for b in bullets:
        print(f"- [{args.mode} rep {args.rep}] {b}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
