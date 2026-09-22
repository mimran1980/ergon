"""Raw JSON capture provenance.

Live Nautilus 1.231.0 has no public raw-frame API on either venue, and we
do not patch or wrap private adapter handlers. Exact JSON bytes are
recorded only when this process already owns them (fixture replay).

Live recording uses Nautilus's public Actor callbacks (trades, quotes,
books, funding) and our generated SBE. Those are labelled
`origin = normalized_public_feed`. They are not original exchange frames.
"""

from __future__ import annotations

# capture_mode values stored on raw_exchange_messages.
CAPTURE_FIXTURE = 1
CAPTURE_OWNED_BYTES = 1  # alias: bytes this process produced or loaded


class CaptureProvenance:
    __slots__ = ("connection_id", "reconnect_generation", "receive_sequence")

    def __init__(self, connection_id: str) -> None:
        self.connection_id = connection_id
        self.reconnect_generation = 0
        self.receive_sequence = 0

    def next_frame(self) -> tuple[str, int, int]:
        self.receive_sequence += 1
        return (self.connection_id, self.reconnect_generation, self.receive_sequence)

    def reconnected(self) -> None:
        self.reconnect_generation += 1
