"""Live mode must refuse ambient exchange credentials."""

from __future__ import annotations

from market_recorder.credentials import exchange_credential_vars, require_public_only
from market_recorder.main import main


def test_live_rejects_api_key(monkeypatch) -> None:
    monkeypatch.setenv("BINANCE_API_KEY", "should-not-be-here")
    monkeypatch.setattr("sys.argv", ["market-recorder", "--exchange", "binance", "--mode", "live", "--instance", "x"])
    assert main() == 2


def test_live_rejects_secret(monkeypatch) -> None:
    monkeypatch.setenv("BYBIT_API_SECRET", "nope")
    monkeypatch.setattr("sys.argv", ["market-recorder", "--exchange", "bybit", "--mode", "live", "--instance", "x"])
    assert main() == 2


def test_detector_ignores_unrelated_api_keys(monkeypatch) -> None:
    monkeypatch.setenv("DEEPSEEK_API_KEY", "not-an-exchange")
    assert exchange_credential_vars() == []


def test_require_public_only_raises(monkeypatch) -> None:
    monkeypatch.setenv("BYBIT_TESTNET_API_KEY", "nope")
    try:
        require_public_only()
    except SystemExit as e:
        assert "BYBIT_TESTNET_API_KEY" in str(e)
    else:
        raise AssertionError("expected SystemExit")
