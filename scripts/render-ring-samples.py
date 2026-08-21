#!/usr/bin/env python3
"""Render the heat ring samples used by the docs.

Each sample is a streak card cropped down to the ring by rewriting the SVG
viewBox, so the images stay vector and reproduce exactly from the CLI.

    python3 scripts/render-ring-samples.py
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target" / "debug" / "github-personal-stats"
# The long fixture carries a real 117-day streak; the showcase fixture has quiet
# days, which is what a fixed window needs to show.
LONG = ROOT / "examples" / "streak-117.json"
SHOWCASE = ROOT / "examples" / "showcase.json"
OUTPUT = ROOT / "docs" / "images" / "heat-ring"
# A wide card keeps the neighbouring columns well clear of the crop.
CARD = ("--card", "streak", "--width", "1000", "--height", "220")

# Half-width of the crop around the ring centre, and how far below it to keep
# room for the caption lines.
CROP_HALF = 82
CROP_BELOW = 122

SAMPLES = {
    "window-streak": (LONG, []),
    "window-fixed-30": (SHOWCASE, ["--heat-window", "30"]),
    "window-limit-30": (LONG, ["--heat-limit", "30", "--heat-label", "{Y} of {Z}"]),
    "shape-ticks-30": (SHOWCASE, ["--heat-window", "30", "--heat-shape", "ticks"]),
    "shape-ticks-117": (LONG, ["--heat-shape", "ticks"]),
    "shape-arcs-117": (LONG, ["--heat-shape", "arcs"]),
    "shape-bands-117": (LONG, ["--heat-shape", "bands"]),
    "scale-linear": (LONG, ["--heat-scale", "linear"]),
    "scale-sqrt": (LONG, ["--heat-scale", "sqrt"]),
    "scale-log": (LONG, ["--heat-scale", "log"]),
    "scale-quantile": (LONG, ["--heat-scale", "quantile"]),
    "label-streak": (LONG, []),
    "label-active-of-window": (SHOWCASE, ["--heat-window", "30"]),
    "label-three-counts": (
        SHOWCASE, ["--heat-window", "30", "--heat-label", "{X}/{Y}/{Z}"],
    ),
    "label-arrow": (
        SHOWCASE, ["--heat-window", "30", "--heat-label", "{X} of {Y} \u2192 {Z}"],
    ),
}

PALETTE_SAMPLE = ["--heat-window", "30", "--heat-scale", "sqrt"]

for palette in (
    "heat-orange", "github-blue", "forest", "violet", "crimson", "graphite",
):
    SAMPLES[f"palette-{palette}"] = (
        SHOWCASE, [*PALETTE_SAMPLE, "--heat-color", palette],
    )

SAMPLES["palette-derived"] = (SHOWCASE, [*PALETTE_SAMPLE, "--heat-color", "#8250df"])
SAMPLES["palette-explicit"] = (
    SHOWCASE,
    [*PALETTE_SAMPLE, "--heat-color", "#dbe9d5,#a3cf9a,#5aa04f,#1f6f2f"],
)

# The same ring on both surfaces, to show the ramp turning around rather than
# keeping light stops that would outshine the busy days on a dark card.
SAMPLES["theme-light"] = (SHOWCASE, [*PALETTE_SAMPLE, "--theme", "light"])
SAMPLES["theme-dark"] = (SHOWCASE, [*PALETTE_SAMPLE, "--theme", "dark"])
SAMPLES["theme-dark-explicit"] = (
    SHOWCASE,
    [*PALETTE_SAMPLE, "--theme", "dark", "--heat-color", "#ffe3ad,#ffc65c,#ffa726,#fb8c00"],
)


def ring_centre(svg: str) -> tuple[int, int]:
    """The caption sits a known distance under the ring, so it locates it."""
    match = re.search(
        r'<text x="(\d+)" y="(\d+)"[^>]*>(?:Current Streak|Last \d+ Days)<', svg
    )
    if not match:
        raise SystemExit("could not find the ring caption")
    return int(match.group(1)), int(match.group(2)) - 26 - 26


def crop(svg: str) -> str:
    centre_x, centre_y = ring_centre(svg)
    left = centre_x - CROP_HALF
    top = centre_y - CROP_HALF
    width = CROP_HALF * 2
    height = CROP_HALF + CROP_BELOW

    svg = re.sub(r'width="\d+" height="\d+"', f'width="{width}" height="{height}"', svg, count=1)
    svg = re.sub(r'viewBox="[^"]+"', f'viewBox="{left} {top} {width} {height}"', svg, count=1)
    # The background rect sits at the card origin, which the crop has moved away
    # from, so anchor it to the new viewBox instead.
    return re.sub(
        r'<rect width="100%" height="100%"',
        f'<rect x="{left}" y="{top}" width="{width}" height="{height}"',
        svg,
        count=1,
    )


def main() -> int:
    if not BINARY.exists():
        raise SystemExit(
            f"build the CLI first: cargo build -p github-personal-stats ({BINARY})"
        )

    OUTPUT.mkdir(parents=True, exist_ok=True)
    scratch = OUTPUT / ".scratch.svg"

    for name, (fixture, options) in SAMPLES.items():
        subprocess.run(
            [str(BINARY), "generate", "--fixture", str(fixture), *CARD,
             "--output", str(scratch), *options],
            check=True,
        )
        (OUTPUT / f"{name}.svg").write_text(crop(scratch.read_text()))
        print(f"{name}.svg")

    scratch.unlink()
    return 0


if __name__ == "__main__":
    sys.exit(main())
