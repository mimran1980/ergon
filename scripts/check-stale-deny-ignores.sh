#!/usr/bin/env bash
# An ignored cargo-deny advisory is stale when its crate is absent from every
# Cargo.lock. Explicit advisory mappings keep human-written reasons out of the
# decision; an unfamiliar advisory requires a mapping before it can pass.
set -euo pipefail

root=${1:-$(cd "$(dirname "$0")/.." && pwd)}

python3 - "$root" <<'PY'
import os, pathlib, sys, tomllib

root = pathlib.Path(sys.argv[1])
advisory_crates = {"RUSTSEC-2026-0235": "rkyv"}
with (root / "deny.toml").open("rb") as handle:
    ignores = tomllib.load(handle).get("advisories", {}).get("ignore", [])
packages = set()
for directory, dirs, files in os.walk(root):
    dirs[:] = [name for name in dirs if name not in {"target", ".git"}]
    if "Cargo.lock" in files:
        with (pathlib.Path(directory) / "Cargo.lock").open("rb") as handle:
            packages.update(p["name"] for p in tomllib.load(handle).get("package", []))
failed = False
for entry in ignores:
    advisory = entry.get("id") if isinstance(entry, dict) else entry
    crate = advisory_crates.get(advisory) if isinstance(advisory, str) else None
    if crate is None:
        print(f"check-stale-deny-ignores: FAIL — no crate mapping for {advisory!r}")
        failed = True
        continue
    if crate not in packages:
        print(
            f"check-stale-deny-ignores: FAIL — {advisory} ignores {crate}, "
            "which is in no Cargo.lock"
        )
        failed = True
if failed:
    sys.exit(1)
print(f"check-stale-deny-ignores: PASS ({len(ignores)} ignores)")
PY
