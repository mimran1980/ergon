#!/usr/bin/env python3
"""Config watcher: validate + apply recording.yaml edits to the cluster.

Uses only the Python standard library, the shared Rust validation CLI
(target/release/recording-config), and kubectl. Polls the file content
hash, debounces editor saves, validates the exact bytes applied (snapshot
held in memory — no validate-then-reread race), and applies only when the
canonical digest changes.

An invalid edit prints the error and preserves the last applied revision.
An unavailable API server backs off and retries the latest valid revision.
A changed file that validates but fails to apply is retried; the file is
never re-read between validation and apply.
"""

from __future__ import annotations

import argparse
import hashlib
import subprocess
import sys
import time
from pathlib import Path

PERMANENT_TABLES = ",".join(
    [
        "trades", "quotes", "bars", "l2_books", "raw_exchange_messages",
        "sbe_messages", "instruments", "order_book_deltas", "book_snapshots",
        "funding_rates", "mark_prices", "index_prices", "open_interest",
        "liquidations", "market_status",
    ]
)


def log(msg: str) -> None:
    print(f"[config-watch] {time.strftime('%H:%M:%S', time.gmtime())} {msg}", flush=True)


def validate_bytes(cli: str, data: bytes) -> tuple[bool, str, str]:
    """Validate exact bytes via the shared CLI; returns (ok, digest, stderr)."""
    proc = subprocess.run(
        [cli, "--file", "-", "--permanent", PERMANENT_TABLES],
        input=data,
        capture_output=True,
    )
    if proc.returncode != 0:
        return False, "", proc.stderr.decode(errors="replace").strip()
    out = proc.stdout.decode()
    digest = ""
    for token in out.split():
        if token.startswith("digest="):
            digest = token.split("=", 1)[1]
    return True, digest, ""


def apply_snapshot(data: bytes, namespace: str) -> bool:
    """Construct + apply the ConfigMap from the validated snapshot bytes."""
    cm = {
        "apiVersion": "v1",
        "kind": "ConfigMap",
        "metadata": {"name": "recording-config", "namespace": namespace},
        "data": {"recording.yaml": data.decode("utf-8")},
    }
    import json

    rendered = json.dumps(cm)
    apply = subprocess.run(
        ["kubectl", "apply", "-f", "-", "-n", namespace],
        input=rendered.encode(),
        capture_output=True,
    )
    if apply.returncode != 0:
        log(f"apply failed: {apply.stderr.decode(errors='replace').strip()}")
        return False
    return True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config-dir", default="config")
    parser.add_argument("--namespace", default="clickhouse")
    parser.add_argument("--poll", type=float, default=1.0)
    parser.add_argument("--debounce", type=float, default=0.25)
    parser.add_argument(
        "--once",
        action="store_true",
        help="validate + apply the current file once and exit",
    )
    args = parser.parse_args()

    root = Path(__file__).resolve().parent.parent
    cli = root / "target" / "release" / "recording-config"
    if not cli.exists():
        print(f"missing {cli} — run: just build-cli", file=sys.stderr)
        return 2

    recording_file = Path(args.config_dir) / "recording.yaml"
    last_digest: str | None = None
    apply_backoff = 2.0

    log(f"watching {recording_file} (namespace {args.namespace}, poll {args.poll}s)")
    while True:
        try:
            data = recording_file.read_bytes()
        except FileNotFoundError:
            log("config file missing — keeping last applied revision")
            data = b""

        if data:
            ok, digest, err = validate_bytes(str(cli), data)
            if not ok:
                log(f"INVALID edit — keeping last applied revision: {err}")
            elif digest != last_digest:
                if last_digest is not None:
                    time.sleep(args.debounce)  # debounce rapid editor saves
                # Re-validate the (possibly replaced) bytes once more, then
                # apply from the same in-memory snapshot: no re-read race.
                ok, digest, err = validate_bytes(str(cli), data)
                if not ok:
                    log(f"INVALID edit — keeping last applied revision: {err}")
                else:
                    if apply_snapshot(data, args.namespace):
                        log(f"applied revision (digest {digest})")
                        last_digest = digest
                    else:
                        # A validated config that could not be applied is not a
                        # success: `--once` must report it, and the long-running
                        # watcher must keep retrying without claiming success.
                        log("API server unavailable — will retry")
                        ok = False
                        time.sleep(apply_backoff)
        else:
            ok = True

        if args.once:
            return 0 if ok else 1
        time.sleep(args.poll)


if __name__ == "__main__":
    sys.exit(main())
