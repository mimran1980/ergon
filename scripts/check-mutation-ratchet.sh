#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output="${1:-${repo_root}/mutants.out}"

# shellcheck disable=SC1091
source "${repo_root}/.github/mutation-baseline.env"

count_nonempty_lines() {
  local file="$1"
  if [[ ! -f "${file}" ]]; then
    echo 0
    return
  fi
  awk 'NF { count += 1 } END { print count + 0 }' "${file}"
}

missed="$(count_nonempty_lines "${output}/missed.txt")"
timed_out="$(count_nonempty_lines "${output}/timeout.txt")"

if (( missed > MAX_MISSED_MUTANTS )); then
  echo "mutation ratchet: ${missed} missed exceeds baseline ${MAX_MISSED_MUTANTS}" >&2
  exit 1
fi
if (( timed_out > MAX_TIMEOUT_MUTANTS )); then
  echo "mutation ratchet: ${timed_out} timeouts exceeds baseline ${MAX_TIMEOUT_MUTANTS}" >&2
  exit 1
fi

echo "mutation ratchet: missed ${missed} <= ${MAX_MISSED_MUTANTS}"
echo "mutation ratchet: timeouts ${timed_out} <= ${MAX_TIMEOUT_MUTANTS}"
