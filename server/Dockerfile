FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

# 1. Plan
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

# 2. Build
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --bin spektrum

# 3. Runtime
FROM debian:trixie-slim AS runtime
WORKDIR /app

RUN apt-get update \
 && apt-get install -y ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/spektrum /usr/local/bin/

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/spektrum"]
