#!/usr/bin/env python3
"""Summarise many_streams runs: one CSV per table, from run dirs given as arguments.

Each run dir holds summary.tsv, memory.tsv, stdout.txt, stalls-*.tsv; its sibling <dir>-sys
holds syscalls.<pid> from sysccount.so. Writes layout-*.csv next to the first dir's parent.
"""
import csv, glob, os, re, sys

def us(ns): return f"{int(ns)/1000:.2f}"

def stalls(run, arm, n):
    out = {}
    for f in glob.glob(f"{run}/stalls-{arm}-{n}-*.tsv"):
        role = f[len(f"{run}/stalls-{arm}-{n}-"):-4]
        rows = []
        for line in open(f).read().splitlines()[1:]:
            at, ns, _w, p = line.split("\t")
            rows.append((float(at), int(ns), int(p)))
        out[role] = rows
    return out

def attribute(run, row, st):
    """Why the max happened: the matching stall's preemption, or the overlapping thread stall."""
    mx = int(row["max_ns"])
    if mx < 100_000:
        return "under 100 us"
    role = row["role"]
    at = float(row["max_at_ms"])
    if role != "rtt":
        # the histogram rounds its max, so take this thread's largest kept stall
        kept = max(st.get(role, []), key=lambda r: r[1], default=None)
        if kept is None or abs(kept[1] - mx) > mx * 0.01:
            return "not kept"
        return "preempted" if kept[2] > 0 else "no switch"
    best = None
    for r, rows in st.items():
        if r == "rtt":
            continue
        for a, ns, p in rows:
            if a <= at + mx / 1e6 and a + ns / 1e6 >= at and (best is None or ns > best[1]):
                best = (r, ns, p)
    if best is None:
        return "no overlapping thread stall"
    return f"{best[0]} stall {us(best[1])} us, {'preempted' if best[2] > 0 else 'no switch'}"

def syscalls(run):
    """(arm, n, role) -> {syscall: per cycle} from stdout agent lines and the shim's dumps."""
    per_tid = {}
    for f in glob.glob(f"{run}-sys/syscalls.*"):
        for line in open(f):
            parts = line.split()
            per_tid[int(parts[1])] = {k: int(v) for k, v in (p.split("=") for p in parts[2:])}
    out, pending = {}, []
    for line in open(f"{run}/stdout.txt"):
        m = re.match(r"agent (\S+) tid (\d+) cycles (\d+)", line)
        if m:
            pending.append((m.group(1), int(m.group(2)), int(m.group(3))))
            continue
        m = re.match(r"(\S+)\s+n=(\d+)\s", line)
        if m and pending:
            for role, tid, cycles in pending:
                counts = per_tid.get(tid, {})
                out[(m.group(1), m.group(2), role)] = {k: v / cycles for k, v in counts.items() if v} if cycles else {}
            pending = []
    return out

def main():
    runs = sys.argv[1:]
    dest = os.path.dirname(os.path.abspath(runs[0]))
    lat = csv.writer(open(f"{dest}/layout-latency.csv", "w"))
    lat.writerow(["run", "arm", "n", "role", "count", "p50_us", "p99_us", "max_us", "max_cause", "syscalls_per_cycle"])
    mem = csv.writer(open(f"{dest}/layout-memory.csv", "w"))
    mem.writerow(["run", "arm", "n", "log_files", "log_apparent_mib", "log_allocated_mib", "rss_mib", "rss_anon_mib", "rss_shmem_mib"])
    for run in runs:
        name = os.path.basename(run)
        sc = syscalls(run)
        cache = {}
        for row in csv.DictReader(open(f"{run}/summary.tsv"), delimiter="\t"):
            if row["role"].startswith("aeron-") or row["arm"] == "mark":
                continue
            key = (row["arm"], row["n"])
            if key not in cache:
                cache[key] = stalls(run, *key)
            calls = sc.get((row["arm"], row["n"], row["role"]), {})
            lat.writerow([name, row["arm"], row["n"], row["role"], row["count"], us(row["p50_ns"]), us(row["p99_ns"]),
                          us(row["max_ns"]), attribute(run, row, cache[key]),
                          " ".join(f"{k}={v:.2f}" for k, v in sorted(calls.items()))])
        if os.path.exists(f"{run}/memory.tsv"):
            mib = lambda b: f"{int(b) / 2**20:.1f}"
            for row in csv.DictReader(open(f"{run}/memory.tsv"), delimiter="\t"):
                mem.writerow([name, row["arm"], row["n"], row["log_files"], mib(row["log_apparent_bytes"]),
                              mib(row["log_allocated_bytes"]), mib(row["rss"]), mib(row["rss_anon"]), mib(row["rss_shmem"])])

main()
