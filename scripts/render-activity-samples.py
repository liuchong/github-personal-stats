#!/usr/bin/env python3
"""Render the coding activity samples used by the docs.

Every other sample in the guide is drawn from `examples/showcase.json`, an
invented profile, so that anyone can reproduce the pictures and nobody's real
figures end up in the documentation. Activity needs the same thing, and a record
is a directory of days rather than one file, so this writes a sample record and
renders from it.

The days are laid out as offsets from today rather than as fixed dates, which is
what makes the output stable: a window is `last 30 days`, so a record pinned to
calendar dates would slide out of that window and the pictures would go blank a
month after they were generated. The offsets are the same on every run, so the
numbers are too.

    python3 scripts/render-activity-samples.py

It prints the chart blocks the guide quotes. When the shape of the sample or the
renderer changes, re-run it and paste the output back into `docs/user-guide.md`,
because the guide showing something the tool no longer produces is the one thing
these samples exist to prevent.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from datetime import date, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target" / "debug" / "github-personal-stats"
RECORD = ROOT / "target" / "activity-sample" / "snapshots"
OUTPUT = ROOT / "docs" / "images" / "activity"

# A little over three months, so `all time` is meaningfully longer than the
# recent window without the sample becoming a wall of files.
DAYS = 96

# How the sample's work divides. These are shares, not minutes: the day's total
# is split by them, so the mix stays the same as the daily total moves.
#
# The mix changes partway back, because the card's whole point is that a bar is
# the recent window and the mark on it is the longer one. A sample that worked on
# the same things all year would draw every mark at the end of its own bar and
# demonstrate nothing.
LANGUAGES = {
    "Rust": 31,
    "Markdown": 24,
    "Go": 17,
    "TypeScript": 11,
    "Python": 7,
    "Zig": 6,
    "Shell": 4,
}

EARLIER = {
    "Go": 29,
    "Rust": 21,
    "Markdown": 19,
    "Python": 12,
    "TypeScript": 10,
    "Shell": 5,
    "Zig": 4,
}

# Where the mix turns over, in days back from today. Inside the recent window, so
# the two spans genuinely differ.
CHANGED_AT = 26

MODELS = {
    "claude-opus-5": 32,
    "gpt-5.6-sol": 27,
    "gpt-5.5": 21,
    "grok-4.6": 13,
    "unnamed": 7,
}

# Hours an agent worked, before the weekday shaping below. Chosen to look like a
# heavy but plausible week rather than to flatter anybody.
BASE_AGENT_HOURS = 7.5

# The share of agent time no source can put a language to. Terminal agents report
# when they were working without reporting what on, and on a real record that is
# most of the total, which is the whole reason a block of hours by language has to
# declare what it left out. A sample that hid this would make the feature look
# tidier than it is.
UNPLACED_SHARE = 0.62

# Editor presence against agent time. The two overlap and are never added: this is
# a plausible ratio for someone who watches some of the work and not all of it.
EDITOR_SHARE = 0.55

# Hours imported from another tracker, on the older days only, so the guide can
# show two measures side by side over a period where only one of them was being
# collected.
IMPORTED_FROM_DAY = 34
IMPORTED_HOURS = 5.0

# Lines the editor watched appear, per hour of agent time.
LINES_PER_HOUR = 620

# The share of lines nothing accounts for: a formatter, a shell command, a
# terminal agent editing outside the editor. Small, and never zero.
UNATTRIBUTED_SHARE = 0.0007

TOKENS_PER_HOUR = {"input": 1_450_000, "output": 21_000, "cached": 190_000}

MACHINE = "m-5a1e9c72"


def shaped(offset: int) -> float:
    """A day's weight. Two days in seven are lighter, and no two weeks alike.

    Both factors are taken from how far back the day is rather than from what
    day of the week it lands on, so that every run produces the same figures.
    Shaped by the real calendar, the quiet days would move through the recent
    window as the week turned and every quoted number in the guide would change
    daily, which is exactly the churn a fixed sample exists to avoid.
    """
    quiet = 0.42 if offset % 7 >= 5 else 1.0
    cycle = (0.86, 1.12, 0.95, 1.21, 1.03, 0.78, 1.09)[offset % 7]
    return quiet * cycle


def divided(total: int, shares: dict[str, int]) -> dict[str, int]:
    """Splits a total by shares, giving the remainder to the largest part."""
    weight = sum(shares.values())
    parts = {name: total * share // weight for name, share in shares.items()}
    largest = max(shares, key=lambda name: shares[name])
    parts[largest] += total - sum(parts.values())
    return {name: amount for name, amount in parts.items() if amount}


def mix(offset: int) -> dict[str, int]:
    """What was being worked on that far back."""
    return LANGUAGES if offset < CHANGED_AT else EARLIER


def day_file(offset: int) -> dict:
    when = date.today() - timedelta(days=offset)
    weight = shaped(offset)
    languages_that_day = mix(offset)

    agent_seconds = round(BASE_AGENT_HOURS * 3600 * weight)
    placed = round(agent_seconds * (1 - UNPLACED_SHARE))
    languages = divided(placed, languages_that_day)
    languages[""] = agent_seconds - sum(languages.values())

    facts = [
        {"language": language, "author": "agent", "model": model, "seconds": seconds}
        for language, language_seconds in sorted(languages.items())
        if language
        for model, seconds in sorted(divided(language_seconds, MODELS).items())
    ]
    facts += [
        {"author": "agent", "model": model, "seconds": seconds}
        for model, seconds in sorted(divided(languages[""], MODELS).items())
    ]

    time = {
        "agent": {
            "seconds": agent_seconds,
            "languages": dict(sorted(languages.items())),
            "sessions": max(4, round(18 * weight)),
            "facts": facts,
        },
        "editor": {
            "seconds": round(agent_seconds * EDITOR_SHARE),
            "languages": divided(round(agent_seconds * EDITOR_SHARE), languages_that_day),
            "sessions": max(2, round(9 * weight)),
        },
    }
    if offset >= IMPORTED_FROM_DAY:
        time["imported"] = {
            "seconds": round(IMPORTED_HOURS * 3600 * weight),
            "languages": {},
            "sessions": 1,
        }

    total_lines = round(BASE_AGENT_HOURS * weight * LINES_PER_HOUR)
    lines = []
    for language, language_lines in sorted(divided(total_lines, languages_that_day).items()):
        unattributed = round(language_lines * UNATTRIBUTED_SHARE)
        for model, added in sorted(divided(language_lines - unattributed, MODELS).items()):
            lines.append(
                {
                    "language": language,
                    "author": "agent",
                    "model": model,
                    "added": added,
                    "deleted": 0,
                }
            )
        if unattributed:
            lines.append(
                {
                    "language": language,
                    "author": "human",
                    "model": "",
                    "added": unattributed,
                    "deleted": 0,
                }
            )

    hours = BASE_AGENT_HOURS * weight
    tokens = {
        model: {
            kind: round(amount * hours * share / sum(MODELS.values()))
            for kind, amount in TOKENS_PER_HOUR.items()
        }
        for model, share in sorted(MODELS.items())
    }

    return {
        "schema": 2,
        "machine": MACHINE,
        "date": when.isoformat(),
        "time": time,
        "lines": lines,
        "tokens": tokens,
        "requests": max(6, round(52 * weight)),
    }


def write_record() -> None:
    machine = RECORD / MACHINE
    shutil.rmtree(RECORD, ignore_errors=True)
    machine.mkdir(parents=True)
    for offset in range(DAYS):
        written = day_file(offset)
        (machine / f"{written['date']}.json").write_text(json.dumps(written))
    print(f"wrote a {DAYS}-day sample record to {RECORD.relative_to(ROOT)}")


CARDS = {
    "card-light": ["--width", "900", "--theme", "light"],
    "card-dark": ["--width", "900", "--theme", "dark"],
    "card-tile": ["--width", "275", "--theme", "light"],
}

CHARTS = {
    "the default blocks": [],
    "hours by language": ["--activity-blocks", "time/languages"],
    "the agent share of each language": [
        "--activity-blocks",
        "lines/languages,authors=on",
    ],
    "hours beside the lines they produced": [
        "--activity-blocks",
        "lines/languages,limit=4,time=on",
    ],
    "two measures that must not be added": [
        "--activity-blocks",
        "time/windows;time/windows,measure=imported",
    ],
    "what the agents were billed for": ["--activity-blocks", "tokens/models,limit=4"],
}

# The dates a chart opens with are the sample record's, and the sample record
# ends today, so a quoted date line is out of date the day after it is pasted.
# The guide quotes the line once, where it is being explained, and shows the
# other samples as the single blocks they are, which do not carry it anyway.
DATED = "the default blocks"


def main() -> int:
    if not BINARY.exists():
        raise SystemExit(f"build the CLI first: cargo build ({BINARY})")

    write_record()
    OUTPUT.mkdir(parents=True, exist_ok=True)

    for name, options in CARDS.items():
        subprocess.run(
            [str(BINARY), "generate", "--card", "activity",
             "--activity-record", str(RECORD), "--height", "auto",
             "--output", str(OUTPUT / f"{name}.svg"), *options],
            check=True,
        )
        print(f"{name}.svg")

    for described, options in CHARTS.items():
        # Flushed because the chart is written by a subprocess straight to the
        # same stream, and a buffered heading would arrive after its chart.
        print(f"\n=== {described} ===", flush=True)
        dates = [] if described == DATED else ["--activity-dates", "off"]
        subprocess.run(
            [str(BINARY), "chart", "--activity-record", str(RECORD), *options, *dates],
            check=True,
        )

    return 0


if __name__ == "__main__":
    sys.exit(main())
