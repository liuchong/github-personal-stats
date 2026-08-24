#!/usr/bin/env python3
"""Render the composable tile samples used by the docs.

Every sample is a whole card at a tile width, fitted to its content, so the set
can be pasted into one README row on a desktop and stacks on a phone without
either being scaled down.

    python3 scripts/render-tile-samples.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target" / "debug" / "github-personal-stats"
SHOWCASE = ROOT / "examples" / "showcase.json"
OUTPUT = ROOT / "docs" / "images" / "tiles"

# Three of these fit a desktop README column, and each one stays legible on its
# own in a phone column.
TILE_WIDTH = "275"

SAMPLES = {
    "tile-stats": ["--card", "stats"],
    "tile-languages": ["--card", "languages"],
    "tile-streak": ["--card", "streak"],
    "tile-heat": ["--card", "heat"],
    "tile-total": ["--card", "metric", "--metric", "total"],
    "tile-longest": ["--card", "metric", "--metric", "longest"],
    "tile-stars": ["--card", "metric", "--metric", "stars"],
}


def main() -> int:
    if not BINARY.exists():
        raise SystemExit(
            f"build the CLI first: cargo build -p github-personal-stats ({BINARY})"
        )

    OUTPUT.mkdir(parents=True, exist_ok=True)

    for name, options in SAMPLES.items():
        subprocess.run(
            [str(BINARY), "generate", "--fixture", str(SHOWCASE),
             "--user", "showcase", "--width", TILE_WIDTH, "--height", "auto",
             "--output", str(OUTPUT / f"{name}.svg"), *options],
            check=True,
        )
        print(f"{name}.svg")

    return 0


if __name__ == "__main__":
    sys.exit(main())
