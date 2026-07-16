#!/usr/bin/env bash
# Repository hygiene check: rejects tracked build/generated artifacts.
# Exit 1 if any tracked target/ files or generated SBE codecs exist.
set -euo pipefail

errors=0

# Check for tracked target/ directories
tracked_targets=$(git ls-files '**/target/**' | head -5)
if [ -n "$tracked_targets" ]; then
  echo "ERROR: tracked build artifacts under target/:" >&2
  echo "$tracked_targets" >&2
  errors=$((errors + 1))
fi

# Check for tracked generated SBE codecs under persist/src/gen
tracked_gen=$(git ls-files 'persist/src/gen/*.rs')
if [ -n "$tracked_gen" ]; then
  echo "ERROR: tracked generated SBE codecs:" >&2
  echo "$tracked_gen" >&2
  errors=$((errors + 1))
fi

if [ "$errors" -gt 0 ]; then
  echo "Hygiene check FAILED with $errors issue(s)." >&2
  exit 1
fi

echo "Hygiene check passed: no tracked build/generated artifacts."
