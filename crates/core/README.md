# github-personal-stats-core

The library behind [GitHub Personal Stats](https://github.com/liuchong/github-personal-stats). It
fetches a GitHub profile, aggregates it, and renders SVG cards: a stats panel with a rank ring, a
language breakdown, and a contribution heat ring.

```rust
use github_personal_stats_core::{
    GithubClient, GithubGraphqlClient, GithubStatsConfig, HeatRing, OutputKind,
    aggregate_card_data, render_card,
};

let config = GithubStatsConfig::new("octocat")?
    .with_theme("dark")?
    .with_size(1000, 420)?;

let client = GithubGraphqlClient::new("https://api.github.com/graphql");
let data = client.fetch_user_data(&config)?;
let card = aggregate_card_data(&data, OutputKind::Dashboard, &HeatRing::default());
let svg = render_card(&card, &config);
```

Rendering is pure: it takes aggregated data and returns a `String`, touching neither the network nor
the filesystem, so cards can be produced from sanitized fixtures in tests via `MockGithubClient`.

Every option is documented in the
[user guide](https://github.com/liuchong/github-personal-stats/blob/master/docs/user-guide.md).
