"""Builds the documentation site from the guide.

The pages are generated rather than written, because the guide already says all
of this and two copies of an explanation drift apart quietly: the wrong one is
usually the one being read. `user-guide.md` stays canonical and readable on its
own, and this turns it into a site.

The markdown understood here is only the subset the guide uses. Anything else
raises rather than being passed through, so an unrecognised construct fails the
build instead of appearing as literal asterisks on the page.

Run from the repository root: python3 docs/build.py
"""

import html
import pathlib
import re
import shutil
import sys

ROOT = pathlib.Path(__file__).resolve().parent
GUIDE = ROOT / "user-guide.md"
PAGES_DIR = ROOT / "pages"
EXAMPLES = ROOT.parent / "examples"

# The site is published from this directory, so nothing above it can be reached
# over HTTP however well the path resolves on a filesystem. The cards the guide
# points at live in the repository's `examples`, and are copied in rather than
# linked up and out.
CARRIED = ROOT / "examples"

# Which sections of the guide make up each page, in the order a reader meets
# them. Every `##` in the guide must appear exactly once below; the build checks.
PAGES = [
    (
        "getting-started",
        "Getting started",
        "Install it as an action, a command, or a service, and see a card.",
        ["Setup Overview", "GitHub Action", "Command Line", "Local Preview"],
    ),
    (
        "cards",
        "Cards and layout",
        "Every card, how to place them in a README, and how a row behaves on a phone.",
        ["Card Types", "README Usage", "Composing Tiles", "Sizing"],
    ),
    (
        "configuration",
        "Configuration",
        "What each panel reports, and which repositories count towards a language.",
        ["Panel Content", "Language Scope"],
    ),
    (
        "heat-ring",
        "The heat ring",
        "The window it covers, the shape it takes, how it is scaled, and what sits in the middle.",
        ["Heat Ring"],
    ),
    (
        "themes",
        "Themes and colour",
        "Light, dark, and transparent, and why a ramp cannot serve both surfaces unchanged.",
        ["Themes", "Visual Notes"],
    ),
    (
        "activity",
        "Coding activity",
        "Collecting time and lines from your own machine, and where the record is kept.",
        ["Coding Activity Section", "Local Activity Storage"],
    ),
    (
        "private-data",
        "Private repository data",
        "Counting work that is not public, with a token scoped to just that.",
        ["Private Repository Data"],
    ),
]


class Unrecognised(Exception):
    """A construct the converter was not taught. Better loud than silently wrong."""


def anchor(text):
    """The id GitHub would give a heading, so anchors in the guide keep working."""
    kept = re.sub(r"[^\w\- ]", "", text.lower())
    return kept.strip().replace(" ", "-")


def inline(text, depth):
    """Inline markdown. Escaping happens here, before any tag is introduced."""
    placeholders = []

    def keep(markup):
        placeholders.append(markup)
        return f"\x00{len(placeholders) - 1}\x00"

    # Code first: nothing inside a span of code is markup.
    text = re.sub(
        r"`([^`]+)`",
        lambda m: keep(f"<code>{html.escape(m.group(1))}</code>"),
        text,
    )
    text = re.sub(
        r"!\[([^\]]*)\]\(([^)]+)\)",
        lambda m: keep(
            f'<img src="{relocate(m.group(2), depth)}" alt="{html.escape(m.group(1))}" />'
        ),
        text,
    )
    text = re.sub(
        r"\[([^\]]+)\]\(([^)]+)\)",
        lambda m: keep(
            f'<a href="{relocate(m.group(2), depth)}">{html.escape(m.group(1))}</a>'
        ),
        text,
    )
    text = re.sub(
        r"\*\*([^*]+)\*\*",
        lambda m: keep(f"<strong>{html.escape(m.group(1))}</strong>"),
        text,
    )

    text = html.escape(text)
    for index, markup in enumerate(placeholders):
        text = text.replace(f"\x00{index}\x00", markup)
    return text


ANCHORS = {}


def contained(target):
    """Rewrites a reference that would climb out of the published directory.

    The guide sits in `docs/` and points up at `../examples/` for the cards; a
    site published from `docs/` cannot follow that, even though the path is
    perfectly good on a filesystem. The copies carried in are used instead.
    """
    return re.sub(r"(?:\.\./)+examples/", "examples/", target)


