#!/usr/bin/env bash
# update-historic-baseline.sh — snapshot current ergo historic benchmarks
# into sbe/benchmarks/ergo-historic-baseline.env.
set -euo pipefail

python3 << 'PYEOF'
import json, os

criterion_dir = os.environ.get("CRITERION_DIR", "target/criterion")
baseline_file = "sbe/benchmarks/ergo-historic-baseline.env"

baselines = {}
for root, _, files in os.walk(criterion_dir):
    if "new" in root and "estimates.json" in files:
        rel = os.path.relpath(root, criterion_dir)
        parts = rel.split("/")
        if len(parts) >= 3 and parts[-1] == "new":
            group_dir = parts[0]
            fn_name = parts[1]
            key = group_dir.replace("ergo_historic_", "ergo_historic/", 1)
            key = f"{key}/{fn_name}"
            with open(os.path.join(root, "estimates.json")) as fh:
                e = json.load(fh)
            baselines[key] = e.get("slope", e["median"])["point_estimate"]

with open(baseline_file, "w") as fh:
    fh.write("# Historic ergo benchmark baselines.\n")
    fh.write("# Gate: scripts/check-bench-historic.sh\n")
    for k, v in sorted(baselines.items()):
        fh.write(f"{k}={v}\n")
print(f"Wrote {len(baselines)} baselines to {baseline_file}")
PYEOF
