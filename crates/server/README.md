# github-personal-stats-server

An HTTP front end for [GitHub Personal Stats](https://github.com/liuchong/github-personal-stats),
rendering profile cards as SVG on request.

| Route | Response |
| --- | --- |
| `/api`, `/api/dashboard` | Dashboard card |
| `/api/stats`, `/api/languages`, `/api/streak` | Individual cards |
| `/health` | `ok` |
| `/info` | Workspace metadata as JSON |

Card routes read `username`, `card`, `width`, `height`, and `theme` from the query string, as in
`/api?username=octocat&theme=dark`. Heat ring options are currently CLI only.

Committing generated SVGs is usually the better choice for a profile page, since it keeps rendering
off the request path. This crate exists for deployments that want cards on demand.