def relocate(target, depth):
    """Rewrites a link written for the guide so it still points somewhere.

    Three kinds arrive here. A bare anchor referred to another part of one long
    document and now has to name the page it landed on. A path was relative to
    `docs/`, and a page one directory down has to climb. Anything absolute is
    left alone.
    """
    if target.startswith("#"):
        found = ANCHORS.get(target[1:])
        if not found:
            raise Unrecognised(f"link to {target}, which is not a heading in the guide")
        page, fragment = found
        return f"{page}.html#{fragment}" if depth else f"pages/{page}.html#{fragment}"
    if target.startswith(("http://", "https://", "mailto:", "/")):
        return target
    return "../" * depth + contained(target).removeprefix("./")


def relocate_html(markup, depth):
    """The same climb, for the raw HTML blocks the guide uses to centre images."""
    return re.sub(
        r'(src|href)="(?!https?://|/)([^"]+)"',
        lambda m: f'{m.group(1)}="{"../" * depth}{contained(m.group(2)).removeprefix("./")}"',
        markup,
    )


def convert(lines, depth):
    """The guide's markdown subset, as HTML."""
    out = []
    index = 0
    while index < len(lines):
        line = lines[index]

        if not line.strip():
            index += 1
        elif line.startswith("```"):
            language = line[3:].strip()
            index += 1
            body = []
            while index < len(lines) and not lines[index].startswith("```"):
                body.append(lines[index])
                index += 1
            index += 1
            marked = f' data-language="{html.escape(language)}"' if language else ""
            out.append(
                f"<pre{marked}><code>{html.escape(chr(10).join(body))}</code></pre>"
            )
        elif line.startswith(("## ", "### ")):
            level = 3 if line.startswith("### ") else 2
            text = line.lstrip("#").strip()
            out.append(
                f'<h{level} id="{anchor(text)}">{inline(text, depth)}'
                f'<a class="permalink" href="#{anchor(text)}" '
                f'aria-label="Link to this section">#</a></h{level}>'
            )
            index += 1
        elif line.lstrip().startswith("|"):
            rows = []
            while index < len(lines) and lines[index].lstrip().startswith("|"):
                rows.append(lines[index].strip())
                index += 1
            out.append(table(rows, depth))
        elif re.match(r"^\s*[-*] ", line):
            items = []
            while index < len(lines) and re.match(r"^\s*[-*] ", lines[index]):
                items.append(re.sub(r"^\s*[-*] ", "", lines[index]))
                index += 1
            listed = "".join(f"<li>{inline(item, depth)}</li>" for item in items)
            out.append(f"<ul>{listed}</ul>")
        elif re.match(r"^\s*\d+\. ", line):
            items = []
            while index < len(lines) and re.match(r"^\s*\d+\. ", lines[index]):
                items.append(re.sub(r"^\s*\d+\. ", "", lines[index]))
                index += 1
            listed = "".join(f"<li>{inline(item, depth)}</li>" for item in items)
            out.append(f"<ol>{listed}</ol>")
        elif line.lstrip().startswith("<"):
            block = []
            while index < len(lines) and lines[index].strip():
                block.append(lines[index])
                index += 1
            out.append(relocate_html("\n".join(block), depth))
        elif line.startswith("#"):
            raise Unrecognised(f"heading level not used by the site: {line!r}")
        else:
            paragraph = []
            while (
                index < len(lines)
                and lines[index].strip()
                and not lines[index].startswith(("```", "#", "|", "<"))
                and not re.match(r"^\s*([-*]|\d+\.) ", lines[index])
            ):
                paragraph.append(lines[index].strip())
                index += 1
            out.append(f"<p>{inline(' '.join(paragraph), depth)}</p>")
    return "\n".join(out)


def table(rows, depth):
    def cells(row):
        return [cell.strip() for cell in row.strip().strip("|").split("|")]

    head = cells(rows[0])
    body = rows[1:]
    # A separator row of dashes is a rule, not data.
    if body and all(set(cell) <= set("-: ") for cell in cells(body[0])):
        body = body[1:]

    if any("<img" in cell for row in body for cell in cells(row)):
        return comparison(head, [cells(row) for row in body], depth)

    thead = "".join(f"<th>{inline(cell, depth)}</th>" for cell in head)
    tbody = "".join(
        "<tr>" + "".join(f"<td>{inline(cell, depth)}</td>" for cell in cells(row)) + "</tr>"
        for row in body
    )
    return (
        f'<div class="table-scroll"><table><thead><tr>{thead}</tr></thead>'
        f"<tbody>{tbody}</tbody></table></div>"
    )


