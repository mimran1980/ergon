#!/bin/bash
# Start/stop a ClickHouse container for integration tests.
# Usage: ./run-clickhouse.sh [start|stop]
set -euo pipefail

CONTAINER_NAME="ergo-persist-test"
PORT="8123"

case "${1:-start}" in
  start)
    echo "Starting ClickHouse container..."
    docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
    docker run -d \
      --name "$CONTAINER_NAME" \
      -p "$PORT":8123 \
      -e CLICKHOUSE_USER=default \
      -e CLICKHOUSE_PASSWORD=test123 \
      clickhouse/clickhouse-server
    echo "Waiting for ClickHouse to be ready..."
    for i in $(seq 1 30); do
      if curl -s "http://localhost:$PORT/ping" > /dev/null 2>&1; then
        echo "ClickHouse ready after ${i}s"
        exit 0
      fi
      sleep 1
    done
    echo "Timed out waiting for ClickHouse"
    exit 1
    ;;
  stop)
    echo "Stopping ClickHouse container..."
    docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
    echo "Done"
    ;;
  *)
    echo "Usage: $0 [start|stop]"
    exit 1
    ;;
esac
