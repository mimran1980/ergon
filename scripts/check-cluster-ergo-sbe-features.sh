#!/usr/bin/env bash
# Cluster's ergo-sbe edges must not enable default features. The default is
# miette's fancy renderer. Cargo 1.95 rejects default-features on an inherited
# dependency, so the flag lives on the workspace dependency both edges inherit.
# Leaving either edge on its own defaults keeps fancy through unification.
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

def keeps_defaults(dep):
    if isinstance(dep, dict) and dep.get("default-features") is False:
        return False
    if isinstance(dep, dict) and dep.get("workspace") is True:
        return not (
            isinstance(inherited, dict) and inherited.get("default-features") is False
        )
    return True

failed = False
for table in ("dependencies", "build-dependencies"):
    dep = (cluster.get(table) or {}).get("ergo-sbe")
    if dep is None or keeps_defaults(dep):
        print(
            f"check-cluster-ergo-sbe-features: FAIL — [{table}] ergo-sbe "
            "keeps default features"
        )
        failed = True
if failed:
    sys.exit(1)
print("check-cluster-ergo-sbe-features: PASS")
PY
