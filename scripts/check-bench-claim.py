#!/usr/bin/env python3
"""Decide whether a bench run may be reported as "ergon <= 1.00 of sbe-tool".

`check-bench-gate.sh` answers a different question: did the run stay inside
its ceilings (1.01 and 1.05 for two documented cluster exceptions) and is an
INCONCLUSIVE pair a non-failure. This validator reads the same artifacts and
refuses the claim unless, in both profiles (lto and no-lto) and every
maintained pair:

- the gate itself exits 0 for the run id (provenance, repeats, arm order);
- both arms have estimates in every invocation;
- no documented exception is in force for the pair;
- the ratio is <= 1.00 in every invocation and the ratio-CI envelope over all
  invocations is <= 1.00 (a straddling or INCONCLUSIVE pair is no proof);
- the equal-work A/A control(s) of the gate are measured with confidence
  intervals.

The ceilings and the pair table are read from the gate script, never copied.
It changes no ceiling and no gate exit policy.

Usage: check-bench-claim.py --suite sbe|cluster --run-id ID [--gate PATH] DIR DIR
where each DIR is one profile's criterion directory. Exit 0: claim holds;
1: refused (reasons on stdout); 2: usage.
"""
import argparse
import json
import math
import os
import re
import subprocess
import sys


class Refuse(Exception):
    """A reason the claim cannot be made."""


