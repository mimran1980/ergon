#!/usr/bin/env python3
"""Judge registered two-arm instruction-probe pairs.

Fails when a pair is missing an arm or ergon Ir/op exceeds sbe-tool.
Prints both counts either way. Does not run Callgrind.
"""

from __future__ import annotations

import json
import sys
from collections import defaultdict
from pathlib import Path


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
            if ergo_ir > tool_ir:
                print(
                    f"FAIL {profile}/{pair}: ergon Ir/op {ergo_ir:.2f} "
                    f"exceeds sbe-tool {tool_ir:.2f}"
                )
                failed = True
        iterator = measured.get(("decode_full_message", profile), {}).get("ergon")
        ordered = measured.get(("decode_full_message_ordered", profile), {}).get("ergon")
        if iterator is not None and ordered is not None:
            iterator_ir = iterator["instructions_per_operation"]
            ordered_ir = ordered["instructions_per_operation"]
            print(
                f"  {profile}/decode_full_message_ordered vs iterator: "
                f"ordered Ir/op={ordered_ir:.2f}  iterator Ir/op={iterator_ir:.2f}"
            )
            if ordered_ir >= iterator_ir:
                print(
                    f"FAIL {profile}/decode_full_message_ordered: ordered Ir/op "
                    f"{ordered_ir:.2f} is not strictly below iterator {iterator_ir:.2f}"
                )
                failed = True
        mutable = measured.get(
            ("decode_full_message_mutable_ordered", profile), {}
        ).get("ergon")
        if iterator is not None and mutable is not None:
            iterator_ir = iterator["instructions_per_operation"]
            mutable_ir = mutable["instructions_per_operation"]
            print(
                f"  {profile}/decode_full_message_mutable_ordered vs iterator: "
                f"mutable ordered Ir/op={mutable_ir:.2f}  iterator Ir/op={iterator_ir:.2f}"
            )
            if mutable_ir >= iterator_ir:
                print(
                    f"FAIL {profile}/decode_full_message_mutable_ordered: "
                    f"mutable ordered Ir/op {mutable_ir:.2f} is not strictly below "
                    f"iterator {iterator_ir:.2f}"
                )
                failed = True
    if failed:
        return 1
    print("paired comparison ok: ergon Ir/op does not exceed sbe-tool")
    return 0


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: compare-sbe-probe-pairs.py ROOT MANIFEST", file=sys.stderr)
        return 2
    return judge(Path(sys.argv[1]), Path(sys.argv[2]))


if __name__ == "__main__":
    sys.exit(main())
