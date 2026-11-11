#!/usr/bin/env python3
"""Per-arm n, mean and batch-mean percentiles for a cluster Criterion directory,
and, with the gate's output, every pair's verdict.

Usage: bench-cluster-report.py <criterion-dir> [label] [--gate <gate-output>]

Invocations are repeats/<k> and repeats-reversed/<k>, else the single run in
<group>/<arm>/new.

- mean: the average of every invocation's Criterion mean (estimates.json).
- min, p50, p99, p99.99, max: percentiles of the per-sample times (sample.json:
  times[i] / iters[i], one batch of 10,000 operations divided down to one
  operation), pooled over every invocation that kept sample.json. A sample is
  a batch mean, so these are batch-mean percentiles, not per-message tails:
  the max is the slowest batch's per-operation mean, not one operation's
  latency. Give each max a cause (host stall, preemption, or unexplained).
- kept: how many invocations kept sample.json. An invocation that kept
  estimates but no samples is named below the table; with none kept, the
  percentiles come from the final invocation's <group>/<arm>/new alone, and
  that is said too.

--gate reads check-bench-gate.sh's output and sorts every pair into PASS,
INCONCLUSIVE or FAIL. The gate passes an inconclusive pair (over its ceiling,
inside the measured A/A band): that establishes no winner, so it is not
parity, and it is listed apart from the passes. A gate output with no graded
pair, no final PASS/FAIL line, or a FAIL count the parsed verdicts do not match
gets a warning under the counts.
"""
import json
import math
import os
import re
import sys

BATCH = 1e4  # BATCH_SIZE in cluster_codec_bench: one iteration is 10,000 operations
VERDICTS = ("PASS", "INCONCLUSIVE", "FAIL")

# One pair's graded line, in either of the gate's two forms:
#   "  <label> (ergo-sbe/sbe-tool)   0.9900–1.0046 (max 1.00)  ok"
#   "  <label> (ergo-sbe/sbe-tool)   99.0 / 100.0 = 0.9900 (max 1.00)  ok"
PAIR = re.compile(
    r"^  (\S+) \(ergo-sbe/sbe-tool\)\s+(.+?) \(max ([\d.]+)\)"
    r"  (ok|above ceiling|unproven — blocking|FAIL)$"
)
# The line under it: "      <ratio CI or 'not measured'>  <resolution>  <band>[  over N repeats ...]"
CI = re.compile(r"ratio CI \[([\d.]+), ([\d.]+)\]")
REPEATS = re.compile(r"over (\d+) repeats")
# The gate's resolution strings, most specific first, and what the report says.
RESOLUTIONS = (
    ("resolved breach", "resolved breach"),
    ("resolved", "resolved"),
    ("the envelope straddles it", "envelope straddles the ceiling"),
    ("inconclusive", "no winner: inside the measured A/A band"),
    ("unproven", "unproven: no measured A/A band reaches it"),
    ("uncertainty not measured", "uncertainty not measured"),
)
# Failures that are not a graded pair: missing estimates, provenance, repeats,
# arm order, A/A control, a stale arm, malformed evidence.
FAIL_PAREN = re.compile(r"^  FAIL (.+?) \((.*)\)\.?$")
FAIL_DASH = re.compile(r"^  FAIL (.+?) — (.*)$")
FAIL_COLON = re.compile(r"^  (\S+)(?: \(ergo-sbe/sbe-tool\))?: FAIL — (.*)$")
# The gate's last word, at column 0: "FAIL: <n> maintained ratio(s) ..." or "PASS: ...".
SUMMARY = re.compile(r"^(PASS|FAIL): (?:(\d+) )?", re.M)


def per_op(path):
    with open(path, encoding="utf-8") as handle:
        data = json.load(handle)
    return [t / i for t, i in zip(data["times"], data["iters"]) if i]


