# syntax=docker/dockerfile:1
FROM rust:1.83 AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/workspace

# Copy workspace files
COPY Cargo.toml Cargo.lock ./

# Copy all source code
COPY packages/ ./packages/

# Build the application with cache mounts
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/workspace/target \
    cargo build --release --package zk-mpc-node --locked && \
    cp /usr/src/workspace/target/release/zk-mpc-node /tmp/zk-mpc-node

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /tmp/zk-mpc-node /app/zk-mpc-node
COPY packages/zk-mpc-node/address /app/address
RUN chmod +x /app/zk-mpc-node

EXPOSE 9000

ENTRYPOINT ["/app/zk-mpc-node"]