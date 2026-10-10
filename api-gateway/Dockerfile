# Build from the repository root, like the other services:
#   docker build -f api-gateway/Dockerfile -t artellas/rust-api-gateway .
FROM lukemathwalker/cargo-chef:latest-rust-1.98.1 AS chef
WORKDIR /app/api-gateway

FROM chef AS planner
COPY ./api-gateway/Cargo.toml ./api-gateway/Cargo.lock ./
COPY ./api-gateway/src ./src
RUN cargo chef prepare

FROM chef AS builder
COPY --from=planner /app/api-gateway/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY ./api-gateway .
RUN cargo build --release --bin api-gateway

FROM debian:trixie-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/api-gateway/target/release/api-gateway /usr/local/bin/app
EXPOSE 8070
ENTRYPOINT ["/usr/local/bin/app"]
