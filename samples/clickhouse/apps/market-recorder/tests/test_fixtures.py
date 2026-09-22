"""Fixture-frame parsing and actor recording without Nautilus or network."""

from __future__ import annotations

from pathlib import Path

from market_recorder.actor import RecordingActor
from market_recorder.convert import to_mantissa
from market_recorder.fixtures import iter_events, load_frames

ROOT = Path(__file__).resolve().parents[3]


def test_binance_trade_mantissa_is_exact() -> None:
    assert to_mantissa("114562.30") == 11_456_230_000_000
    assert to_mantissa("0.014") == 1_400_000


def test_binance_fixture_yields_trade_and_book() -> None:
    kinds = set()
    for raw in load_frames("binance", ROOT / "fixtures"):
        for event in iter_events("binance", raw):
            kinds.add(event.kind)
    assert "trade" in kinds
    assert "book" in kinds


def test_bybit_fixture_yields_trade_and_book() -> None:
    kinds = set()
    for raw in load_frames("bybit", ROOT / "fixtures"):
        for event in iter_events("bybit", raw):
            kinds.add(event.kind)
    assert "trade" in kinds
    assert "book" in kinds


def test_maintained_book_applies_deltas_and_snapshots() -> None:
    actor = RecordingActor("binance", "book-test")
    bids, asks = actor.maintain_book("BTCUSDT", [(100, 5), (99, 1)], [(101, 2)], replace=True)
    assert bids == [(100, 5), (99, 1)]
    assert asks == [(101, 2)]

    # A delta merges and a zero quantity deletes.
    bids, asks = actor.maintain_book("BTCUSDT", [(98, 7), (100, 0)], [(102, 3)], replace=False)
    assert bids == [(99, 1), (98, 7)]
    assert asks == [(101, 2), (102, 3)]

    # A snapshot replaces the whole book.
    bids, asks = actor.maintain_book("BTCUSDT", [(90, 1)], [], replace=True)
    assert bids == [(90, 1)]
    assert asks == []


def test_fixture_books_are_sorted_with_positive_quantities() -> None:
    actor = RecordingActor("bybit", "bybit-book-test")
    for raw in load_frames("bybit", ROOT / "fixtures"):
        for event in iter_events("bybit", raw):
            if event.kind != "book":
                continue
            bids, asks = actor.maintain_book(
                event.payload["symbol"],
                event.payload["bids"],
                event.payload["asks"],
                replace=event.payload["flags"] == 1,
            )
            assert bids == sorted(bids, reverse=True), "bids not sorted desc"
            assert asks == sorted(asks), "asks not sorted asc"
            assert all(q > 0 for _, q in bids + asks), "non-positive quantity"


def test_actor_records_both_venues_without_network() -> None:
    for exchange in ("binance", "bybit"):
        actor = RecordingActor(exchange, f"{exchange}-test")
        actor.on_start()
        for raw in load_frames(exchange, ROOT / "fixtures"):
            actor.process_frame(raw)
        assert actor.processed >= 1
        assert actor.writer.published() >= 1
        actor.on_stop()
