#!/usr/bin/env bash
# ci-monitor.sh — Check latest CI run status for ErgoSBE
# Usage: ./ci-monitor.sh [limit=N]

set -euo pipefail

LIMIT="${1:-3}"

echo "=== Latest CI runs ==="
gh run list --limit "$LIMIT" --json headBranch,conclusion,status,displayTitle,createdAt,number,workflowName \
  | jq -r '.[] | "\(.workflowName) #\(.number): \(.conclusion // .status) — \(.headBranch): \(.displayTitle | .[0:80])"'

echo ""

FAILED_RUNS=$(gh run list --limit "$LIMIT" --json conclusion,databaseId,displayTitle \
  | jq -r '.[] | select(.conclusion == "failure" or .conclusion == "cancelled")')

if [ -n "$FAILED_RUNS" ]; then
  echo "=== Failed jobs per run ==="
  echo "$FAILED_RUNS" | jq -c '.' | while IFS= read -r RUN; do
    RUN_ID=$(echo "$RUN" | jq -r '.databaseId')
    TITLE=$(echo "$RUN" | jq -r '.displayTitle | .[0:80]')
    echo "--- $TITLE (#$RUN_ID) ---"
    gh run view "$RUN_ID" --json jobs \
      | jq -r '.jobs[] | select(.conclusion == "failure" or .conclusion == "cancelled") | "  FAILED: \(.name)"'
    echo ""
  done
else
  echo "All recent runs passed."
fi
