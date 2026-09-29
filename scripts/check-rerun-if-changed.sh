#!/usr/bin/env bash
# Every literal rerun-if-changed path in a build.rs must exist. Interpolated
# paths are skipped. Paths under a git submodule are exempt: aeron's jar
# directory is produced by Gradle and is not in the repo.
# Fixture mode: set RERUN_ROOT to a directory that is not a git checkout.
# The real mode uses git ls-files so untracked fixtures are invisible.
set -euo pipefail

if [[ -n ${RERUN_ROOT:-} ]]; then
    root=$RERUN_ROOT
    mode=walk
else
    root=$(cd "$(dirname "$0")/.." && pwd)
    mode=git
fi

python3 - "$root" "$mode" <<'PY'
import os, pathlib, re, subprocess, sys

root = pathlib.Path(sys.argv[1]).resolve()
mode = sys.argv[2]
subs = []
gitmodules = root / ".gitmodules"
if gitmodules.exists():
    for line in gitmodules.read_text(errors="replace").splitlines():
        stripped = line.strip()
        if stripped.startswith("path ="):
            subs.append(stripped.split("=", 1)[1].strip())

if mode == "walk":
    files = [
        path.relative_to(root).as_posix()
        for path in root.rglob("build.rs")
        if "target" not in path.parts and ".git" not in path.parts
    ]
else:
    files = subprocess.check_output(
        ["git", "ls-files", "*build.rs"], cwd=root, text=True
    ).splitlines()

literal = re.compile(r"rerun-if-changed=([A-Za-z0-9_./+-]+)")
failed = False
checked = 0
exempt = 0
for rel_file in files:
    text = (root / rel_file).read_text(errors="replace")
    for watched in literal.findall(text):
        resolved = ((root / rel_file).parent / watched)
        rel = os.path.relpath(resolved, root)
        if any(rel == sub or rel.startswith(sub + os.sep) for sub in subs):
            exempt += 1
            continue
        checked += 1
        if not resolved.exists():
            print(
                f"check-rerun-if-changed: FAIL — {rel_file} watches {watched}, "
                "which does not exist"
            )
            failed = True
if failed:
    sys.exit(1)
print(f"check-rerun-if-changed: PASS ({checked} paths, {exempt} submodule-exempt)")
PY
