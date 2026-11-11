#!/usr/bin/env python3
"""Judge registered two-arm instruction-probe pairs.

Fails when a pair is missing an arm, or ergon Ir/op exceeds sbe-tool's
(times the pair's documented exception below, if it has one).
Prints both counts either way. Does not run Callgrind.
"""

from __future__ import annotations

import json
import sys
from collections import defaultdict
from fractions import Fraction
from pathlib import Path

# Documented exceptions: (pair, profile) -> (ergon Ir/op, sbe-tool Ir/op) as
# measured at the commit that recorded them. The ceiling is that exact ratio,
# not a rounded one, so one more ergon instruction per 10,000 operations fails.
# Every other pair stays at ergon <= sbe-tool.
#
# Why these three exceed sbe-tool: ergon's checked decode returns a typed error
# for a malformed frame, where sbe-tool's slice indexing panics after unchecked
# offset adds. Per group entry and var-data field ergon pays saturating_sub
# buffer-remaining checks, overflow-checked offset adds and the schema maxValue
# test on each var-data length, which sbe-tool never makes (CONTRIBUTING:
# checked entry points report malformed input). The probes are fair: both arms
# walk the same fields from one pre-validated buffer with black_box'd inputs,
# the group-with-data sbe-tool arm discards its parent stage as ergon's does,
# and both arms print the same checksum (this judge does not compare them).
# The wall-clock arms assert equal values before timing (fairness_policy_test).
# Wall-clock favours ergon: `just bench` ergon/sbe-tool 0.756-0.788 for the full
# message (staged and ordered) and 0.77-0.87 for group with data, LTO and
# no-LTO, two runs on an Apple M4 with check-bench-gate.sh passing both
# profiles; one no-LTO group-with-data envelope (0.80-1.48) was inconclusive.
#
# Recorded 2026-11-06 at c8d9b8646 on x86-64 (i5-8500T, rustc 1.99.0, Valgrind
# Callgrind), identical in both profiles: decode_full_message 326.0036 vs
# 280.0028 (ratio 1.1643), decode_full_message_ordered 313.0031 vs 280.0029
# (1.1179), extended_group_with_data 58.0032 vs 42.0027 (1.3809). The var-data
# maxValue test is about 2 Ir per field and is kept on purpose.
IR_EXCEPTIONS: dict[tuple[str, str], tuple[float, float]] = {
    ("decode_full_message", "no-lto"): (326.0036, 280.0028),
    ("decode_full_message", "lto"): (326.0036, 280.0028),
    ("decode_full_message_ordered", "no-lto"): (313.0031, 280.0029),
    ("decode_full_message_ordered", "lto"): (313.0031, 280.0029),
    ("extended_group_with_data", "no-lto"): (58.0032, 42.0027),
    ("extended_group_with_data", "lto"): (58.0032, 42.0027),
}


def within_ceiling(pair: str, profile: str, ergo_ir: float, tool_ir: float) -> bool:
    ceiling = IR_EXCEPTIONS.get((pair, profile))
    if ceiling is None:
        return ergo_ir <= tool_ir
    max_ergo, at_tool = (Fraction(v) for v in ceiling)
    return Fraction(ergo_ir) * at_tool <= max_ergo * Fraction(tool_ir)


def judge(root: Path, manifest: Path) -> int:
    registered: dict[str, dict[str, str]] = defaultdict(dict)
    for line in manifest.read_text().splitlines()[1:]:
        if not line.strip():
            continue
        symbol, arm, pair, _topic, _ops = line.split("\t")
        registered[pair][arm] = symbol

    measured: dict[tuple[str, str], dict[str, dict]] = defaultdict(dict)
    for summary in root.glob("*/*.summary.json"):
        rec = json.loads(summary.read_text())
        measured[(rec["pair"], rec["profile"])][rec["arm"]] = rec

    failed = False
    two_arm = {
        pair: arms
        for pair, arms in registered.items()
        if "ergon" in arms and "sbe-tool" in arms
    }
    if not two_arm:
        print("FAIL: manifest has no registered ergon/sbe-tool pairs", file=sys.stderr)
        return 1
    profiles = sorted({profile for _pair, profile in measured})
    if not profiles:
        print("FAIL: no probe summaries to pair", file=sys.stderr)
        return 1
    present = {pair for pair, _profile in measured if pair in two_arm}
    if not present:
        print("paired comparison ok: no two-arm pairs in this selection")
        return 0
    for profile in profiles:
        for pair in sorted(present):
            recs = measured.get((pair, profile), {})
            ergo = recs.get("ergon")
            tool = recs.get("sbe-tool")
            if ergo is None or tool is None:
                print(
                    f"FAIL {profile}/{pair}: missing arm "
                    f"(ergon={ergo is not None} sbe-tool={tool is not None})"
                )
                failed = True
                continue
            ergo_ir = ergo["instructions_per_operation"]
            tool_ir = tool["instructions_per_operation"]
            print(
                f"  {profile}/{pair}: ergon Ir/op={ergo_ir:.2f}  "
                f"sbe-tool Ir/op={tool_ir:.2f}"
            )
            exception = IR_EXCEPTIONS.get((pair, profile))
            if not within_ceiling(pair, profile, ergo_ir, tool_ir):
                beyond = (
                    f" beyond its documented exception {exception[0]}/{exception[1]}"
                    if exception
                    else ""
                )
                print(
                    f"FAIL {profile}/{pair}: ergon Ir/op {ergo_ir:.4f} "
                    f"exceeds sbe-tool {tool_ir:.4f}{beyond}"
                )
                failed = True
            elif exception:
                print(
                    f"  {profile}/{pair}: within documented exception "
                    f"{exception[0]}/{exception[1]}"
                )
        # Ordered keeps a tail cursor; random access re-walks from the message
        # start for every group and var-data field. Ordered must be cheaper.
        random = measured.get(("decode_full_message_random", profile), {}).get("ergon")
        ordered = measured.get(("decode_full_message_ordered", profile), {}).get("ergon")
        if random is not None and ordered is not None:
            random_ir = random["instructions_per_operation"]
            ordered_ir = ordered["instructions_per_operation"]
            print(
                f"  {profile}/decode_full_message_ordered vs random access: "
                f"ordered Ir/op={ordered_ir:.2f}  random Ir/op={random_ir:.2f}"
            )
            if ordered_ir >= random_ir:
                print(
                    f"FAIL {profile}/decode_full_message_ordered: ordered Ir/op "
                    f"{ordered_ir:.2f} is not strictly below random access {random_ir:.2f}"
                )
                failed = True
    if failed:
        return 1
    print(
        "paired comparison ok: ergon Ir/op does not exceed sbe-tool "
        "outside the documented exceptions"
    )
    return 0


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: compare-sbe-probe-pairs.py ROOT MANIFEST", file=sys.stderr)
        return 2
    return judge(Path(sys.argv[1]), Path(sys.argv[2]))


if __name__ == "__main__":
    sys.exit(main())