def comparison(head, body, depth):
    """The guide's side-by-side comparisons, as a grid rather than a table.

    These read column-major: each column is one variant, with its name in the
    header, a picture of it, and a sentence about it. As a table that only works
    while every variant fits across the page — six palettes side by side do not
    fit a phone, and a table can only offer to scroll, which hides variants
    behind an edge with nothing to say they are there. CSS cannot transpose a
    table, so the columns are emitted as cells of a grid instead, and the grid
    reflows to as many across as will fit with nothing hidden.
    """
    figures = []
    for column, name in enumerate(head):
        picture = ""
        notes = []
        for row in body:
            if column >= len(row):
                continue
            cell = row[column]
            if "<img" in cell and not picture:
                picture = relocate_html(cell, depth)
            elif cell:
                notes.append(inline(cell, depth))
        described = "".join(f"<p>{note}</p>" for note in notes)
        figures.append(
            f'<figure><figcaption>{inline(name, depth)}</figcaption>'
            f'<div class="shot">{picture}</div>{described}</figure>'
        )
    return f'<div class="comparison">{"".join(figures)}</div>'


def sections(text):
    """The guide, split on its `##` headings, in order."""
    found = {}
    order = []
    name = None
    body = []
    for line in text.split("\n"):
        if line.startswith("## "):
            if name:
                found[name] = body
            name = line[3:].strip()
            order.append(name)
            body = [line]
        elif name:
            body.append(line)
    if name:
        found[name] = body
    return found, order


def index_anchors(found):
    """Every heading, and which page it will be on, so links can be rewritten."""
    for slug, _, _, wanted in PAGES:
        for name in wanted:
            for line in found[name]:
                if line.startswith(("## ", "### ")):
                    text = line.lstrip("#").strip()
                    ANCHORS[anchor(text)] = (slug, anchor(text))


def nav(current, depth):
    up = "../" * depth
    links = [f'<a href="{up}index.html"{class_for("home", current)}>Home</a>']
    links += [
        f'<a href="{up}pages/{slug}.html"{class_for(slug, current)}>{html.escape(title)}</a>'
        for slug, title, _, _ in PAGES
    ]
    return "\n      ".join(links)


def class_for(slug, current):
    return ' class="current" aria-current="page"' if slug == current else ""


def shell(title, description, current, depth, body, wide=False):
    up = "../" * depth
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>{html.escape(title)} · GitHub Personal Stats</title>
<meta name="description" content="{html.escape(description)}" />
<link rel="icon" type="image/svg+xml" href="{up}brand/icon-small.svg" />
<link rel="stylesheet" href="{up}assets/site.css" />
</head>
<body{' class="wide"' if wide else ""}>
<a class="skip" href="#content">Skip to content</a>
<header class="masthead">
  <a class="brand" href="{up}index.html">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="{up}brand/logo-dark.svg" />
      <img src="{up}brand/logo.svg" alt="GitHub Personal Stats" height="34" />
    </picture>
  </a>
  <nav aria-label="Documentation">
      {nav(current, depth)}
  </nav>
  <a class="repo" href="https://github.com/liuchong/github-personal-stats">GitHub</a>
</header>
<main id="content">
{body}
</main>
<footer class="footer">
  <p>Generated from <a href="https://github.com/liuchong/github-personal-stats/blob/master/docs/user-guide.md">the guide</a>, so the site and the documentation cannot disagree.</p>
