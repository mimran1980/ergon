"""Optional public-endpoint probe. Fails with a named blocker, never skips."""

from __future__ import annotations

import json
import urllib.error
import urllib.request

BINANCE_EXCHANGE_INFO = "https://api.binance.com/api/v3/exchangeInfo?symbol=BTCUSDT"
BYBIT_INSTRUMENTS = "https://api.bybit.com/v5/market/instruments-info?category=spot&symbol=BTCUSDT"


def _get(url: str) -> dict:
    req = urllib.request.Request(url, headers={"User-Agent": "ergo-clickhouse-sample/0.1"})
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return json.loads(resp.read().decode())
    except urllib.error.URLError as e:
        raise AssertionError(f"public endpoint unavailable: {url}: {e}") from e


def test_binance_public_exchange_info_no_auth() -> None:
    body = _get(BINANCE_EXCHANGE_INFO)
    symbols = body.get("symbols") or []
    assert symbols, f"unexpected binance payload keys={list(body)}"
    assert symbols[0]["symbol"] == "BTCUSDT"


def test_bybit_public_instruments_no_auth() -> None:
    body = _get(BYBIT_INSTRUMENTS)
    result = body.get("result") or {}
    lst = result.get("list") or []
    assert lst, f"unexpected bybit payload retCode={body.get('retCode')}"
    assert lst[0]["symbol"] == "BTCUSDT"
