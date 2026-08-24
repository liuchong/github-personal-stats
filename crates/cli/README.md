# github-personal-stats

Render GitHub profile statistics as SVG cards for a profile README: a stats panel with a rank ring,
a language breakdown, and a contribution heat ring.

```sh
cargo install github-personal-stats
```

```sh
export GITHUB_TOKEN=...
github-personal-stats generate --user octocat --theme dark --output stats.svg
```

`help` lists every option, including the heat ring window, shape, threshold, scale, palette, and
centre label.

Rendering never asks GitHub anything, so a set of cards costs one read. `fetch` saves a profile and
`--fixture` draws from it, needing neither token nor network:

```sh
github-personal-stats fetch --user octocat --output profile.json
github-personal-stats generate --fixture profile.json --card stats --output stats.svg
github-personal-stats generate --fixture profile.json --card heat  --output heat.svg
```

For scheduled updates without installing anything, the same renderer ships as a GitHub Action. See
the [project README](https://github.com/liuchong/github-personal-stats) and the
[user guide](https://github.com/liuchong/github-personal-stats/blob/master/docs/user-guide.md).
