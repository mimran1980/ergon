#!/usr/bin/env python3
"""Judge a `scripts/bench-ab.sh` run: base, base-rebuilt (A/A) and branch.

Usage: bench-ab-judge.py <out-dir> [--floor PCT]

Reads <out-dir>/runs/<arm>-r<k>/<profile>/criterion/repeats*/<j>/<group>/<fn>/sample.json
for the arms base, base-rebuilt and branch. A sample is Criterion's
times[i] / iters[i]: the mean of one sample's iterations, in ns per Criterion
iteration (a batch of operations), so the percentiles are batch-mean
percentiles, not per-operation tails. `warmup*` arms are skipped, as the gate
skips them.

Per profile and scenario:
- within: spread of base's per-round p50s, (max - min) / mean, in %.
- A/A:    |p50(base-rebuilt) - p50(base)| / p50(base), pooled, in %: what
          rebuilding unchanged code moves this row by (layout, constant data).
- noise:  max(within, A/A).
- change: (p50(branch) - p50(base)) / p50(base), pooled, in %.

Verdict:
- UNSTABLE when A/A > 3 %: the row moves more than the 3 % cap on unchanged
  code, so this run cannot judge it either way (not a fail, and not a pass:
  the run exits 3 unless another row FAILs).
- FAIL when change > noise and change > floor, or change > 3 %.
- PASS otherwise.

`old` is the rule this replaces (change > within-build spread, or > 3 %), kept
so a reader can see which rows it would have failed.

Exit: 0 PASS, 1 any FAIL, 2 missing or malformed evidence, 3 no FAIL but some
row UNSTABLE (not a pass: settle those rows first).
"""
import glob
import json
import os
import statistics
import sys

ARMS = ("base", "base-rebuilt", "branch")
CAP = 3.0


def die(msg):
    print(f"bench-ab-judge: {msg}", file=sys.stderr)
    sys.exit(2)


def pct(xs, q):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(q * (len(xs) - 1) + 0.5))]


def load(out):
    # rows[(profile, scenario)][arm][round] -> per-sample times
    rows, seen = {}, {}
    for run in sorted(glob.glob(os.path.join(out, "runs", "*-r*"))):
        name = os.path.basename(run)
        arm, _, rnd = name.rpartition("-r")
        if arm not in ARMS or not rnd.isdigit():
            die(f"unexpected run directory {run}")
        keys = set()
        for s in glob.glob(os.path.join(run, "*", "criterion", "repeats*", "*", "*", "*", "sample.json")):
            parts = s.split(os.sep)
            if parts[-2].startswith("warmup"):
                continue  # a throwaway first arm the gate ignores too, not a scenario
            prof, scen = parts[-7], f"{parts[-3]}/{parts[-2]}"
            with open(s, encoding="utf-8") as f:
                d = json.load(f)
            vals = [t / i for t, i in zip(d.get("times", []), d.get("iters", [])) if i]
            if not vals:
                die(f"empty sample: {s}")
            rows.setdefault((prof, scen), {}).setdefault(arm, {}).setdefault(rnd, []).extend(vals)
            keys.add((prof, scen))
        if not keys:
            die(f"no sample.json under {run}")
        seen[name] = keys
    if not seen:
        die(f"no runs under {out}/runs")
    for arm in ARMS:
        if not any(n.rpartition("-r")[0] == arm for n in seen):
            die(f"arm {arm} has no run")
    rounds = {a: sorted(n for n in seen if n.rpartition("-r")[0] == a) for a in ARMS}
    if len({len(r) for r in rounds.values()}) != 1 or len(rounds["base"]) < 2:
        die(f"every arm needs the same number of rounds, at least 2: {rounds}")
    union = set().union(*seen.values())
    for name, keys in sorted(seen.items()):
        if keys != union:
            missing = sorted(f"{p} {s}" for p, s in union - keys)
            die(f"{name} lacks {len(missing)} row(s): {', '.join(missing[:5])}")
    return rows


def judge(rows, floor):
    lines, verdicts = [], {"PASS": 0, "FAIL": 0, "UNSTABLE": 0}
    head = "profile|scenario|arm|n|min|p50|p99|p99.99|max|within %|A/A %|noise %|change %|old|verdict"
    for (prof, scen), arms in sorted(rows.items()):
        pooled = {a: [x for v in arms[a].values() for x in v] for a in ARMS}
        p50 = {a: statistics.median(pooled[a]) for a in ARMS}
        rmed = [statistics.median(v) for v in arms["base"].values()]
        within = (max(rmed) - min(rmed)) / statistics.mean(rmed) * 100
        aa = abs(p50["base-rebuilt"] - p50["base"]) / p50["base"] * 100
        noise = max(within, aa)
        change = (p50["branch"] - p50["base"]) / p50["base"] * 100
        old = "FAIL" if change > within or change > CAP else "PASS"
        if aa > CAP:
            verdict = "UNSTABLE"
        elif (change > noise and change > floor) or change > CAP:
            verdict = "FAIL"
        else:
            verdict = "PASS"
        verdicts[verdict] += 1
        for a in ARMS:
            xs = pooled[a]
            line = (
                f"{prof}|{scen}|{a}|{len(xs)}|{min(xs):.2f}|{p50[a]:.2f}|"
                f"{pct(xs, .99):.2f}|{pct(xs, .9999):.2f}|{max(xs):.2f}"
            )
            if a == "branch":
                line += f"|{within:.2f}|{aa:.2f}|{noise:.2f}|{change:+.2f}|{old}|{verdict}"
            else:
                line += "||||||"
            lines.append(line)
    return head, lines, verdicts


def main(argv):
    args, floor = [], 1.0
    it = iter(argv)
    for a in it:
        if a == "--floor":
            try:
                floor = float(next(it))
            except (StopIteration, ValueError):
                die("--floor needs a number (percent)")
        elif a.startswith("-"):
            die(f"unknown option {a}")
        else:
            args.append(a)
    if len(args) != 1:
        die("usage: bench-ab-judge.py <out-dir> [--floor PCT]")
    head, lines, v = judge(load(args[0]), floor)
    print(
        "ns per Criterion iteration (batch mean); arms pooled over rounds and repeats; "
        "p50 is the median the verdict uses, p99/p99.99 nearest rank (p99.99 is the max when n <= 5000)"
    )
    print(head)
    print("\n".join(lines))
    print(f"\nrows: {v['PASS']} PASS, {v['FAIL']} FAIL, {v['UNSTABLE']} UNSTABLE (floor {floor}%, cap {CAP}%)")
    if v["FAIL"]:
        print(f"verdict: FAIL ({v['FAIL']} row(s) slower than noise and floor, or over {CAP}%)")
        return 1
    if v["UNSTABLE"]:
        print(
            f"verdict: UNSTABLE ({v['UNSTABLE']} row(s) whose A/A moved over {CAP}% are not judged; "
            "settle them with instruction counts or more rounds before calling this a pass)"
        )
        return 3
    print("verdict: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
