FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p github-stats-server

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/github-stats-server /usr/local/bin/github-stats-server
ENV GITHUB_STATS_ADDR=0.0.0.0:3000
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/github-stats-server"]
