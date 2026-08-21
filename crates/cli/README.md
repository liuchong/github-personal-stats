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

`generate --help` lists every option, including the heat ring window, shape, threshold, scale,
palette, and centre label. Cards can also be rendered from a sanitized fixture with `--fixture`,
which needs no token and no network.

For scheduled updates without installing anything, the same renderer ships as a GitHub Action. See
the [project README](https://github.com/liuchong/github-personal-stats) and the
[user guide](https://github.com/liuchong/github-personal-stats/blob/master/docs/user-guide.md).