def positive(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and value > 0


def load(path):
    """(point, lo, hi) of one estimates.json; lo/hi are None without a CI."""
    try:
        with open(path, encoding="utf-8") as handle:
            estimates = json.load(handle)
        estimate = estimates.get("slope") or estimates.get("mean")
        point = estimate["point_estimate"]
        ci = estimate.get("confidence_interval") or {}
    except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
        raise Refuse(f"missing or unreadable estimate {path} ({error})") from None
    lo, hi = ci.get("lower_bound"), ci.get("upper_bound")
    if lo is None or hi is None:
        lo = hi = None
    if not positive(point) or (lo is not None and not (positive(lo) and positive(hi) and lo <= hi)):
        raise Refuse(f"estimate {path} is not a finite positive measurement")
    return point, lo, hi


def invocations(criterion, key):
    """One (point, lo, hi) per invocation; both arm orders combined by geometric mean."""
    repeats = os.path.join(criterion, "repeats")
    names = sorted(n for n in os.listdir(repeats) if os.path.isdir(os.path.join(repeats, n))) if os.path.isdir(repeats) else []
    if not names:
        return [load(os.path.join(criterion, key, "new", "estimates.json"))]
    rows = []
    reversed_root = os.path.join(criterion, "repeats-reversed")
    for name in names:
        row = load(os.path.join(repeats, name, key, "estimates.json"))
        if os.path.isdir(reversed_root):
            back = load(os.path.join(reversed_root, name, key, "estimates.json"))
            row = tuple(None if a is None or b is None else math.sqrt(a * b) for a, b in zip(row, back))
        rows.append(row)
    return rows


def ratios(criterion, ergo_key, ref_key):
    """(point ratios, CI envelope or None) of two arms across the invocations."""
    ergo, ref = invocations(criterion, ergo_key), invocations(criterion, ref_key)
    if len(ergo) != len(ref):
        raise Refuse(f"the arms have {len(ergo)} and {len(ref)} invocations")
    points = [e[0] / r[0] for e, r in zip(ergo, ref)]
    if any(e[1] is None or r[1] is None for e, r in zip(ergo, ref)):
        return points, None
    cis = [(e[1] / r[2], e[2] / r[1]) for e, r in zip(ergo, ref)]
    return points, (min(c[0] for c in cis), max(c[1] for c in cis))


def table(gate, name):
    block = re.search(rf"(?m)^\s*{name}=\((.*?)\n\s*\)", gate, re.S)
    if not block:
        raise Refuse(f"no {name} table in the gate script")
    lines = (line for line in block.group(1).splitlines() if not line.lstrip().startswith("#"))
    rows = [m.split("|") for line in lines for m in re.findall(r'"([^"]*\|[^"]*)"', line)]
    if not rows:
        raise Refuse(f"the {name} table in the gate script has no pairs")
    return rows


def control_band(criterion, controls):
    """Widest A/A envelope of the gate's controls; every one must be measured."""
    bands = []
    for group, arm_a, arm_b in controls:
        try:
            _, band = ratios(criterion, f"{group}/{arm_a}", f"{group}/{arm_b}")
        except Refuse as error:
            raise Refuse(f"equal-work control {group}: {error}") from None
        if band is None:
            raise Refuse(f"equal-work control {group} has no confidence interval")
        bands.append(band)
    if not bands:
        raise Refuse("the gate script names no equal-work control for this suite")
    return min(b[0] for b in bands), max(b[1] for b in bands)


def judge(label, points, ci, band):
    if max(points) > 1.0:
        inside = ci is not None and not (ci[1] < band[0] or ci[0] > band[1])
        return f"{label}: ratio up to {max(points):.4f} is over 1.00" + (" (INCONCLUSIVE: inside the A/A band)" if inside else "")
    if ci is None:
        return f"{label}: uncertainty not measured"
    if ci[1] > 1.0:
        return f"{label}: unresolved, the ratio CI envelope reaches {ci[1]:.4f}"
    return None


def refusals(criterion, suite, run_id, gate_path, gate):
    result = subprocess.run(["bash", gate_path, criterion, "0", suite, "--run-id", run_id], capture_output=True, text=True, check=False)
    if result.returncode != 0:
        return [f"the gate itself fails for run {run_id}"]
    with open(os.path.join(criterion, "run-manifest.json"), encoding="utf-8") as handle:
        profile = json.load(handle)["profile"]
    # The gate overrides a ceiling per profile in an `if [ "$profile" = ... ]; then` line.
    exceptions = {group: ceiling for p, group, ceiling in re.findall(
        r'\[ "\$profile" = "([^"]+)" \] && \[ "\$group" = "([^"]+)" \][^\n]*\n\s*ceiling="([0-9.]+)"', gate) if p == profile}
    if suite == "sbe":
        pairs = [(f"{label}", f"parity_{group.replace('/', '_')}/{e}", f"parity_{group.replace('/', '_')}/{r}", c)
                 for label, group, e, r, c in table(gate, "pairs")]
    else:
        pairs = [(group, f"{group}/{e}", f"{group}/{r}", exceptions.get(group, c))
                 for group, e, r, c in table(gate, "cluster_pairs")]
    controls = [(g, a, b) for g, a, b in re.findall(r'aa_band "([^"]+)" "([^"]+)" "([^"]+)"', gate)
                if g.startswith("parity_" if suite == "sbe" else "cluster_")]
    out = []
    try:
        band = control_band(criterion, controls)
    except Refuse as error:
        out.append(str(error))
        band = (1.0, 1.0)
    for label, ergo_key, ref_key, ceiling in pairs:
        if float(ceiling) > 1.0:
            out.append(f"{label}: a documented exception (ceiling {ceiling}) is in force, so the gate passing says nothing about 1.00")
            continue
        try:
            reason = judge(label, *ratios(criterion, ergo_key, ref_key), band)
        except Refuse as error:
            reason = f"{label}: {error}"
        if reason:
            out.append(reason)
    return [f"{profile}: {r}" for r in out]


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--suite", choices=("sbe", "cluster"), required=True)
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--gate", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "check-bench-gate.sh"))
    parser.add_argument("criterion", nargs="+")
    args = parser.parse_args()
    with open(args.gate, encoding="utf-8") as handle:
        gate = handle.read()
    problems = []
    profiles = []
    for criterion in args.criterion:
        try:
            with open(os.path.join(criterion, "run-manifest.json"), encoding="utf-8") as handle:
                profiles.append(json.load(handle).get("profile"))
        except (OSError, ValueError):
            profiles.append(None)
            problems.append(f"{criterion}: no readable run-manifest.json")
    if sorted(map(str, profiles)) != ["lto", "no-lto"]:
        problems.append(f"a claim needs exactly the lto and no-lto profiles, got {profiles}")
    if not problems:
        for criterion in args.criterion:
            try:
                problems += refusals(criterion, args.suite, args.run_id, args.gate, gate)
            except Refuse as error:
                problems.append(str(error))
    if problems:
        print("CLAIM REFUSED: no <= 1.00 claim for this run")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(f"CLAIM OK: every maintained {args.suite} pair is <= 1.00 in both profiles, with its A/A control measured (run {args.run_id})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
