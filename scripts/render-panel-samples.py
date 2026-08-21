#!/usr/bin/env python3
"""Render the panel content samples used by the docs.

Each sample is a whole card, so no cropping is needed. The language card is kept
wider than the narrow-layout threshold so it shows the two-column list the
dashboard uses rather than the track rows a narrow card falls back to.

    python3 scripts/render-panel-samples.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target" / "debug" / "github-personal-stats"
SHOWCASE = ROOT / "examples" / "showcase.json"
OUTPUT = ROOT / "docs" / "images" / "panels"

STATS = ("--card", "stats", "--width", "460", "--height", "220")
LANGUAGES = ("--card", "languages", "--width", "520", "--height", "170")
STREAK = ("--card", "streak", "--width", "760", "--height", "200")

SAMPLES = {
    "stats-default": (STATS, []),
    "stats-all-six": (
        STATS, ["--stat-rows", "stars,commits,prs,issues,reviews,repos"],
    ),
    "stats-two": (STATS, ["--stat-rows", "reviews,repos"]),
    "languages-default": (LANGUAGES, []),
    "languages-three": (LANGUAGES, ["--language-rows", "3"]),
    "streak-default": (STREAK, []),
    # `current` beside the ring only earns its place when the ring itself is not
    # already reporting the streak, so this pairs it with a fixed window.
    "streak-active-current": (
        STREAK, ["--heat-window", "30", "--streak-sides", "active,current"],
    ),
}


def main() -> int:
    if not BINARY.exists():
        raise SystemExit(
            f"build the CLI first: cargo build -p github-personal-stats ({BINARY})"
        )

    OUTPUT.mkdir(parents=True, exist_ok=True)

    for name, (card, options) in SAMPLES.items():
        subprocess.run(
            [str(BINARY), "generate", "--fixture", str(SHOWCASE),
             "--user", "showcase", *card,
             "--output", str(OUTPUT / f"{name}.svg"), *options],
            check=True,
        )
        print(f"{name}.svg")

    return 0


if __name__ == "__main__":
    sys.exit(main())
