"""Entry point: fixture or live mode, one exchange per process."""

from __future__ import annotations

import argparse
import sys

from .node import run


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exchange", required=True, choices=["binance", "bybit"])
    parser.add_argument("--mode", required=True, choices=["fixture", "live"])
    parser.add_argument("--instance", required=True)
    args = parser.parse_args()
    if args.mode == "live":
        from .credentials import require_public_only, strip_exchange_credentials

        try:
            require_public_only()
        except SystemExit as e:
            print(str(e), file=sys.stderr)
            return 2
        strip_exchange_credentials()
    run(args.exchange, args.mode, args.instance)
    return 0


if __name__ == "__main__":
    sys.exit(main())
