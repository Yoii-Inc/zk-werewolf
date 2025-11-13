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

# Build the application
RUN cargo build --release --package server --locked

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/workspace/target/release/server /app/server

EXPOSE 8080

ENTRYPOINT ["./server"]