"""Node wiring: data-only Nautilus node (live) or fixture replay.

NautilusTrader owns market adapters, subscriptions, reconnection, and book
maintenance. Our code owns recording only. In fixture mode the same actor
code is fed deterministic frames from fixtures/<exchange>/ so tests run
without external network.
"""

from __future__ import annotations

import signal
import threading
from pathlib import Path

from .actor import RecordingActor

PINNED_NAUTILUS = "1.231.0"
PINNED_PYTHON = "3.12"


def _fixtures_root() -> Path:
    here = Path(__file__).resolve()
    sample = here.parents[4] if here.parts[-1] else here
    # apps/market-recorder/src/market_recorder/node.py -> samples/clickhouse
    return Path(__file__).resolve().parents[4] / "fixtures"


def run(exchange: str, mode: str, instance: str) -> None:
    actor = RecordingActor(exchange=exchange, instance=instance)

    def ready() -> None:
        Path("/tmp/recorder-ready").touch()

    if mode == "fixture":
        fixtures_dir = Path("fixtures") / exchange
        if not fixtures_dir.exists():
            fixtures_dir = _fixtures_root() / exchange
        actor.on_start()
        for frame in sorted(fixtures_dir.glob("*.json")):
            actor.process_frame(frame.read_bytes())
        # The fixture burst is finite: close the recording so an ingester can
        # replay it (a bounded replay needs a stop position).
        actor.finish_recording()
        ready()
        _control_loop(actor)
        actor.on_stop()
        return

    _run_live(actor, exchange, instance, ready)


def _control_loop(actor: RecordingActor) -> None:
    stop = threading.Event()

    def handle(sig: int, _frame: object) -> None:
        stop.set()

    signal.signal(signal.SIGTERM, handle)
    signal.signal(signal.SIGINT, handle)
    while not stop.wait(1.0):
        actor.poll_config()


