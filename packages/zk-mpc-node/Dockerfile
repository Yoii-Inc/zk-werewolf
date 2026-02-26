# syntax=docker/dockerfile:1
FROM rust:1.88 AS builder

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
COPY packages/zk-mpc-node/data/groth16 /app/data/groth16
RUN chmod +x /app/zk-mpc-node

ENV ROLE_ASSIGNMENT_GROTH16_PK_PATH=/app/data/groth16/role_assignment_max5_v1.pk
ENV ANONYMOUS_VOTING_GROTH16_PK_PATH=/app/data/groth16/anonymous_voting_max5_v1.pk
ENV DIVINATION_GROTH16_PK_PATH=/app/data/groth16/divination_max5_v1.pk
ENV WINNING_JUDGEMENT_GROTH16_PK_PATH=/app/data/groth16/winning_judgement_max5_v1.pk
ENV KEY_PUBLICIZE_GROTH16_PK_PATH=/app/data/groth16/key_publicize_max5_v1.pk

EXPOSE 9000

ENTRYPOINT ["/app/zk-mpc-node"]
