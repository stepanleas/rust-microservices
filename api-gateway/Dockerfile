# Build from the repository root so the `shared` path dependency is in the context:
#   docker build -f api-gateway/Dockerfile -t artellas/rust-api-gateway .
FROM lukemathwalker/cargo-chef:latest-rust-1.98.1 AS chef
WORKDIR /app/api-gateway

FROM chef AS planner
COPY ./shared /app/shared
COPY ./api-gateway/Cargo.toml ./api-gateway/Cargo.lock ./
COPY ./api-gateway/src ./src
RUN cargo chef prepare
# A skeleton of `shared` for the dependency layer below: its manifest and empty sources. It
# only changes when shared's dependencies do, so editing shared's code keeps the cooked
# dependencies (cargo-chef's recipe covers the service's own crates, not path dependencies).
RUN cd /app/shared \
    && find . -name '*.rs' | while read -r file; do \
         mkdir -p "/app/shared-skeleton/$(dirname "$file")" && : > "/app/shared-skeleton/$file"; \
       done \
    && cp Cargo.toml /app/shared-skeleton/

FROM chef AS builder
COPY --from=planner /app/shared-skeleton /app/shared
COPY --from=planner /app/api-gateway/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY ./shared /app/shared
# Newer than the skeleton just compiled, or cargo would take the empty sources as up to date.
RUN find /app/shared -name '*.rs' -exec touch {} +
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
