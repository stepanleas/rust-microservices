# Build from the repository root so the `shared` path dependency is in the context:
#   docker build -f catalog/Dockerfile -t artellas/rust-catalog .
FROM lukemathwalker/cargo-chef:latest-rust-1.98.1 as chef
WORKDIR /app/catalog

FROM chef AS planner
COPY ./shared /app/shared
COPY ./catalog/Cargo.toml ./catalog/Cargo.lock ./
COPY ./catalog/src ./src
RUN cargo chef prepare

FROM chef AS builder
COPY ./shared /app/shared
COPY --from=planner /app/catalog/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY ./catalog .
RUN cargo build -p starter --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates libpq5 curl \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/catalog/target/release/starter /usr/local/bin/app
ENTRYPOINT ["/usr/local/bin/app"]
