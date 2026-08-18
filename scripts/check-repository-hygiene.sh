#!/usr/bin/env bash
# Repository hygiene check: rejects tracked build/generated artifacts and
# reconstructed files that the repo policy deleted (package-lock.json,
# historical ledgers, bors.toml, ci-monitor.sh, sbe/benches/).
set -euo pipefail

git_dir="${HYGIENE_GIT_DIR:-$(cd "$(dirname "$0")/.." && pwd)}"
errors=0

ls_files() {
  git -C "$git_dir" ls-files "$@"
}

# Check for tracked target/ directories
tracked_targets=$(ls_files '**/target/**' | head -5)
if [ -n "$tracked_targets" ]; then
  echo "ERROR: tracked build artifacts under target/:" >&2
  echo "$tracked_targets" >&2
  errors=$((errors + 1))
fi

# Explicitly deleted artifacts — do not re-create.
forbidden=(
  package-lock.json
  bors.toml
  ci-monitor.sh
  book/src/project/performance-release-ledger.md
)
for path in "${forbidden[@]}"; do
  if [ -n "$(ls_files -- "$path")" ]; then
    echo "ERROR: forbidden reconstructed artifact is tracked: $path" >&2
    errors=$((errors + 1))
  fi
done

tracked_sbe_benches=$(ls_files 'sbe/benches/**' | head -5)
if [ -n "$tracked_sbe_benches" ]; then
  echo "ERROR: forbidden sbe/benches/ artifacts are tracked:" >&2
  echo "$tracked_sbe_benches" >&2
  errors=$((errors + 1))
fi

if [ "$errors" -gt 0 ]; then
  echo "Hygiene check FAILED with $errors issue(s)." >&2
  exit 1
fi

echo "Hygiene check passed: no tracked build/generated or forbidden artifacts."
