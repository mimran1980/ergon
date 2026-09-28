#!/usr/bin/env bash
# An ignored cargo-deny advisory is stale when the first crate named in its
# reason is absent from every Cargo.lock. Putting the ignore back fails this.
set -euo pipefail

root=${1:-$(cd "$(dirname "$0")/.." && pwd)}

python3 - "$root" <<'PY'
import pathlib, re, sys

root = pathlib.Path(sys.argv[1])
deny = root / "deny.toml"
text = deny.read_text() if deny.exists() else ""
locks = [p.read_text(errors="replace") for p in root.rglob("Cargo.lock") if "target" not in p.parts]
blocks = re.findall(
    r'\{\s*id\s*=\s*"([^"]+)"\s*,\s*reason\s*=\s*"([^"]+)"',
    text,
    re.S,
)
stop = {
    "the", "and", "this", "that", "from", "with", "for", "not", "are", "was",
    "optional", "feature", "never", "enables", "verified", "absent", "every",
    "build", "graph", "both", "published", "crates",
}
failed = False
for advisory, reason in blocks:
    names = re.findall(r"\b([a-z][a-z0-9_]*(?:-[a-z0-9_]+)*)\b", reason)
    crate = next((name for name in names if name not in stop and len(name) > 2), None)
    if crate is None:
        print(f"check-stale-deny-ignores: FAIL — {advisory} reason names no crate")
        failed = True
        continue
    needle = f'name = "{crate}"'
    if not any(needle in lock for lock in locks):
        print(
            f"check-stale-deny-ignores: FAIL — {advisory} ignores {crate}, "
            "which is in no Cargo.lock"
        )
        failed = True
if failed:
    sys.exit(1)
print(f"check-stale-deny-ignores: PASS ({len(blocks)} ignores)")
PY