</footer>
</body>
</html>
"""


def landing():
    cards = "\n".join(
        f'    <a class="card" href="pages/{slug}.html">'
        f"<h3>{html.escape(title)}</h3><p>{html.escape(blurb)}</p></a>"
        for slug, title, blurb, _ in PAGES
    )
    body = f"""<section class="hero">
  <div>
    <h1>Your profile, drawn once<br />and read anywhere</h1>
    <p class="lede">One SVG for your GitHub stats, language share, contributions, and
    streak — laid out by the renderer, so your README does not have to fight tables,
    image heights, or fragile HTML alignment.</p>
    <div class="actions">
      <a class="button primary" href="pages/getting-started.html">Get started</a>
      <a class="button" href="pages/cards.html">See the cards</a>
    </div>
    <p class="fineprint">Runs as a GitHub Action, a local command, or a small service.
    Light, dark, and transparent themes. Tiles that reflow on a phone.</p>
  </div>
  <div class="hero-art">
    <img src="examples/dashboard.svg" alt="A dashboard card showing stats, languages, contributions, and a streak ring" />
  </div>
</section>

<section class="section">
  <h2>What it draws</h2>
  <div class="gallery">
    <figure><div class="shot"><img src="examples/stats.svg" alt="Stats card" /></div><figcaption>Stats</figcaption></figure>
    <figure><div class="shot"><img src="examples/languages.svg" alt="Languages card" /></div><figcaption>Languages</figcaption></figure>
    <figure><div class="shot"><img src="examples/streak.svg" alt="Streak card with a heat ring" /></div><figcaption>Streak, with the heat ring</figcaption></figure>
  </div>
</section>

<section class="section">
  <h2>Documentation</h2>
  <div class="cards">
{cards}
  </div>
</section>

<section class="section">
  <h2>One fetch behind every tile</h2>
  <p class="prose">Read a profile once, save it, then draw every card, theme, and width
  from what was saved. A README with five tiles asks GitHub for nothing five times.</p>
  <pre><code>github-personal-stats fetch --user your-login --output profile.json
github-personal-stats generate --fixture profile.json --card stats  --output stats.svg
github-personal-stats generate --fixture profile.json --card heat   --output heat.svg</code></pre>
</section>
"""
    return shell(
        "GitHub profile stats, as one SVG",
        "Generate a polished GitHub profile dashboard as a single SVG: stats, language "
        "share, contributions, and a configurable streak heat ring.",
        "home",
        0,
        body,
        wide=True,
    )


def carry_examples():
    """Copies the cards the site shows into the published directory."""
    CARRIED.mkdir(parents=True, exist_ok=True)
    for card in sorted(EXAMPLES.glob("*.svg")):
        shutil.copyfile(card, CARRIED / card.name)
    return sorted(card.name for card in CARRIED.glob("*.svg"))


def main():
    text = GUIDE.read_text()
    found, order = sections(text)

    claimed = [name for _, _, _, wanted in PAGES for name in wanted]
    missing = [name for name in order if name not in claimed]
    unknown = [name for name in claimed if name not in found]
    if missing or unknown:
        raise Unrecognised(
            f"the site and the guide disagree: unplaced {missing}, absent {unknown}"
        )
    if len(claimed) != len(set(claimed)):
        raise Unrecognised("a section is claimed by two pages")

    index_anchors(found)
    PAGES_DIR.mkdir(parents=True, exist_ok=True)

    for position, (slug, title, blurb, wanted) in enumerate(PAGES):
        lines = []
        for name in wanted:
            lines.extend(found[name])
        body = convert(lines, depth=1)

        steps = []
        if position:
            before = PAGES[position - 1]
            steps.append(
                f'<a class="prev" href="{before[0]}.html">'
                f"<span>Previous</span>{html.escape(before[1])}</a>"
            )
        if position + 1 < len(PAGES):
            after = PAGES[position + 1]
            steps.append(
                f'<a class="next" href="{after[0]}.html">'
                f"<span>Next</span>{html.escape(after[1])}</a>"
            )

        page = (
            f'<article class="doc">\n<h1>{html.escape(title)}</h1>\n'
            f'<p class="lede">{html.escape(blurb)}</p>\n{body}\n'
            f'<nav class="steps">{"".join(steps)}</nav>\n</article>'
        )
        (PAGES_DIR / f"{slug}.html").write_text(shell(title, blurb, slug, 1, page))
        print(f"wrote docs/pages/{slug}.html")

    (ROOT / "index.html").write_text(landing())
    print("wrote docs/index.html")
    print(f"carried {', '.join(carry_examples())} into docs/examples/")


if __name__ == "__main__":
    try:
        main()
    except Unrecognised as problem:
        sys.exit(f"docs/build.py: {problem}")
