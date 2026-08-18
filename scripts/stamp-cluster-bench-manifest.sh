#!/usr/bin/env bash
# Stamp a cluster Criterion tree with run provenance for packaging.
set -euo pipefail
dir="${1:?criterion dir}"
profile="${2:?profile}"
run_id="${3:?run_id}"
commit="${4:?commit}"
rustc_v="${5:?rustc}"
target="${6:?target}"
python3 - "$dir" "$run_id" "$profile" "$commit" "$rustc_v" "$target" <<'PY'
import json, os, sys

directory, run_id, profile, commit, rustc, target = sys.argv[1:7]
stamped = 0
for root, dirs, files in os.walk(directory):
    if os.path.basename(root) == "new" and "estimates.json" in files:
        with open(os.path.join(root, "run-id.txt"), "w", encoding="utf-8") as handle:
            handle.write(run_id)
        stamped += 1
if stamped == 0:
    sys.exit(f"no Criterion estimates were produced under {directory}")
manifest = {
    "run_id": run_id,
    "profile": profile,
    "commit": commit,
    "rustc": rustc,
    "target": target,
    "estimates": stamped,
}
with open(os.path.join(directory, "run-manifest.json"), "w", encoding="utf-8") as handle:
    json.dump(manifest, handle, indent=2)
print(f"stamped cluster {profile} run_id={run_id} estimates={stamped}")
PY