def pct(values, q):
    ordered = sorted(values)
    rank = max(0, min(len(ordered) - 1, math.ceil(q * len(ordered)) - 1))
    return ordered[rank]


def trees(root):
    out = []
    for tree in ("repeats", "repeats-reversed"):
        base = os.path.join(root, tree)
        if os.path.isdir(base):
            for k in sorted((n for n in os.listdir(base) if n.isdigit()), key=int):
                out.append((f"{tree}/{k}", os.path.join(base, k)))
    return out


def point(path, key):
    with open(path, encoding="utf-8") as handle:
        est = json.load(handle)
    return (est.get(key) or {}).get("point_estimate")


def groups(root):
    """{group: [arm, ...]} for every arm with a final-invocation estimate."""
    found = {}
    for entry in sorted(os.listdir(root)):
        gdir = os.path.join(root, entry)
        if entry.startswith(("repeats", "reversed")) or not os.path.isdir(gdir):
            continue
        for arm in sorted(os.listdir(gdir)):
            if os.path.isfile(os.path.join(gdir, arm, "new", "estimates.json")):
                found.setdefault(entry, []).append(arm)
    return found


def arm_stats(root, group, arm, per=BATCH):
    """One arm's figures, per operation (`per` operations to an iteration).

    Returns a dict: inv, kept, n, mean, min, p50, p99, p9999, max (None when
    there is nothing to compute them from), and flag (a note, or None)."""
    pooled = trees(root)
    means, samples, inv, kept, missing = [], [], 0, 0, []
    for name, k in pooled:
        est = os.path.join(k, group, arm, "estimates.json")
        if not os.path.isfile(est):
            continue
        inv += 1
        mean = point(est, "mean")
        if mean:
            means.append(mean)
        smp = os.path.join(k, group, arm, "sample.json")
        if os.path.isfile(smp):
            kept += 1
            samples += per_op(smp)
        else:
            missing.append(name)
    new = os.path.join(root, group, arm, "new")
    if not pooled:
        inv = 1
        means = [point(os.path.join(new, "estimates.json"), "mean")]
    flag = None
    if missing and kept:
        flag = f"no sample.json in {', '.join(missing)}"
    if not samples:
        if os.path.isfile(os.path.join(new, "sample.json")):
            samples = per_op(os.path.join(new, "sample.json"))
            if pooled:
                flag = "no invocation kept sample.json; percentiles from the final invocation (new/) only"
            else:
                kept = 1
        else:
            flag = "no sample.json kept; no percentiles"
    means = [m for m in means if m]
    return {
        "inv": inv,
        "kept": kept,
        "n": len(samples),
        "mean": sum(means) / len(means) / per if means else None,
        "min": min(samples) / per if samples else None,
        "p50": pct(samples, 0.50) / per if samples else None,
        "p99": pct(samples, 0.99) / per if samples else None,
        "p9999": pct(samples, 0.9999) / per if samples else None,
        "max": max(samples) / per if samples else None,
        "flag": flag,
    }


def parse_gate(text):
    """Every graded item in check-bench-gate.sh output, in output order.

    Each is a dict: verdict (PASS, INCONCLUSIVE or FAIL), name, ratio (the
    ratio or the range over repeats, or None), ceiling, ci ((lo, hi) or None),
    repeats, note. The gate's own verdict decides: ok is PASS, above ceiling
    (inside the measured A/A band) is INCONCLUSIVE, anything else is FAIL."""
    items, lines = [], text.splitlines()
    for index, line in enumerate(lines):
        m = PAIR.match(line)
        if m:
            label, ratio, ceiling, verdict = m.groups()
            if "=" in ratio:
                ratio = ratio.rsplit("=", 1)[1].strip()
            detail = lines[index + 1] if index + 1 < len(lines) else ""
            ci = CI.search(detail)
            reps = REPEATS.search(detail)
            note = next((said for key, said in RESOLUTIONS if key in detail), detail.strip())
            items.append({
                "verdict": {"ok": "PASS", "above ceiling": "INCONCLUSIVE"}.get(verdict, "FAIL"),
                "name": label,
                "ratio": ratio,
                "ceiling": ceiling,
                "ci": (ci.group(1), ci.group(2)) if ci else None,
                "repeats": int(reps.group(1)) if reps else 1,
                "note": note,
            })
            continue
        m = FAIL_COLON.match(line) or FAIL_PAREN.match(line) or FAIL_DASH.match(line)
        if m:
            items.append({
                "verdict": "FAIL", "name": m.group(1), "ratio": None, "ceiling": None,
                "ci": None, "repeats": None, "note": m.group(2),
            })
    return items


