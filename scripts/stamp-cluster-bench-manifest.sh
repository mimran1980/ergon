#!/usr/bin/env bash
# Stamp a cluster Criterion tree with run provenance for packaging.
set -euo pipefail
dir="${1:?criterion dir}"
profile="${2:?profile}"
run_id="${3:?run_id}"
commit="${4:?commit}"
rustc_v="${5:?rustc}"
target="${6:?target}"
# The machine the numbers were taken on. `target` alone does not tell one Apple
# Silicon generation from another, and figures from an M1 are not figures from
# an M4: a manifest without a host makes a later comparison guess.
host=""
if command -v sysctl >/dev/null 2>&1; then
    host=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || true)
fi
if [ -z "$host" ] && [ -r /proc/cpuinfo ]; then
    host=$(awk -F': ' '/^model name|^Hardware/ {print $2; exit}' /proc/cpuinfo)
fi
host="${host:-unknown}"
# Machine load around the measured run (`uptime` lines from the runner): a figure
# without it cannot be told from one taken on a busy box.
python3 - "$dir" "$run_id" "$profile" "$commit" "$rustc_v" "$target" "$host" \
    "${BENCH_UPTIME_START:-}" "${BENCH_UPTIME_END:-}" <<'PY'
import json, os, sys

directory, run_id, profile, commit, rustc, target, host, uptime_start, uptime_end = sys.argv[1:10]
stamped = 0
for root, dirs, files in os.walk(directory):
    if os.path.basename(root) == "new" and "estimates.json" in files:
        with open(os.path.join(root, "run-id.txt"), "w", encoding="utf-8") as handle:
            handle.write(run_id)
        stamped += 1
if stamped == 0:
    sys.exit(f"no Criterion estimates were produced under {directory}")
# Each independent invocation the runner took is snapshotted under
# `repeats/<k>/`; stamp those too, and declare how many there are. A repeat
# from another run would widen the gate's envelope with foreign evidence, and
# a manifest claiming more invocations than are on disk is refused by the gate.
def stamp_invocations(tree):
    """Stamp every invocation under `tree` and return how many carried estimates."""
    tree_dir = os.path.join(directory, tree)
    count = 0
    if not os.path.isdir(tree_dir):
        return count
    for name in sorted(
        (n for n in os.listdir(tree_dir) if os.path.isdir(os.path.join(tree_dir, n))),
        key=lambda n: (0, int(n)) if n.isdigit() else (1, n),
    ):
        found = 0
        for root, dirs, files in os.walk(os.path.join(tree_dir, name)):
            if "estimates.json" in files:
                with open(os.path.join(root, "run-id.txt"), "w", encoding="utf-8") as handle:
                    handle.write(run_id)
                found += 1
        if found:
            count += 1
    return count


repeats = stamp_invocations("repeats")
reversed_repeats = stamp_invocations("repeats-reversed")
# The arm order is recorded from what is on disk, never assumed: the gate
# refuses a cluster run whose manifest does not say `paired`.
arm_order = "paired" if repeats and repeats == reversed_repeats else "forward"
manifest = {
    "run_id": run_id,
    "profile": profile,
    "commit": commit,
    "rustc": rustc,
    "target": target,
    "host": host,
    "estimates": stamped,
    "repeats": repeats if repeats > 1 else 1,
    "arm_order": arm_order,
}
if uptime_start:
    manifest["uptime_start"] = uptime_start
if uptime_end:
    manifest["uptime_end"] = uptime_end
with open(os.path.join(directory, "run-manifest.json"), "w", encoding="utf-8") as handle:
    json.dump(manifest, handle, indent=2)
print(f"stamped cluster {profile} run_id={run_id} estimates={stamped} repeats={manifest['repeats']} arm_order={arm_order}")
PY
