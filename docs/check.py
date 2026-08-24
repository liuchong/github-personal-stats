"""Checks the built site the way a browser would see it.

Written because an earlier check resolved references against the filesystem and
passed a page whose every image was broken: the site is published from `docs/`,
so a path climbing above it resolves fine on disk and fetches nothing at all.
Reachability is therefore judged from the published root, not from the repository.

Run from the repository root: python3 docs/check.py
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent


def references(text):
    """Every local reference in a page, with fragments and queries removed."""
    for kind, target in re.findall(r'(src|srcset|href)="([^"]+)"', text):
        target = target.split()[0] if kind == "srcset" else target
        if target.startswith(("http://", "https://", "mailto:", "#", "data:")):
            continue
        yield kind, target.split("#")[0].split("?")[0]


def main():
    pages = sorted(ROOT.glob("**/*.html"))
    if not pages:
        sys.exit("docs/check.py: nothing built; run docs/build.py first")

    faults = []
    for page in pages:
        text = page.read_text()
        shown = page.relative_to(ROOT)

        for kind, target in references(text):
            if not target:
                continue
            landing = (page.parent / target).resolve()
            # A reference that leaves the published directory is unreachable
            # over HTTP whatever it resolves to on this machine.
            if ROOT not in landing.parents and landing != ROOT:
                faults.append(f"{shown}: {kind}={target} climbs out of docs/")
            elif not landing.exists():
                faults.append(f"{shown}: {kind}={target} points at nothing")

        # Markdown that was not converted, ignoring anything inside code.
        prose = re.sub(r"<pre[^>]*>.*?</pre>|<code>.*?</code>", "", text, flags=re.S)
        for pattern, what in ((r"\*\*", "bold markers"), (r"\]\(", "markdown links")):
            left = len(re.findall(pattern, prose))
            if left:
                faults.append(f"{shown}: {left} unconverted {what}")

        for tag in ("table", "tr", "td", "ul", "li", "p", "pre", "div", "figure"):
            opened = len(re.findall(f"<{tag}[ >]", text))
            closed = len(re.findall(f"</{tag}>", text))
            if opened != closed:
                faults.append(f"{shown}: <{tag}> opened {opened}, closed {closed}")

        if "<title>" not in text:
            faults.append(f"{shown}: no title")

    if faults:
        sys.exit("docs/check.py:\n  " + "\n  ".join(faults))
    print(f"{len(pages)} pages: every reference reachable from docs/, nothing unconverted")


if __name__ == "__main__":
    main()
