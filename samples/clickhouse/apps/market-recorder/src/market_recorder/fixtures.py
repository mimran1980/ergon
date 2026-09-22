"""Parse public-format fixture frames into typed recording events.

Fixtures are labelled synthetic/public-format captures. They exercise the
same actor/recorder path as live Nautilus callbacks, without a network.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterator

from .convert import to_mantissa

VENUE_BINANCE = 1
VENUE_BYBIT = 2
PRODUCT_SPOT = 1
PRODUCT_PERP = 2


@dataclass(frozen=True)
class ParsedEvent:
    kind: str
    payload: dict[str, Any]


def load_frames(exchange: str, fixtures_root: Path) -> list[bytes]:
    directory = fixtures_root / exchange
    return [p.read_bytes() for p in sorted(directory.glob("*.json"))]


def iter_events(exchange: str, raw: bytes) -> Iterator[ParsedEvent]:
    body = json.loads(raw)
    items = body if isinstance(body, list) else [body]
    for item in items:
        parsed = parse_event(exchange, item)
        if parsed is not None:
            yield parsed


def parse_event(exchange: str, item: dict[str, Any]) -> ParsedEvent | None:
    if exchange == "binance":
        return _parse_binance(item)
    if exchange == "bybit":
        return _parse_bybit(item)
    return None


def _parse_binance(item: dict[str, Any]) -> ParsedEvent | None:
    kind = item.get("e")
    if kind == "aggTrade" or "a" in item and "p" in item and "q" in item and "s" in item and "bids" not in item:
        if "lastUpdateId" in item:
            return _binance_book(item)
        return ParsedEvent(
            "trade",
            {
                "symbol": item["s"],
                "price_raw": to_mantissa(item["p"]),
                "qty_raw": to_mantissa(item["q"]),
                "side": 2 if item.get("m") else 1,  # m=true -> buyer is maker -> sell aggressor
                "event_time_ns": int(item.get("T") or item.get("E") or 0) * 1_000_000,
                "trade_id": int(item.get("a") or item.get("t") or 0),
            },
        )
    if "lastUpdateId" in item and "bids" in item:
        return _binance_book(item)
    if item.get("e") == "bookTicker" or ("b" in item and "a" in item and "B" in item and "A" in item):
        return ParsedEvent(
            "quote",
            {
                "symbol": item["s"],
                "bid_price_raw": to_mantissa(item["b"]),
                "bid_qty_raw": to_mantissa(item["B"]),
                "ask_price_raw": to_mantissa(item["a"]),
                "ask_qty_raw": to_mantissa(item["A"]),
                "event_time_ns": int(item.get("E") or item.get("u") or 0) * 1_000_000,
            },
        )
    if item.get("e") == "markPriceUpdate" or "r" in item and "s" in item and "p" in item:
        return ParsedEvent(
            "funding",
            {
                "symbol": item["s"],
                "rate_raw": to_mantissa(item.get("r") or "0"),
                "mark_raw": to_mantissa(item["p"]) if "p" in item else None,
                "index_raw": to_mantissa(item["i"]) if "i" in item else None,
                "interval_sec": 8 * 3600,
                "event_time_ns": int(item.get("E") or item.get("T") or 0) * 1_000_000,
            },
        )
    return None


def _binance_book(item: dict[str, Any]) -> ParsedEvent:
    bids = [(to_mantissa(p), to_mantissa(q)) for p, q in item.get("bids", [])]
    asks = [(to_mantissa(p), to_mantissa(q)) for p, q in item.get("asks", [])]
    return ParsedEvent(
        "book",
        {
            "symbol": item.get("s") or "BTCUSDT",
            "generation": 1,
            "source_sequence": int(item.get("lastUpdateId") or 0),
            "flags": 1,  # snapshot
            "event_time_ns": int(item.get("E") or item.get("T") or 1_758_000_000_000) * 1_000_000,
            "bids": bids,
            "asks": asks,
            "validity": 1,  # live after snapshot
        },
    )


def _parse_bybit(item: dict[str, Any]) -> ParsedEvent | None:
    if "p" in item and "v" in item and "s" in item and "b" not in item:
        side = 1 if str(item.get("S", "Buy")).lower().startswith("b") else 2
        trade_id = item.get("i") or "0"
        numeric_id = int("".join(ch for ch in str(trade_id) if ch.isdigit()) or "0")
        return ParsedEvent(
            "trade",
            {
                "symbol": item["s"],
                "price_raw": to_mantissa(item["p"]),
                "qty_raw": to_mantissa(item["v"]),
                "side": side,
                "event_time_ns": int(item.get("T") or 0) * 1_000_000,
                "trade_id": numeric_id,
            },
        )
    if "b" in item and "a" in item and isinstance(item.get("b"), list):
        bids = [(to_mantissa(p), to_mantissa(q)) for p, q in item.get("b", [])]
        asks = [(to_mantissa(p), to_mantissa(q)) for p, q in item.get("a", [])]
        return ParsedEvent(
            "book",
            {
                "symbol": item.get("s") or "BTCUSDT",
                "generation": 1,
                "source_sequence": int(item.get("u") or item.get("seq") or 0),
                "flags": 2,  # end of batch
                "event_time_ns": int(item.get("T") or 1_758_000_000_000) * 1_000_000,
                "bids": bids,
                "asks": asks,
                "validity": 1,
            },
        )
    if "fundingRate" in item:
        return ParsedEvent(
            "funding",
            {
                "symbol": item["s"],
                "rate_raw": to_mantissa(item["fundingRate"]),
                "interval_sec": 8 * 3600,
                "event_time_ns": int(item.get("ts") or item.get("T") or 0) * 1_000_000,
            },
        )
    return None
