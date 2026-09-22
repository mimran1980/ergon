"""Disabled temporary tables must not evaluate expensive Python expressions."""

from __future__ import annotations

from market_recorder.actor import RecordingActor


def test_debug_table_starts_disabled() -> None:
    actor = RecordingActor("binance", "guard")
    assert actor.debug.enabled() is False


def test_expensive_expression_not_run_while_disabled() -> None:
    actor = RecordingActor("binance", "guard")
    ran = {"n": 0}

    def expensive() -> int:
        ran["n"] += 1
        raise AssertionError("disabled diagnostic must not evaluate")

    if actor.debug.enabled():
        expensive()
        actor.debug.record_debug(actor.writer, 1, 1, expensive(), 0, 1)
    assert ran["n"] == 0


def test_enable_via_config_then_disable() -> None:
    actor = RecordingActor("binance", "binance-0")
    yaml_on = b"""
api_version: recording/v1
defaults:
  temporary_enabled: false
rules:
  - process: market-recorder
    instance: "*"
    table: book_debug
    enabled: true
"""
    assert actor.apply_config_bytes(yaml_on) is True
    assert actor.debug.enabled() is True
    yaml_off = yaml_on.replace(b"enabled: true", b"enabled: false")
    assert actor.apply_config_bytes(yaml_off) is False
    assert actor.debug.enabled() is False
