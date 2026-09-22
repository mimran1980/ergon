"""Pinned Nautilus public-capability matrix for task 0."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import json
from pathlib import Path

PINNED = {
    "nautilus_trader": "1.231.0",
    "python": "3.12",
    "spot_market_data_mode": "not present on 1.231.0; JSON public is default when api_key=None",
    "live_recording_source": "public Actor callbacks only (on_trade_tick / on_quote_tick / on_order_book*)",
    "raw_json": "fixture-owned bytes only; no Nautilus patch, no private-handler wrap, no second stream",
}


@dataclass(frozen=True)
class Cell:
    family: str
    binance: str
    bybit: str
    required: bool


MATRIX = [
    Cell("instruments", "supported", "supported", True),
    Cell("trades", "supported (aggTrade JSON)", "supported", True),
    Cell("quotes", "supported (bookTicker)", "supported", True),
    Cell("l2_deltas", "supported", "supported", True),
    Cell("l2_book", "supported (Nautilus OrderBook)", "supported", True),
    Cell("bars", "supported", "supported", True),
    Cell("funding_rates", "supported on USDT_FUTURES", "supported (ticker)", True),
    Cell("mark_prices", "supported on USDT_FUTURES", "optional", False),
    Cell("index_prices", "supported on USDT_FUTURES", "optional", False),
    Cell("open_interest", "optional public REST", "optional", False),
    Cell("liquidations", "optional", "optional", False),
    Cell("raw_json_capture", "fixture-owned bytes only", "fixture-owned bytes only", False),
]


def write_matrix(path: Path) -> None:
    payload = {
        "pin": PINNED,
        "matrix": [asdict(c) for c in MATRIX],
    }
    path.write_text(json.dumps(payload, indent=2) + "\n")
