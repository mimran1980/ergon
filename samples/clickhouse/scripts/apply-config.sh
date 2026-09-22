#!/usr/bin/env bash
# Apply a saved config edit once, without running the watcher.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 scripts/config-watch.py --once
