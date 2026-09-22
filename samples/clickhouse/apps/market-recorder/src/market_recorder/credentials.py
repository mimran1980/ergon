"""Public-only live mode: this sample has no exchange API keys.

Nautilus Bybit configs source BYBIT_API_KEY from the environment when
api_key is None. We therefore refuse to start live if any exchange
credential variable is present, then strip the denylist so a later
adapter cannot load a key that appeared after the check.
"""

from __future__ import annotations

import os
from collections.abc import Mapping

_PREFIXES = (
    "BINANCE_",
    "BINANCEUS_",
    "BYBIT_",
)
_NEEDLES = ("API_KEY", "APISECRET", "API_SECRET", "SECRET_KEY", "ED25519")


def exchange_credential_vars(environ: Mapping[str, str] | None = None) -> list[str]:
    env = os.environ if environ is None else environ
    found: list[str] = []
    for key in env:
        upper = key.upper()
        if not any(upper.startswith(p) for p in _PREFIXES):
            continue
        if any(n in upper for n in _NEEDLES):
            found.append(key)
    return sorted(found)


def require_public_only(environ: Mapping[str, str] | None = None) -> None:
    offenders = exchange_credential_vars(environ)
    if offenders:
        raise SystemExit(
            "live mode is public-only and must run without exchange credentials: "
            + ", ".join(offenders)
        )


def strip_exchange_credentials() -> None:
    for key in exchange_credential_vars():
        os.environ.pop(key, None)
