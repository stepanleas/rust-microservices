# Build from the repository root so the `shared` path dependency is in the context:
#   docker build -f customer/Dockerfile -t artellas/rust-customer .
FROM lukemathwalker/cargo-chef:latest-rust-1.98.1 AS chef
WORKDIR /app/customer

FROM chef AS planner
COPY ./shared /app/shared
COPY ./customer/Cargo.toml ./customer/Cargo.lock ./
COPY ./customer/src ./src
RUN cargo chef prepare

FROM chef AS builder
COPY ./shared /app/shared
COPY --from=planner /app/customer/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY ./customer .
RUN cargo build -p starter --release

FROM debian:trixie-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates libpq5 curl \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/customer/target/release/starter /usr/local/bin/app
ENTRYPOINT ["/usr/local/bin/app"]
