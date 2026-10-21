#!/usr/bin/env bash
# Every Aeron this repo builds or tests against must be the same release:
#   - the Aeron each rusteron crate in Cargo.lock bundles (its aeron/version.txt,
#     found through `cargo metadata`, so a [patch] or path source counts), and
#     in every other Cargo.lock in the tree, such as the samples' (gitignored,
#     resolved by `just build`),
#   - the `aeron` submodule: its gitlink in the index must be the commit
#     scripts/aeron-pin.txt records for its version; when aeron/ is checked out,
#     its HEAD and aeron/version.txt must match the pin as well, its tracked
#     files must have no local changes, and the version in the name of every
#     jar Gradle left in its build/libs (the harness's classpath) must match.
# The jars carry no byte hashes: the pinned commit built by another JDK 17
# update gives different bytes, so the pin and this check are the guarantee;
# jar contents are not read.
# The submodule need not be checked out. No network (cargo runs --offline
# --locked) and no Java.
# Fixture mode: set AERON_VERSION_ROOT to another git checkout.
set -euo pipefail

root=${AERON_VERSION_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}

python3 - "$root" <<'PY'
import json, os, pathlib, re, subprocess, sys, tomllib

root = pathlib.Path(sys.argv[1]).resolve()
NAME = "check-aeron-version"
# The rusteron crates that package Aeron's sources. Others (rusteron-code-gen)
# count only if they carry aeron/version.txt.
BUNDLING = {"rusteron-client", "rusteron-archive", "rusteron-media-driver"}
JARS = ("aeron-all", "aeron-cluster", "aeron-archive")
PIN = "scripts/aeron-pin.txt"

versions = []  # (version, source)
commits = []  # (commit, source)
problems = []
notes = []


def show(path):
    path = pathlib.Path(path)
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        home = pathlib.Path.home()
        try:
            return "~/" + path.relative_to(home).as_posix()
        except ValueError:
            return path.as_posix()


def run(*args, cwd=root):
    return subprocess.run(args, cwd=cwd, capture_output=True, text=True)


# Pin: one "<version> <commit>" line.
pin_version = pin_commit = None
try:
    lines = [
        line.strip()
        for line in (root / PIN).read_text().splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]
except FileNotFoundError:
    lines = []
    problems.append(f"{PIN} is missing")
if lines:
    match = re.fullmatch(r"(\S+)\s+([0-9a-f]{40})", lines[0])
    if len(lines) != 1 or match is None:
        problems.append(f'{PIN} must hold exactly one "<version> <40-hex commit>" line')
    else:
        pin_version, pin_commit = match.groups()
        versions.append((pin_version, f"{PIN} (aeron submodule pin)"))
        commits.append((pin_commit, f"{PIN} (aeron submodule pin)"))
elif (root / PIN).exists():
    problems.append(f"{PIN} holds no pin line")

# rusteron: every rusteron-* package in Cargo.lock and in every other
# Cargo.lock in the tree (the samples' are gitignored and resolved by
# `just build`), located by cargo metadata. A crate another lock shares with
# Cargo.lock (same name, version and registry source) is the same package, so
# cargo is asked only about a lock resolving a rusteron of its own; a sample
# lock merely stale after a version bump does not fail the check.
PRUNE = {"target", "aeron", "simple-binary-encoding", "node_modules"}
root_lock = root / "Cargo.lock"


def other_locks():
    for directory, dirs, files in os.walk(root):
        dirs[:] = sorted(d for d in dirs if d not in PRUNE and not d.startswith("."))
        lock = pathlib.Path(directory) / "Cargo.lock"
        if "Cargo.lock" in files and lock != root_lock:
            yield lock


def key(name, version, source, lock):
    # Two path crates may share a name and version, so a path source is
    # keyed by the lock that resolves it.
    return (name, version, source or f"path in {show(lock)}")


def rusteron_in(lock):
    try:
        packages = tomllib.loads(lock.read_text()).get("package", [])
    except (OSError, tomllib.TOMLDecodeError) as error:
        problems.append(f"cannot read {show(lock)}: {error}")
        return []
    return [
        key(p["name"], p["version"], p.get("source"), lock)
        for p in packages
        if p["name"].startswith("rusteron-")
    ]


located = {}  # crate key -> its Cargo.toml


def locate(lock, wanted):
    meta = run(
        "cargo", "metadata", "--format-version", "1", "--locked", "--offline",
        "--all-features", "--manifest-path", str(lock.parent / "Cargo.toml"),
    )
    if meta.returncode != 0:
        problems.append(
            f"cargo metadata --locked --offline failed for {show(lock)} (if a "
            "crate is not downloaded yet, run `cargo fetch --locked` there once):\n      "
            + meta.stderr.strip().replace("\n", "\n      ")
        )
        return
    found = {
        key(p["name"], p["version"], p["source"], lock): pathlib.Path(p["manifest_path"])
        for p in json.loads(meta.stdout)["packages"]
    }
    for crate in wanted:
        if crate in found:
            located[crate] = found[crate]
        else:
            problems.append(f"cargo metadata did not resolve {crate[0]} {crate[1]} from {show(lock)}")


locks = {}  # lock -> the rusteron crates it resolves
if root_lock.is_file():
    locks[root_lock] = rusteron_in(root_lock)
    if not locks[root_lock]:
        problems.append("Cargo.lock lists no rusteron crate")
