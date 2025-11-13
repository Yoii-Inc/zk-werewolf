FROM rust:1.83 AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/workspace

# Copy workspace Cargo.toml first
COPY Cargo.toml Cargo.lock ./

# Copy package Cargo.toml files for dependency caching
COPY packages/server/Cargo.toml packages/server/
COPY packages/zk-mpc-node/Cargo.toml packages/zk-mpc-node/
COPY packages/mpc-circuits/Cargo.toml packages/mpc-circuits/
COPY packages/mpc-algebra-wasm/Cargo.toml packages/mpc-algebra-wasm/

# Create dummy source files to build dependencies
RUN mkdir -p packages/server/src packages/zk-mpc-node/src packages/mpc-circuits/src packages/mpc-algebra-wasm/src
RUN echo "fn main() {}" > packages/server/src/main.rs
RUN echo "fn main() {}" > packages/zk-mpc-node/src/main.rs
RUN echo "" > packages/zk-mpc-node/src/lib.rs
RUN echo "" > packages/mpc-circuits/src/lib.rs
RUN echo "" > packages/mpc-algebra-wasm/src/lib.rs

# Build dependencies only
RUN cargo build --release --package zk-mpc-node --locked

# Remove dummy files
RUN rm -rf packages/*/src

# Copy actual source code
COPY packages/ ./packages/

# Build the application
RUN cargo build --release --package zk-mpc-node --locked

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/workspace/target/release/zk-mpc-node /app/zk-mpc-node
COPY packages/zk-mpc-node/address /app/address
RUN chmod +x /app/zk-mpc-node

EXPOSE 9000 9001 9002

ENTRYPOINT ["/app/zk-mpc-node"]