"""Exact decimal conversion. Never uses float."""

from __future__ import annotations

from decimal import Decimal

SCALE = 8
QUANTUM = Decimal(10) ** -SCALE
NAUTILUS_FIXED = 16
NAUTILUS_DIV = 10 ** (NAUTILUS_FIXED - SCALE)
I64_MIN = -(1 << 63)
I64_MAX = (1 << 63) - 1


def to_mantissa(value: str | int | Decimal, scale: int = SCALE) -> int:
    """Parse an exact decimal string into a scaled integer mantissa."""
    d = value if isinstance(value, Decimal) else Decimal(str(value))
    q = Decimal(10) ** scale
    scaled = d * q
    if scaled != scaled.to_integral_value():
        raise ValueError(f"{value!r} is not exact at 1e-{scale}")
    n = int(scaled)
    if n < I64_MIN or n > I64_MAX:
        raise OverflowError(f"{value!r} overflows i64 at 1e-{scale}")
    return n


def nautilus_raw_to_i64(raw: int) -> int:
    """Convert Nautilus high-precision (1e-16) raw to our 1e-8 mantissa."""
    if raw % NAUTILUS_DIV:
        raise ValueError(f"nautilus raw {raw} is not exact at 1e-{SCALE}")
    n = raw // NAUTILUS_DIV
    if n < I64_MIN or n > I64_MAX:
        raise OverflowError(f"nautilus raw {raw} overflows i64 at 1e-{SCALE}")
    return n
