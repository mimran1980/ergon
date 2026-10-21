#!/usr/bin/env python3
"""benchmarks.md from one latency-pods lab run: the environment block, the
results table, the max investigations, what the extensions changed, and what
is not measured. The lab script's `report` step runs it on the collected
directory; the run appends nothing by hand.

latency-bench-report.py --collected <dir> --stamp <eks-stamp> [-o benchmarks.md]

<dir> (what aws/eks-nri.sh collects under target/md-dpdk-bench/<stamp>/):

  results-latency.csv     latency-bench-csv.py's lines (the fixed schema)
  results-latency.md      latency-bench-csv.py --markdown rows (full rtt line) when
                      the lab collected them
  facts.txt           node facts (kernel, containerd, runc, numactl, ptp,
                      RT_GROUP_SCHED, rmem_max, ...)
  attribution/*.txt   latency-bench-attribute.py's bullets (one file per mode/rep)
  skipped.txt         "mode<TAB>reason" lines for modes that could not run

Every latency figure in the table carries its max; every max is investigated
in the bullets section or listed as OPEN. A surprising win is a suspected
benchmark defect: the verdict lines say so where a mode beat baseline.
Latency verdicts come from Linux only.
"""
import argparse
import csv
import glob
import os
import sys

# The only mode set is M0..M6 (aws/eks-nri.sh and latency-bench-modes.sh).
PLAN_MODES = (
    "baseline",
    "hugepages",
    "partition",
    "seccomp",
    "devices",
    "probe",
    "all",
)
DESC = {
    "baseline": "M0: plain emptyDir AERON_DIR, default seccomp, no extension annotations, no probe",
    "hugepages": "M1: sized hugetlbfs on /dev/hugepages via NRI AddMount (pagesize=2M,size=)",
    "partition": "M2: cpuset.cpus.partition=isolated on the exclusive CPUs (createRuntime child cgroup)",
    "seccomp": "M3: RuntimeDefault plus the io_uring_* allow rule (never Unconfined)",
    "devices": "M4: PHC /dev/ptp* read-only via NRI AddDevice (cpuqos is x86 only)",
    "probe": "M5: baseline plus a livenessProbe.exec every 1s: the harm the admission extension removes",
    "all": "M6: hugepages + partition + seccomp + devices, no probe",
}