else:
    problems.append("Cargo.lock is missing")
for lock in other_locks():
    if crates := rusteron_in(lock):
        locks[lock] = crates
users = {}  # crate key -> the locks resolving it
for lock, crates in locks.items():
    for crate in crates:
        users.setdefault(crate, []).append(show(lock))
    if wanted := [crate for crate in crates if crate not in located]:
        locate(lock, wanted)
for crate, lock_names in users.items():
    name, version, _ = crate
    if crate not in located:
        continue  # its lock's failure is already a problem
    stamp = located[crate].parent / "aeron" / "version.txt"
    if stamp.is_file():
        versions.append((stamp.read_text().strip(), f"{name} {version} ({', '.join(lock_names)}): {show(stamp)}"))
    elif name in BUNDLING:
        problems.append(f"{name} {version} has no {show(stamp)} to read its Aeron version from")
root_crates = locks.get(root_lock, [])
if (
    root_crates
    and all(crate in located for crate in root_crates)
    and not any((located[crate].parent / "aeron" / "version.txt").is_file() for crate in root_crates)
):
    problems.append("no rusteron crate in Cargo.lock bundles Aeron (aeron/version.txt)")

# Submodule gitlink, from the index: what HEAD has on a clean checkout and what
# a commit would record during a bump.
staged = run("git", "ls-files", "--stage", "--", "aeron")
match = re.match(r"160000 ([0-9a-f]{40}) 0\taeron$", staged.stdout.strip())
if staged.returncode != 0 or match is None:
    problems.append(f"no aeron gitlink in the git index of {root} ({staged.stderr.strip() or 'git ls-files found none'})")
else:
    commits.append((match.group(1), "aeron gitlink (git index)"))

# A checked-out submodule must sit on the pin. Only consult git inside aeron/
# when aeron/.git exists, or git would answer for the superproject.
aeron = root / "aeron"
if (aeron / ".git").exists():
    head = run("git", "rev-parse", "HEAD", cwd=aeron)
    if head.returncode != 0:
        problems.append(f"cannot read the HEAD of aeron/: {head.stderr.strip()}")
    else:
        commits.append((head.stdout.strip(), "aeron/ checkout HEAD"))
    stamp = aeron / "version.txt"
    if stamp.is_file():
        versions.append((stamp.read_text().strip(), "aeron/version.txt (checkout)"))
    else:
        problems.append("aeron/ is checked out but has no version.txt")
    # Local edits to tracked files leave HEAD and version.txt on the pin, and
    # Gradle names a jar built from them for the pin, so the tree must be
    # clean. Untracked and ignored files (Gradle's build/ output) do not count.
    # --no-optional-locks: a read-only check must not rewrite aeron's index.
    status = run(
        "git", "--no-optional-locks", "status", "--porcelain", "--untracked-files=no", cwd=aeron
    )
    if status.returncode != 0:
        problems.append(f"cannot read the status of aeron/: {status.stderr.strip()}")
    elif changed := status.stdout.splitlines():
        shown = changed[:5] + ([f"... and {len(changed) - 5} more"] if len(changed) > 5 else [])
        problems.append(
            "aeron/ has local changes to tracked files (git -C aeron status), so it, and any jar "
            "built from it, is not the pinned release:\n        "
            + "\n        ".join(shown)
            + "\n      From the repo root, `git submodule update --force aeron` restores the pinned "
            "tree (discarding them), then `just build-aeron-jars` rebuilds the jars."
        )
elif aeron.is_dir() and any(aeron.iterdir()):
    problems.append("aeron/ has files but no .git, so its commit cannot be checked")
else:
    notes.append("aeron/ not checked out: gitlink checked against the pin only")

# Built jars: the cluster harness puts every jar in these build/libs on its
# classpath and loads whichever aeron-all-*.jar it lists first, and Gradle
# leaves an earlier pin's jars in place, so each must be the pinned release.
for module in JARS + ("aeron-samples",):
    for jar in sorted((aeron / module / "build" / "libs").glob("*.jar")):
        built = re.fullmatch(re.escape(module) + r"-(.+?)(?:-(?:sources|javadoc|tests))?\.jar", jar.name)
        if built is None:
            problems.append(f"{show(jar)} is not named {module}-<version>.jar")
        else:
            versions.append((built.group(1), f"{show(jar)} (built)"))

distinct = sorted({v for v, _ in versions})
if len(distinct) > 1:
    problems.append("Aeron versions disagree: " + ", ".join(distinct))
if len({c for c, _ in commits}) > 1:
    problems.append("aeron commits disagree: " + ", ".join(sorted({c for c, _ in commits})))

if problems:
    print(f"{NAME}: FAIL")
    for version, source in versions:
        print(f"  version {version:<10} {source}")
    for commit, source in commits:
        print(f"  commit  {commit}  {source}")
    for note in notes:
        print(f"  note: {note}")
    for problem in problems:
        print(f"  - {problem}")
    sys.exit(1)
print(f"{NAME}: PASS (Aeron {distinct[0]} at {commits[0][0][:12]}; {len(versions)} versions, {len(commits)} commits agree)")
for version, source in versions:
    print(f"  version {version:<10} {source}")
for commit, source in commits:
    print(f"  commit  {commit}  {source}")
for note in notes:
    print(f"  note: {note}")
PY
