"""Recording actor: forwards typed events to the PyO3 recorder.

Python evaluates arguments eagerly, so the temporary-table pattern is an
explicit `enabled()` guard before constructing optional state. The Rust
side rechecks the gate for concurrent disable changes.
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any

import ergo_recorder as rec

from . import fixtures
from .convert import nautilus_raw_to_i64, to_mantissa
from .raw_json import CAPTURE_FIXTURE, CaptureProvenance

# Channel and stream the recorder pod publishes on and the Archive records.
# The ingester must be configured with the same stream id.
AERON_RECORDED_CHANNEL = os.environ.get("ERGO_RECORDED_CHANNEL", "aeron:ipc")
AERON_STREAM_ID = int(os.environ.get("ERGO_RECORDED_STREAM_ID", "42"))

# Mounted by the Deployment; the recorder reads it directly so a table's
# retention is known before the table is declared. Overridable so the local
# fixture run can point at the checked-in config and exercise the same path.
RECORDING_CONFIG = os.environ.get(
    "ERGO_RECORDING_CONFIG", "/etc/recording/recording.yaml"
)

ORIGIN_NORMALIZED = 1
VALIDITY_LIVE = 1
VALIDITY_SYNCING = 0


class RecordingActor:
    def __init__(self, exchange: str, instance: str) -> None:
        self.exchange = exchange
        self.instance = instance
        self.venue = exchange
        self.product = fixtures.PRODUCT_SPOT
        self.provenance = CaptureProvenance(f"{exchange}-public")

        # The pod publishes over pod-local Aeron IPC; the lab (no AERON_DIR)
        # keeps the in-memory ring. Recording must be started before the
        # publication has a subscriber, and only this process can do it — the
        # Archive records shared memory that no other pod can reach.
        self.aeron_dir = os.environ.get("AERON_DIR")
        self.recording_subscription: int | None = None
        if self.aeron_dir:
            self.recording_subscription = rec.PyRecorderSession.ensure_recording(
                AERON_RECORDED_CHANNEL, AERON_STREAM_ID, self.aeron_dir
            )
            self.session = rec.PyRecorderSession.connect(
                "market-recorder", instance, (AERON_RECORDED_CHANNEL, AERON_STREAM_ID)
            )
            # The Archive's recording subscription needs time to present an
            # image. An offer to a publication with no subscriber is rejected,
            # and the first thing this session sends is its declaration — so
            # without this wait the app dies at construction.
            if not self.session.wait_connected(30.0):
                raise RuntimeError(
                    "Aeron publication never connected to the Archive's recording"
                )
        else:
            self.session = rec.PyRecorderSession.connect("market-recorder", instance)
        self.session.metadata("process", "market-recorder")
        self.session.metadata("instance", instance)
        self.session.declare_session_start()

        self.trades = self.session.trade_table("trades", False)
        self.quotes = self.session.quote_table("quotes", False)
        self.raw_frames = self.session.raw_table("raw_exchange_messages", False)
        self.sbe = self.session.sbe_table("sbe_messages", False)
        self.l2 = self.session.l2_table("l2_books", False)
        self.deltas = self.session.l2_table("order_book_deltas", False)
        self.funding = self.session.funding_table("funding_rates", False)
        self.mark = self.session.price_table("mark_prices", False)
        self.index = self.session.price_table("index_prices", False)
        # PLAN §5: a table's declared policy has to carry the retention the
        # operator configured, because that declaration is what the ingester
        # persists and freezes into its layout binding. It must be published
        # *before* the table is declared — a policy that has already gone out
        # on the wire cannot be reached afterwards, which is why applying the
        # config later (in on_start) could never have set a retention.
        self.config_path = Path(RECORDING_CONFIG)
        if self.config_path.exists():
            self.session.apply_retention(self.config_path.read_bytes(), "book_debug")

        self.debug = self.session.debug_table("book_debug")
        self.writer = self.session.writer()
        self.instrument_ids: dict[str, int] = {}
        self.connection_ids: dict[str, int] = {}
        self.processed = 0
        self.generation = 1
        self.book_valid = VALIDITY_SYNCING
        self.books: dict[str, dict[str, dict[int, int]]] = {}

    def instrument_id(self, symbol: str) -> int:
        if symbol not in self.instrument_ids:
            self.instrument_ids[symbol] = self.session.intern_symbol(symbol)
        return self.instrument_ids[symbol]

    def connection_id(self) -> int:
        key = self.provenance.connection_id
        if key not in self.connection_ids:
            self.connection_ids[key] = self.session.intern_symbol(key)
        return self.connection_ids[key]

    def on_raw_frame(self, payload: bytes, *, capture_mode: int = CAPTURE_FIXTURE) -> None:
        conn, gen, seq = self.provenance.next_frame()
        _ = conn
        self.raw_frames.record_frame(
            self.writer,
            payload,
            capture_mode,
            self.connection_id(),
            gen,
            seq,
            self.venue,
            self.product,
            seq,
        )

    def process_frame(self, payload: bytes) -> None:
        self.on_raw_frame(payload, capture_mode=CAPTURE_FIXTURE)
        for event in fixtures.iter_events(self.exchange, payload):
            self.process_event(event.kind, event.payload)
        self.processed += 1

    def process_event(self, kind: str, event: dict[str, Any]) -> None:
        if kind == "trade":
            self.record_trade(
                event["symbol"],
                event["price_raw"],
                event["qty_raw"],
                event["side"],
                event["event_time_ns"],
                event["trade_id"],
            )
        elif kind == "quote":
            self.record_quote(
                event["symbol"],
                event["bid_price_raw"],
                event["bid_qty_raw"],
                event["ask_price_raw"],
                event["ask_qty_raw"],
                event["event_time_ns"],
            )
        elif kind == "book":
            self.record_book(
                event["symbol"],
                event["generation"],
                event["source_sequence"],
                event["flags"],
                event["event_time_ns"],
                event["bids"],
                event["asks"],
                event["validity"],
            )
        elif kind == "funding":
            self.record_funding(
                event["symbol"],
                event["rate_raw"],
                event.get("interval_sec", 8 * 3600),
                event["event_time_ns"],
            )
            if event.get("mark_raw") is not None:
                self.mark.record_price(
                    self.writer, self.instrument_id(event["symbol"]), event["mark_raw"], event["event_time_ns"]
                )
            if event.get("index_raw") is not None:
                self.index.record_price(
                    self.writer, self.instrument_id(event["symbol"]), event["index_raw"], event["event_time_ns"]
                )

    def record_trade(
        self,
        symbol: str,
        price_raw: int,
        qty_raw: int,
        side: int,
        event_time_ns: int,
        trade_id: int,
    ) -> None:
        trade = rec.PyTrade(
            self.venue,
            self.instrument_id(symbol),
            price_raw,
            qty_raw,
            side,
            event_time_ns,
            trade_id,
        )
        self.trades.record_trade(self.writer, trade)
        sbe = self.session.encode_trade_sbe(trade)
        self.sbe.record_sbe(self.writer, sbe, 88, 1, ORIGIN_NORMALIZED, event_time_ns)

    def record_quote(
        self,
        symbol: str,
        bid_px: int,
        bid_qty: int,
        ask_px: int,
        ask_qty: int,
        event_time_ns: int,
    ) -> None:
        quote = rec.PyQuote(self.venue, self.instrument_id(symbol), bid_px, bid_qty, ask_px, ask_qty, event_time_ns)
        self.quotes.record_quote(self.writer, quote)
        sbe = self.session.encode_quote_sbe(quote)
        self.sbe.record_sbe(self.writer, sbe, 88, 2, ORIGIN_NORMALIZED, event_time_ns)

    def maintain_book(
        self, symbol: str, bids: list[tuple[int, int]], asks: list[tuple[int, int]], *, replace: bool
    ) -> tuple[list[tuple[int, int]], list[tuple[int, int]]]:
        """Apply an update batch to the maintained book and return its top 20 sides.

        A zero quantity is a delete, per both venues' depth protocols. `replace`
        (#1 snapshot flag) clears the side first; otherwise the batch merges.
        """
        book = self.books.setdefault(symbol, {"bids": {}, "asks": {}})
        for side, levels in (("bids", bids), ("asks", asks)):
            if replace:
                book[side].clear()
            for price, qty in levels:
                if qty == 0:
                    book[side].pop(price, None)
                else:
                    book[side][price] = qty
        top_bids = sorted(book["bids"].items(), key=lambda lvl: -lvl[0])[:20]
        top_asks = sorted(book["asks"].items(), key=lambda lvl: lvl[0])[:20]
        return top_bids, top_asks

    def record_book(
        self,
        symbol: str,
        generation: int,
        source_sequence: int,
        flags: int,
        event_time_ns: int,
        bids: list[tuple[int, int]],
        asks: list[tuple[int, int]],
        validity: int,
    ) -> None:
        iid = self.instrument_id(symbol)
        top_bids, top_asks = self.maintain_book(symbol, bids, asks, replace=flags == 1)
        self.l2.record_l2(
            self.writer,
            self.venue,
            symbol,
            iid,
            self.product,
            generation,
            source_sequence,
            validity,
            event_time_ns,
            top_bids,
            top_asks,
        )
        self.deltas.record_l2(
            self.writer,
            self.venue,
            symbol,
            iid,
            self.product,
            generation,
            source_sequence,
            flags,
            event_time_ns,
            bids,
            asks,
        )
        sbe = self.session.encode_book_delta_sbe(
            iid, generation, source_sequence, flags, event_time_ns, bids, asks
        )
        self.sbe.record_sbe(self.writer, sbe, 88, 4, ORIGIN_NORMALIZED, event_time_ns)
        if self.debug.enabled():
            self.debug.record_debug(self.writer, iid, source_sequence, len(bids) + len(asks), validity, event_time_ns)

    def record_funding(self, symbol: str, rate_raw: int, interval_sec: int, event_time_ns: int) -> None:
        self.funding.record_funding(
            self.writer, self.venue, symbol, self.instrument_id(symbol), rate_raw, interval_sec, event_time_ns
        )

    # --- Nautilus typed callbacks (v1 Actor names) ---

    def on_trade_tick(self, tick: Any) -> None:
        side = 1 if int(tick.aggressor_side) == 1 else 2
        trade_id = int("".join(ch for ch in str(tick.trade_id) if ch.isdigit()) or "0")
        self.record_trade(
            str(tick.instrument_id.symbol),
            nautilus_raw_to_i64(int(tick.price.raw)),
            nautilus_raw_to_i64(int(tick.size.raw)),
            side,
            int(tick.ts_event),
            trade_id,
        )

    def on_quote_tick(self, tick: Any) -> None:
        self.record_quote(
            str(tick.instrument_id.symbol),
            nautilus_raw_to_i64(int(tick.bid_price.raw)),
            nautilus_raw_to_i64(int(tick.bid_size.raw)),
            nautilus_raw_to_i64(int(tick.ask_price.raw)),
            nautilus_raw_to_i64(int(tick.ask_size.raw)),
            int(tick.ts_event),
        )

    def on_order_book(self, book: Any) -> None:
        # `BookLevel` mixes its two accessors, and the live pod died on every
        # book update because of it:
        #   * `price` is a *property* returning a `Price` (1e-16 raw), so it
        #     converts with `nautilus_raw_to_i64` like every other live path;
        #   * `size` is a *method* returning a plain **float** — `lvl.size`
        #     yields a bound method (`AttributeError: ... has no attribute
        #     'raw'`) and `lvl.size().raw` yields a float with no `.raw`
        #     either. A float goes through `to_mantissa`, the same exact-decimal
        #     path the fixture uses, which also refuses a value that is not
        #     exact at 1e-8 rather than silently rounding it.
        bids = [
            (
                nautilus_raw_to_i64(int(lvl.price.raw)),
                to_mantissa(str(lvl.size())),
            )
            for lvl in book.bids()[:20]
        ]
        asks = [
            (
                nautilus_raw_to_i64(int(lvl.price.raw)),
                to_mantissa(str(lvl.size())),
            )
            for lvl in book.asks()[:20]
        ]
        self.record_book(
            str(book.instrument_id.symbol),
            self.generation,
            int(getattr(book, "sequence", 0) or 0),
            2,
            int(book.ts_event),
            bids,
            asks,
            VALIDITY_LIVE,
        )

    def on_order_book_deltas(self, deltas: Any) -> None:
        if getattr(deltas, "is_snapshot", False):
            self.generation += 1
            self.book_valid = VALIDITY_LIVE
        flags = 1 if getattr(deltas, "is_snapshot", False) else 2
        bids: list[tuple[int, int]] = []
        asks: list[tuple[int, int]] = []
        for d in deltas.deltas:
            order = d.order
            px = nautilus_raw_to_i64(int(order.price.raw))
            qty = 0 if d.is_delete or d.is_clear else nautilus_raw_to_i64(int(order.size.raw))
            # Nautilus BookAction: ADD/UPDATE/DELETE; side from order.side
            side = int(getattr(order, "side", 0) or 0)
            if side == 1:
                bids.append((px, qty))
            else:
                asks.append((px, qty))
        self.record_book(
            str(deltas.instrument_id.symbol),
            self.generation,
            int(deltas.sequence or 0),
            flags,
            int(deltas.ts_event),
            bids,
            asks,
            self.book_valid,
        )

    def on_funding_rate(self, update: Any) -> None:
        rate = update.rate
        if hasattr(rate, "raw"):
            rate_raw = nautilus_raw_to_i64(int(rate.raw))
        else:
            rate_raw = to_mantissa(str(rate))
        interval = int(getattr(update, "interval", 0) or 8 * 3600)
        self.record_funding(str(update.instrument_id.symbol), rate_raw, interval, int(update.ts_event))

    def on_mark_price(self, update: Any) -> None:
        self.mark.record_price(
            self.writer,
            self.instrument_id(str(update.instrument_id.symbol)),
            nautilus_raw_to_i64(int(update.value.raw)),
            int(update.ts_event),
        )

    def on_index_price(self, update: Any) -> None:
        self.index.record_price(
            self.writer,
            self.instrument_id(str(update.instrument_id.symbol)),
            nautilus_raw_to_i64(int(update.value.raw)),
            int(update.ts_event),
        )

    def apply_config_bytes(self, yaml_bytes: bytes) -> bool:
        return bool(self.session.apply_config(yaml_bytes, "book_debug", self.debug))

    def poll_config(self) -> None:
        if not self.config_path.exists():
            return
        yaml_bytes = self.config_path.read_bytes()
        self.apply_config_bytes(yaml_bytes)
        # PLAN §5: "Temporary policy changes are versioned." A retention edit
        # has to mint a new policy id *and* a new layout, because the ingester
        # binds a layout to storage on first sight and deliberately never
        # recomputes it — editing the policy in place would update the status
        # table while every row kept the old expiry. Re-declaring the table
        # does both: `apply_retention` has already recorded the new values, so
        # the new declaration carries them, and the new layout mints a binding
        # that uses them. Rows already written keep the expiry they were
        # written with, which is the point of versioning rather than mutating.
        if self.session.apply_retention(yaml_bytes, "book_debug"):
            self.debug = self.session.debug_table("book_debug")

    def finish_recording(self) -> None:
        """Close the Archive recording so it can be replayed.

        A bounded replay needs a stop position and the replay source refuses an
        active recording, so a producer that has finished its burst — the
        fixture profile is exactly that — must end the recording.
        """
        if self.aeron_dir and self.recording_subscription is not None:
            rec.PyRecorderSession.stop_recording(self.recording_subscription, self.aeron_dir)
            self.recording_subscription = None

    def on_start(self) -> None:
        self.poll_config()

    def on_stop(self) -> None:
        # Emit the session boundary the ingester and archive expect. Without
        # it a recording had no end marker, so a consumer could not tell a
        # finished session from a stalled one.
        self.session.declare_session_end(self.writer.sequence())
        self.finish_recording()
