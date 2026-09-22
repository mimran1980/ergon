#!/usr/bin/env bash
# Clean build artifacts, containers, and caches; report disk headroom.
# Refuses to touch volumes (kind/PVC data) unless --all is given.
set -euo pipefail
cd "$(dirname "$0")/.."

MODE="${1:-safe}"
case "$MODE" in
    --all) MODE="full" ;;
    safe|full) ;;
    *) echo "usage: $0 [safe|full|--all]" >&2; exit 2 ;;
esac

report() { df -h / | awk 'NR==2 {print "[clean] disk: " $3 " used, " $4 " free (" $5 ")"}'; }

report
echo "[clean] mode: $MODE"

# Rust artifacts (safe + full)
cargo clean 2>/dev/null || true

if [[ "$MODE" == "full" ]]; then
    # Docker build cache (always reclaimable) — volumes are NEVER touched
    # here; use scripts/stop.sh --purge-data for project volumes.
    docker builder prune -af 2>/dev/null | tail -1 || true
    # Stale test containers from interrupted runs.
    docker rm -f $(docker ps -aq --filter "name=ergo-ch-test") 2>/dev/null || true
    docker rm -f $(docker ps -aq --filter "name=ergo-s3-test") 2>/dev/null || true
fi

# Temp files from test harnesses.
rm -rf /tmp/ergo-archive-test-* /tmp/ergo-archive-basic-* /tmp/ergo-lab 2>/dev/null || true

report
echo "[clean] done (volumes/PVCs preserved)"
