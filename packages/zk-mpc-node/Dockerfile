FROM rust:1.89 AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/workspace

# Copy workspace Cargo.toml first
COPY Cargo.toml Cargo.lock ./

# Copy all source code
COPY packages/ ./packages/

# Build the application
WORKDIR /usr/src/workspace/packages/zk-mpc-node
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/workspace/packages/zk-mpc-node/target/release/zk-mpc-node /app/zk-mpc-node
COPY packages/zk-mpc-node/address /app/address
RUN chmod +x /app/zk-mpc-node

EXPOSE 9000 9001 9002

ENTRYPOINT ["/app/zk-mpc-node"]