"""Draws the brand marks.

The mark is the product's own heat ring rather than a new shape invented for it,
so the icon and the cards it stands for read as the same thing. Geometry is
computed rather than hand-written because a ring of ticks drawn by eye is never
quite even, and unevenness is the first thing the eye catches at small sizes.

Run from the repository root: python3 docs/brand/build.py
"""

import math
import pathlib

HEAT = ["#ffe3ad", "#ffc65c", "#ffa726", "#fb8c00"]
BLUE = "#0969da"
BLUE_DARK = "#4493f8"
INK = "#1f2328"
INK_DARK = "#e6edf3"

OUT = pathlib.Path("docs/brand")

# A plausible month rather than a flat ring: a mark that stands for a chart
# should look like it is carrying data.
DAYS = [2, 1, 3, 3, 2, 0, 1, 3, 4, 3, 2, 1, 0, 2, 4, 4, 3, 2, 1, 1, 3, 4, 4, 3]


def segments(centre, radius, width, heat, gap_degrees=3.0, quiet=None):
    """A ring of arc segments, one per day, coloured by how much was done.

    Arcs rather than radial ticks, which is the shape the cards themselves use
    for a long streak. It also matters at this size: a ring of separated ticks
    around a gapped circle is the spinner every interface draws while it waits,
    and a mark for a chart must not be read as one.
    """
    parts = []
    count = len(heat)
    step = 360 / count
    for index, level in enumerate(heat):
        start = -90 + index * step + gap_degrees / 2
        end = -90 + (index + 1) * step - gap_degrees / 2
        x1 = centre + radius * math.cos(math.radians(start))
        y1 = centre + radius * math.sin(math.radians(start))
        x2 = centre + radius * math.cos(math.radians(end))
        y2 = centre + radius * math.sin(math.radians(end))
        # A quiet day is drawn, faintly, because a gap would read as missing
        # data rather than as a day with nothing on it. On a dark background a
        # pale colour held at low opacity reads as grime rather than as an empty
        # day, so the quiet tone is given rather than derived by fading.
        if level:
            colour, opacity = HEAT[min(level, len(HEAT) - 1)], 1.0
        elif quiet:
            colour, opacity = quiet, 1.0
        else:
            colour, opacity = HEAT[0], 0.32
        parts.append(
            f'<path d="M {x1:.2f} {y1:.2f} A {radius:.2f} {radius:.2f} 0 0 1 '
            f'{x2:.2f} {y2:.2f}" fill="none" stroke="{colour}" '
            f'stroke-width="{width}" stroke-linecap="butt" opacity="{opacity}"/>'
        )
    return parts


def bars(centre, colour, scale, heights=(0.42, 0.72, 1.0)):
    """Three rising bars, which no one has ever mistaken for a spinner."""
    parts = []
    width = 4.2 * scale
    spacing = 6.8 * scale
    tallest = 18.5 * scale
    base = centre + 9.2 * scale
    left = centre - spacing
    for index, share in enumerate(heights):
        height = tallest * share
        parts.append(
            f'<rect x="{left + index * spacing - width / 2:.2f}" '
            f'y="{base - height:.2f}" width="{width:.2f}" height="{height:.2f}" '
            f'rx="{width / 2:.2f}" fill="{colour}"/>'
        )
    return parts


def arc(centre, radius, width, colour, fraction):
    """The rank ring: a circle closed as far as the percentile reaches."""
    circumference = 2 * math.pi * radius
    return (
        f'<circle cx="{centre}" cy="{centre}" r="{radius:.2f}" fill="none" '
        f'stroke="{colour}" stroke-width="{width}" stroke-linecap="round" '
        f'stroke-dasharray="{circumference * fraction:.2f} {circumference:.2f}" '
        f'transform="rotate(-90 {centre} {centre})"/>'
    )


def icon(size=64, detailed=True, dark=False):
    centre = size / 2
    scale = size / 64
    # The small mark is not the large one shrunk: at sixteen pixels a
    # twenty-four segment ring is a smudge, so it keeps eight fat segments and
    # larger bars, and stays recognisably the same shape.
    body = segments(
        centre,
        radius=26 * scale,
        width=(7.0 if detailed else 10.0) * scale,
        heat=DAYS if detailed else [3, 1, 4, 2, 4, 3, 1, 3],
        gap_degrees=3.2 if detailed else 7.0,
        quiet=QUIET_DARK if dark else None,
    )
    body.extend(
        bars(centre, BLUE_DARK if dark else BLUE, scale if detailed else scale * 1.18)
    )
    drawn = "\n  ".join(body)
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" '
        f'width="{size}" height="{size}" role="img" aria-label="GitHub Personal Stats">\n'
        f"  <title>GitHub Personal Stats</title>\n  {drawn}\n</svg>\n"
    )


FONT = (
    "ui-sans-serif, -apple-system, BlinkMacSystemFont, 'Segoe UI', "
    "Helvetica, Arial, sans-serif"
)


# On dark, an empty day is a dim ember rather than a faded cream, which would
# read as dirt on the mark.
QUIET_DARK = "#43301a"


def logo(dark=False):
    """The horizontal lockup: mark, then name, on one baseline.

    No tagline. A line of prose set inside a logo cannot be re-worded, cannot be
    translated, and is unreadable at the size a logo is usually placed at; it
    belongs in the page beside the mark.
    """
    ink = INK_DARK if dark else INK
    blue = BLUE_DARK if dark else BLUE
    height, width = 64, 268
    body = segments(32, radius=26, width=7.0, heat=DAYS, quiet=QUIET_DARK if dark else None)
    body.extend(bars(32, blue, 1.0))
    drawn = "\n  ".join(body)
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" '
        f'width="{width}" height="{height}" role="img" '
        f'aria-label="GitHub Personal Stats">\n'
        f"  <title>GitHub Personal Stats</title>\n  {drawn}\n"
        f'  <text x="74" y="38" font-family="{FONT}" font-size="18" '
        f'font-weight="700" fill="{ink}" letter-spacing="-0.4">GitHub Personal Stats</text>\n'
        f"</svg>\n"
    )


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    written = {
        "icon.svg": icon(64, detailed=True),
        "icon-small.svg": icon(32, detailed=False),
        "icon-dark.svg": icon(64, detailed=True, dark=True),
        "logo.svg": logo(dark=False),
        "logo-dark.svg": logo(dark=True),
    }
    for name, body in written.items():
        (OUT / name).write_text(body)
        print(f"wrote {OUT / name}")


if __name__ == "__main__":
    main()
