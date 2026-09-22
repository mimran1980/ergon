"""Module entry point: `python3 -m market_recorder`.

The packaged console script (`market-recorder`) covers installed use; this
covers running from a source tree on `PYTHONPATH`, which is how the container
image runs the app so that `_fixtures_root()` keeps resolving.
"""

from __future__ import annotations

import sys

from .main import main

if __name__ == "__main__":
    sys.exit(main())
