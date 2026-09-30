#!/usr/bin/env bash
# Cluster's ergo-sbe edges must not enable default features. The default is
# miette's fancy renderer. Cargo 1.95 rejects default-features on an inherited
# dependency, so the flag lives on the workspace dependency both edges inherit.
# Leaving either edge on defaults or explicitly enabling default/fancy keeps
# the renderer through feature unification.
set -euo pipefail

root=${1:-$(cd "$(dirname "$0")/.." && pwd)}

python3 - "$root" <<'PY'
import pathlib, sys, tomllib

root = pathlib.Path(sys.argv[1])
cluster = tomllib.loads((root / "cluster" / "Cargo.toml").read_text())
workspace = tomllib.loads((root / "Cargo.toml").read_text())
inherited = (
    workspace.get("workspace", {})
    .get("dependencies", {})
    .get("ergo-sbe")
)

def enables_fancy(dep):
    if not isinstance(dep, dict):
        return True
    features = set(dep.get("features", []))
    if dep.get("workspace") is True:
        if not isinstance(inherited, dict):
            return True
        features.update(inherited.get("features", []))
        defaults = inherited.get("default-features", True) or dep.get("default-features", False)
    else:
        defaults = dep.get("default-features", True)
    return defaults or bool(features & {"default", "fancy"})

failed = False
for table in ("dependencies", "build-dependencies"):
    dep = (cluster.get(table) or {}).get("ergo-sbe")
    if dep is None or enables_fancy(dep):
        print(
            f"check-cluster-ergo-sbe-features: FAIL — [{table}] ergo-sbe "
            "keeps default features or enables fancy explicitly"
        )
        failed = True
if failed:
    sys.exit(1)
print("check-cluster-ergo-sbe-features: PASS")
PY
