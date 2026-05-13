FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p github-personal-stats-server

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/github-personal-stats-server /usr/local/bin/github-personal-stats-server
ENV GITHUB_PERSONAL_STATS_ADDR=0.0.0.0:3000
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/github-personal-stats-server"]