def med(xs):
    xs = sorted(xs)
    return xs[len(xs) // 2] if xs else 0


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--collected", required=True)
    p.add_argument("--stamp", required=True)
    p.add_argument("-o", "--output", default="-")
    args = p.parse_args()
    d = args.collected

    rows = []
    path = os.path.join(d, "results-latency.csv")
    if os.path.exists(path):
        with open(path, newline="", encoding="utf-8") as f:
            rows = list(csv.DictReader(f))
    skipped = {}
    sp = os.path.join(d, "skipped.txt")
    if os.path.exists(sp):
        with open(sp, encoding="utf-8") as f:
            for ln in f:
                if "\t" in ln:
                    m, why = ln.rstrip("\n").split("\t", 1)
                    skipped[m] = why

    facts = ""
    fp = os.path.join(d, "facts.txt")
    if os.path.exists(fp):
        with open(fp, encoding="utf-8") as f:
            facts = f.read().strip()

    out = []
    out.append(f"# Latency-pod extensions on EKS (ergon-lab-{args.stamp})")
    out.append("")
    out.append("Latency verdicts come from Linux only. Every figure carries its max;")
    out.append("every max is investigated below. A surprising win is a suspected benchmark defect.")
    out.append("")
    out.append("## Environment")
    out.append("")
    out.append("```text")
    out.append(facts or "(facts.txt missing)")
    out.append("```")
    out.append("")
    out.append("## Results (one row per measured rep, nanoseconds)")
    out.append("")
    out.append("| mode | rep | p50 | p90 | p99 | p99.9 | p99.99 | max | mean | samples | loss (NAK/retransmit/short) | max cause |")
    out.append("|---|---|---|---|---|---|---|---|---|---|---|---|")
    md = os.path.join(d, "results-latency.md")
    if os.path.exists(md):
        with open(md, encoding="utf-8") as f:
            for ln in f:
                if ln.startswith("| "):
                    out.append(ln.rstrip())
    else:
        seen = {}
        for r in rows:
            mode = r["mode"]
            seen.setdefault(mode, 0)
            seen[mode] += 1
            out.append(
                f"| {mode} | {seen[mode]} | {r['p50_ns']} | n/a | {r['p99_ns']} | "
                f"{r['p999_ns']} | {r['p9999_ns']} | **{r['MAX_NS']}** | n/a | "
                f"{r['messages']} | {r['loss']} | {r['max_cause']} |"
            )
    for m in PLAN_MODES:
        if m in skipped:
            out.append(f"| {m} | - | - | - | - | - | - | - | - | 0 | - | SKIPPED: {skipped[m]} |")
    if not rows and not skipped:
        out.append("| (no runs collected) | | | | | | | | | | | |")
    out.append("")
    out.append("## Max investigations")
    out.append("")
    bullets = []
    for bp in sorted(glob.glob(os.path.join(d, "attribution", "*.txt"))):
        with open(bp, encoding="utf-8") as f:
            for ln in f:
                if ln.startswith("- "):
                    bullets.append(ln.rstrip())
    for r in rows:
        cause = r.get("max_cause", "")
        if cause.startswith("OPEN:"):
            bullets.append(f"- [{r['mode']}] {cause}")
    if bullets:
        out.extend(bullets)
    else:
        out.append("(no max to investigate: no runs collected)")
    out.append("")
    out.append("## What the extensions changed")
    out.append("")
    for m in PLAN_MODES:
        mine = [r for r in rows if r["mode"] == m]
        base = [r for r in rows if r["mode"] == "baseline"]
        if m in skipped:
            out.append(f"- **{m}**: not run — {skipped[m]}")
            continue
        if not mine:
            out.append(f"- **{m}**: no rows collected")
            continue
        if m == "baseline" or not base:
            out.append(f"- **{m}**: reference ({DESC[m]})")
            continue
        bmax, mmax = med([int(r["MAX_NS"]) for r in base]), med([int(r["MAX_NS"]) for r in mine])
        bp50, mp50 = med([int(r["p50_ns"]) for r in base]), med([int(r["p50_ns"]) for r in mine])
        verdict = "unchanged"
        if mmax < bmax * 0.9 or mp50 < bp50 * 0.9:
            verdict = f"faster than baseline (p50 {bp50}->{mp50}, max {bmax}->{mmax}) — suspected benchmark defect unless explained"
        elif mmax > bmax * 1.1 or mp50 > bp50 * 1.1:
            verdict = f"slower (p50 {bp50}->{mp50}, max {bmax}->{mmax})"
        out.append(f"- **{m}**: {DESC.get(m, '')} — {verdict}")
    out.append("")
    out.append("## Not measured / open")
    out.append("")
    out.append("- the veth path (this lab runs hostNetwork: design.md test 4)")
    out.append("- hardware packet timestamps (ENA has none; the PHC is a clock source only)")
    out.append("- SCHED_FIFO (measured not worth it on AWS: io/benches/BENCHMARK.md)")
    out.append("- SR-IOV VF per pod (Nitro guests have one ENA per ENI, no in-guest VFs)")
    out.append("- NUMA cpuset.mems on this lab size (c8g.xlarge is single-NUMA)")
    out.append("- IRQ pinning to the shared CPUs (our Graviton data says it costs the tail)")
    out.append("- netns sysctls at create (the named knobs are not netns sysctls: node checklist only)")
    for m in PLAN_MODES:
        if m in skipped:
            out.append(f"- {m}: {skipped[m]}")
    for r in rows:
        if str(r.get("max_cause", "")).startswith("OPEN:"):
            out.append(f"- OPEN max: [{r['mode']}] {r['max_cause']}")
    out.append("")

    text = "\n".join(out)
    if args.output == "-":
        sys.stdout.write(text)
    else:
        with open(args.output, "w", encoding="utf-8") as f:
            f.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