def _run_live(actor: RecordingActor, exchange: str, instance: str, ready) -> None:
    import nautilus_trader

    if nautilus_trader.__version__ != PINNED_NAUTILUS:
        raise RuntimeError(
            f"nautilus_trader pin mismatch: have {nautilus_trader.__version__}, want {PINNED_NAUTILUS}"
        )

    from nautilus_trader.common.actor import Actor
    from nautilus_trader.common.config import ActorConfig
    from nautilus_trader.config import LoggingConfig, TradingNodeConfig
    from nautilus_trader.live.node import TradingNode
    from nautilus_trader.model.enums import BookType
    from nautilus_trader.model.identifiers import InstrumentId, Venue

    wanted = {"BTCUSDT", "ETHUSDT"}

    class BridgeActor(Actor):
        def __init__(self, rec: RecordingActor) -> None:
            super().__init__(ActorConfig(component_id=f"recorder-{rec.exchange}"))
            self._rec = rec

        def on_start(self) -> None:
            self._rec.on_start()
            venue = Venue("BINANCE" if self._rec.exchange == "binance" else "BYBIT")
            # Do NOT `request_instruments()` here. The Binance and Bybit
            # adapters load instruments through their InstrumentProvider at
            # node startup and do not implement `_request_instruments` at all
            # (grep the adapter: there is no such coroutine), so the call
            # reaches `LiveMarketDataClient`'s stub and raises
            # `NotImplementedError(implement the '_request_instruments'
            # coroutine)` — which is what the live pod died on. Subscribe to
            # what the provider has already loaded instead. An empty set is
            # reported rather than leaving a recorder that subscribes to
            # nothing and looks healthy.
            #
            # Subscribe here rather than from a clock timer, for two reasons
            # the in-cluster run made concrete. The provider loads its
            # instruments while the data engine connects, which the node does
            # *before* it starts the actors — `RECORDER-001.BinanceSpot
            # InstrumentProvider: Loaded 2 instruments` lands ~1ms ahead of
            # this callback, so the cache is already populated. And a timer is
            # the wrong instrument regardless: its callback runs on Nautilus'
            # timer thread, where touching the bridge panics with
            # "ergo_recorder::PyTable is unsendable, but sent to another
            # thread" — `on_start` runs on the actor's own thread, which is the
            # one that created the session.
            self._venue = venue
            instruments = [
                i
                for i in self.cache.instruments(venue=venue)
                if any(str(i.id.symbol).startswith(s) for s in wanted)
            ]
            if not instruments:
                self.log.error(
                    f"no {venue} instruments in the cache at start; "
                    f"the recorder will subscribe to nothing"
                )
                return
            for instrument in instruments:
                self._subscribe_instrument(instrument)

        def on_instrument(self, instrument) -> None:  # noqa: ANN001
            # Not the startup path — nothing subscribes the actor to instrument
            # events, so this does not fire for the initial load. It is kept for
            # the provider's periodic refresh (`update_instruments_interval_mins`
            # is 60), which re-issues the subscriptions on the actor's thread.
            self._subscribe_instrument(instrument)

        def _subscribe_instrument(self, instrument) -> None:  # noqa: ANN001
            symbol = str(instrument.id.symbol)
            if not any(symbol.startswith(s) for s in wanted):
                return
            iid: InstrumentId = instrument.id
            # Logged per instrument: "subscribed nothing, silently" is the
            # failure this lane exists to catch, and the subscribe calls below
            # are otherwise invisible outside the venue's own stream.
            self.log.info(f"subscribing {symbol} on {self._venue}")
            self.subscribe_trade_ticks(iid)
            self.subscribe_quote_ticks(iid)
            self.subscribe_order_book_deltas(iid, book_type=BookType.L2_MBP)
            # No `subscribe_order_book_at_interval`. Nautilus' interval
            # aggregator fires `on_order_book` from its *timer* thread, and the
            # bridge's `PyTable` is `unsendable` — so every snapshot panicked
            # with "PyTable is unsendable, but sent to another thread" (two per
            # second, one per instrument) instead of recording. The MBP deltas
            # above carry the same book and arrive on the actor's own thread;
            # `book_snapshots` is therefore fixture-only until a snapshot is
            # synthesised on that thread rather than subscribed to.
            try:
                self.subscribe_funding_rates(iid)
            except Exception:
                pass
            try:
                self.subscribe_mark_prices(iid)
            except Exception:
                pass
            try:
                self.subscribe_index_prices(iid)
            except Exception:
                pass

        def on_trade_tick(self, tick) -> None:  # noqa: ANN001
            self._rec.on_trade_tick(tick)

        def on_quote_tick(self, tick) -> None:  # noqa: ANN001
            self._rec.on_quote_tick(tick)

        def on_order_book(self, book) -> None:  # noqa: ANN001
            self._rec.on_order_book(book)

        def on_order_book_deltas(self, deltas) -> None:  # noqa: ANN001
            self._rec.on_order_book_deltas(deltas)

        def on_funding_rate(self, update) -> None:  # noqa: ANN001
            self._rec.on_funding_rate(update)

        def on_mark_price(self, update) -> None:  # noqa: ANN001
            self._rec.on_mark_price(update)

        def on_index_price(self, update) -> None:  # noqa: ANN001
            self._rec.on_index_price(update)

        def on_stop(self) -> None:
            self._rec.on_stop()

    if exchange == "binance":
        from nautilus_trader.adapters.binance import (
            BINANCE,
            BinanceDataClientConfig,
            BinanceInstrumentProviderConfig,
            BinanceLiveDataClientFactory,
        )
        from nautilus_trader.adapters.binance.common.enums import BinanceAccountType

        data_clients = {
            BINANCE: BinanceDataClientConfig(
                api_key=None,  # public JSON market data; this sample has no keys
                api_secret=None,
                account_type=BinanceAccountType.SPOT,
                use_agg_trade_ticks=True,
                # Without a loading directive the provider loads nothing and
                # warns "No loading configured", so the recorder subscribes to
                # an empty set. `load_ids` rather than `load_all`: Binance spot
                # lists thousands of instruments and this actor wants two.
                instrument_provider=BinanceInstrumentProviderConfig(
                    load_ids=frozenset(f"{sym}.BINANCE" for sym in wanted)
                ),
            ),
        }
        factory_name, factory = BINANCE, BinanceLiveDataClientFactory
    else:
        from nautilus_trader.adapters.bybit import BYBIT, BybitDataClientConfig, BybitLiveDataClientFactory
        from nautilus_trader.common.config import InstrumentProviderConfig
        from nautilus_trader.core.nautilus_pyo3 import BybitProductType

        data_clients = {
            BYBIT: BybitDataClientConfig(
                api_key=None,
                api_secret=None,
                product_types=(BybitProductType.SPOT, BybitProductType.LINEAR),
                # Bybit takes the generic provider config; same reason as above.
                # The symbol must carry its product type: Nautilus parses the
                # suffix during construction and panics with "symbol checked for
                # suffix" on a bare `BTCUSDT.BYBIT`, and the provider then loads
                # nothing ("No instruments were loaded, verify config"). The
                # `-SPOT` ids match the product type configured above.
                instrument_provider=InstrumentProviderConfig(
                    load_ids=frozenset(f"{sym}-SPOT.BYBIT" for sym in wanted)
                ),
            ),
        }
        factory_name, factory = BYBIT, BybitLiveDataClientFactory

    config = TradingNodeConfig(
        trader_id="RECORDER-001",
        logging=LoggingConfig(log_level="INFO"),
        data_clients=data_clients,
        exec_clients={},  # data-only; this sample never submits orders
    )
    node = TradingNode(config=config)
    node.add_data_client_factory(factory_name, factory)
    node.build()
    node.trader.add_actor(BridgeActor(actor))
    ready()
    node.run()
