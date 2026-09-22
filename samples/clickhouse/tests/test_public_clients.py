"""Task 0: prove the pinned Nautilus public clients.

Live recording uses public Actor callbacks only. Exact JSON is fixture-owned.
Live network probes fail with a named blocker rather than being skipped.
"""

from __future__ import annotations

import json
from pathlib import Path

import nautilus_trader
from nautilus_trader.adapters.binance.config import BinanceDataClientConfig
from nautilus_trader.adapters.bybit.config import BybitDataClientConfig
from nautilus_trader.common.actor import Actor

from market_recorder import raw_json
from market_recorder.capability import MATRIX, PINNED, write_matrix

SAMPLE = Path(__file__).resolve().parent.parent


def test_pinned_version() -> None:
    assert nautilus_trader.__version__ == PINNED["nautilus_trader"]


def test_actor_has_v1_market_handlers() -> None:
    for name in (
        "on_trade_tick",
        "on_quote_tick",
        "on_order_book_deltas",
        "on_order_book",
        "on_funding_rate",
        "on_mark_price",
        "on_index_price",
        "subscribe_trade_ticks",
        "subscribe_order_book_deltas",
    ):
        assert hasattr(Actor, name), name


def test_binance_public_config_documents_no_key() -> None:
    doc = BinanceDataClientConfig.__doc__ or ""
    assert "public market data" in doc
    assert "spot_market_data_mode" not in BinanceDataClientConfig.__annotations__
    fields = BinanceDataClientConfig.__annotations__
    assert "api_key" in fields
    assert "account_type" in fields


def test_no_nautilus_handler_wrap_or_patch() -> None:
    assert not hasattr(raw_json, "install_binance_tap")
    assert not hasattr(raw_json, "install_bybit_tap")
    src = Path(__file__).resolve().parents[1] / "apps" / "market-recorder" / "src" / "market_recorder"
    tree = (src / "node.py").read_text() + (src / "raw_json.py").read_text()
    assert "_handle_ws_message" not in tree
    assert "_handle_msg" not in tree
    assert "patches/nautilus" not in tree


def test_bybit_config_has_no_exec_requirement() -> None:
    assert "api_key" in BybitDataClientConfig.__annotations__
    assert "product_types" in BybitDataClientConfig.__annotations__


def test_no_exchange_keys_in_environment() -> None:
    from market_recorder.credentials import exchange_credential_vars

    assert exchange_credential_vars() == []


def test_write_capability_matrix(tmp_path: Path | None = None) -> None:
    path = SAMPLE / "artifacts" / "nautilus-capability-matrix.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    write_matrix(path)
    data = json.loads(path.read_text())
    assert data["pin"]["nautilus_trader"] == "1.231.0"
    families = {row["family"] for row in data["matrix"]}
    assert "trades" in families
    assert "raw_json_capture" in families
    required = [c for c in MATRIX if c.required]
    assert required
