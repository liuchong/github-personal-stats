# Deployment

## Local Server

```sh
cargo run -p github-personal-stats-server
```

Endpoints:

- `/health`
- `/info`
- `/api?username=octo`
- `/api/stats?username=octo`
- `/api/languages?username=octo`
- `/api/streak?username=octo`
- `/api/wakatime-text`

## Docker

```sh
docker build -t github-personal-stats .
docker run --rm -p 3000:3000 github-personal-stats
```

## Kubernetes

```sh
kubectl apply -f deploy/k8s/deployment.yaml
```

The manifest is an example and should be reviewed before use in a real cluster.