def gate_problem(text, items):
    """Why the verdicts parsed from gate output cannot be trusted, or None."""
    if not items:
        return "no graded pair in the gate output (empty, crashed, or its format changed)"
    summary = SUMMARY.findall(text)
    if not summary:
        return "no final PASS or FAIL line in the gate output (the gate did not finish)"
    word, count = summary[-1]
    said = int(count) if word == "FAIL" and count else 0
    parsed = sum(item["verdict"] == "FAIL" for item in items)
    if said != parsed:
        return f"the gate counted {said} failure(s) but {parsed} were parsed"
    return None


def cell(value):
    return "n/a" if value is None else f"{value:.3f}"


def print_verdicts(path):
    with open(path, encoding="utf-8") as handle:
        text = handle.read()
    items = parse_gate(text)
    counts = ", ".join(f"{sum(i['verdict'] == v for i in items)} {v}" for v in VERDICTS)
    print("")
    print(f"gate verdicts: {counts}  ({path})")
    problem = gate_problem(text, items)
    if problem:
        print(f"  warning: {problem}; these counts are not the gate's verdict")
    print("  note: INCONCLUSIVE passes the gate but is not parity: no winner is established.")
    for verdict in VERDICTS:
        for item in items:
            if item["verdict"] == verdict:
                print(f"{verdict:<13}{item['name']:<46}{item['ratio'] or '':<16}{item['note']}")


def main(argv):
    gate = None
    if "--gate" in argv:
        at = argv.index("--gate")
        if at + 1 >= len(argv):
            sys.exit("usage: bench-cluster-report.py <criterion-dir> [label] [--gate <gate-output>]")
        gate = argv[at + 1]
        argv = argv[:at] + argv[at + 2:]
    if not argv:
        sys.exit("usage: bench-cluster-report.py <criterion-dir> [label] [--gate <gate-output>]")
    root = argv[0]
    print(argv[1] if len(argv) > 1 else root)
    print(
        f"{'group / arm':<62}{'inv':>4}{'kept':>6}{'n':>7}{'mean':>9}{'min':>9}{'p50':>9}"
        f"{'p99':>9}{'p99.99':>9}{'max':>9}"
    )
    print(
        "  ns per op; kept = invocations with samples; n = batches; min, p50, p99, p99.99 and max"
        " are batch-mean percentiles, not per-message tails (a sample is one 10,000-op batch's mean)"
    )
    flags = []
    for group, arms in groups(root).items():
        for arm in arms:
            s = arm_stats(root, group, arm)
            print(
                f"{group + ' / ' + arm:<62}{s['inv']:>4}{str(s['kept']) + '/' + str(s['inv']):>6}"
                f"{s['n']:>7}{cell(s['mean']):>9}{cell(s['min']):>9}{cell(s['p50']):>9}"
                f"{cell(s['p99']):>9}{cell(s['p9999']):>9}{cell(s['max']):>9}"
            )
            if s["flag"]:
                flags.append(f"{group} / {arm}: {s['flag']}")
    for flag in flags:
        print(f"  missing samples: {flag}")
    if gate:
        print_verdicts(gate)


if __name__ == "__main__":
    main(sys.argv[1:])
