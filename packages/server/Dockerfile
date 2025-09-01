FROM rust:1.82 as builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/workspace

# Copy Cargo.toml files first for better caching
COPY packages/server/Cargo.toml packages/server/
COPY packages/zk-mpc-node/Cargo.toml packages/zk-mpc-node/
COPY packages/mpc-circuits/Cargo.toml packages/mpc-circuits/
COPY packages/mpc-algebra-wasm/Cargo.toml packages/mpc-algebra-wasm/

# Create dummy source files to build dependencies
RUN mkdir -p packages/server/src packages/zk-mpc-node/src packages/mpc-circuits/src packages/mpc-algebra-wasm/src
RUN echo "fn main() {}" > packages/server/src/main.rs
RUN echo "fn main() {}" > packages/zk-mpc-node/src/main.rs
RUN echo "" > packages/mpc-circuits/src/lib.rs
RUN echo "" > packages/mpc-algebra-wasm/src/lib.rs

# Build dependencies only
RUN cargo build --release --package server

# Now copy the actual source code
COPY . .

# Build the application (this will be fast as dependencies are cached)
RUN cargo build --release --package server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/workspace/target/release/server /app/server

EXPOSE 8080

ENTRYPOINT ["./server"]