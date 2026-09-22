#!/usr/bin/env python3
"""Record both venues' fixtures and export envelopes for the Rust ingester."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "apps" / "market-recorder" / "src"))

from market_recorder.actor import RecordingActor  # noqa: E402
from market_recorder.fixtures import load_frames  # noqa: E402


def main() -> int:
    out = ROOT / "artifacts" / "fixture-export.bin"
    out.parent.mkdir(parents=True, exist_ok=True)
    # One session covering both venues so layouts are declared once.
    actor = RecordingActor("binance", "fixture-0")
    actor.on_start()
    for exchange in ("binance", "bybit"):
        actor.exchange = exchange
        actor.venue = exchange
        actor.provenance.connection_id = f"{exchange}-public"
        for raw in load_frames(exchange, ROOT / "fixtures"):
            actor.process_frame(raw)
    n = actor.session.export(actor.writer, str(out))
    print(f"exported {n} frames to {out}")
    print(f"published={actor.writer.published()} dropped={actor.writer.dropped()} invalid={actor.writer.invalid()}")
    return 0 if actor.writer.published() > 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
